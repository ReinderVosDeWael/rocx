"""Exercise the JSON process boundary, with an installed Python API byte oracle."""
import json
import hashlib
import os
from pathlib import Path
import subprocess
import sys
from zipfile import ZipFile

import docx
from docx.enum.table import WD_CELL_VERTICAL_ALIGNMENT
from docx.enum.text import WD_ALIGN_PARAGRAPH
import pytest
from rocx import declarative as d
from rocx.editor import (
    CellAlignmentEdit, HeaderImageEdit, ImportedDocument, ParagraphAlignmentEdit,
    ParagraphInsertion, RunTextEdit, TableInsertion,
)

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests/fixtures"


@pytest.fixture(scope="module")
def cli():
    output = subprocess.check_output(
        ["cargo", "build", "--locked", "--bin", "rocx", "--message-format=json"],
        cwd=ROOT, text=True,
    )
    return next(msg["executable"] for line in output.splitlines()
                if (msg := json.loads(line)).get("executable"))


def call(cli, command, source, *, success=True, **fields):
    payload = json.dumps(dict(command=command, source=str(source), **fields))
    p = subprocess.run([cli], input=payload, text=True, capture_output=True)
    msg = json.loads(p.stdout)
    assert msg["ok"] is success and (p.returncode == 0) is success, (command, msg)
    return msg["result"] if success else msg["error"]


def parts(path):
    with ZipFile(path) as archive:
        return {n: archive.read(n) for n in archive.namelist()}


def test_header_targets_exclude_footer_and_even_slots(cli, tmp_path):
    source, output = tmp_path / "slots.docx", tmp_path / "copy.docx"
    doc = docx.Document()
    doc.add_paragraph("Keep")
    section = doc.sections[0]
    section.footer.paragraphs[0].text = "Default footer only"
    section.even_page_header.paragraphs[0].text = "Even header"
    section.first_page_header.paragraphs[0].text = "Supported first header"
    doc.save(source)
    original = source.read_bytes()
    seen = call(cli, "inspect", source)
    assert [h["target"] for h in seen["headers"]] == [dict(section_index=0, variant="first")]
    png = list((FIXTURES / "images/sample.png").read_bytes())
    for variant in ("default", "even"):
        batch = dict(snapshot=seen["snapshot"], edits=[
            dict(target=0, alignment="center"),
            dict(kind="header-image", target=dict(section_index=0, variant=variant),
                 mode="shared", bytes=png),
        ])
        call(cli, "preview", source, success=False, **batch)
        call(cli, "save-copy", source, success=False, output=str(output), **batch)
        assert not output.exists()
    assert source.read_bytes() == original


@pytest.mark.parametrize(("variant", "mode"), [("default", "shared"), ("first", "section-local")])
async def test_all_seven_edits(cli, tmp_path, variant, mode):
    source, output = tmp_path / "source.docx", tmp_path / "copy.docx"
    doc = docx.Document(FIXTURES / "imported-editor.docx")
    doc.sections[0].first_page_header.paragraphs[0].text = "First"
    doc.paragraphs[-1].add_run("drop")
    doc.add_table(rows=1, cols=2).cell(0, 1).text = "original cell"
    doc.save(source)
    original = source.read_bytes()
    seen = call(cli, "inspect", source)
    first, second, _, tail, table = seen["body"]
    header = next(h for h in seen["headers"] if h["target"] == {"section_index": 0, "variant": variant})
    png = (FIXTURES / "images/sample.png").read_bytes()
    replaced = next(r["inline_index"] for r in first["runs"] if r["text"] == "Patch")
    removed = next(r["inline_index"] for r in tail["runs"] if r["text"] == "drop")
    cell = table["cells"][1]
    edits = [
        dict(kind="alignment", target=first["target"], alignment="center"),
        dict(kind="insert-paragraph", target=tail["target"], text="new & placed"),
        dict(kind="insert-table", target=second["target"], rows=[["a", "b"]]),
        dict(kind="header-image", target=header["target"], mode=mode, bytes=list(png)),
        dict(kind="run-text", target=first["target"], inline_index=replaced, text=" edited "),
        dict(kind="run-remove", target=tail["target"], inline_index=removed),
        dict(kind="cell-alignment", target=table["target"], **cell, alignment="bottom"),
    ]
    batch = dict(snapshot=seen["snapshot"], edits=edits)
    preview = call(cli, "preview", source, **batch)
    assert preview["changes"] == [dict(target=first["target"], before=None, after="Center")]
    assert preview["table_insertions"] == [dict(target=second["target"], rows=1, columns=2)]
    assert preview["paragraph_insertions"] == [tail["target"]]
    assert preview["run_text"] == [dict(target=first["target"], inline_index=replaced,
                                        before="Patch", after=" edited ")]
    assert preview["run_removals"] == [dict(target=tail["target"], inline_index=removed, before="drop")]
    assert preview["cell_alignments"] == [dict(target=table["target"], **cell, before=None, after="bottom")]
    change, = preview["header_images"]
    assert change["mode"] == mode and change["source_part"] == header["part"]
    assert change["affected_sections"] == ([0, 1] if mode == "shared" else [0])
    assert change["restored_section"] == (None if mode == "shared" else 1)
    assert not output.exists()

    for bad in [
        dict(edits[0], kind="not-an-edit"), dict(edits[1], text=None),
        dict(edits[1], text="illegal \x00"), dict(edits[2], rows=[]),
        dict(edits[2], rows=[["a"], []]), dict(edits[2], rows=[[1]]),
        dict(edits[3], mode="typo"), dict(edits[3], bytes=[-1]),
        dict(edits[3], bytes=[256]), dict(edits[3], bytes=[]),
        dict(edits[3], target=dict(header["target"], extra=1)),
        dict(edits[3], target=dict(header["target"], variant="even")),
        dict(edits[4], inline_index=1),  # bookmark, not the next run in runs[]
        dict(edits[5], text="extra"), dict(edits[6], row_index=-1),
        dict(edits[6], cell_index=999), dict(edits[6], alignment="right"),
        dict(edits[6], target=first["target"]),
    ]:
        # Use a valid prefix without duplicate targets masking a malformed edit.
        call(cli, "save-copy", source, output=str(output), snapshot=seen["snapshot"],
             edits=[dict(target=tail["target"], alignment="left"), bad], success=False)
        assert not output.exists()
    conflict = dict(kind="run-text", target=tail["target"], inline_index=removed, text="conflict")
    call(cli, "save-copy", source, output=str(output), snapshot=seen["snapshot"],
         edits=edits + [conflict], success=False)
    assert not output.exists()

    assert call(cli, "save-copy", source, output=str(output), **batch) == preview
    assert source.read_bytes() == original
    assert "stale snapshot" in call(cli, "preview", output, success=False, **batch)

    imported = ImportedDocument.open(original)
    p, q, _, t, tbl = imported.body
    hdr = next(s for s in imported.stories if s.kind == "header" and
               s.variant == variant and s.section_index == 0)
    declared_table = d.Table([d.TableRow([d.TableCell([d.Paragraph(text=s)]) for s in ("a", "b")])])
    oracle = imported.preview(
        [ParagraphAlignmentEdit(p.anchor, "center")],
        table_insertions=[TableInsertion(q.anchor, await declared_table.resolve())],
        paragraph_insertions=[ParagraphInsertion(t.anchor, await d.Paragraph(text="new & placed").resolve())],
        header_images=[HeaderImageEdit(hdr.anchor, await d.ImageRun(data=png).resolve(),
                                      mode=mode.replace("-", "_"))],
        run_text=[RunTextEdit(p.inline_structure[replaced].anchor, " edited ")],
        run_removals=[t.inline_structure[removed].anchor],
        cell_alignments=[CellAlignmentEdit(tbl.anchor, 0, 1, "bottom")],
    )
    assert output.read_bytes() == oracle.to_docx()
    saved = docx.Document(output)
    assert saved.paragraphs[0].alignment == WD_ALIGN_PARAGRAPH.CENTER
    assert saved.paragraphs[0].runs[0].bold and saved.paragraphs[0].runs[0].text == " edited "
    assert saved.paragraphs[0].hyperlinks[0].url == doc.paragraphs[0].hyperlinks[0].url
    assert saved.paragraphs[0]._p.xpath("w:bookmarkStart/@w:name") == ["BodyLink"]
    assert saved.paragraphs[0]._p.xpath("w:bookmarkEnd/@w:id") == ["0"]
    assert [p.text for p in saved.paragraphs[-2:]] == ["Second section", "new & placed"]
    assert [c.text for c in saved.tables[0].rows[0].cells] == ["a", "b"]
    c = saved.tables[1].cell(0, 1)
    assert (c.text, c.vertical_alignment) == ("original cell", WD_CELL_VERTICAL_ALIGNMENT.BOTTOM)
    for i, section in enumerate(saved.sections):
        h = section.header if variant == "default" else section.first_page_header
        embeds = h._element.xpath(".//a:blip/@r:embed")
        if i in change["affected_sections"]:
            rid, = embeds
            assert h.part.related_parts[rid].blob == png
        else:
            assert not embeds
    old, new = parts(source), parts(output)
    modified = {"word/document.xml", "[Content_Types].xml"}
    if mode == "shared":
        modified.add(header["part"].lstrip("/"))
        modified.add("word/_rels/header1.xml.rels")
    else:
        modified.add("word/_rels/document.xml.rels")
    assert old.keys() <= new.keys()
    for name in old.keys() - modified:
        assert old[name] == new[name], name
    if os.environ.get("ROCX_REQUIRE_SCHEMAS") == "1":
        subprocess.run([sys.executable, ROOT / "tools/validate_ooxml.py", output], check=True)


