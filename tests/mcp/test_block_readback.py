"""Installed stdio block payloads compared with the native block readers."""

import json
import subprocess

import docx
from docx.enum.table import WD_CELL_VERTICAL_ALIGNMENT
from docx.enum.text import WD_BREAK
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Inches, Pt
import pytest

from test_stdio import anyio_backend, call, executable, failure, launch

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
    tabs = word("tabs")
    tabs.append(word("tab", val="right", pos="480", leader="dot"))
    props.append(tabs)
    rich_paragraph(paragraph, "Body", "0017")
    lexical = document.add_paragraph()
    lexical_props = lexical._p.get_or_add_pPr()
    lexical_props.append(word("jc", val="start"))

    table = document.add_table(2, 2)
    table.autofit = False
    table._tbl.tblGrid.gridCol_lst[1].attrib.pop(qn("w:w"))
    lead = table.cell(0, 0)
    lead.text = "Lead"
    lead.vertical_alignment = WD_CELL_VERTICAL_ALIGNMENT.BOTTOM
    rich_paragraph(lead.paragraphs[0], " cell")
    lead.add_table(1, 1).cell(0, 0).text = "Native block remains opaque"
    lead.paragraphs[-1].text = "After nested table"
    lead.add_paragraph()
    merged = table.cell(1, 0).merge(table.cell(1, 1))
    merged._tc.get_or_add_tcPr().append(word("vMerge"))
    first_row = table._tbl.tr_lst[0]
    row_props = first_row.get_or_add_trPr()
    row_props.append(word("tblHeader", val="0"))
    row_props.append(word("cantSplit"))
    first_row.tc_lst[0].addprevious(word("bookmarkStart", id="20", name="row"))
    first_row.addprevious(word("bookmarkStart", id="21", name="table"))
    first_row.addnext(word("bookmarkEnd", id="21"))

    header = document.sections[0].header
    rich_paragraph(header.paragraphs[0], "Header")
    header.add_table(1, 1, Inches(1))
    header.add_paragraph()
    terminal = document.sections[-1]._sectPr
    opaque = word("customXml", element="unmodeled")
    opaque.append(word("p"))
    terminal.addprevious(opaque)
    first_section = word("sectPr")
    first_section.append(word("type", val="continuous"))
    first_section.append(word("titlePg", val="0"))
    borders = word("pgBorders")
    borders.append(word("top", val="single", sz="8", color="112233"))
    first_section.append(borders)
    props.append(first_section)
    lexical_props.append(word("sectPr"))
    terminal.append(word("type", val="unmodeled"))
    document.save(source)
    return source


def native_readback(source, *mode):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_block_readback_oracle")
    return json.loads(subprocess.check_output([oracle, source, *mode], timeout=20))


def text_projection(items):
    text = []
    opaque = False
    for item in items:
        if item["kind"] == "text":
            text.append(item["text"])
        elif item["kind"] == "tab":
            text.append("\t")
        elif item["kind"] == "break" and item["break_type"] in {"line", "text-wrapping"}:
            text.append("\n")
        else:
            opaque = True
    return dict(text="".join(text), has_opaque_content=opaque)


def assert_inline_node(actual, native):
    """Compare typed payload only; validate handle identity separately."""
    expected = dict(native)
    if expected["kind"] == "run":
        expected.update(text_projection(native["items"]))
    payload = {key: value for key, value in actual.items() if key != "inline_handle"}
    if "children" in native:
        for child, target in zip(payload.pop("children"), expected.pop("children"), strict=True):
            assert_inline_node(child, target)
    assert payload == expected
    assert ("inline_handle" in actual) == (native["kind"] != "opaque")


def compare_blocks(actual, native):
    assert len(actual) == len(native)
    for index, (block, expected) in enumerate(zip(actual, native, strict=True)):
        assert block["index"] == index
        if isinstance(expected, str):
            assert block == dict(type="opaque", index=index, name=expected)
        else:
            assert block["type"] == "paragraph"
            assert block["inline_items"] == expected
            assert {key: block[key] for key in text_projection(expected)} == text_projection(expected)


