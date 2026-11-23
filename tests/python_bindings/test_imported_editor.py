"""Python inspection and anchored edits use the native imported-document core."""

import io
from pathlib import Path
from zipfile import ZipFile

import docx
import pytest
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.shared import Pt

from rocx import declarative as d
from rocx.editor import (
    BodyOpaque,
    BodyParagraph,
    BodyTable,
    HeaderImageChange,
    HeaderImageEdit,
    HeaderImagePreview,
    ImportedDocument,
    ImportedHeader,
    ImportedHeaderParagraph,
    ImportedCellParagraph,
    ImportedTableCell,
    ImportedTableRow,
    ParagraphAlignmentEdit,
    TableInsertion,
    TableInsertionChange,
    TableAnchor,
)

FIXTURES = Path(__file__).parents[1] / "fixtures"


def _parts(data):
    with ZipFile(io.BytesIO(data)) as package:
        return {name: package.read(name) for name in package.namelist()}


def test_python_docx_header_story_with_opaque_table():
    source = docx.Document()
    header = source.sections[0].header
    paragraph = header.paragraphs[0]
    paragraph.add_run("A").add_tab()
    paragraph.add_run("B")
    header.add_table(rows=1, cols=1, width=docx.shared.Inches(1))
    source.add_section()
    stream = io.BytesIO()
    source.save(stream)
    data = stream.getvalue()
    independently_read = docx.Document(io.BytesIO(data))
    assert independently_read.sections[0].header.paragraphs[0].text == "A\tB"
    first, second = ImportedDocument.open(data).default_headers
    assert first.part_name == second.part_name
    assert [first.inherited, second.inherited] == [False, True]
    assert [(item.kind, item.value) for item in first.blocks[0].inline_items] == [
        ("text", "A"), ("tab", None), ("text", "B"),
    ]
    assert isinstance(first.blocks[1], BodyOpaque)
    assert first.blocks[1].xml == "w:tbl"
    assert _parts(data)[first.part_name.lstrip("/")].startswith(b"<?xml")


async def test_python_docx_fixture_body_edits():
    source = (FIXTURES / "imported-editor.docx").read_bytes()
    before = docx.Document(io.BytesIO(source))
    assert before.paragraphs[0].text == "Patch me"
    assert before.paragraphs[0].alignment is None
    assert before.paragraphs[0].runs[0].bold is True
    assert before.paragraphs[0].paragraph_format.space_after == Pt(12)

    imported = ImportedDocument.open(source)
    assert [(s.body_start, s.body_end) for s in imported.sections] == [(0, 3), (3, 4)]
    assert [(item.kind, item.value) for item in imported.body[0].inline_items] == [
        ("text", "Patch"), ("text", " me"),
    ]
    table = await d.Table([
        d.TableRow([d.TableCell([d.Paragraph(text="inserted")])]),
    ]).resolve()
    preview = imported.preview(
        [ParagraphAlignmentEdit(imported.body[0].anchor, "center")],
        table_insertions=[TableInsertion(imported.body[1].anchor, table)],
    )
    assert preview.insertions[0].after_body_index == 1
    output = preview.to_docx()
    assert [(s.body_start, s.body_end) for s in ImportedDocument.open(output).sections] == [
        (0, 4), (4, 5),
    ]
    after = docx.Document(io.BytesIO(output))
    patch, anchor, inserted, section_break, next_paragraph = after.iter_inner_content()
    assert patch.text == "Patch me"
    assert patch.alignment == WD_ALIGN_PARAGRAPH.CENTER
    assert patch.runs[0].bold is True
    assert patch.paragraph_format.space_after == Pt(12)
    assert anchor.text == "Insert table after me"
    assert inserted.cell(0, 0).text == "inserted"
    assert section_break.text == ""
    assert next_paragraph.text == "Second section"
    originals, edited = _parts(source), _parts(output)
    assert originals.keys() == edited.keys()
    assert {name for name in originals if originals[name] != edited[name]} == {
        "word/document.xml"
    }


@pytest.mark.parametrize("mode,affected,restored,drawings", [
    ("shared", [0, 1], None, [1, 1]),
    ("section_local", [0], 1, [1, 0]),
])
async def test_python_docx_fixture_header_edits(mode, affected, restored, drawings):
    source = (FIXTURES / "imported-editor.docx").read_bytes()
    before = docx.Document(io.BytesIO(source))
    assert [s.header.is_linked_to_previous for s in before.sections] == [False, True]
    imported = ImportedDocument.open(source)
    first, second = imported.default_headers
    assert first.part_name == second.part_name
    assert [first.inherited, second.inherited] == [False, True]
    for header in (first, second):
        assert isinstance(header.blocks[0], ImportedHeaderParagraph)
        assert "".join(item.value for item in header.blocks[0].inline_items if item.kind == "text") == "Shared header link"

    image = await d.ImageRun(data=(FIXTURES / "images/sample.png").read_bytes()).resolve()
    preview = imported.preview_header_image(HeaderImageEdit(first.anchor, image, mode=mode))
    assert preview.change.affected_sections == affected
    assert preview.change.restored_section == restored
    output = preview.to_docx()
    after = docx.Document(io.BytesIO(output))
    headers = [s.header for s in after.sections]
    assert [len(h._element.xpath(".//w:drawing")) for h in headers] == drawings
    for header in headers:
        assert header.paragraphs[0].text == "Shared header link"
        assert header.paragraphs[0].hyperlinks[0].url == "https://example.invalid/header"
    reopened = ImportedDocument.open(output).default_headers
    assert [h.part_name for h in reopened] == [preview.change.output_part, first.part_name]
    if mode == "section_local":
        originals, edited = _parts(source), _parts(output)
        original_part = first.part_name.lstrip("/")
        directory, filename = original_part.rsplit("/", 1)
        relationships = f"{directory}/_rels/{filename}.rels"
        assert edited[original_part] == originals[original_part]
        assert edited[relationships] == originals[relationships]


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
    inspected = document.body[1]
    assert isinstance(inspected.anchor, TableAnchor)
    assert isinstance(inspected.children[0], ImportedTableRow)
    assert isinstance(inspected.children[0].children[0], ImportedTableCell)
    block = inspected.children[0].children[0].blocks[0]
    assert isinstance(block, ImportedCellParagraph)
    assert [(item.kind, item.value) for item in block.inline_items] == [("text", "cell")]
    with pytest.raises(TypeError):
        ParagraphAlignmentEdit(inspected.anchor, "center")

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


def test_independent_table_readback_keeps_unsupported_blocks_and_package():
    source = docx.Document()
    source.add_paragraph("anchor")
    cell = source.add_table(rows=1, cols=1).cell(0, 0)
    cell.text = "independent"
    cell.add_table(rows=1, cols=1).cell(0, 0).text = "nested"
    data = io.BytesIO()
    source.save(data)
    original = data.getvalue()
    document = ImportedDocument.open(original)
    blocks = document.body[1].children[0].children[0].blocks
    assert [(item.kind, item.value) for item in blocks[0].inline_items] == [("text", "independent")]
    assert blocks[1].xml == "w:tbl"
    assert _parts(document.preview([]).to_docx()) == _parts(original)


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
