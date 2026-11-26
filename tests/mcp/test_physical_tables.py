"""Installed SDK checks for snapshot-anchored physical table transactions."""

from copy import deepcopy
import json
from pathlib import PurePosixPath
import subprocess
from xml.etree import ElementTree

import docx
from docx.enum.table import WD_CELL_VERTICAL_ALIGNMENT
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.opc.packuri import PackURI
from docx.oxml import OxmlElement, parse_xml
from docx.oxml.ns import qn
from docx.shared import Inches
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_stdio import (
    anyio_backend, call, executable, failure, launch, make_entry_source, parts, validate,
)

pytestmark = pytest.mark.anyio
MAIN = "word/relocated-main.xml"
HEADER = "word/stories/relocated-header.xml"
LINK = "https://example.org/physical-table"
RICH = [
    dict(kind="paragraph", paragraph=dict(inlines=[
        dict(kind="text", text="Rich 😀", format=dict(bold=True)),
        dict(kind="hyperlink", target=dict(kind="url", url=LINK),
             children=[dict(kind="text", text=" link")]),
    ])),
    dict(kind="table", rows=[["nested payload"]], column_widths=[1440]),
]
DONOR = dict(rows=[dict(cells=[dict(blocks=[
    dict(kind="paragraph", paragraph=dict(inlines=[dict(kind="text", text="not copied")])),
])])], properties=dict(column_widths=[2100], layout="fixed"))


@pytest.fixture
def table_source(tmp_path):
    path = tmp_path / "source.docx"
    make_entry_source(path)
    document = docx.Document(path)
    document.part.partname = PackURI(f"/{MAIN}")
    outer = document.add_table(1, 1)
    nested = outer.cell(0, 0).add_table(1, 1)
    nested.cell(0, 0).text = "nested body"
    plain = document.add_table(2, 2)
    for index, cell in enumerate(cell for row in plain.rows for cell in row.cells):
        cell.text = f"plain {index}"
    plain.autofit = True
    plain.cell(0, 0).vertical_alignment = WD_CELL_VERTICAL_ALIGNMENT.CENTER
    flag = OxmlElement("w:cantSplit")
    flag.set(qn("w:val"), "0")
    plain.rows[0]._tr.get_or_add_trPr().append(flag)
    merged = document.add_table(2, 2)
    merged.cell(0, 0).merge(merged.cell(0, 1)).text = "merged interval"

    section = document.sections[0]
    header = section.header
    header.add_table(1, 1, Inches(1)).cell(0, 0).text = "header"
    header.part.partname = PackURI(f"/{HEADER}")
    section.footer.add_table(1, 1, Inches(1)).cell(0, 0).text = "footer"
    document.add_section()
    for reltype, name in [
        (RELATIONSHIP_TYPE.FOOTNOTES, "footnote"),
        (RELATIONSHIP_TYPE.ENDNOTES, "endnote"),
    ]:
        part = document.part.part_related_by(reltype)
        root = parse_xml(part.blob)
        for entry in root:
            # Reserved entries can contain tables but are not candidates.
            if entry.get(qn("w:id")) in ("007", "-1"):
                entry.append(deepcopy(nested._tbl))
                entry[-1].find(f".//{qn('w:t')}").text = name
        part._blob = etree.tostring(root, encoding="UTF-8", xml_declaration=True)
    document.save(path)
    return path


async def page(client, document, **options):
    return await call(client, "inspect_document", document=document,
                      view=dict(kind="physical-tables", **options))


async def open_document(client, source):
    return (await call(client, "open_document", path=source.name))["document"]


def edit(table, kind, **payload):
    return dict(anchor=table["anchor"], edit=dict(kind=kind, **payload))


def operation(owner, *edits):
    return dict(kind="edit-part-tables", anchor=owner["anchor"], edits=list(edits))


async def save(client, document, output, request):
    preview = await call(client, "preview_edits", document=document, operation=request)
    assert preview["changes"] is None
    saved = await call(client, "save_copy", document=document, preview=preview["preview"],
                       output=output.name)
    assert saved["output_snapshot"] == preview["output_snapshot"]


