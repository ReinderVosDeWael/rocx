"""Direct body paragraph and section layout through the installed stdio binary."""

import subprocess
from xml.etree import ElementTree

import docx
from docx.enum.text import WD_ALIGN_PARAGRAPH, WD_TAB_ALIGNMENT, WD_TAB_LEADER
from docx.oxml.ns import qn
from docx.shared import Twips
from jsonschema.validators import validator_for
import pytest

from test_stdio import anyio_backend, call, digest, executable, failure, launch, parts, validate

pytestmark = pytest.mark.anyio

TAB = dict(position=720, alignment="right", leader="dot")
BORDER = dict(size=8, space=4, color="336699")


def make_source(path):
    document = docx.Document()
    document.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    document.add_table(1, 1).cell(0, 0).text = "inside the table"
    paragraph = document.add_paragraph("body text")
    paragraph.paragraph_format.widow_control = True
    paragraph.paragraph_format.space_after = Twips(180)
    document.sections[0].header.paragraphs[0].text = "unchanged header"
    document.sections[0].different_first_page_header_footer = True
    document.add_section()
    document.save(path)
    return path.read_bytes()


async def page(client, document, **view):
    return await call(client, "inspect_document", document=document, view=dict(kind="layout", **view))


def operation(*edits):
    return dict(kind="edit-layout", edits=list(edits))


def edit(anchor, field, value):
    return dict(anchor=anchor, field=field, value=value)


def native_bytes(source, output, *args):
    oracle = executable("ROCX_MCP_ORACLE").with_name("mcp_layout_oracle")
    subprocess.run([oracle, source, output, *args], check=True, cwd=source.parent, timeout=20)
    return output.read_bytes()


async def test_layout_discovery(launch):
    async with launch() as client:
        schemas = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
        inspect = validator_for(schemas["inspect_document"])(schemas["inspect_document"])
        preview = validator_for(schemas["preview_edits"])(schemas["preview_edits"])
        assert inspect.is_valid(dict(document="doc", view=dict(kind="layout", limit=1)))
        assert not inspect.is_valid(dict(document="doc", view=dict(kind="layout", limit=0)))

        def request(*edits):
            return dict(document="doc", operation=operation(*edits))

        assert preview.is_valid(request())
        for field, value in [
            ("firstLineIndent", -240),
            ("alignment", "center"),
            ("keepWithNext", False),
            ("titlePage", None),
            ("tabStops", [TAB]),
            ("borderBottom", BORDER),
        ]:
            assert preview.is_valid(request(edit("target", field, value)))
        for invalid in [
            dict(anchor="target", field="alignment"),
            edit("target", "first_line_indent", -240),
            edit("target", "unknown", None),
            edit("target", "marginTop", 2**31),
            edit("target", "marginTop", 1.5),
            edit("target", "tabStops", [dict(alignment="right")]),
            edit("target", "tabStops", [dict(TAB, extra=True)]),
            edit("target", "tabStops", [dict(TAB, position=-(2**31) - 1)]),
            edit("target", "borderBottom", dict(BORDER, type="double")),
            edit("target", "borderBottom", dict(BORDER, size=2**31)),
        ]:
            assert not preview.is_valid(request(invalid)), invalid