def checked_copy(cli, source, output, edits):
    snapshot = hashlib.sha256(source.read_bytes()).hexdigest()
    batch = dict(snapshot=snapshot, edits=edits)
    preview = call(cli, "preview", source, **batch)
    assert not output.exists()
    assert call(cli, "save-copy", source, output=str(output), **batch) == preview
    assert preview["output_snapshot"] == hashlib.sha256(output.read_bytes()).hexdigest()
    assert snapshot == hashlib.sha256(source.read_bytes()).hexdigest()
    if os.environ.get("ROCX_REQUIRE_SCHEMAS") == "1":
        subprocess.run([sys.executable, ROOT / "tools/validate_ooxml.py", output], check=True)
    return call(cli, "inspect", output)


def independent_document():
    # The python-docx default template omits the schema-required zoom percentage.
    from docx.oxml.ns import qn
    doc = docx.Document()
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    return doc


async def test_recovered_body_structure_mixes_against_installed_core(cli, tmp_path):
    from rocx.editor import BlockEdit, TableStructureEdit as E

    source, output = tmp_path / "body.docx", tmp_path / "mixed.docx"
    doc = independent_document()
    doc.add_table(1, 1).cell(0, 0).text = "First section table"
    doc.add_paragraph("First section text")
    doc.sections[0].header.paragraphs[0].text = "Shared header"
    doc.add_section()
    doc.add_paragraph("Second section text")
    doc.add_table(1, 2)
    doc.add_table(1, 1).cell(0, 0).text = "Remove this table"
    doc.add_table(1, 1).cell(0, 0).text = "Replace this table"
    doc.save(source)

    before = source.read_bytes()
    imported = ImportedDocument.open(before)
    seen = call(cli, "inspect", source)
    section = seen["sections"][0]["target"]
    target, drop, replace = [b["target"] for b in seen["body"] if b["type"] == "table"][1:]
    donor = await d.Table(
        [d.TableRow([d.TableCell([d.Paragraph("not donor text")])], repeat_header=False)],
        alignment="center",
    ).resolve()
    fresh = await d.Table([d.TableRow([d.TableCell([d.Paragraph("Replacement")])])],
                          column_widths=[1200]).resolve()
    edits = [
        dict(kind="split-section", target=section, offset=1),
        dict(kind="alignment", target=target - 1, alignment="right"),
        dict(kind="table-structure", target=target, operation=dict(
            kind="patch-property", field="alignment", source=dict(rows=[["not donor text"]],
                                                                    alignment="center"))),
        dict(kind="table-structure", target=target, operation=dict(
            kind="patch-property", field="repeatHeader", row=0, source=dict(
                rows=[["not donor text"]], first_row=dict(repeat_header=False)))),
        dict(kind="table-structure", target=drop, operation=dict(kind="remove-table")),
        dict(kind="table-structure", target=replace, operation=dict(kind="replace-node", source=dict(
            rows=[["Replacement"]], column_widths=[1200]))),
    ]
    for invalid in [
        edits[0] | {"offset": 100},
        dict(kind="merge-section", target=1),
        dict(kind="table-structure", target=target, operation=dict(
            kind="patch-property", field="width", row=0, column=0, source=None)),
        dict(kind="table-structure", target=target, operation=dict(kind="patch-property", field="layout")),
        edits[2],
    ]:
        call(cli, "save-copy", source, success=False, snapshot=seen["snapshot"],
             output=str(output), edits=[*edits, invalid])
        assert not output.exists() and source.read_bytes() == before
    result = checked_copy(cli, source, output, edits)
    assert len(result["sections"]) == 3
    oracle = imported.preview(
        [ParagraphAlignmentEdit(imported.body[target - 1].anchor, "right")],
        block_edits=[BlockEdit.split_section(imported.sections[0].anchor, 1)],
        table_structure=[
            E.patch_property(imported.body[target].anchor, "alignment", donor),
            E.patch_property(imported.body[target].anchor, "repeatHeader", donor, row=0),
            E.remove_table(imported.body[drop].anchor),
            E.replace_node(imported.body[replace].anchor, fresh),
        ],
    )
    assert output.read_bytes() == oracle.to_docx()
    old, new = parts(source), parts(output)
    assert old.keys() == new.keys()
    assert {p for p in old if old[p] != new[p]} == {"word/document.xml"}
    saved = docx.Document(output)
    assert saved.tables[-1].cell(0, 0).text == "Replacement"
    assert all(s.header.paragraphs[0].text == "Shared header" for s in saved.sections)
    reopened = ImportedDocument.open(output.read_bytes())
    merged = tmp_path / "merged.docx"
    merge = [dict(kind="merge-section", target=0)]
    call(cli, "save-copy", output, success=False, snapshot=seen["snapshot"], output=str(merged), edits=merge)
    assert not merged.exists()
    checked_copy(cli, output, merged, merge)
    assert merged.read_bytes() == reopened.preview(
        [], block_edits=[BlockEdit.merge_section(reopened.sections[0].anchor)]).to_docx()


