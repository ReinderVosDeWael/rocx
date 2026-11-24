"""Python inspection and anchored edits use the native imported-document core."""

import io
from pathlib import Path
from xml.etree import ElementTree
from zipfile import ZipFile

import docx
import pytest
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.shared import Pt

from rocx import declarative as d
from rocx.editor import (
    BodyOpaque,
    BodyContentControl,
    BodyParagraph,
    BodyTable,
    CellAlignmentEdit,
    HeaderImageChange,
    HeaderImageEdit,
    HeaderImagePreview,
    ImportedDocument,
    ImportedHeader,
    ImportedHeaderParagraph,
    ImportedHyperlink,
    ImportedRevision,
    ImportedRun,
    ImportedStory,
    ImportedStoryParagraph,
    InlineAnchor,
    ImportedCellParagraph,
    ImportedTableCell,
    ImportedTableRow,
    ParagraphAlignmentEdit,
    ParagraphInsertion,
    RunTextEdit,
    TableInsertion,
    TableInsertionChange,
    TableAnchor,
)

FIXTURES = Path(__file__).parents[1] / "fixtures"


def test_inline_revision_inspection_preserves_opaque_boundaries():
    from docx.oxml import OxmlElement
    from docx.oxml.ns import qn

    source = docx.Document()
    paragraph = source.add_paragraph()
    for name, text in [("ins", "new"), ("del", "old")]:
        revision = OxmlElement("w:" + name)
        revision.set(qn("w:id"), "7")
        revision.set(qn("w:author"), "Ada")
        run = OxmlElement("w:r")
        value = OxmlElement("w:t" if name == "ins" else "w:delText")
        value.text = text
        run.append(value)
        run.append(OxmlElement("w:drawing"))
        revision.append(run)
        paragraph._p.append(revision)
    buf = io.BytesIO()
    source.save(buf)
    imported = ImportedDocument.open(buf.getvalue())
    insertion, deletion = imported.body[0].inline_structure
    assert isinstance(insertion, ImportedRevision)
    assert (insertion.kind, insertion.id, insertion.author, insertion.date) == ("insertion", "7", "Ada", None)
    assert deletion.kind == "deletion"
    for revision, expected in [(insertion, "new"), (deletion, "old")]:
        run, = revision.children
        assert [(item.kind, item.value) for item in run.items] == [("text", expected), ("opaque", "w:drawing")]
        assert imported.inline_at(revision.anchor).kind == revision.kind
        assert imported.inline_at(run.anchor).items[0].value == expected
        with pytest.raises(ValueError):
            imported.preview([], run_text=[RunTextEdit(run.anchor, "unsafe")])
        with pytest.raises(ValueError):
            ImportedDocument.open(buf.getvalue()).inline_at(run.anchor)
    assert _parts(imported.preview([]).to_docx()) == _parts(buf.getvalue())


async def test_mixed_cell_paragraph_and_run_edits_use_original_indices_atomically():
    from docx.enum.table import WD_CELL_VERTICAL_ALIGNMENT

    source = docx.Document()
    source.add_paragraph("start")
    source.add_table(rows=1, cols=1).cell(0, 0).text = "cell"
    tail = source.add_paragraph("keep")
    tail.add_run("remove")
    stream = io.BytesIO()
    source.save(stream)
    original = stream.getvalue()
    imported = ImportedDocument.open(original)
    start, table, tail = imported.body
    target = tail.inline_structure[1].anchor
    batch = dict(
        paragraph_insertions=[ParagraphInsertion(start.anchor, await d.Paragraph(text="inserted").resolve())],
        cell_alignments=[CellAlignmentEdit(table.anchor, 0, 0, "bottom")],
        run_removals=[target],
    )
    preview = imported.preview([], **batch)
    assert preview.paragraph_insertions == [0]
    assert [(c.body_index, c.row_index, c.cell_index) for c in preview.cell_alignment_changes] == [(1, 0, 0)]
    assert [(r.body_index, r.inline_index, r.before) for r in preview.run_removals] == [(2, 1, "remove")]
    output = preview.to_docx()
    readback = docx.Document(io.BytesIO(output))
    assert [p.text for p in readback.paragraphs] == ["start", "inserted", "keep"]
    cell = readback.tables[0].cell(0, 0)
    assert (cell.text, cell.vertical_alignment) == ("cell", WD_CELL_VERTICAL_ALIGNMENT.BOTTOM)
    assert isinstance(ImportedDocument.open(output).body[2], BodyTable)
    original_parts, output_parts = _parts(original), _parts(output)
    assert original_parts.keys() == output_parts.keys()
    assert all(output_parts[n] == original_parts[n] for n in original_parts if n != "word/document.xml")

    with pytest.raises(ValueError, match="duplicate|conflicting"):
        imported.preview([], **batch, run_text=[RunTextEdit(target, "conflict")])
    assert _parts(imported.preview([]).to_docx()) == original_parts
    assert imported.preview([], **batch).to_docx() == output