def native_oracle(source, output, case, owner):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_table_oracle")
    subprocess.run([oracle, source, output, case, owner],
                   check=True, cwd=source.parent, timeout=20)
    return output.read_bytes()


def compare_native(source, output, case, owner):
    assert output.read_bytes() == native_oracle(
        source, output.with_name("native.docx"), case, owner,
    )


async def test_physical_table_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
    inspect = validator_for(schemas["inspect_document"])(schemas["inspect_document"])
    preview = validator_for(schemas["preview_edits"])(schemas["preview_edits"])
    assert inspect.is_valid(dict(document="doc", view=dict(kind="physical-tables", limit=1)))
    assert not inspect.is_valid(dict(document="doc", view=dict(kind="physical-tables", limit=0)))

    def accepted(kind, **payload):
        change = edit(dict(anchor="table"), kind, **payload)
        return preview.is_valid(dict(document="doc",
                                     operation=operation(dict(anchor="owner"), change)))

    assert accepted("layout", value=None)
    assert accepted("insert-row", row=0)
    assert accepted("replace-cell", row=0, cell=0, blocks=RICH)
    assert accepted("clear-property", site=dict(kind="column", column=0), field="width")
    assert accepted("set-property", site=dict(kind="table"), field="layout", source=DONOR)
    assert not accepted("layout")
    assert not accepted("layout", value="invalid")
    assert not accepted("insert-row", row=-1)
    assert not accepted("insert-cell", row=0, cell=0, extra=True)
    assert not accepted("set-property", site=dict(kind="table"), field="layout", source=None)
    assert not accepted("replace-node", source=dict(rows=[["unsupported in scope"]]))
    assert not accepted("replace-node", site=dict(kind="table"), source=DONOR)
    assert not accepted("remove-table")

    def ordinary(kind, **payload):
        return preview.is_valid(dict(document="doc", edits=[
            body_edit(dict(anchor="table"), kind, **payload),
        ]))

    assert inspect.is_valid(dict(document="doc", view=dict(kind="body-tables", limit=1)))
    assert ordinary("remove-table")
    assert ordinary("replace-node", site=dict(kind="table"), source=DONOR)
    assert ordinary("insert-column", column=0)
    assert not ordinary("remove-table", cell=0)
    assert not ordinary("replace-node", site=dict(kind="column", column=0), source=DONOR)