def test_recovered_layout_batch_uses_original_owners(cli, tmp_path):
    from rocx.editor import LayoutEdit as L

    source, output = tmp_path / "layout.docx", tmp_path / "laid-out.docx"
    doc = independent_document()
    doc.add_table(1, 1)
    p = doc.add_paragraph("Layout target")
    p.alignment = WD_ALIGN_PARAGRAPH.RIGHT
    doc.save(source)
    imported = ImportedDocument.open(source.read_bytes())
    seen = call(cli, "inspect", source)
    assert seen["body"][1]["layout"]["alignment"] == "right"
    edits = [
        dict(kind="paragraph-layout", target=1, field="alignment", value=None),
        dict(kind="paragraph-layout", target=1, field="tab_stops",
             value=[dict(position=720, alignment="right", leader="dot")]),
        dict(kind="section-layout", target=0, field="margin_top", value=-220),
        dict(kind="section-layout", target=0, field="border_top", value=dict(size=8, color="123456")),
    ]
    for invalid in [edits[0], edits[3] | {"target": 4},
                    dict(kind="section-layout", target=0, field="page_width"),
                    dict(kind="alignment", target=1, alignment="center")]:
        call(cli, "save-copy", source, success=False, snapshot=seen["snapshot"],
             output=str(output), edits=[*edits, invalid])
        assert not output.exists()
    after = checked_copy(cli, source, output, edits)
    assert "alignment" not in after["body"][1]["layout"]
    assert after["sections"][0]["layout"]["margin_top"] == -220
    assert output.read_bytes() == imported.preview_layout([
        L.paragraph(imported.body[1].anchor, e["field"], e["value"]) if e["kind"] == "paragraph-layout"
        else L.section(imported.sections[0].anchor, e["field"], e["value"]) for e in edits
    ]).to_docx()


async def test_recovered_story_splices_and_references_match_installed_core(cli, tmp_path):
    source = tmp_path / "references.docx"
    doc = independent_document()
    doc.add_paragraph("Keep body")
    doc.sections[0].header.paragraphs[0].text = "Shared header"
    doc.add_section()
    doc.save(source)
    original = source.read_bytes()
    footer = dict(section_index=1, kind="footer", variant="first")
    wire_blocks = [dict(kind="rich-paragraph", inlines=[dict(
        kind="hyperlink", url="https://example.invalid/story", text="Owner hyperlink")])]
    block = await d.Paragraph(children=[d.Hyperlink("https://example.invalid/story", [
        d.TextRun("Owner hyperlink")])]).resolve()

    part = None
    for i, action in enumerate(("create", "remove", "link")):
        current = ImportedDocument.open(source.read_bytes())
        wire = dict(kind=action)
        kwargs = {}
        if action == "create":
            wire["blocks"], kwargs["blocks"] = wire_blocks, [block]
        if action == "link":
            wire["part"], kwargs["part_name"] = part, part
        dest = tmp_path / f"story-{i}.docx"
        seen = checked_copy(cli, source, dest, [dict(kind="story-reference", target=footer, action=wire)])
        assert dest.read_bytes() == current.edit_story_reference(1, "footer", "first", action, **kwargs)
        if action == "create":
            part = next(s["part"] for s in seen["stories"] if s["target"] == footer)
        if action == "remove":
            assert part in parts(dest)
        source = dest

    header = dict(section_index=0, kind="header", variant="default")
    current = ImportedDocument.open(source.read_bytes())
    dest = tmp_path / "local.docx"
    splice = dict(kind="story-blocks", target=header, mode="section-local", index=1,
                  remove=0, blocks=wire_blocks)
    seen = call(cli, "inspect", source)
    assert next(s for s in seen["stories"] if s["target"] == header)["block_count"] == 1
    for edits in [[splice, dict(kind="core-title", value="conflicting scope")],
                  [splice | {"index": 9}]]:
        call(cli, "save-copy", source, success=False, snapshot=seen["snapshot"],
             output=str(dest), edits=edits)
        assert not dest.exists()
    checked_copy(cli, source, dest, [splice])
    assert dest.read_bytes() == current.edit_story_blocks(
        current.default_headers[0].anchor, "section_local", 1, 0, [block])
    readback = docx.Document(dest)
    assert readback.sections[0].header.paragraphs[1].hyperlinks[0].address == "https://example.invalid/story"
    assert len(readback.sections[1].header.paragraphs) == 1
    assert parts(source)["word/header1.xml"] == parts(dest)["word/header1.xml"]
    assert original != dest.read_bytes()


async def test_physical_table_cli_keeps_nested_cell_relationships_in_footer(cli, tmp_path):
    from rocx.editor import TableStructureEdit as E

    source, output = tmp_path / "physical.docx", tmp_path / "physical-edit.docx"
    doc = independent_document()
    doc.add_table(1, 1)
    footer = doc.sections[0].footer
    outer = footer.add_table(1, 1, docx.shared.Inches(2))
    outer.cell(0, 0).add_table(1, 1).cell(0, 0).text = "Old cell"
    footer.add_table(1, 1, docx.shared.Inches(2)).autofit = False
    doc.save(source)
    before = source.read_bytes()
    owner = "word/footer1.xml"
    seen = call(cli, "inspect", source, table_owner=owner)
    parent, nested, sibling = seen["part_tables"]
    assert nested["owner_part"] == owner and nested["depth"] == 1
    changes = [
        dict(path=nested["path"], operation=dict(kind="replace-cell", row=0, cell=0, blocks=[
            dict(kind="rich-paragraph", inlines=[dict(kind="hyperlink",
                 url="https://example.invalid/footer", text="Footer link")])])),
        dict(path=sibling["path"], operation=dict(kind="layout", value="autofit")),
    ]
    scoped = dict(kind="part-tables", owner_part=owner, operations=changes)
    for invalid in [
        scoped | {"owner_part": "word/document.xml"},
        scoped | {"operations": [*changes, dict(path=parent["path"], operation=dict(kind="insert-row", index=1))]},
        scoped | {"operations": [dict(path=nested["path"], operation=dict(kind="remove-table"))]},
    ]:
        call(cli, "save-copy", source, success=False, snapshot=seen["snapshot"], output=str(output), edits=[invalid])
        assert not output.exists() and source.read_bytes() == before
    checked_copy(cli, source, output, [scoped])
    imported = ImportedDocument.open(before)
    _, nested_table, sibling_table = imported.part_tables(owner)
    linked = await d.Paragraph(children=[d.Hyperlink(
        "https://example.invalid/footer", [d.TextRun("Footer link")])]).resolve()
    assert output.read_bytes() == imported.edit_part_tables(owner, [
        E.replace_cell(nested_table.anchor, 0, 0, [linked]),
        E.layout(sibling_table.anchor, "autofit"),
    ])
    old, new = parts(source), parts(output)
    assert new.keys() - old.keys() == {"word/_rels/footer1.xml.rels"}
    assert {p for p in old if old[p] != new[p]} == {owner}
    readback = docx.Document(output)
    cell = readback.sections[0].footer.tables[0].cell(0, 0).tables[0].cell(0, 0)
    assert cell.paragraphs[0].hyperlinks[0].address == "https://example.invalid/footer"

    noop = tmp_path / "noop.docx"
    checked_copy(cli, source, noop, [scoped | {"operations": [
        dict(path=sibling["path"], operation=dict(kind="layout", value="fixed"))]}])
    assert parts(noop) == old
    assert noop.read_bytes() == imported.edit_part_tables(owner, [E.layout(sibling_table.anchor, "fixed")])


