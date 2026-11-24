import io

import docx
import pytest
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from rocx.editor import ImportedDocument, RunTextEdit, TextRangeEdit


def open_document(source):
    file = io.BytesIO()
    source.save(file)
    return ImportedDocument.open(file.getvalue())


def test_ranges_use_original_scalar_offsets_and_preserve_noops():
    source = docx.Document()
    p = source.add_paragraph()
    p.add_run("a😀").bold = True
    p.add_run("e\u0301z").italic = True
    doc = open_document(source)
    p = doc.body[0]

    def edit(start, end, text):
        return TextRangeEdit(p.anchor, start, end, text)

    noop = doc.preview([], text_ranges=[edit(1, 3, "😀e")])
    assert noop.run_text_changes == []
    assert noop.to_docx() == doc.preview([]).to_docx()
    edits = [edit(3, 4, ""), edit(1, 3, " first "), edit(4, 5, "tail")]
    preview = doc.preview([], text_ranges=edits)
    paragraph = docx.Document(io.BytesIO(preview.to_docx())).paragraphs[0]
    assert [(r.text, r.bold, r.italic) for r in paragraph.runs] == [
        ("a first ", True, None), ("tail", None, True)]
    assert [(r.inline_index, r.before, r.after) for r in preview.run_text_changes] == [
        (0, "a😀", "a first "), (1, "e\u0301z", "tail")]
    for bad in [edit(0, 0, "x"), edit(3, 1, "x"), edit(0, 6, "x"),
                edit(2, 4, "overlap"), edit(1, 3, "duplicate"), edit(0, 1, "\t")]:
        with pytest.raises(ValueError):
            doc.preview([], text_ranges=[edits[1], bad])
    for extra in [dict(run_removals=[p.inline_structure[1].anchor]),
                  dict(run_text=[RunTextEdit(p.inline_structure[0].anchor, "collision")])]:
        with pytest.raises(ValueError):
            doc.preview([], text_ranges=edits, **extra)
    with pytest.raises(ValueError, match="snapshot"):
        ImportedDocument.open(preview.to_docx()).preview([], text_ranges=edits)
    assert doc.preview([], text_ranges=edits[::-1]).to_docx() == preview.to_docx()


@pytest.mark.parametrize("boundary", [
    "w:hyperlink", "w:ins", "w:del", "w:fldSimple", "w:sdt", "w:proofErr",
    "w:bookmarkStart", "w:rPrChange", "raw", "mixed", "previous-field",
])
def test_ambiguous_projection_or_active_boundary_rejects_batch(boundary):
    from lxml import etree

    source = docx.Document()
    safe = source.add_paragraph("safe")
    p = source.add_paragraph("left")
    if boundary == "raw":
        p._p.append(etree.Comment("opaque"))
    elif boundary == "mixed":
        p.add_run("tab\there")
    elif boundary == "previous-field":
        field = OxmlElement("w:fldChar")
        field.set(qn("w:fldCharType"), "begin")
        source.add_paragraph().add_run()._r.append(field)
        p._p.addprevious(source.paragraphs[-1]._p)
    elif boundary == "w:rPrChange":
        p.runs[0]._r.get_or_add_rPr().append(OxmlElement(boundary))
    else:
        p._p.append(OxmlElement(boundary))
    p.add_run("right")
    doc = open_document(source)
    valid = TextRangeEdit(doc.body[0].anchor, 0, len(safe.text), "ok")
    with pytest.raises(ValueError):
        doc.preview([], text_ranges=[valid, TextRangeEdit(doc.body[-1].anchor, 1, 6, "cross")])
    assert doc.body[0].text == "safe"
    assert doc.preview([], text_ranges=[valid]).run_text_changes[0].after == "ok"