def test_remove_runs_is_atomic_and_preserves_unrelated_parts():
    source = docx.Document()
    p = source.add_paragraph()
    p.add_run("first").italic = True
    p.add_run("keep").bold = True
    p.add_run("last")
    p.add_run("tab\tunsafe")
    stream = io.BytesIO()
    source.save(stream)
    data = stream.getvalue()
    imported = ImportedDocument.open(data)
    first, keep, last, unsafe = imported.body[0].inline_structure
    alignment = ParagraphAlignmentEdit(imported.body[0].anchor, "center")
    preview = imported.preview([alignment], run_removals=[last.anchor, first.anchor])
    assert [(c.body_index, c.inline_index, c.before) for c in preview.run_removals] == [
        (0, 0, "first"), (0, 2, "last")]
    output = preview.to_docx()
    paragraph = docx.Document(io.BytesIO(output)).paragraphs[0]
    assert paragraph.alignment == WD_ALIGN_PARAGRAPH.CENTER
    runs = paragraph.runs
    assert [(r.text, r.bold) for r in runs] == [("keep", True), ("tab\tunsafe", None)]
    assert all(value == _parts(output)[name] for name, value in _parts(data).items()
               if name != "word/document.xml")
    other = ImportedDocument.open(data).body[0].inline_structure[0]
    for removals, edits in [
        ([first.anchor, first.anchor], []),
        ([first.anchor, unsafe.anchor], []),
        ([first.anchor, other.anchor], []),
        ([first.anchor], [RunTextEdit(first.anchor, "conflict")]),
    ]:
        with pytest.raises(ValueError):
            imported.preview([], run_removals=removals, run_text=edits)
    assert _parts(imported.preview([]).to_docx()) == _parts(data)


def test_cell_alignment_on_independent_table_and_atomic_rejection():
    from docx.enum.table import WD_CELL_VERTICAL_ALIGNMENT

    source = docx.Document()
    table = source.add_table(rows=2, cols=2)
    table.cell(0, 0).text = "keep"
    table.cell(0, 0).merge(table.cell(0, 1))
    data = io.BytesIO()
    source.save(data)
    original = data.getvalue()
    imported = ImportedDocument.open(original)
    table_view = next(block for block in imported.body if isinstance(block, BodyTable))
    edit = CellAlignmentEdit(table_view.anchor, 0, 0, "center")
    preview = imported.preview([], cell_alignments=[edit])
    assert [(c.row_index, c.cell_index, c.before, c.after) for c in preview.cell_alignment_changes] == [(0, 0, None, "center")]
    result = preview.to_docx()
    cell = docx.Document(io.BytesIO(result)).tables[0].cell(0, 0)
    assert cell.text == "keep"
    assert cell.vertical_alignment == WD_CELL_VERTICAL_ALIGNMENT.CENTER
    assert all(value == _parts(result)[name] for name, value in _parts(original).items() if name != "word/document.xml")
    with pytest.raises(ValueError):
        imported.preview([], cell_alignments=[edit, edit])
    with pytest.raises(ValueError):
        ImportedDocument.open(original).preview([], cell_alignments=[edit])
    with pytest.raises(ValueError):
        imported.preview([], cell_alignments=[CellAlignmentEdit(table_view.anchor, 8, 0, "top")])
    assert _parts(imported.preview([]).to_docx()) == _parts(original)


def test_run_text_batch_preserves_formatting_and_is_atomic():
    source = docx.Document()
    p = source.add_paragraph()
    p.add_run("first").italic = True
    p.add_run("neighbor").bold = True
    p.add_run("tab\tand text")
    buf = io.BytesIO()
    source.save(buf)
    imported = ImportedDocument.open(buf.getvalue())
    first, second, mixed = imported.body[0].inline_structure
    change = RunTextEdit(first.anchor, " first & last ")
    alignment = ParagraphAlignmentEdit(imported.body[0].anchor, "center")
    preview = imported.preview([alignment], run_text=[RunTextEdit(second.anchor, "neighbor"), change])
    assert [(c.body_index, c.inline_index, c.before, c.after) for c in preview.run_text_changes] == [
        (0, 0, "first", " first & last ")]
    reopened = docx.Document(io.BytesIO(preview.to_docx())).paragraphs[0]
    assert reopened.alignment == WD_ALIGN_PARAGRAPH.CENTER
    assert reopened.runs[0].text == " first & last " and reopened.runs[0].italic
    assert reopened.runs[1].text == "neighbor" and reopened.runs[1].bold
    assert reopened.runs[2].text == "tab\tand text"
    other = ImportedDocument.open(buf.getvalue()).body[0].inline_structure[0]
    for bad in [change, RunTextEdit(mixed.anchor, "bad"), RunTextEdit(other.anchor, "stale"),
                RunTextEdit(second.anchor, "a\nb")]:
        with pytest.raises(ValueError):
            imported.preview([alignment], run_text=[change, bad])
    assert _parts(imported.preview([]).to_docx()) == _parts(buf.getvalue())


