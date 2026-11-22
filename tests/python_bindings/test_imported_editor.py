"""Python inspection and anchored edits use the native imported-document core."""

import io

import docx
import pytest

from rocx import declarative as d
from rocx.editor import (
    BodyParagraph,
    BodyTable,
    ImportedDocument,
    ParagraphAlignmentEdit,
)


async def test_inspect_preview_and_save_alignment():
    table = d.Table([d.TableRow([d.TableCell([d.Paragraph(text="cell")])])])
    original = await d.Document([d.Section([
        d.Paragraph(text="hello", alignment="right"), table,
    ])]).to_bytes()
    document = ImportedDocument.open(original)
    paragraph = document.body[0]
    assert isinstance(paragraph, BodyParagraph)
    assert paragraph.text == "hello"
    assert paragraph.alignment == "right"
    assert paragraph.alignment_xml == "right"
    assert isinstance(paragraph.has_opaque_content, bool)
    assert isinstance(document.body[1], BodyTable)

    preview = document.preview([ParagraphAlignmentEdit(paragraph.anchor, "center")])
    assert [(c.body_index, c.before, c.after) for c in preview.changes] == [
        (0, "right", "center")
    ]
    assert document.body[0].alignment == "right"
    output = preview.to_docx()
    assert isinstance(output, bytes)
    assert docx.Document(io.BytesIO(output)).paragraphs[0].alignment == 1
    assert ImportedDocument.open(output).body[0].alignment == "center"


async def test_invalid_edit_and_foreign_anchor_are_rejected():
    original = await d.Document([d.Section([d.Paragraph(text="hello")])]).to_bytes()
    first = ImportedDocument.open(original)
    second = ImportedDocument.open(original)
    with pytest.raises(ValueError):
        ParagraphAlignmentEdit(first.body[0].anchor, "invalid")
    with pytest.raises(ValueError):
        second.preview([ParagraphAlignmentEdit(first.body[0].anchor, "left")])
    with pytest.raises(ValueError):
        ImportedDocument.open(b"not a docx")
