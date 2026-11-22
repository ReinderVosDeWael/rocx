"""Python inspection and anchored edits use the native imported-document core."""

import io
from pathlib import Path

import docx
import pytest

from rocx import declarative as d
from rocx.editor import (
    BodyParagraph,
    BodyTable,
    HeaderImageChange,
    HeaderImageEdit,
    HeaderImagePreview,
    ImportedDocument,
    ImportedHeader,
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


@pytest.mark.parametrize("mode,selected,affected", [
    ("shared", 0, [0, 1]),
    ("section_local", 1, [1]),
])
async def test_header_image_preview(mode, selected, affected):
    original = await d.Document([
        d.Section([d.Paragraph(text="first")], headers={
            "default": d.Header([d.Paragraph(text="header")]),
        }),
        d.Section([d.Paragraph(text="second")]),
    ]).to_bytes()
    document = ImportedDocument.open(original)
    headers = document.default_headers
    assert all(isinstance(header, ImportedHeader) for header in headers)
    assert [(h.section_index, h.inherited) for h in headers] == [(0, False), (1, False)]
    assert headers[0].part_name == headers[1].part_name
    image = await d.ImageRun(
        data=(Path(__file__).parents[1] / "fixtures/images/sample.png").read_bytes()
    ).resolve()
    preview = document.preview_header_image(
        HeaderImageEdit(headers[selected].anchor, image, mode=mode)
    )
    assert isinstance(preview, HeaderImagePreview)
    change = preview.change
    assert isinstance(change, HeaderImageChange)
    assert change.mode == mode
    assert change.source_part == headers[selected].part_name
    assert change.affected_sections == affected
    assert change.restored_section is None
    assert (change.output_part == change.source_part) == (mode == "shared")
    saved = docx.Document(io.BytesIO(preview.to_docx()))
    assert [len(s.header._element.xpath(".//w:drawing")) for s in saved.sections] == (
        [1, 1] if mode == "shared" else [0, 1]
    )
    assert document.default_headers[1].part_name == headers[0].part_name


async def test_header_image_edit_rejects_invalid_inputs_and_foreign_anchor():
    original = await d.Document([
        d.Section([d.Paragraph(text="body")], headers={
            "default": d.Header([d.Paragraph(text="header")]),
        })
    ]).to_bytes()
    first, second = ImportedDocument.open(original), ImportedDocument.open(original)
    anchor = first.default_headers[0].anchor
    image = await d.ImageRun(
        data=(Path(__file__).parents[1] / "fixtures/images/sample.png").read_bytes()
    ).resolve()
    with pytest.raises(TypeError):
        HeaderImageEdit(anchor, image)
    with pytest.raises(TypeError):
        HeaderImageEdit(anchor, image, "shared")
    with pytest.raises(ValueError):
        HeaderImageEdit(anchor, image, mode="invalid")
    with pytest.raises(TypeError):
        HeaderImageEdit(anchor, await d.Paragraph(text="not image").resolve(), mode="shared")
    with pytest.raises(TypeError):
        HeaderImageEdit(anchor, d.ImageRun(data=b""), mode="shared")
    with pytest.raises(ValueError):
        second.preview_header_image(HeaderImageEdit(anchor, image, mode="shared"))
