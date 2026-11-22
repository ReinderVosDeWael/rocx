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
    TableInsertion,
    TableInsertionChange,
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
    assert preview.insertions == []
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


async def test_insert_resolved_table_with_alignment_edit():
    original = await d.Document([d.Section([
        d.Paragraph(text="first"), d.Paragraph(text="second"),
    ])]).to_bytes()
    document = ImportedDocument.open(original)
    anchor = document.body[0].anchor
    table = await d.Table([
        d.TableRow([d.TableCell([d.Paragraph(text="inserted")])]),
    ]).resolve()
    preview = document.preview(
        [ParagraphAlignmentEdit(anchor, "center")],
        table_insertions=[TableInsertion(anchor, table)],
    )
    assert len(preview.insertions) == 1
    change = preview.insertions[0]
    assert isinstance(change, TableInsertionChange)
    assert (change.after_body_index, change.rows, change.columns) == (0, 1, 1)
    assert preview.changes[0].after == "center"
    assert len(document.body) == 2
    saved = docx.Document(io.BytesIO(preview.to_docx()))
    assert [p.text for p in saved.paragraphs] == ["first", "second"]
    assert saved.paragraphs[0].alignment == 1
    assert saved.tables[0].cell(0, 0).text == "inserted"
    assert isinstance(ImportedDocument.open(preview.to_docx()).body[1], BodyTable)


async def test_table_insertion_rejects_invalid_inputs_and_foreign_anchor():
    original = await d.Document([d.Section([d.Paragraph(text="hello")])]).to_bytes()
    first = ImportedDocument.open(original)
    second = ImportedDocument.open(original)
    anchor = first.body[0].anchor
    table = await d.Table([
        d.TableRow([d.TableCell([d.Paragraph(text="cell")])]),
    ]).resolve()
    with pytest.raises(TypeError):
        TableInsertion(anchor, d.Table([]))
    with pytest.raises(TypeError):
        TableInsertion(anchor, await d.Paragraph(text="not a table").resolve())
    with pytest.raises((TypeError, ValueError)):
        TableInsertion(anchor, await d.Table([], condition=False).resolve())
    with pytest.raises(ValueError):
        TableInsertion(anchor, await d.Table([]).resolve())
    with pytest.raises(ValueError):
        TableInsertion(anchor, await d.Table([d.TableRow([])]).resolve())
    with pytest.raises(ValueError):
        second.preview([], table_insertions=[TableInsertion(anchor, table)])
    with pytest.raises(TypeError):
        first.preview([], [TableInsertion(anchor, table)])