async def test_physical_table_pagination_and_rejected_batches(launch, table_source):
    async with launch() as client:
        document = await open_document(client, table_source)
        first = await page(client, document, limit=1)
        foreign = await open_document(client, table_source)
        await failure(client, "inspect_document", "wrong_owner", document=foreign,
                      view=dict(kind="physical-tables", cursor=first["next_cursor"]))
        await failure(client, "inspect_document", "invalid_cursor", document=document,
                      view=dict(kind="entries", cursor=first["next_cursor"]))
        (outer,) = first["physical_tables"]
        rest = await page(client, document, cursor=first["next_cursor"])
        rows = [outer, *rest["physical_tables"]]
        assert rest["next_cursor"] is None
        assert rows == (await page(client, document))["physical_tables"]
        assert len({row["anchor"] for row in rows}) == len(rows) == 9

        body = [row for row in rows if row["owner"]["kind"] == "body"]
        _, nested, plain, merged = body
        assert all(row["owner"] == outer["owner"] for row in body)
        assert outer["owner"]["part_name"] == MAIN
        assert nested["depth"] == 1 and outer["depth"] == 0
        assert nested["path"][:-3] == outer["path"]
        assert plain["properties"]["layout"] == {"type": "autofit"}
        assert plain["row_properties"] == [{"cantSplit": {"val": "0"}}, {}]
        assert plain["cell_properties"][0][0] == {"verticalAlignment": {"val": "center"}}
        assert len(plain["grid_widths"]) == 2 and all(plain["grid_widths"])
        assert merged["cell_properties"][0][0]["gridSpan"] == {"val": "2"}

        (header,) = [row for row in rows if row["owner"]["kind"] == "header"]
        assert header["owner"]["part_name"] == HEADER
        assert header["owner"]["slots"] == [
            dict(kind="header", variant="default", section_index=0, inherited=False),
            dict(kind="header", variant="default", section_index=1, inherited=True),
        ]
        assert sorted(row["entry_id"] for row in rows if row["entry_id"] is not None) == [
            "0", "007", "007",
        ]
        for part in {row["owner"]["part_name"] for row in rows}:
            projected = [
                dict(part=part, **{key: value for key, value in row.items()
                                   if key not in {"anchor", "owner"}})
                for row in rows if row["owner"]["part_name"] == part
            ]
            native = native_oracle(table_source, table_source.with_name("native.json"), "inspect",
                                   "body" if part == MAIN else part)
            assert projected == json.loads(native)

        owner = outer["owner"]
        layout = edit(plain, "layout", value="fixed")
        foreign_table = (await page(client, foreign, limit=1))["physical_tables"][0]
        for category, request in [
            ("wrong_owner", operation(owner, edit(foreign_table, "layout", value=None))),
            ("invalid_anchor", operation(owner, edit(
                dict(anchor=f"{document}/physical-table:0:999"), "layout", value=None,
            ))),
            ("wrong_anchor_type", operation(outer, layout)),
            ("wrong_anchor_type", operation(owner, edit(owner, "layout", value=None))),
            ("rejected_batch", operation(owner, layout, edit(header, "layout", value=None))),
            ("rejected_batch", operation(owner, layout, layout)),
            ("rejected_batch", operation(owner, edit(outer, "layout", value="fixed"),
                                         edit(nested, "layout", value="fixed"))),
            ("rejected_batch", operation(owner, edit(merged, "remove-column", column=0))),
            ("rejected_batch", operation(owner, edit(outer, "replace-cell",
                                                      row=0, cell=0, blocks=[]))),
        ]:
            await failure(client, "preview_edits", category, document=document, operation=request)
        await call(client, "preview_edits", document=document, operation=operation(owner, layout))


@pytest.mark.parametrize(("case", "kind", "payload", "shape"), [
    ("insert-row", "insert-row", dict(row=2), (2, [2, 2, 2])),
    ("remove-row", "remove-row", dict(row=1), (2, [2])),
    ("insert-cell", "insert-cell", dict(row=0, cell=2), (3, [3, 2])),
    ("remove-cell", "remove-cell", dict(row=0, cell=1), (2, [1, 2])),
    ("insert-column", "insert-column", dict(column=1), (3, [3, 3])),
    ("remove-column", "remove-column", dict(column=1), (1, [1, 1])),
    ("fixed", "layout", dict(value="fixed"), (2, [2, 2])),
    ("unchanged-layout", "layout", dict(value="autofit"), (2, [2, 2])),
    ("clear-layout", "layout", dict(value=None), (2, [2, 2])),
    ("clear-properties", None, {}, (2, [2, 2])),
    ("set-properties", None, {}, (2, [2, 2])),
])
async def test_physical_geometry_and_layout_native_bytes(
    launch, table_source, tmp_path, case, kind, payload, shape,
):
    output = tmp_path / "edited.docx"
    original = table_source.read_bytes()
    async with launch() as client:
        document = await open_document(client, table_source)
        table = (await page(client, document))["physical_tables"][2]
        if case == "clear-properties":
            edits = [
                edit(table, "clear-property", site=site, field=field)
                for site, field in [
                    (dict(kind="table"), "layout"), (dict(kind="row", row=0), "cantSplit"),
                    (dict(kind="cell", row=0, cell=0), "verticalAlignment"),
                    (dict(kind="column", column=0), "width"),
                ]
            ]
        elif case == "set-properties":
            edits = [
                edit(table, "set-property", site=site, field=field, source=DONOR)
                for site, field in [(dict(kind="table"), "layout"),
                                    (dict(kind="column", column=0), "width")]
            ]
        else:
            edits = [edit(table, kind, **payload)]
        await save(client, document, output, operation(table["owner"], *edits))
        reopened = await open_document(client, output)
        result = (await page(client, reopened))["physical_tables"][2]
        assert (len(result["grid_widths"]), list(map(len, result["cell_properties"]))) == shape
        if case in ("fixed", "set-properties"):
            assert result["properties"]["layout"] == {"type": "fixed"}
        if case == "set-properties":
            assert result["grid_widths"][0] == 2100
        if case.startswith("clear-"):
            assert "layout" not in result["properties"]
        if case == "clear-properties":
            assert result["grid_widths"][0] is None
            assert result["row_properties"][0] == {}
            assert result["cell_properties"][0][0] == {}
    compare_native(table_source, output, case, "body")
    assert table_source.read_bytes() == original
    before, after = parts(table_source), parts(output)
    assert before.keys() == after.keys()
    changed = set() if case == "unchanged-layout" else {MAIN}
    assert {part for part in before if before[part] != after[part]} == changed
    independent = docx.Document(output)
    assert independent.paragraphs[0].text == "Body unchanged"
    assert independent.tables[1].cell(0, 0).text == "plain 0"
    validate(output)


