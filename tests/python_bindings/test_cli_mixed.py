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