def test_structural_property_transport(cli, tmp_path):
    from rocx.editor import BlockEdit, ResourceEdit, TableStructureEdit, TextInsertion

    source, output = tmp_path / "source.docx", tmp_path / "edited.docx"
    doc = independent_document()
    doc.add_paragraph("body").paragraph_format.widow_control = False
    doc.add_table(rows=1, cols=2).autofit = True
    doc.add_paragraph("a😀b")
    doc.save(source)
    seen = call(cli, "inspect", source)
    start = seen["sections"][0]["body_start"]
    assert seen["body"][start]["flags"]["widow_control"] is False
    edits = [dict(kind="paragraph-flag", target=start, flag="widow_control", value=None),
             dict(kind="section-flag", target=start, flag="rtl_gutter", value=False),
             dict(kind="section-type", target=start, value="evenPage"),
             dict(kind="table-structure", target=1, operation=dict(kind="layout", value="fixed")),
             dict(kind="text-insertion", target=2, position=2, text="!"),
             dict(kind="core-subject", value=""),
             dict(kind="metadata", field="manager", value="Editor"),
             dict(kind="custom-property", name="Reviewed", value=False)]
    for invalid in [dict(kind="section-type", target=start),
                    dict(kind="table-structure", target=1, operation=dict(kind="layout")),
                    dict(kind="metadata", field="manager"),
                    dict(kind="custom-property", name="Reviewed", value=1),
                    dict(kind="text-insertion", target=2, position=4, text="!")]:
        call(cli, "save-copy", source, success=False, snapshot=seen["snapshot"],
             output=str(output), edits=[edits[0], invalid])
        assert not output.exists()
    result = checked_copy(cli, source, output, edits)
    details = call(cli, "inspect", output, metadata_fields=["manager"], custom_properties=["Reviewed", "Missing"])
    assert details["core_subject"] == ""
    assert details["metadata"] == {"manager": "Editor"}
    assert details["custom_properties"] == {"Reviewed": False, "Missing": None}
    assert result["body"][2]["text"] == "a😀!b"
    assert result["body"][start]["flags"] == {}
    assert result["sections"][0]["flags"]["rtl_gutter"] is False
    assert result["sections"][0]["section_type_xml"] == "evenPage"
    assert result["body"][1]["layout"] == "fixed"
    assert result["body"][1]["grid_widths"] == seen["body"][1]["grid_widths"]
    imported = ImportedDocument.open(source.read_bytes())
    p, t, text = imported.body
    assert output.read_bytes() == imported.preview([], block_edits=[
        BlockEdit.paragraph_flag(p.anchor, "widow_control", None),
        BlockEdit.section_flag(p.anchor, "rtl_gutter", False),
        BlockEdit.section_type(p.anchor, "evenPage")],
        table_structure=[TableStructureEdit.layout(t.anchor, "fixed")],
        text_insertions=[TextInsertion(text.anchor, 2, "!")],
        resources=[ResourceEdit.core_subject(""), ResourceEdit.metadata("manager", "Editor"),
                   ResourceEdit.custom_property("Reviewed", False)]).to_docx()


def test_extended_mixed_binding_and_atomic_rejection(cli, tmp_path):
    from docx.oxml import parse_xml
    from docx.oxml.ns import nsdecls
    from rocx.editor import (
        BlockEdit, ControlEdit, InlineEdit, ResourceEdit, StyleEdit, TableStructureEdit, TextRangeEdit,
    )

    source, output = tmp_path / "original.docx", tmp_path / "copy.docx"
    doc = independent_document()
    p = doc.add_paragraph()
    p.add_run("a😀").bold = True
    p.add_run("bc").italic = True
    doc.add_paragraph("keep")
    doc.add_paragraph("inline")
    doc.add_table(rows=1, cols=2)
    doc.add_paragraph()._p.append(parse_xml(
        f'<w:sdt {nsdecls("w")}><w:sdtPr><w:tag w:val="original"/><w:text/></w:sdtPr>'
        '<w:sdtContent><w:r><w:t>old</w:t></w:r></w:sdtContent></w:sdt>'))
    doc.save(source)
    seen = call(cli, "inspect", source)
    a = seen["abstract_starts"][0]
    control, = seen["controls"]
    assert control["context"] == "inline"
    edits = [
        dict(kind="core-title", value="CLI title"), dict(kind="default-alignment", value="justify"),
        dict(kind="abstract-start", **(a | {"value": 9})),
        dict(kind="style-add", definition=dict(kind="character", name="CliEmphasis", bold=True)),
        dict(kind="text-range", target=0, start=1, end=3, text="!"),
        dict(kind="keep-with-next", target=1, value=True),
        dict(kind="run-bold", target=2, inline_index=0, value=True),
        dict(kind="table-structure", target=3, operation=dict(kind="insert-column", index=1)),
        dict(kind="control", target=control["target"], tag="changed", text="new"),
    ]
    for bad in [
        dict(kind="core-title"), dict(kind="style-remove", id="Normal"),
        dict(kind="insert-inline", target=1, inline_index=0, inline=dict(kind="text", txt="typo")),
        dict(kind="table-structure", target=3, operation=dict(kind="remove-row", index=999)),
        dict(kind="text-range", target=0, start=1, end=2, text="overlap"),
        dict(kind="tracked-text", target=dict(target=2, inline_index=0), author="R", action=dict(kind="delete")),
    ]:
        call(cli, "save-copy", source, success=False, snapshot=seen["snapshot"],
             output=str(output), edits=[edits[4], bad])
        assert not output.exists()
    after = checked_copy(cli, source, output, edits)
    assert (after["core_title"], after["default_alignment"], after["abstract_starts"][0]["value"]) == (
        "CLI title", "justify", 9)
    assert after["controls"][0]["tag"] == "changed"
    assert any(s["id"] == "CliEmphasis" for s in after["styles"]["definitions"])
    saved = docx.Document(output)
    assert saved.paragraphs[0].text == "a!c" and saved.paragraphs[0].runs[1].italic
    assert saved.paragraphs[1].paragraph_format.keep_with_next and saved.paragraphs[2].runs[0].bold
    assert len(saved.tables[0].columns) == 3
    original = ImportedDocument.open(source.read_bytes())
    p, q, r, t, _ = original.body
    expected = original.preview([], resources=[
        ResourceEdit.core_title("CLI title"), ResourceEdit.default_alignment("justify"),
        ResourceEdit.abstract_start(a["abstract_id"], a["level"], 9)],
        style_edits=[StyleEdit.add(d.CharacterStyleDefinition(name="CliEmphasis", bold=True))],
        text_ranges=[TextRangeEdit(p.anchor, 1, 3, "!")],
        block_edits=[BlockEdit.keep_with_next(q.anchor, True)],
        inline_edits=[InlineEdit.set_bold(r.inline_structure[0].anchor, True)],
        table_structure=[TableStructureEdit.insert_column(t.anchor, 1)],
        controls=[ControlEdit(original.controls[0].anchor, tag="changed", text="new")])
    assert output.read_bytes() == expected.to_docx()
    before, final = parts(source), parts(output)
    assert before.keys() == final.keys()
    assert {n for n in before if before[n] != final[n]} == {
        "word/document.xml", "word/styles.xml", "word/numbering.xml", "docProps/core.xml"}
    cleared = checked_copy(cli, output, tmp_path / "clear.docx", [
        dict(kind="core-title", value=None), dict(kind="default-alignment", value=None),
        a | dict(kind="abstract-start", value=None)])
    assert cleared["core_title"] is cleared["default_alignment"] is cleared["abstract_starts"][0]["value"] is None


