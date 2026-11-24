"""Exercise the JSON process boundary, with an installed Python API byte oracle."""
import json
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