def test_entry_stories_distinguish_identical_comment_runs_and_preserve_source():
    source = docx.Document()
    paragraph = source.add_paragraph("Before")
    for author in ["Ada", "Sam"]:
        source.add_comment(paragraph.runs, text="Review", author=author, initials=author[0])
    stream = io.BytesIO()
    source.save(stream)
    data = stream.getvalue()
    imported = ImportedDocument.open(data)
    entries = imported.entry_stories
    assert [(e.kind, e.id, e.author, e.initials) for e in entries] == [
        ("comment", "0", "Ada", "A"), ("comment", "1", "Sam", "S")]
    runs = [e.blocks[0].inline_structure[-1] for e in entries]
    assert runs[0].anchor != runs[1].anchor
    owner = entries[0].part_name
    changed = imported.edit_story_text(owner, [RunTextEdit(runs[1].anchor, "Revised")])
    assert [c.text for c in docx.Document(io.BytesIO(changed)).comments] == ["Review", "Revised"]
    assert [k for k, v in _parts(data).items() if _parts(changed)[k] != v] == [owner]
    for run in runs:
        assert imported.inline_at(run.anchor).items[0].value == "Review"
    with pytest.raises(ValueError):
        ImportedDocument.open(data).inline_at(runs[0].anchor)
    output = imported.preview([ParagraphAlignmentEdit(imported.body[0].anchor, "center")]).to_docx()
    assert _parts(output)["word/comments.xml"] == _parts(data)["word/comments.xml"]
    assert [c.text for c in docx.Document(io.BytesIO(output)).comments] == ["Review", "Review"]


def _parts(data):
    with ZipFile(io.BytesIO(data)) as package:
        return {name: package.read(name) for name in package.namelist()}

def test_direct_body_control_metadata_and_preservation():
    from docx.oxml import OxmlElement
    from docx.oxml.ns import qn

    source = docx.Document()
    source.add_paragraph("outside")
    control = OxmlElement("w:sdt")
    props = OxmlElement("w:sdtPr")
    for name, value in (("tag", "customer & id"), ("alias", "Name"), ("lock", "futureLock")):
        node = OxmlElement(f"w:{name}")
        node.set(qn("w:val"), value)
        props.append(node)
    control.append(props)
    content = OxmlElement("w:sdtContent")
    content.append(source.add_paragraph("secret")._p)
    control.append(content)
    source.element.body.insert(0, control)
    stream = io.BytesIO()
    source.save(stream)
    data = stream.getvalue()
    imported = ImportedDocument.open(data)
    block = imported.body[0]
    assert isinstance(block, BodyContentControl)
    assert (block.tag, block.alias, block.lock) == ("customer & id", "Name", "futureLock")
    assert not hasattr(block, "anchor")
    assert _parts(imported.preview([]).to_docx()) == _parts(data)
    output = imported.preview([ParagraphAlignmentEdit(imported.body[1].anchor, "center")]).to_docx()
    before, after = _parts(data), _parts(output)
    assert all(before[k] == after[k] for k in before if k != "word/document.xml")
    ns = {"w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main"}
    def control_xml(parts):
        return ElementTree.tostring(ElementTree.fromstring(parts["word/document.xml"]).find(".//w:sdt", ns))
    assert control_xml(before) == control_xml(after)
    assert docx.Document(io.BytesIO(output)).paragraphs[0].text == "outside"


def test_style_inspection_matches_python_docx_and_preserves_part():
    from docx.enum.style import WD_STYLE_TYPE
    source = docx.Document()
    base = source.styles.add_style("Source Base", WD_STYLE_TYPE.PARAGRAPH)
    child = source.styles.add_style("Source Child", WD_STYLE_TYPE.PARAGRAPH)
    child.base_style = base
    source.add_paragraph("text", style=child)
    stream = io.BytesIO()
    source.save(stream)
    data = stream.getvalue()
    imported = ImportedDocument.open(data)
    styles = imported.styles
    assert styles.part_name == "word/styles.xml"
    style = next(item for item in styles.styles if item.id == child.style_id)
    assert (style.kind, style.kind_xml, style.name, style.based_on) == (
        "paragraph", "paragraph", child.name, base.style_id)
    assert _parts(imported.preview([]).to_docx())["word/styles.xml"] == _parts(data)["word/styles.xml"]


