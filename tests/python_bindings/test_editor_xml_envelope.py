"""Document-level XML nodes survive edits to independently generated DOCX."""

import io
from zipfile import ZipFile

import docx
from docx.enum.text import WD_ALIGN_PARAGRAPH

from rocx.editor import ImportedDocument, ParagraphAlignmentEdit


def parts(data):
    with ZipFile(io.BytesIO(data)) as package:
        return {name: package.read(name) for name in package.namelist()}


def test_document_envelope_survives_alignment_edit():
    source = docx.Document()
    source.add_paragraph("independent")
    stream = io.BytesIO()
    source.save(stream)
    original_parts = parts(stream.getvalue())
    xml = original_parts["word/document.xml"]
    declaration, root = xml.split(b"?>", 1)
    original_parts["word/document.xml"] = (
        declaration + b"?><!--before--><?before data?>" + root
        + b"<?after data?><!--after-->"
    )
    stream = io.BytesIO()
    with ZipFile(stream, "w") as package:
        for name, payload in original_parts.items():
            package.writestr(name, payload)

    imported = ImportedDocument.open(stream.getvalue())
    output = imported.preview(
        [ParagraphAlignmentEdit(imported.body[0].anchor, "center")]
    ).to_docx()
    edited = parts(output)
    assert edited.keys() == original_parts.keys()
    assert {name for name in edited if edited[name] != original_parts[name]} == {
        "word/document.xml"
    }
    result = edited["word/document.xml"]
    assert result.index(b"<!--before-->") < result.index(b"<?before data?>")
    assert result.index(b"<?before data?>") < result.index(b"<w:document")
    assert result.index(b"</w:document>") < result.index(b"<?after data?>")
    assert result.index(b"<?after data?>") < result.index(b"<!--after-->")
    paragraph = docx.Document(io.BytesIO(output)).paragraphs[0]
    assert paragraph.text == "independent"
    assert paragraph.alignment == WD_ALIGN_PARAGRAPH.CENTER