async def test_layout_pages_mixed_transaction_and_native_bytes(launch, tmp_path):
    source, output, native = [tmp_path / name for name in ("source.docx", "copy.docx", "native.docx")]
    original = make_source(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        first = await page(client, document, limit=1)
        second = await page(client, document, cursor=first["next_cursor"], limit=2)
        last = await page(client, document, cursor=second["next_cursor"], limit=1)
        assert last["next_cursor"] is None
        inspection = await page(client, document)
        rows = first["layout"] + second["layout"] + last["layout"]
        assert inspection["layout"] == rows
        assert [(row["kind"], row["index"]) for row in rows] == [
            ("paragraph", 1), ("paragraph", 2), ("section", 0), ("section", 1),
        ]
        first_paragraph, empty_paragraph, first_section, final_section = rows
        assert first_paragraph["layout"] == dict(widowControl=True, spaceAfter=180)
        assert empty_paragraph["layout"] == {}
        assert first_section["layout"]["titlePage"] is True
        assert "borderBottom" not in final_section["layout"]
        p, empty, initial, final = [row["anchor"] for row in rows]
        assert len({p, empty, initial, final}) == 4

        change = operation(
            edit(p, "alignment", "center"),
            edit(p, "firstLineIndent", -240),
            edit(p, "keepWithNext", False),
            edit(p, "widowControl", None),
            edit(p, "tabStops", [TAB]),
            edit(initial, "titlePage", None),
            edit(final, "marginTop", 900),
            edit(final, "borderBottom", BORDER),
        )
        draft = await call(client, "preview_edits", document=document, operation=change)
        assert draft["document"] == document and isinstance(draft["changes"], dict)
        assert list(tmp_path.iterdir()) == [source]
        saved = await call(client, "save_copy", document=document, preview=draft["preview"],
                           output=output.name)
        assert saved["output_snapshot"] == draft["output_snapshot"] == digest(output.read_bytes())
        assert await page(client, document) == inspection

        reopened = (await call(client, "open_document", path=output.name))["document"]
        await failure(client, "preview_edits", "wrong_owner", document=reopened, operation=change)
        new_p, new_empty, new_initial, new_final = (await page(client, reopened))["layout"]
        assert new_p["layout"] == dict(
            alignment="center", firstLineIndent=-240, keepWithNext=False, spaceAfter=180,
            tabStops=[TAB],
        )
        assert new_empty["layout"] == {}
        assert new_initial["layout"] == {k: v for k, v in first_section["layout"].items()
                                         if k != "titlePage"}
        assert new_final["layout"] == dict(final_section["layout"], marginTop=900, borderBottom=BORDER)

    assert source.read_bytes() == original
    assert output.read_bytes() == native_bytes(source, native)
    before, after = parts(source), parts(output)
    assert after.keys() == before.keys()
    assert {name for name in before if before[name] != after[name]} == {"word/document.xml"}
    body = ElementTree.fromstring(after["word/document.xml"]).find(qn("w:body"))
    borders = body.find(f"./{qn('w:sectPr')}/{qn('w:pgBorders')}/{qn('w:bottom')}")
    assert borders.attrib == {qn("w:val"): "single", qn("w:sz"): "8", qn("w:space"): "4",
                              qn("w:color"): "336699"}

    independent = docx.Document(output)
    assert [p.text for p in independent.paragraphs] == ["body text", ""]
    assert independent.tables[0].cell(0, 0).text == "inside the table"
    assert independent.sections[0].header.paragraphs[0].text == "unchanged header"
    assert not independent.sections[0].different_first_page_header_footer
    assert independent.sections[1].top_margin == Twips(900)
    p = independent.paragraphs[0].paragraph_format
    assert p.alignment == WD_ALIGN_PARAGRAPH.CENTER
    assert p.first_line_indent == Twips(-240)
    assert p.keep_with_next is False and p.widow_control is None
    assert p.space_after == Twips(180)
    tab, = p.tab_stops
    assert (tab.position, tab.alignment, tab.leader) == (
        Twips(720), WD_TAB_ALIGNMENT.RIGHT, WD_TAB_LEADER.DOTS,
    )
    validate(source, output)


async def test_layout_rejects_conflicts_and_mismatches_then_allows_retry(launch, tmp_path):
    source = tmp_path / "layout.docx"
    make_source(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        inspection = await page(client, document)
        p, _, section, _ = [row["anchor"] for row in inspection["layout"]]
        body = (await call(client, "inspect_document", document=document))["body"]
        run = body[1]["runs"][0]["anchor"]
        for ops, category in [
            ([edit(p, "alignment", "center"), edit(p, "alignment", None)], "rejected_batch"),
            ([edit(section, "margins", None), edit(section, "marginTop", 900)], "rejected_batch"),
            ([edit(p, "marginTop", 900)], "rejected_batch"),
            ([edit(section, "alignment", "center")], "rejected_batch"),
            ([edit(p, "alignment", True)], "rejected_batch"),
            ([edit(section, "margins", True)], "rejected_batch"),
            ([edit(run, "alignment", "center")], "wrong_anchor_type"),
            ([dict(anchor=p, field="widowControl")], "invalid_input"),
        ]:
            await failure(client, "preview_edits", category, document=document, operation=operation(*ops))
        assert await page(client, document) == inspection
        assert list(tmp_path.iterdir()) == [source]

        empty = await call(client, "preview_edits", document=document, operation=operation())
        noop = await call(client, "preview_edits", document=document,
                          operation=operation(edit(p, "alignment", None)))
        saved = await call(client, "save_copy", document=document, preview=empty["preview"],
                           output="noop.docx")
        output = tmp_path / "noop.docx"
        assert saved["output_snapshot"] == empty["output_snapshot"] == noop["output_snapshot"]
    assert output.read_bytes() == native_bytes(source, tmp_path / "native.docx", "empty")
    assert saved["output_snapshot"] == digest(output.read_bytes())
    assert parts(output) == parts(source)