def test_style_inspection_missing_and_unrecognized_kinds():
    stream = io.BytesIO()
    docx.Document().save(stream)
    contents = _parts(stream.getvalue())
    contents["word/styles.xml"] = (
        b'<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        b'<w:style w:type="future" w:styleId="A"/><w:style/></w:styles>'
    )
    rewritten = io.BytesIO()
    with ZipFile(rewritten, "w") as archive:
        for name, data in contents.items():
            archive.writestr(name, data)
    styles = ImportedDocument.open(rewritten.getvalue()).styles.styles
    assert [(s.id, s.kind, s.kind_xml, s.name, s.based_on) for s in styles] == [
        ("A", "unsupported", "future", None, None), (None, "missing", None, None, None)]

    def package(parts):
        output = io.BytesIO()
        with ZipFile(output, "w") as archive:
            for name, data in parts.items():
                archive.writestr(name, data)
        return output.getvalue()

    rels = ElementTree.fromstring(contents["word/_rels/document.xml.rels"])
    for rel in list(rels):
        if rel.get("Type", "").endswith("/styles"):
            rels.remove(rel)
    missing = {**contents, "word/_rels/document.xml.rels": ElementTree.tostring(rels)}
    assert ImportedDocument.open(package(missing)).styles is None
    invalid = {**contents, "word/styles.xml": b'<w:invalid xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>'}
    with pytest.raises(ValueError, match="invalid styles root"):
        ImportedDocument.open(package(invalid)).styles


def test_python_docx_table_cell_inline_anchors():
    source = (FIXTURES / "imported-table-cells.docx").read_bytes()
    independent = docx.Document(io.BytesIO(source))
    physical_cell = independent.tables[0].cell(0, 0)
    assert physical_cell.paragraphs[0].text == "Cell label\t\ncell link"
    assert physical_cell.tables[0].cell(0, 0).text == "Opaque nested text"
    before_link, = physical_cell.paragraphs[0].hyperlinks
    assert before_link.url == "https://example.invalid/cell"
    assert [run.text for run in before_link.runs] == ["cell ", "link"]

    imported = ImportedDocument.open(source)
    first, other_cell = imported.body[1].children[0].children
    paragraph, nested, later, empty = first.blocks
    assert isinstance(paragraph, ImportedCellParagraph)
    assert isinstance(nested, BodyOpaque) and nested.xml == "w:tbl"
    assert empty.inline_items == empty.inline_structure == []
    label, start, link, end = paragraph.inline_structure
    assert isinstance(label, ImportedRun)
    assert all(isinstance(node, BodyOpaque) for node in (start, end))
    assert [start.xml, end.xml] == ["w:bookmarkStart", "w:bookmarkEnd"]
    assert isinstance(link, ImportedHyperlink) and link.bookmark is None
    assert link.relationship_id == before_link._element.get(docx.oxml.ns.qn("r:id"))
    assert all(isinstance(run, ImportedRun) for run in link.children)
    assert [[item.value for item in run.items] for run in link.children] == [["cell "], ["link"]]
    assert [(item.kind, item.value) for item in paragraph.inline_items] == [
        ("text", "Cell label"), ("tab", None), ("break", "textWrapping"),
        ("opaque", "w:bookmarkStart"), ("text", "cell "), ("text", "link"),
        ("opaque", "w:bookmarkEnd"),
    ]
    later_nodes = [
        imported.body[0].inline_structure[0],
        later.inline_structure[0],
        other_cell.blocks[0].inline_structure[0],
        imported.body[1].children[1].children[0].blocks[0].inline_structure[0],
        imported.body[2].children[0].children[0].blocks[0].inline_structure[0],
    ]
    assert [node.items[0].value for node in later_nodes] == ["Cell label"] * 5
    seen = []
    for node in [label, link, *link.children, *later_nodes]:
        assert node.anchor not in seen
        seen.append(node.anchor)
        resolved = imported.inline_at(node.anchor)
        assert type(resolved) is type(node)
        assert resolved.anchor == node.anchor
    assert imported.body[1].children[0].children[0].blocks[0].inline_structure[0].anchor == label.anchor
    output = imported.preview([]).to_docx()
    assert _parts(output) == _parts(source)
    assert imported.inline_at(link.children[1].anchor).anchor == link.children[1].anchor
    for data in (source, output):
        with pytest.raises(ValueError, match="different document snapshot"):
            ImportedDocument.open(data).inline_at(link.children[1].anchor)


