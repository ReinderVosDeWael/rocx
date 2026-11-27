"""Installed stdio block payloads compared with the native block readers."""

import json
import re
import subprocess

import docx
from docx.enum.table import WD_CELL_VERTICAL_ALIGNMENT
from docx.enum.text import WD_BREAK
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Inches, Pt
import pytest

from test_stdio import anyio_backend, call, executable, launch

pytestmark = pytest.mark.anyio


def word(tag, **attributes):
    element = OxmlElement(f"w:{tag}")
    for key, value in attributes.items():
        element.set(qn(f"w:{key}"), value)
    return element


def rich_paragraph(paragraph, text, note=None):
    run = paragraph.add_run(text)
    run.add_tab()
    run._r.append(word("cr"))
    run.add_break()
    run.add_break(WD_BREAK.PAGE)
    run.add_break(WD_BREAK.COLUMN)
    if note is not None:
        run._r.append(word("footnoteReference", id=note))
    link = word("hyperlink", anchor="destination")
    link.append(paragraph.add_run("Link 😀")._r)
    paragraph._p.append(link)
    revision = word("ins", id="07", author="Reviewer")
    revision.append(paragraph.add_run("Not part of flat text")._r)
    paragraph._p.append(revision)


@pytest.fixture
def block_source(tmp_path):
    source = tmp_path / "block-readback.docx"
    document = docx.Document()
    paragraph = document.add_paragraph()
    paragraph.paragraph_format.keep_with_next = True
    paragraph.paragraph_format.keep_together = False
    paragraph.paragraph_format.space_before = Pt(6)
    props = paragraph._p.get_or_add_pPr()
    props.append(word("jc", val="mediumKashida"))
    props.append(word("widowControl", val="ambiguous"))
    mark = word("rPr")
    mark.append(word("ins", id="0009", author="Reviewer"))
    props.append(mark)
    rich_paragraph(paragraph, "Body", "0017")
    document.add_paragraph()._p.get_or_add_pPr().append(word("jc", val="start"))

    table = document.add_table(2, 2)
    table.autofit = False
    table._tbl.tblGrid.gridCol_lst[1].attrib.pop(qn("w:w"))
    lead = table.cell(0, 0)
    lead.text = "Lead"
    lead.vertical_alignment = WD_CELL_VERTICAL_ALIGNMENT.BOTTOM
    rich_paragraph(lead.paragraphs[0], " cell")
    lead.add_table(1, 1).cell(0, 0).text = "Native block remains opaque"
    lead.paragraphs[-1].text = "After nested table"
    merged = table.cell(1, 0).merge(table.cell(1, 1))
    merged._tc.get_or_add_tcPr().append(word("vMerge"))
    first_row = table._tbl.tr_lst[0]
    first_row.tc_lst[0].addprevious(word("bookmarkStart", id="20", name="row"))
    first_row.addprevious(word("bookmarkStart", id="21", name="table"))
    first_row.addnext(word("bookmarkEnd", id="21"))

    header = document.sections[0].header
    rich_paragraph(header.paragraphs[0], "Header")
    header.add_table(1, 1, Inches(1))
    header.add_paragraph()
    document.save(source)
    return source


def native_blocks(source):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_block_readback_oracle")
    return json.loads(subprocess.check_output([oracle, source], timeout=20))


def kebab(variant):
    return re.sub(r"(?<!^)(?=[A-Z])", "-", variant).lower()


def inline_tuples(items):
    return [
        [item["kind"]] + [
            item[key] for key in ("note_kind", "id", "break_type", "text", "name") if key in item
        ]
        for item in items
    ]


def native_inline_tuples(items):
    return [
        [kind, kebab(values[0]), *values[1:]] if kind in {"break", "note-reference"}
        else [kind, *values]
        for kind, *values in items
    ]


def compare_blocks(actual, native):
    assert len(actual) == len(native)
    for index, (block, expected) in enumerate(zip(actual, native, strict=True)):
        assert block["index"] == index
        if isinstance(expected, str):
            assert block == dict(type="opaque", index=index, name=expected)
        else:
            assert block["type"] == "paragraph"
            assert inline_tuples(block["inline_items"]) == native_inline_tuples(expected)