@pytest.mark.parametrize("kind", ["body", "header", "footer", "footnote", "endnote", "comment"])
async def test_rich_physical_table_content_native_bytes(launch, table_source, tmp_path, kind):
    output = tmp_path / "edited.docx"
    async with launch() as client:
        document = await open_document(client, table_source)
        rows = (await page(client, document))["physical_tables"]
        (table,) = [row for row in rows if row["owner"]["kind"] == kind
                    and (kind != "body" or row["depth"] == 1)]
        await save(client, document, output, operation(
            table["owner"], edit(table, "replace-cell", row=0, cell=0, blocks=RICH),
        ))
        reopened = await open_document(client, output)
        rows = (await page(client, reopened))["physical_tables"]
        (nested,) = [row for row in rows if row["owner"]["kind"] == kind
                     and row["path"][:-3] == table["path"]]
        assert nested["depth"] == table["depth"] + 1
        assert nested["entry_id"] == table["entry_id"]
        assert nested["grid_widths"] == [1440]

    part = table["owner"]["part_name"]
    compare_native(table_source, output, "rich", "body" if kind == "body" else part)
    before, after = parts(table_source), parts(output)
    path = PurePosixPath(part)
    relationships = str(path.parent / "_rels" / f"{path.name}.rels")
    assert not before.keys() - after.keys()
    assert {key for key in after if before.get(key) != after[key]} == {part, relationships}
    assert LINK.encode() in after[relationships]
    node = ElementTree.fromstring(after[part])
    for index in table["path"]:
        node = node[index]
    cell = node.find(f"./{qn('w:tr')}/{qn('w:tc')}")
    assert [text.text for text in cell.iter(qn("w:t"))] == [
        "Rich 😀", " link", "nested payload",
    ]
    assert cell[-1].tag == qn("w:p")
    assert docx.Document(output).paragraphs[0].text == "Body unchanged"
    validate(output)


def body_edit(table, kind, **payload):
    return dict(kind="table-structure", **edit(table, kind, **payload))


def body_donor(*texts):
    return dict(
        rows=[dict(properties=dict(repeat_header=False), cells=[
            dict(properties=dict(vertical_alignment="bottom"), blocks=[
                dict(kind="paragraph", paragraph=dict(inlines=[
                    dict(kind="text", text=text, format=dict(bold=True)),
                ])),
            ])
            for text in texts
        ])],
        properties=dict(layout="fixed", column_widths=[1800] * len(texts)),
    )