def test_python_docx_header_story_with_opaque_table():
    source = docx.Document()
    header = source.sections[0].header
    paragraph = header.paragraphs[0]
    paragraph.add_run("A").add_tab()
    start = docx.oxml.OxmlElement("w:bookmarkStart")
    start.set(docx.oxml.ns.qn("w:id"), "7")
    start.set(docx.oxml.ns.qn("w:name"), "HeaderMark")
    paragraph._p.append(start)
    paragraph.add_run("B")
    end = docx.oxml.OxmlElement("w:bookmarkEnd")
    end.set(docx.oxml.ns.qn("w:id"), "7")
    paragraph._p.append(end)
    header.add_table(rows=1, cols=1, width=docx.shared.Inches(1))
    source.add_section()
    stream = io.BytesIO()
    source.save(stream)
    data = stream.getvalue()
    independently_read = docx.Document(io.BytesIO(data))
    assert independently_read.sections[0].header.paragraphs[0].text == "A\tB"
    imported = ImportedDocument.open(data)
    first, second = imported.default_headers
    assert first.part_name == second.part_name
    assert [first.inherited, second.inherited] == [False, True]
    assert [(item.kind, item.value) for item in first.blocks[0].inline_items] == [
        ("text", "A"), ("tab", None), ("opaque", "w:bookmarkStart"),
        ("text", "B"), ("opaque", "w:bookmarkEnd"),
    ]
    run_a, start, run_b, end = first.blocks[0].inline_structure
    assert isinstance(run_a, ImportedRun) and isinstance(run_b, ImportedRun)
    assert isinstance(start, BodyOpaque) and start.xml == "w:bookmarkStart"
    assert isinstance(end, BodyOpaque) and end.xml == "w:bookmarkEnd"
    assert run_a.anchor == second.blocks[0].inline_structure[0].anchor
    assert imported.inline_at(run_b.anchor).items[0].value == "B"
    assert isinstance(first.blocks[1], BodyOpaque)
    assert first.blocks[1].xml == "w:tbl"
    assert _parts(imported.preview([]).to_docx()) == _parts(data)


def test_python_docx_section_stories_inherit_and_override_each_slot_independently():
    source = docx.Document()
    source.add_paragraph("Leading section without any stories")
    second = source.add_section()
    slots = [
        ("header", "default", "header"),
        ("header", "first", "first_page_header"),
        ("header", "even", "even_page_header"),
        ("footer", "default", "footer"),
        ("footer", "first", "first_page_footer"),
        ("footer", "even", "even_page_footer"),
    ]
    for kind, variant, attr in slots:
        story = getattr(second, attr)
        story.is_linked_to_previous = False
        story.paragraphs[0].add_run(f"{kind} {variant}")
    second.even_page_footer.add_table(rows=1, cols=1, width=docx.shared.Inches(1))
    third = source.add_section()
    third.first_page_footer.is_linked_to_previous = False
    third.first_page_footer.paragraphs[0].add_run("overridden first footer")
    source.add_section()
    stream = io.BytesIO()
    source.save(stream)
    data = stream.getvalue()

    independently_read = docx.Document(io.BytesIO(data))
    assert independently_read.sections[0].header.is_linked_to_previous
    assert not independently_read.sections[1].different_first_page_header_footer
    assert not independently_read.settings.odd_and_even_pages_header_footer
    assert independently_read.sections[1].even_page_footer.paragraphs[0].text == "footer even"
    assert independently_read.sections[3].first_page_footer.paragraphs[0].text == "overridden first footer"

    imported = ImportedDocument.open(data)
    stories = imported.stories
    assert all(isinstance(story, ImportedStory) for story in stories)
    assert [(s.section_index, s.kind, s.variant) for s in stories] == [
        (index, kind, variant) for index in range(1, 4) for kind, variant, _ in slots
    ]
    assert len({s.part_name for s in stories}) == 7
    first_by_part = {}
    for story in stories:
        paragraph = story.blocks[0]
        assert isinstance(paragraph, ImportedStoryParagraph)
        run, = paragraph.inline_structure
        assert isinstance(run, ImportedRun)
        items = [(item.kind, item.value) for item in imported.inline_at(run.anchor).items]
        overridden = story.section_index >= 2 and (story.kind, story.variant) == ("footer", "first")
        label = "overridden first footer" if overridden else f"{story.kind} {story.variant}"
        assert items == [(item.kind, item.value) for item in paragraph.inline_items] == [("text", label)]
        assert story.inherited == (story.part_name in first_by_part)
        assert first_by_part.setdefault(story.part_name, run.anchor) == run.anchor
        if (story.kind, story.variant) == ("footer", "even"):
            opaque, = story.blocks[1:]
            assert isinstance(opaque, BodyOpaque) and opaque.xml == "w:tbl"
    assert all(a != b for i, a in enumerate(first_by_part.values())
               for b in list(first_by_part.values())[i + 1:])
    for header in imported.default_headers:
        broad = next(s for s in stories
                     if (s.section_index, s.kind, s.variant) == (header.section_index, "header", "default"))
        assert header.blocks[0].inline_structure[0].anchor == broad.blocks[0].inline_structure[0].anchor
    with pytest.raises(ValueError, match="different document snapshot"):
        ImportedDocument.open(data).inline_at(next(iter(first_by_part.values())))
    assert _parts(imported.preview([]).to_docx()) == _parts(data)