def test_discovered_footer_and_revision_scopes(cli, tmp_path):
    source, story, moved = [tmp_path / f"{n}.docx" for n in ("source", "story", "moved")]
    doc = independent_document()
    p = doc.add_paragraph("left")
    p.add_run("right")
    doc.sections[0].even_page_footer.paragraphs[0].text = "Shared"
    doc.save(source)
    seen = call(cli, "inspect", source)
    slot, = seen["stories"]
    assert slot["target"] == dict(section_index=0, kind="footer", variant="even")
    block, = slot["blocks"]
    run, = block["inlines"]
    original = ImportedDocument.open(source.read_bytes())
    checked_copy(cli, source, story, [dict(kind="story-text", part=slot["part"], runs=[
        dict(block_index=block["block_index"], inline_index=run["inline_index"], text="Edited")])])
    physical, = original.stories
    assert story.read_bytes() == original.edit_story_text(slot["part"], [
        RunTextEdit(physical.blocks[0].inline_structure[0].anchor, "Edited")])
    assert docx.Document(story).sections[0].even_page_footer.paragraphs[0].text == "Edited"
    image_path = tmp_path / "image.docx"
    png = (FIXTURES / "images/sample.png").read_bytes()
    checked_copy(cli, source, image_path, [dict(kind="story-image", target=slot["target"], mode="shared", bytes=list(png))])
    footer = docx.Document(image_path).sections[0].even_page_footer
    rid, = footer._element.xpath(".//a:blip/@r:embed")
    assert footer.part.related_parts[rid].blob == png

    addr = lambda i: dict(target=0, inline_index=i)
    date = "2024-02-29T12:34:56Z"
    result = checked_copy(cli, source, moved, [dict(kind="tracked-move", target=addr(0),
        after=addr(1), name="Move1", author="Reviewer", date=date)])
    first, second = original.body[0].inline_structure
    assert moved.read_bytes() == original.preview_tracked_move(
        first.anchor, second.anchor, "Move1", "Reviewer", date).to_docx()
    moves = {n["revision_kind"]:addr(n["inline_index"]) for n in result["body"][0]["inlines"]
             if n["kind"] == "revision"}
    accepted = tmp_path / "accepted.docx"
    checked_copy(cli, moved, accepted, [dict(kind="review-revision", target=moves["moveSource"],
        action="accept", paired=moves["moveDestination"])])
    assert docx.Document(accepted).paragraphs[0].text == "rightleft"

    tracked = tmp_path / "replaced.docx"
    result = checked_copy(cli, source, tracked, [dict(kind="tracked-text", target=addr(0),
        author="Reviewer", date=date, action=dict(kind="replace", text="new"))])
    assert tracked.read_bytes() == original.preview_tracked_text(first.anchor, "replace", "Reviewer",
        text="new", date=date).to_docx()
    assert [n["revision_kind"] for n in result["body"][0]["inlines"] if n["kind"] == "revision"] == [
        "deletion", "insertion"]


def test_declarations_bind_original_indices_and_reresolve_styles(cli, tmp_path):
    source, output, second = [tmp_path / f"{n}.docx" for n in ("source", "copy", "next")]
    doc = independent_document()
    for text in ["drop", "old block", "turn into link", "last"]:
        doc.add_paragraph(text)
    doc.add_table(1, 1)
    doc.save(source)
    edited = checked_copy(cli, source, output, [
        dict(kind="remove-paragraph", target=0),
        dict(kind="replace-paragraph", target=1, text="new block"),
        dict(kind="replace-inline", target=2, inline_index=0,
             inline=dict(kind="hyperlink", text="link", url="https://example.org/cli")),
        dict(kind="insert-inline", target=3, inline_index=0,
             inline=dict(kind="text", text=" after", italic=True)),
        dict(kind="table-structure", target=4, operation=dict(kind="replace-cell", row=0, cell=0,
             blocks=[dict(kind="table", rows=[["nested"]]), dict(kind="paragraph", text="end")])),
        dict(kind="style-add", definition=dict(kind="paragraph", name="Memo"))])
    saved = docx.Document(output)
    assert [p.text for p in saved.paragraphs] == ["new block", "link", "last after"]
    assert saved.paragraphs[1].hyperlinks[0].url == "https://example.org/cli"
    assert saved.paragraphs[2].runs[-1].italic
    assert saved.tables[0].cell(0, 0).tables[0].cell(0, 0).text == "nested"
    assert edited["body"][1]["inlines"] == [dict(kind="hyperlink", inline_index=0)]
    definition = next(s for s in edited["styles"]["definitions"] if s["name"] == "Memo")
    checked_copy(cli, output, second, [
        dict(kind="replace-inline", target=1, inline_index=0, inline=dict(kind="text", text="plain")),
        dict(kind="style-patch", id=definition["id"], bold=True)])
    assert docx.Document(second).styles["Memo"].font.bold
    assert not docx.Document(second).paragraphs[1].hyperlinks


def test_entry_text_cannot_select_another_owner(cli, tmp_path):
    source, output = tmp_path / "comments.docx", tmp_path / "edited.docx"
    doc = independent_document()
    run = doc.add_paragraph("annotated").runs[0]
    doc.add_comment(run, "original", author="Reviewer")
    doc.save(source)
    seen = call(cli, "inspect", source)
    entry, = seen["entries"]
    block, = entry["blocks"]
    target, = [n for n in block["inlines"] if n.get("text") == "original"]
    edit = dict(kind="story-text", part=entry["part"], runs=[dict(entry_index=entry["entry_index"],
        block_index=block["block_index"], inline_index=target["inline_index"], text="updated")])
    call(cli, "save-copy", source, success=False, output=str(output), snapshot=seen["snapshot"],
         edits=[edit | dict(part="word/document.xml")])
    assert not output.exists()
    checked_copy(cli, source, output, [edit])
    comment, = docx.Document(output).comments
    assert comment.author == "Reviewer" and comment.text == "updated"
    old, new = parts(source), parts(output)
    assert {n for n in old.keys() | new.keys() if old.get(n) != new.get(n)} == {entry["part"].lstrip("/")}


def test_section_replacement_and_geometry_use_original_coordinates(cli, tmp_path):
    from rocx.editor import TableStructureEdit

    source, output = tmp_path / "section.docx", tmp_path / "tables.docx"
    doc = independent_document()
    doc.add_paragraph("old").add_run(" section")
    doc.sections[0].left_margin = docx.shared.Inches(1.5)
    doc.save(source)
    blocks = [dict(kind="table", rows=[["a", "b"], ["c", "d"]]) for _ in range(6)]
    checked_copy(cli, source, output, [dict(kind="replace-section", target=0, blocks=blocks)])
    copy = docx.Document(output)
    assert len(copy.tables) == 6 and not copy.paragraphs
    assert copy.sections[0].left_margin == doc.sections[0].left_margin
    operations = [
        ("insert-row", {"index": 1}), ("remove-row", {"index": 1}),
        ("insert-column", {"index": 1}), ("remove-column", {"index": 1}),
        ("insert-cell", {"row": 1, "cell": 0}), ("remove-cell", {"row": 1, "cell": 0}),
    ]
    geometry = tmp_path / "geometry.docx"
    checked_copy(cli, output, geometry, [dict(kind="table-structure", target=i,
        operation=dict(kind=k, **indices)) for i, (k, indices) in enumerate(operations)])
    imported = ImportedDocument.open(output.read_bytes())
    edits = [getattr(TableStructureEdit, k.replace("-", "_"))(t.anchor, *indices.values())
             for t, (k, indices) in zip(imported.body, operations, strict=True)]
    assert geometry.read_bytes() == imported.preview([], table_structure=edits).to_docx()