def compare_table(actual, native):
    assert actual["layout"] == native["layout"]
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
    native = native_readback(block_source)
    async with launch() as client:
        opened = await call(client, "open_document", path=block_source.name)
        document = opened["document"]
        body = (await inspect(client, document, "body"))["body"]
        assert (await call(client, "inspect_document", document=document))["body"] == body
        paragraph, unknown, table, opaque = body
        assert table["type"] == "table"
        assert opaque == dict(type="opaque", index=3, name=native["opaque_body_block"])
        assert opaque["name"] == "w:customXml"
        for index, (row, expected) in enumerate(zip(body[:2], native["paragraphs"], strict=True)):
            assert {key: row[key] for key in expected} == expected
            assert row["anchor"] == f"{document}/paragraph:{index}"
        assert paragraph["alignment"] == "justify-medium"
        assert paragraph["flags"] == {
            "keepWithNext": True, "keepTogether": False,
        }
        assert paragraph["layout"]["spaceBefore"] == 120
        assert paragraph["layout"]["tabStops"] == [dict(position=480, alignment="right", leader="dot")]
        assert paragraph["mark_revision"] == dict(kind="insertion", id="0009", author="Reviewer", date=None)
        assert dict(kind="note-reference", note_kind="footnote", id="0017") in paragraph["inline_items"]
        assert paragraph["inline_items"][-1] == dict(kind="opaque", name="w:ins")
        assert paragraph["text"] == "Body\t\n\nLink 😀" and paragraph["has_opaque_content"]
        assert unknown["alignment"] is None and unknown["alignment_xml"] == "start"
        assert unknown["flags"] == {} and unknown["inline_items"] == [] and unknown["mark_revision"] is None

        first = await inspect(client, document, "sections", limit=1)
        rest = await inspect(client, document, "sections", cursor=first["next_cursor"])
        sections = (await inspect(client, document, "sections"))["sections"]
        assert first["sections"] + rest["sections"] == sections
        for index, (section, expected) in enumerate(zip(sections, native["sections"], strict=True)):
            assert {key: section[key] for key in expected} == expected
            assert section["anchor"] == f"{document}/section:{index}"
        assert [(row["body_start"], row["body_end"]) for row in sections] == [(0, 1), (1, 2), (2, 4)]
        assert sections[0]["section_type"] == "continuous"
        assert sections[0]["flags"] == {"titlePage": False}
        assert sections[0]["layout"]["borderTop"] == dict(size=8, space=None, color="112233")
        assert sections[1]["section_type"] is None and sections[1]["section_type_xml"] is None
        assert sections[2]["section_type"] is None and sections[2]["section_type_xml"] == "unmodeled"
        assert sections[2]["first_paragraph"] is None  # Range starts with a table.


async def test_table_and_story_block_projection(launch, block_source):
    native = native_readback(block_source)
    original = block_source.read_bytes()
    independent = docx.Document(block_source)
    assert independent.tables[0].cell(0, 0).tables[0].cell(0, 0).text == "Native block remains opaque"

    async with launch() as client:
        opened = await call(client, "open_document", path=block_source.name)
        document = opened["document"]
        compact_table = (await inspect(client, document, "body"))["body"][2]
        (table,) = (await inspect(client, document, "body-tables"))["body_tables"]
        compare_table(table, native["table"])
        assert (await inspect(client, document, "body-tables", limit=1))["body_tables"] == [table]
        assert table["anchor"] == f"{document}/body-table:2"
        assert table["grid_widths"][1] is None
        assert compact_table["cells"][0]["anchor"].endswith("/cell:2:1:1")
        assert (compact_table["cells"][0]["row_index"], compact_table["cells"][0]["cell_index"]) == (1, 1)
        row = table["children"][1]
        assert row["properties"] == {"repeatHeader": {"val": "0"}, "cantSplit": {}}
        lead, vacant = row["children"][1:]
        assert [block["type"] for block in lead["blocks"]] == ["paragraph", "opaque", "paragraph", "paragraph"]
        assert (lead["grid_span"], lead["vertical_merge"]) == (None, None)
        assert vacant["properties"] == {}
        merged = table["children"][-1]["children"][0]
        assert (merged["grid_span"], merged["vertical_merge"]) == (2, "continue")
        assert merged["properties"]["verticalMerge"] == {}  # Raw attribute map stays empty.
        assert compact_table["has_opaque_content"]

        selector = dict(kind="table-cell-inlines", anchor=table["anchor"], row_index=0, cell_index=0)
        lead_inlines = native["table"]["children"][1]["cells"][1]["inlines"]
        for index, expected in enumerate(lead_inlines):
            view = dict(selector, block_index=index)
            if expected is None:
                await failure(client, "inspect_document", "inspection_failed", document=document, view=view)
                continue
            rows = (await call(client, "inspect_document", document=document, view=view))["inlines"]
            assert len(rows) == len(expected)
            for inline_index, (node, want) in enumerate(zip(rows, expected, strict=True)):
                assert node["inline_index"] == inline_index
                assert_inline_node({key: value for key, value in node.items() if key != "inline_index"}, want)
            assert (await call(client, "inspect_document", document=document, view=view))["inlines"] == rows
        assert lead_inlines[1] is None and lead_inlines[3] == [] and lead_inlines[4] is None

        header = next(story for story in (await inspect(client, document, "stories"))["stories"]
                      if story["kind"] == "header" and story["variant"] == "default" and story["owner"])
        compare_blocks(header["blocks"], native["story"])
        assert header["blocks"][0]["inlines"]  # Preserve compact structured readback.
    assert block_source.read_bytes() == original