@pytest.mark.parametrize("mode,affected,drawings", [
    ("shared", [0, 1], [1, 1]),
    ("section_local", [0], [1, 0]),
])
async def test_combined_transaction_on_independent_fixture(mode, affected, drawings):
    source = (FIXTURES / "imported-editor.docx").read_bytes()
    document = ImportedDocument.open(source)
    first, second = document.default_headers
    patch = ParagraphAlignmentEdit(document.body[0].anchor, "center")
    table = await d.Table([d.TableRow([d.TableCell([d.Paragraph(text="combined")])])]).resolve()
    insertion = TableInsertion(document.body[1].anchor, table)
    image = await d.ImageRun(data=(FIXTURES / "images/sample.png").read_bytes()).resolve()
    header = HeaderImageEdit(first.anchor, image, mode=mode)

    # Both operations work independently, but must also work against one snapshot.
    assert document.preview([patch], table_insertions=[insertion]).insertions
    assert document.preview_header_image(header).change.affected_sections == affected
    preview = document.preview([patch], table_insertions=[insertion], header_images=[header])
    change, = preview.header_changes
    assert change.affected_sections == affected
    assert change.restored_section == (1 if mode == "section_local" else None)
    output = preview.to_docx()
    saved = docx.Document(io.BytesIO(output))
    assert saved.paragraphs[0].alignment == WD_ALIGN_PARAGRAPH.CENTER
    assert saved.paragraphs[0].runs[0].bold
    assert saved.tables[0].cell(0, 0).text == "combined"
    assert [len(s.header._element.xpath(".//w:drawing")) for s in saved.sections] == drawings
    if mode == "section_local":
        part = first.part_name.lstrip("/")
        assert _parts(output)[part] == _parts(source)[part]
    assert document.body[0].alignment is None
    assert document.default_headers[1].part_name == second.part_name


async def test_combined_transaction_rejects_foreign_targets_and_alias_conflicts():
    source = (FIXTURES / "imported-editor.docx").read_bytes()
    document, other = (ImportedDocument.open(source) for _ in range(2))
    patch = ParagraphAlignmentEdit(document.body[0].anchor, "center")
    foreign_patch = ParagraphAlignmentEdit(other.body[0].anchor, "center")
    image = await d.ImageRun(data=(FIXTURES / "images/sample.png").read_bytes()).resolve()
    first, second = document.default_headers
    shared = HeaderImageEdit(first.anchor, image, mode="shared")
    local = HeaderImageEdit(second.anchor, image, mode="section_local")
    stale = HeaderImageEdit(other.default_headers[1].anchor, image, mode="section_local")
    for edits in [[shared, shared, stale], [stale, shared, shared]]:
        with pytest.raises(ValueError, match="different document snapshot"):
            document.preview([patch], header_images=edits)
    with pytest.raises(ValueError, match="different document snapshot"):
        document.preview([foreign_patch], header_images=[shared])
    for edits in [[shared, local], [local, shared]]:
        with pytest.raises(ValueError, match="same physical part"):
            document.preview([patch], header_images=edits)
    image_table = await d.Table([d.TableRow([d.TableCell([d.Paragraph(children=[
        d.ImageRun(data=(FIXTURES / "images/sample.png").read_bytes()),
    ])])])]).resolve()
    insertion = TableInsertion(document.body[1].anchor, image_table)
    with pytest.raises(ValueError, match="unsupported inline content in an inserted table"):
        document.preview([patch], table_insertions=[insertion], header_images=[local])
    assert _parts(document.preview([]).to_docx()) == _parts(source)
    assert document.preview([patch], header_images=[shared]).header_changes[0].affected_sections == [0, 1]