def test_resource_and_style_transport(cli, tmp_path):
    from docx.enum.style import WD_STYLE_TYPE
    from rocx.editor import ResourceEdit as R, StyleEdit as S

    source, output = tmp_path / "resources.docx", tmp_path / "edited.docx"
    word = independent_document()
    word.add_paragraph("List entry")
    word.styles.add_style("CLI Table", WD_STYLE_TYPE.TABLE)
    word.styles.add_style("CLI Paragraph", WD_STYLE_TYPE.PARAGRAPH)
    word.save(source)
    original = source.read_bytes()
    imported = ImportedDocument.open(original)
    paragraph = imported.body[0].anchor
    table = imported.style_anchor("CLITable")
    paragraph_style = imported.style_anchor("CLIParagraph")
    level = dict(level=0, start=1, format="decimal", text="%1.")
    replacement = level | dict(start=2, format="lowerRoman")
    definition = dict(kind="numbering", name="CLI List", numbering_id=702)
    edits = [
        dict(kind="style-add", definition=definition),
        dict(kind="style-region", id="CLITable", region="firstRow", value={"bold": True}),
        dict(kind="style-field", id="CLIParagraph", field="i", region=None, value="0"),
        dict(kind="core-subject", value="Review"),
        dict(kind="document-default", field="bold", value=False),
        dict(kind="document-default", field="tabStops", value=[
            dict(position_twips=720, alignment="right", leader="dot")]),
        dict(kind="add-instance", num_id=703, abstract_id=701),
        dict(kind="add-instance", num_id=702, abstract_id=701),
        dict(kind="add-abstract", abstract_id=701, levels=[level]),
        dict(kind="instance-start", num_id=703, level=0, value=6),
        dict(kind="instance-level", num_id=703, level=0, value=replacement),
        dict(kind="paragraph-numbering", target=0, value=dict(id=703, level=0)),
        dict(kind="add-abstract", abstract_id=704, levels=[level]),
    ]
    snapshot = call(cli, "inspect", source, numbering_targets=[0])
    assert snapshot["paragraph_numbering"] == {"0": None}
    for bad in (dict(kind="metadata", field="subject", value="Duplicate"),
                dict(kind="document-default", field="italic"),
                dict(kind="paragraph-numbering", target=1, value=None)):
        call(cli, "save-copy", source, success=False, output=str(output),
             snapshot=snapshot["snapshot"], edits=[*edits, bad])
        assert not output.exists()
    inspected = checked_copy(cli, source, output, edits)
    assert inspected["core_subject"] == "Review"
    assert inspected["document_defaults"]["bold"] is False
    assert inspected["document_defaults"]["tabStops"] == edits[5]["value"]
    instance, = [i for i in inspected["numbering_instances"] if i["num_id"] == 703]
    assert instance == dict(num_id=703, abstract_id=701, replacement_levels=[0],
                            start_overrides=[dict(level=0, start=6)])
    result = call(cli, "inspect", output, numbering_targets=[0])
    assert result["paragraph_numbering"] == {"0": {"id": 703, "level": 0}}
    output_bytes = output.read_bytes()
    assert output_bytes == imported.preview([], style_edits=[
        S.add(d.NumberingStyleDefinition("CLI List", 702)),
        S.region(table, "firstRow", d.TableSectionFormat(bold=True)),
        S.field(paragraph_style, "i", "0"),
    ], resources=[
        R.core_subject("Review"), R.document_default("bold", False),
        R.document_default("tabStops", [(720, "right", "dot")]),
        R.add_instance(703, 701), R.add_instance(702, 701),
        R.add_abstract(701, [d.NumberingLevel(**level)]), R.instance_start(703, 0, 6),
        R.instance_level(703, 0, d.NumberingLevel(**replacement)),
        R.paragraph_numbering(paragraph, (703, 0)), R.add_abstract(704, [d.NumberingLevel(**level)]),
    ]).to_docx()
    assert docx.Document(output).styles["CLI List"].element.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["702"]
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {p for p in before if before[p] != after[p]} == {
        "word/document.xml", "word/styles.xml", "word/numbering.xml", "docProps/core.xml"}

    target = tmp_path / "cleared.docx"
    call(cli, "save-copy", output, success=False, output=str(target),
         snapshot=result["snapshot"], edits=[dict(kind="remove-instance", num_id=703)])
    assert not target.exists()
    cleared = checked_copy(cli, output, target, [
        dict(kind="paragraph-numbering", target=0, value=None),
        dict(kind="style-region", id="CLITable", region="firstRow", value=None),
        dict(kind="remove-instance", num_id=703),
        dict(kind="remove-abstract", abstract_id=704),
    ])
    assert all(i["num_id"] != 703 for i in cleared["numbering_instances"])
    assert all(a["abstract_id"] != 704 for a in cleared["abstract_starts"])
    assert call(cli, "inspect", target, numbering_targets=[0])["paragraph_numbering"] == {"0": None}
    assert source.read_bytes() == original and output.read_bytes() == output_bytes


def test_recovered_numbering_round_trips_and_rejects_opaque_edits(cli, tmp_path):
    from lxml import etree
    from rocx.editor import ResourceEdit as R
    from test_resource_fields import pack

    source, added, edited = [tmp_path / f"{stem}.docx" for stem in ("source", "added", "edited")]
    word = independent_document()
    word.add_paragraph("Body")
    word.add_table(rows=1, cols=1).cell(0, 0).text = "Cell"
    word.sections[0].footer.paragraphs[0].text = "Footer"
    word.save(source)
    level = dict(level=0, start=1, format="decimal", text="%1.")
    custom = level | dict(start=4, format="upperRoman")
    seen = checked_copy(cli, source, added, [
        dict(kind="add-abstract", abstract_id=701, levels=[level]),
        dict(kind="add-abstract", abstract_id=702, levels=[level]),
        dict(kind="add-instance", num_id=703, abstract_id=701),
    ])
    footer, = [s for s in seen["stories"] if s["target"]["kind"] == "footer"]
    story = dict(kind="story", part=footer["part"], block_index=0)
    cell = dict(kind="cell", target=1, row_index=0, cell_index=0, block_index=0)
    numbering = dict(id=703, level=0)
    edits = [
        dict(kind="replace-abstract", abstract_id=701, levels=[custom]),
        dict(kind="abstract-level", abstract_id=702, level=0, value=custom),
        dict(kind="replace-instance", definition=dict(id=703, abstract_id=702)),
        *[dict(kind="story-paragraph-numbering", target=site, value=numbering) for site in (story, cell)],
        dict(kind="document-default", field="bold", value=False),
        dict(kind="metadata", field="title", value="Recovery"),
    ]
    readback = checked_copy(cli, added, edited, edits)
    result = call(cli, "inspect", edited, numbering_story_targets=[story, cell])
    assert result["story_paragraph_numbering"] == [numbering, numbering]
    assert readback["document_defaults"]["bold"] is False and readback["core_title"] == "Recovery"
    imported = ImportedDocument.open(added.read_bytes())
    anchor = imported.story_paragraph_anchor(story["part"], 0)
    cell_anchor = imported.cell_paragraph_anchor(imported.body[1].anchor, 0, 0, 0)
    assert edited.read_bytes() == imported.preview([], resources=[
        R.replace_abstract(701, [d.NumberingLevel(**custom)]),
        R.abstract_level(702, 0, d.NumberingLevel(**custom)),
        R.replace_instance(d.NumberingDefinition(703, abstract_id=702)),
        R.story_paragraph_numbering(anchor, (703, 0)),
        R.story_paragraph_numbering(cell_anchor, (703, 0)),
        R.document_default("bold", False), R.metadata("title", "Recovery"),
    ]).to_docx()
    untouched = call(cli, "preview", edited, snapshot=result["snapshot"], edits=[])
    assert call(cli, "preview", edited, snapshot=result["snapshot"], edits=edits) == untouched
    rejected = tmp_path / "rejected.docx"
    for batch in ([dict(kind="remove-instance", num_id=703)], [edits[3], edits[3]],
                  [edits[0], dict(kind="abstract-start", abstract_id=701, level=0, value=None)]):
        call(cli, "save-copy", edited, success=False, snapshot=result["snapshot"],
             output=str(rejected), edits=batch)
        assert not rejected.exists()
    state = parts(edited)
    root = etree.fromstring(state["word/numbering.xml"])
    root.find("{*}abstractNum[@{http://schemas.openxmlformats.org/wordprocessingml/2006/main}abstractNumId='701']/{*}lvl").append(
        etree.Comment("keep opaque formatting"))
    state["word/numbering.xml"] = etree.tostring(root)
    opaque = tmp_path / "opaque.docx"
    opaque.write_bytes(pack(state))
    seen = call(cli, "inspect", opaque)
    assert "unmodeled" in call(cli, "save-copy", opaque, success=False, snapshot=seen["snapshot"],
                               output=str(rejected), edits=[edits[0]])
    assert not rejected.exists()