def compare_table(actual, native):
    assert actual["layout"] == kebab(native["layout"])
    assert actual["grid_widths"] == native["grid_widths"]
    assert actual["properties"] == native["properties"]
    assert len(actual["children"]) == len(native["children"])
    rows = []
    for index, (row, expected) in enumerate(zip(actual["children"], native["children"], strict=True)):
        if isinstance(expected, str):
            assert row == dict(type="opaque", index=index, name=expected)
            continue
        assert (row["type"], row["index"], row["row_index"]) == ("row", index, len(rows))
        assert row["properties"] == expected["properties"]
        assert len(row["children"]) == len(expected["cells"])
        cells = []
        for position, (cell, expected_cell) in enumerate(zip(row["children"], expected["cells"],
                                                             strict=True)):
            if isinstance(expected_cell, str):
                assert cell == dict(type="opaque", index=position, name=expected_cell)
                continue
            assert (cell["type"], cell["index"], cell["cell_index"]) == ("cell", position, len(cells))
            for key in ("properties", "grid_span", "vertical_merge"):
                assert cell[key] == expected_cell[key]
            compare_blocks(cell["blocks"], expected_cell["blocks"])
            cells.append(cell["properties"])
        rows.append(dict(properties=row["properties"], cells=cells))
    assert actual["rows"] == rows  # Existing structural rows/cells omit opaque siblings.


async def inspect(client, document, kind, **kwargs):
    return await call(client, "inspect_document", document=document, view=dict(kind=kind, **kwargs))


async def test_body_paragraph_projection(launch, block_source):
    native = native_blocks(block_source)
    async with launch() as client:
        opened = await call(client, "open_document", path=block_source.name)
        document = opened["document"]
        body = (await inspect(client, document, "body"))["body"]
        assert (await call(client, "inspect_document", document=document))["body"] == body
        paragraph, unknown, _ = body
        for key in ("text", "has_opaque_content", "alignment_xml"):
            assert paragraph[key] == native[key]
        assert paragraph["alignment"] == kebab(native["alignment"]) == "justify-medium"
        assert paragraph["flags"] == dict(native["flags"]) == {
            "keepWithNext": True, "keepTogether": False,
        }
        assert paragraph["layout"]["spaceBefore"] == native["space_before"] == 120
        assert [paragraph["mark_revision"][key] for key in ("kind", "id", "author", "date")] == (
            native["mark_revision"]
        )
        assert inline_tuples(paragraph["inline_items"]) == native_inline_tuples(native["inline_items"])
        assert ["note-reference", "footnote", "0017"] in inline_tuples(paragraph["inline_items"])
        assert paragraph["inline_items"][-1] == dict(kind="opaque", name="w:ins")
        assert unknown["alignment"] is None and unknown["alignment_xml"] == native["lexical_alignment_xml"]
        assert unknown["flags"] == {} and unknown["inline_items"] == []


async def test_table_and_story_block_projection(launch, block_source):
    native = native_blocks(block_source)
    original = block_source.read_bytes()
    independent = docx.Document(block_source)
    assert independent.tables[0].cell(0, 0).tables[0].cell(0, 0).text == "Native block remains opaque"

    async with launch() as client:
        opened = await call(client, "open_document", path=block_source.name)
        document = opened["document"]
        compact_table = (await inspect(client, document, "body"))["body"][2]
        (table,) = (await inspect(client, document, "body-tables"))["body_tables"]
        compare_table(table, native["table"])
        assert table["grid_widths"][1] is None
        assert compact_table["cells"][0]["anchor"].endswith("/cell:2:1:1")
        assert (compact_table["cells"][0]["row_index"], compact_table["cells"][0]["cell_index"]) == (1, 1)
        lead = table["children"][1]["children"][1]
        assert [block["type"] for block in lead["blocks"]] == ["paragraph", "opaque", "paragraph"]
        lead_run, = (await inspect(client, document, "table-cell-inlines", anchor=table["anchor"],
                                   row_index=0, cell_index=0, block_index=2))["inlines"]
        assert lead_run["text"] == "After nested table"

        header = next(story for story in (await inspect(client, document, "stories"))["stories"]
                      if story["kind"] == "header" and story["variant"] == "default")
        compare_blocks(header["blocks"], native["story"])
        assert header["blocks"][0]["inlines"]  # Preserve compact structured readback.
    assert block_source.read_bytes() == original