async def test_python_docx_fixture_body_edits():
    source = (FIXTURES / "imported-editor.docx").read_bytes()
    before = docx.Document(io.BytesIO(source))
    assert before.paragraphs[0].text == "Patch me"
    assert before.paragraphs[0].alignment is None
    assert before.paragraphs[0].runs[0].bold is True
    assert before.paragraphs[0].paragraph_format.space_after == Pt(12)
    before_link, = before.paragraphs[0].hyperlinks
    assert [run.text for run in before_link.runs] == [" ", "me"]
    assert before_link.url == "https://example.invalid/body"

    imported = ImportedDocument.open(source)
    assert [(s.body_start, s.body_end) for s in imported.sections] == [(0, 3), (3, 4)]
    assert [(item.kind, item.value) for item in imported.body[0].inline_items] == [
        ("text", "Patch"), ("opaque", "w:bookmarkStart"), ("text", " "),
        ("text", "me"), ("opaque", "w:bookmarkEnd"),
    ]
    label, start, empty, link, end = imported.body[0].inline_structure
    assert isinstance(label, ImportedRun) and isinstance(empty, ImportedRun)
    assert [item.value for item in label.items] == ["Patch"]
    assert empty.items == []
    assert isinstance(start, BodyOpaque) and start.xml == "w:bookmarkStart"
    assert isinstance(end, BodyOpaque) and end.xml == "w:bookmarkEnd"
    assert isinstance(link, ImportedHyperlink) and link.bookmark is None
    assert link.relationship_id == before_link._element.get(docx.oxml.ns.qn("r:id"))
    space, word = link.children
    assert all(isinstance(run, ImportedRun) for run in [space, word])
    assert [[item.value for item in run.items] for run in [space, word]] == [[" "], ["me"]]
    assert isinstance(word.anchor, InlineAnchor)
    assert word.anchor != space.anchor and word.anchor != link.anchor
    assert imported.inline_at(word.anchor).anchor == word.anchor
    assert imported.inline_at(empty.anchor).items == []
    assert isinstance(imported.inline_at(link.anchor), ImportedHyperlink)
    with pytest.raises(ValueError, match="different document snapshot"):
        ImportedDocument.open(source).inline_at(word.anchor)
    with pytest.raises(TypeError):
        ParagraphAlignmentEdit(word.anchor, "center")
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
    assert [run.text for run in patch.hyperlinks[0].runs] == [" ", "me"]
    assert patch.hyperlinks[0].url == before_link.url
    bookmark_elements = "w:bookmarkStart | w:bookmarkEnd"
    assert [e.attrib for e in patch._p.xpath(bookmark_elements)] == [
        e.attrib for e in before.paragraphs[0]._p.xpath(bookmark_elements)
    ]
    assert anchor.text == "Insert table after me"
    assert inserted.cell(0, 0).text == "inserted"
    assert section_break.text == ""
    assert next_paragraph.text == "Second section"
    originals, edited = _parts(source), _parts(output)
    assert originals.keys() == edited.keys()
    assert {name for name in originals if originals[name] != edited[name]} == {
        "word/document.xml"
    }


async def test_python_docx_reads_nested_auto_table_in_inserted_spanning_cell():
    source = (FIXTURES / "imported-editor.docx").read_bytes()
    imported = ImportedDocument.open(source)
    nested = d.Table([d.TableRow([
        d.TableCell([d.Paragraph(text="nested A")]),
        d.TableCell([d.Paragraph(text="nested B")]),
    ])])
    table = await d.Table([d.TableRow([
        d.TableCell([d.Paragraph(text="neighbor")]),
        d.TableCell([nested], grid_span=2),
    ])], column_widths=[400, 700, 1100]).resolve()
    output = imported.preview(
        [], table_insertions=[TableInsertion(imported.body[1].anchor, table)],
    ).to_docx()

    outer, = docx.Document(io.BytesIO(output)).tables
    cell = outer.cell(0, 1)
    assert cell.width.twips == 1800
    nested_read, = cell.tables
    assert [column.width.twips for column in nested_read.columns] == [900, 900]
    assert [cell.text for cell in nested_read.rows[0].cells] == ["nested A", "nested B"]
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
    label, link = first.blocks[0].inline_structure
    other_label, other_link = second.blocks[0].inline_structure
    assert isinstance(label, ImportedRun) and isinstance(link, ImportedHyperlink)
    assert label.anchor == other_label.anchor
    assert link.anchor == other_link.anchor
    li, nk = link.children
    assert isinstance(li.anchor, InlineAnchor) and li.anchor != nk.anchor
    assert li.anchor == other_link.children[0].anchor
    assert imported.inline_at(li.anchor).items[0].value == "li"
    assert imported.inline_at(link.anchor).anchor == link.anchor
    with pytest.raises(ValueError, match="different document snapshot"):
        ImportedDocument.open(source).inline_at(li.anchor)
    with pytest.raises(TypeError):
        ParagraphAlignmentEdit(li.anchor, "center")
    before_link, = before.sections[0].header.paragraphs[0].hyperlinks
    assert [run.text for run in before_link.runs] == ["li", "nk"]
    assert link.relationship_id == before_link._element.get(docx.oxml.ns.qn("r:id"))

    original_part = first.part_name.lstrip("/")
    directory, filename = original_part.rsplit("/", 1)
    relationships = f"{directory}/_rels/{filename}.rels"
    originals = _parts(source)
    rel, = [r for r in ElementTree.fromstring(originals[relationships])
            if r.get("Id") == link.relationship_id]
    assert rel.get("Type").endswith("/hyperlink")
    assert rel.get("Target") == before_link.url == "https://example.invalid/header"
    assert rel.get("TargetMode") == "External"

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
        assert [run.text for run in header.paragraphs[0].hyperlinks[0].runs] == ["li", "nk"]
    reopened = ImportedDocument.open(output).default_headers
    assert [h.part_name for h in reopened] == [preview.change.output_part, first.part_name]
    first_link, second_link = [h.blocks[0].inline_structure[1] for h in reopened]
    assert first_link.relationship_id == second_link.relationship_id
    assert (first_link.anchor == second_link.anchor) == (mode == "shared")
    if mode == "section_local":
        edited = _parts(output)
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