def test_style_retarget_transport(cli, tmp_path):
    from docx.enum.style import WD_STYLE_TYPE
    from rocx.editor import StyleEdit

    source, target = tmp_path / "styles.docx", tmp_path / "retargeted.docx"
    word = independent_document()
    word.styles.add_style("Migration Source Test", WD_STYLE_TYPE.PARAGRAPH)
    word.styles.add_style("Migration Destination Test", WD_STYLE_TYPE.PARAGRAPH)
    word.add_paragraph("Keep body", "Migration Source Test")
    word.sections[0].header.paragraphs[0].style = "Migration Source Test"
    word.save(source)
    edit = dict(kind="style-retarget", id="MigrationSourceTest", replacement_id="MigrationDestinationTest")
    snapshot = call(cli, "inspect", source)["snapshot"]
    call(cli, "save-copy", source, success=False, snapshot=snapshot, output=str(target),
         edits=[edit, dict(kind="style-field", id=edit["replacement_id"], field="b", value="1")])
    assert not target.exists()
    checked_copy(cli, source, target, [edit])
    api = ImportedDocument.open(source.read_bytes())
    assert api.preview([], style_edits=[
        StyleEdit.retarget(api.style_anchor(edit["id"]), api.style_anchor(edit["replacement_id"])),
    ]).to_docx() == target.read_bytes()
    readback = docx.Document(target)
    assert readback.paragraphs[0].style.name == "Migration Destination Test"
    assert readback.sections[0].header.paragraphs[0].style.name == "Migration Destination Test"
    assert "Migration Source Test" not in readback.styles


def test_full_numbering_catalog_and_reciprocal_link_guards(cli, tmp_path):
    from runpy import run_path
    from rocx.editor import ResourceEdit as R

    fixture = run_path(str(FIXTURES / "generate_numbering_catalog.py"))
    source, output = tmp_path / "linked.docx", tmp_path / "edited.docx"
    source.write_bytes(fixture["source"]())
    doc = ImportedDocument.open(source.read_bytes())
    seen = call(cli, "inspect", source)
    catalog = doc.numbering_catalog
    assert seen["numbering_catalog"] == catalog
    owner = next(a for a in catalog["abstracts"] if a["abstract_id"] == 710)
    alias = next(a for a in catalog["abstracts"] if a["abstract_id"] == 711)
    concrete = next(i for i in catalog["instances"] if i["num_id"] == 712)
    assert alias == dict(abstract_id=711, num_style_link="FixtureList", levels=[])
    assert owner["style_link"] == concrete["style_ids"][0] == "FixtureList"
    level, = owner["levels"]
    assert not level["has_opaque_content"]
    assert concrete["level_overrides"] == [level]
    assert concrete["start_overrides"] == [dict(level=0, start=6)]
    assert {k: level[k] for k in ("level", "start", "format", "text", "restart", "is_legal",
                                  "suffix", "alignment", "paragraph_style")} == dict(
        level=0, start=1, format="lowerRoman", text="(%1)", restart=0, is_legal=False,
        suffix="nothing", alignment="end", paragraph_style="Heading1")
    assert level["label_format"] == dict(font="Aptos", font_east_asia="Mincho", font_complex_script="Arial",
        bold=False, italic=True, bold_complex_script=False, size=12.5, size_complex_script=13,
        underline="double", subscript=True, color=dict(theme="accent2", tint=16),
        language=dict(latin="en-US", east_asia="ja-JP", bidi="ar"))
    assert level["paragraph_format"] == dict(keep_with_next=False, alignment="center",
        left_indent=24, right_indent=2, first_line_indent=-12, spacing_before=1, spacing_after=2,
        line_height=dict(kind="exact", value=10),
        tab_stops=[dict(position=12, alignment="right", leader="dot")])
    edits = [
        dict(kind="replace-abstract", abstract_id=710, levels=[fixture["LEVEL"]]),
        dict(kind="paragraph-numbering", target=0, value=dict(id=713, level=0)),
        dict(kind="document-default", field="trackRevisions", value=False),
        dict(kind="metadata", field="comments", value="Numbering fixture review"),
    ]
    result = checked_copy(cli, source, output, edits)
    assert output.read_bytes() == doc.preview([], resources=fixture["edits"](doc)).to_docx()
    original, changed = parts(source), parts(output)
    assert {p for p in original if original[p] != changed[p]} == {
        "word/numbering.xml", "word/document.xml", "word/settings.xml", "docProps/core.xml"}
    assert next(i for i in result["numbering_catalog"]["instances"] if i["num_id"] == 712) == concrete
    assert next(a for a in result["numbering_catalog"]["abstracts"] if a["abstract_id"] == 711) == alias
    typed = next(a for a in result["numbering_catalog"]["abstracts"] if a["abstract_id"] == 710)
    assert typed["style_link"] == "FixtureList" and typed["levels"][0]["paragraph_style"] == "Heading2"
    assert docx.Document(output).paragraphs[0]._p.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["713"]
    assert call(cli, "inspect", output, metadata_fields=["comments", "description"])["metadata"] == {
        "comments": "Numbering fixture review", "description": "Numbering fixture review"}
    noop = call(cli, "preview", output, snapshot=result["snapshot"], edits=[])
    assert call(cli, "preview", output, snapshot=result["snapshot"], edits=edits) == noop

    rejected = tmp_path / "rejected.docx"
    for bad in [
        dict(kind="relink-instance", num_id=712, abstract_id=0),
        dict(kind="abstract-level", abstract_id=710, level=0, value=None),
        dict(kind="abstract-level", abstract_id=711, level=0, value=fixture["LEVEL"]),
        dict(kind="remove-instance", num_id=712),
        dict(kind="abstract-level", abstract_id=710, level=0, value=fixture["LEVEL"] | {"paragraph_style": "Unknown"}),
    ]:
        call(cli, "save-copy", output, success=False, snapshot=result["snapshot"],
             edits=[bad], output=str(rejected))
        assert not rejected.exists()
    call(cli, "preview", source, success=False, snapshot=seen["snapshot"],
         edits=[edits[3], dict(kind="metadata", field="description", value=None)])
    for fault, error in [("opaque", "unmodeled"), ("dangling", "nonreciprocal"), ("nonreciprocal", "nonreciprocal")]:
        path = tmp_path / f"{fault}.docx"
        path.write_bytes(fixture["source"](fault))
        bad = ImportedDocument.open(path.read_bytes())
        if fault == "opaque":
            assert next(a for a in bad.numbering_catalog["abstracts"] if a["abstract_id"] == 710)["levels"][0]["has_opaque_content"]
        seen = call(cli, "inspect", path)
        assert error in call(cli, "save-copy", path, success=False, snapshot=seen["snapshot"],
                             edits=[edits[0]], output=str(rejected))
        assert not rejected.exists()
        with pytest.raises(ValueError, match=error):
            bad.preview([], resources=fixture["edits"](bad))