async def test_ordinary_body_table_mixed_whole_nodes_native_bytes(launch, table_source, tmp_path):
    source, output = tmp_path / "body.docx", tmp_path / "edited.docx"
    fixture = docx.Document(table_source)
    for name in ("whole", "row", "cell", "geometry"):
        table = fixture.add_table(2, 2)
        for row_index, row in enumerate(table.rows):
            for cell_index, cell in enumerate(row.cells):
                cell.text = f"{name} {row_index},{cell_index}"
    fixture.add_paragraph("Original tail")
    fixture.save(source)
    original = source.read_bytes()

    async with launch() as client:
        document = await open_document(client, source)
        first = await call(client, "inspect_document", document=document,
                           view=dict(kind="body-tables", limit=1))
        second = await call(client, "inspect_document", document=document,
                            view=dict(kind="body-tables", cursor=first["next_cursor"]))
        tables = first["body_tables"] + second["body_tables"]
        assert second["next_cursor"] is None
        outer, remove, merged, whole, row, cell, geometry = tables
        assert [item["body_index"] for item in tables] == sorted(
            {item["body_index"] for item in tables},
        )
        seen = await call(client, "inspect_document", document=document)
        tail = seen["body"][-1]["runs"][0]["anchor"]
        (physical_outer,) = (await page(client, document, limit=1))["physical_tables"]
        await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                      edits=[body_edit(physical_outer, "remove-table")])
        await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                      operation=operation(physical_outer["owner"],
                                          edit(outer, "insert-row", row=1)))
        await failure(client, "preview_edits", "rejected_batch", document=document,
                      edits=[body_edit(outer, "remove-table")])  # Contains a nested table.
        await failure(client, "preview_edits", "rejected_batch", document=document,
                      edits=[body_edit(row, "replace-node", site=dict(kind="row", row=1),
                                       source=body_donor("wrong grid"))])

        edits = [
            body_edit(remove, "remove-table"),
            body_edit(whole, "replace-node", site=dict(kind="table"),
                      source=body_donor("new table")),
            body_edit(row, "replace-node", site=dict(kind="row", row=1),
                      source=body_donor("new row left", "new row right")),
            body_edit(cell, "replace-node", site=dict(kind="cell", row=0, cell=1),
                      source=body_donor("new cell")),
            body_edit(geometry, "insert-column", column=1),
            dict(kind="run-text", anchor=tail, text="Edited tail 😀"),
        ]
        preview = await call(client, "preview_edits", document=document, edits=edits)
        assert preview["changes"]["run_text"][0]["after"] == "Edited tail 😀"
        saved = await call(client, "save_copy", document=document, preview=preview["preview"],
                           output=output.name)
        assert saved["output_snapshot"] == preview["output_snapshot"]
        reopened = await open_document(client, output)
        await failure(client, "preview_edits", "wrong_owner", document=reopened, edits=edits)
        readback = await call(client, "inspect_document", document=reopened,
                              view=dict(kind="body-tables"))
        updated = readback["body_tables"]
        assert len(updated) == len(tables) - 1
        assert updated[1]["grid_widths"] == merged["grid_widths"]
        assert updated[2]["grid_widths"] == [1800]
        assert updated[2]["properties"]["layout"] == {"type": "fixed"}
        assert len(updated[-1]["grid_widths"]) == 3

    compare_native(source, output, "body-mixed", "body")
    assert source.read_bytes() == original
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {key for key in before if before[key] != after[key]} == {MAIN}
    independent = docx.Document(output)
    _, _, whole_result, row_result, cell_result, geometry_result = independent.tables

    def assert_donor(actual, text):
        assert actual.text == text
        assert actual.paragraphs[0].runs[0].bold is True
        assert actual.vertical_alignment == WD_CELL_VERTICAL_ALIGNMENT.BOTTOM

    assert_donor(whole_result.cell(0, 0), "new table")
    assert_donor(row_result.cell(1, 0), "new row left")
    assert_donor(row_result.cell(1, 1), "new row right")
    assert row_result.cell(0, 1).text == "row 0,1"
    assert_donor(cell_result.cell(0, 1), "new cell")
    assert cell_result.cell(0, 0).text == "cell 0,0"
    assert [cell.text for cell in geometry_result.rows[0].cells] == [
        "geometry 0,0", "", "geometry 0,1",
    ]
    assert independent.paragraphs[-1].text == "Edited tail 😀"
    validate(output)