async def test_insert_resolved_paragraph_with_alignment_and_reject_invalid_batch():
    source = docx.Document()
    source.add_paragraph("first")
    source.add_paragraph("second")
    stream = io.BytesIO()
    source.save(stream)
    data = stream.getvalue()
    imported = ImportedDocument.open(data)
    anchor = imported.body[0].anchor
    content = await d.Paragraph(text="inserted").resolve()
    edit = ParagraphInsertion(anchor, content)
    preview = imported.preview(
        [ParagraphAlignmentEdit(anchor, "center")], paragraph_insertions=[edit]
    )
    assert preview.paragraph_insertions == [0]
    paragraphs = docx.Document(io.BytesIO(preview.to_docx())).paragraphs
    assert [p.text for p in paragraphs] == ["first", "inserted", "second"]
    assert paragraphs[0].alignment == WD_ALIGN_PARAGRAPH.CENTER
    for name, payload in _parts(data).items():
        if name != "word/document.xml":
            assert _parts(preview.to_docx())[name] == payload
    with pytest.raises(ValueError):
        imported.preview([], paragraph_insertions=[edit, edit])
    table = await d.Table([d.TableRow([d.TableCell([d.Paragraph(text="cell")])])]).resolve()
    with pytest.raises(ValueError):
        imported.preview([], paragraph_insertions=[edit], table_insertions=[TableInsertion(anchor, table)])
    with pytest.raises(ValueError):
        ImportedDocument.open(data).preview([], paragraph_insertions=[edit])
    with pytest.raises(TypeError):
        ParagraphInsertion(anchor, d.Paragraph(text="unresolved"))
    assert _parts(imported.preview([]).to_docx()) == _parts(data)


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


async def test_invalid_nested_row_height_rejects_fragment_before_editing():
    source = (FIXTURES / "imported-editor.docx").read_bytes()
    imported = ImportedDocument.open(source)
    nested = d.Table([d.TableRow([d.TableCell([])], height=d.RowHeight.exact(-1))])
    table = await d.Table([d.TableRow([d.TableCell([nested])])]).resolve()
    insertion = TableInsertion(imported.body[1].anchor, table)

    with pytest.raises(ValueError, match=r"children\[0\]\.rows\[0\]\.properties\.height"):
        imported.preview([], table_insertions=[insertion])
    assert _parts(imported.preview([]).to_docx()) == _parts(source)


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
    assert [(h.section_index, h.inherited) for h in headers] == [(0, False), (1, True)]
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


@pytest.mark.parametrize("mode", ["shared", "section_local"])
@pytest.mark.parametrize(("kind", "variant", "attr"), [
    ("header", "first", "first_page_header"), ("header", "even", "even_page_header"),
    ("footer", "default", "footer"), ("footer", "first", "first_page_footer"),
    ("footer", "even", "even_page_footer"),
])
async def test_story_image_edit(mode, kind, variant, attr):
    source = docx.Document()
    getattr(source.sections[0], attr).paragraphs[0].text = "Story"
    source.add_section()
    source.add_section()
    buf = io.BytesIO()
    source.save(buf)
    original = buf.getvalue()
    imported = ImportedDocument.open(original)
    first = next(s for s in imported.stories if (s.section_index, s.kind, s.variant) == (1, kind, variant))
    assert first.inherited and first.anchor is not None
    image = await d.ImageRun(data=(FIXTURES / "images/sample.png").read_bytes()).resolve()
    preview = imported.preview_header_image(HeaderImageEdit(first.anchor, image, mode=mode))
    assert preview.change.affected_sections == ([0, 1, 2] if mode == "shared" else [1])
    output = preview.to_docx()
    reopened = docx.Document(io.BytesIO(output))
    assert [len(getattr(s, attr)._element.xpath(".//w:drawing")) for s in reopened.sections] == (
        [1, 1, 1] if mode == "shared" else [0, 1, 0]
    )
    target = getattr(reopened.sections[1], attr)
    rid, = target._element.xpath(".//a:blip/@r:embed")
    assert target.part.related_parts[rid].blob == (FIXTURES / "images/sample.png").read_bytes()
    if mode == "section_local":
        assert preview.change.restored_section == 2
        assert _parts(original)[first.part_name] == _parts(output)[first.part_name]
    other = ImportedDocument.open(original)
    with pytest.raises(ValueError, match="different document snapshot"):
        other.preview_header_image(HeaderImageEdit(first.anchor, image, mode=mode))


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