def test_physical_owner_numbering_style_and_section_layout_transport(cli, tmp_path):
    import asyncio
    from runpy import run_path

    parity = run_path(str(ROOT / "javascript/tests/parity/editor.py"))
    source = tmp_path / "physical-resources-source.docx"
    stages = [tmp_path / name for name in ("staged.docx", "table.docx", "physicalResources.docx")]
    parity["generate_physical_resources"](source)
    before = source.read_bytes()
    owner = "word/footer1.xml"
    story = dict(kind="story", part=owner, block_index=0)
    reference = dict(id=773, level=0)
    checked_copy(cli, source, stages[0], [
        dict(kind="style-add", definition=dict(kind="table", name="Owner Table Style")),
        dict(kind="style-add", definition=dict(kind="numbering", name="Owner Numbering Style", numbering_id=773)),
        dict(kind="add-abstract", abstract_id=772, levels=[
            dict(level=0, start=1, format="decimal", text="%1.")]),
        dict(kind="add-instance", num_id=773, abstract_id=772),
        dict(kind="story-paragraph-numbering", target=story, value=reference),
    ])
    seen = call(cli, "inspect", stages[0], table_owner=owner, numbering_story_targets=[story])
    assert seen["story_paragraph_numbering"] == [reference]
    table, = seen["part_tables"]
    physical = dict(kind="part-tables", owner_part=owner, operations=[
        dict(path=table["path"], operation=dict(kind="patch-property", field="style", source=dict(
            rows=[["Donor text must not move"]], style="Owner Table Style")))])
    checked_copy(cli, stages[0], stages[1], [physical])
    layouts = [
        dict(kind="section-layout", target=0, field="margin_left", value=1440),
        dict(kind="section-layout", target=0, field="border_top", value=dict(size=12, color="123456")),
    ]
    checked_copy(cli, stages[1], stages[2], layouts)
    assert source.read_bytes() == before
    # Independently replay the core API against the same source, and check the
    # changed-part sets and opaque XML for each stage rather than bytes alone.
    assert tuple(path.read_bytes() for path in stages) == asyncio.run(parity["physical_resources"](source))
    parity["verify_physical_resources"](tmp_path)
    done = call(cli, "inspect", stages[2], table_owner=owner, numbering_story_targets=[story])
    assert done["story_paragraph_numbering"] == [reference]
    assert done["part_tables"][0]["properties"]["style"] == {"val": "OwnerTableStyle"}
    reject = tmp_path / "rejected.docx"
    for edits, reason in [
        ([dict(kind="remove-instance", num_id=773)], "references"),
        ([dict(kind="section-layout", target=0, field="border_right", value=None)], "opaque"),
        ([physical, *layouts], "cannot be mixed"),
    ]:
        error = call(cli, "save-copy", stages[2], success=False, snapshot=done["snapshot"],
                     output=str(reject), edits=edits)
        assert reason in error and not reject.exists()


async def test_staged_table_and_numbering_share_one_cli_batch(cli, tmp_path):
    from rocx.editor import ResourceEdit as R, StyleEdit as S, TableStructureEdit as T

    source, output = tmp_path / "source.docx", tmp_path / "staged.docx"
    word = independent_document()
    word.add_table(1, 1).cell(0, 0).text = "replace"
    word.add_table(1, 1).cell(0, 0).text = "remove"
    word.add_paragraph("number after deletion")
    word.sections[0].footer.add_table(1, 1, docx.shared.Inches(2))
    word.save(source)
    original = source.read_bytes()
    owner = "word/footer1.xml"
    inspect = dict(table_owner=owner, numbering_targets=[2])
    seen = call(cli, "inspect", source, **inspect)
    assert seen["paragraph_numbering"] == {"2": None}
    footer, = seen["part_tables"]
    assert footer["owner_part"] == owner

    level = dict(level=0, start=1, format="decimal", text="%1.")
    donor = dict(rows=[["staged"]], style="CLI Staged", alignment="center")
    edits = [
        dict(kind="table-structure", target=0, operation=dict(kind="replace-node", source=donor)),
        dict(kind="table-structure", target=1, operation=dict(kind="remove-table")),
        dict(kind="style-add", definition=dict(kind="table", name=donor["style"])),
        dict(kind="add-instance", num_id=91, abstract_id=90),
        dict(kind="add-abstract", abstract_id=90, levels=[level]),
        dict(kind="paragraph-numbering", target=2, value=dict(id=91, level=0)),
    ]
    incompatible = dict(kind="part-tables", owner_part=owner, operations=[
        dict(path=footer["path"], operation=dict(kind="insert-row", index=1))])
    for invalid, reason in [(dict(kind="remove-instance", num_id=1), "references"),
                            (incompatible, "only edit")]:
        error = call(cli, "save-copy", source, success=False, snapshot=seen["snapshot"],
                     output=str(output), edits=[*edits, invalid])
        assert reason in error and not output.exists() and source.read_bytes() == original

    checked_copy(cli, source, output, edits)
    doc = ImportedDocument.open(original)
    first, second, paragraph = doc.body
    table = await d.Table([d.TableRow([d.TableCell([d.Paragraph(text="staged")])])],
                          style=donor["style"], alignment=donor["alignment"]).resolve()
    assert output.read_bytes() == doc.preview([], table_structure=[
        T.replace_node(first.anchor, table), T.remove_table(second.anchor)
    ], style_edits=[S.add(d.TableStyleDefinition(donor["style"]))], resources=[
        R.add_instance(91, 90), R.add_abstract(90, [d.NumberingLevel(**level)]),
        R.paragraph_numbering(paragraph.anchor, (91, 0)),
    ]).to_docx()
    saved = docx.Document(output)
    assert len(saved.tables) == 1 and saved.tables[0].style.name == donor["style"]
    assert saved.paragraphs[0].text == "number after deletion"
    assert saved.paragraphs[0]._p.xpath("w:pPr/w:numPr/w:numId/@w:val") == ["91"]
    after = call(cli, "inspect", output, **(inspect | {"numbering_targets": [1]}))
    assert after["part_tables"] == [footer]
    assert after["paragraph_numbering"] == {"1": {"id": 91, "level": 0}}
    before, changed = parts(source), parts(output)
    assert before.keys() == changed.keys()
    assert {p for p in before if before[p] != changed[p]} == {
        "word/document.xml", "word/styles.xml", "word/numbering.xml"}

    noop = tmp_path / "noop.docx"
    checked_copy(cli, output, noop, [
        dict(kind="table-structure", target=0, operation=dict(
            kind="patch-property", field="alignment", source=donor)),
        dict(kind="paragraph-numbering", target=1, value=dict(id=91, level=0)),
    ])
    assert parts(noop) == changed
