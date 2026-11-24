"""Typed catalog assertions shared by the installed Python catalog tests.

These check the editor's projection, not schema validity, relationships or
serialization. See docs/EDITOR-ACCEPTANCE.md for those separate gates.
"""

import pytest
from rocx import editor as e


def paragraph(doc, prefix):
    p, = (p for p in doc.body if isinstance(p, e.BodyParagraph) and p.text.startswith(prefix))
    return p


def items(block):
    return [(i.kind, i.value) for i in block.inline_items]


def entry(doc, kind, id):
    match, = (x for x in doc.entry_stories if (x.kind, x.id) == (kind, id))
    return match


def authored(doc):
    # Labels locate samples; the assertions use the imported properties, not
    # the labels that describe them.
    assert doc.core_title == "All features visual inspection"
    assert doc.custom_property("Project") == "Atlas"
    assert doc.custom_property("Published") is False
    assert [s.section_type for s in doc.sections] == [
        None, "nextPage", "nextColumn", "continuous", "evenPage", "oddPage",
    ]
    assert doc.sections[0].flags == {"bidi": False, "rtl_gutter": False, "title_page": True}
    layout = doc.sections[1].layout
    assert (layout["page_width"], layout["column_count"], layout["page_format"]) == (15840, 2, "lowerRoman")
    assert doc.sections[0].body_start == 0 and doc.sections[-1].body_end == len(doc.body)
    assert all(a.body_end == b.body_start for a, b in zip(doc.sections, doc.sections[1:]))
    assert paragraph(doc, "Paragraph alignment=justify:").alignment == "justify"
    indented = paragraph(doc, "First-line indent=")
    assert indented.layout["first_line_indent"] == 360
    assert indented.flags == dict.fromkeys(
        ["bidi", "keep_together", "keep_with_next", "page_break_before", "widow_control"], False)
    assert [r.format for r in paragraph(doc, "Builder bold unspecified").inline_structure] == [
        {}, {"bold": True}, {"bold": False, "italic": True},
    ]
    assert paragraph(doc, "Arial 16 pt").inline_structure[0].format == {
        "font": "Arial", "size": 16, "color": (128, 48, 32),
    }
    for prefix, key, value in [
        ("Strike=true", "strike", True), ("ALL CAPS=true", "all_caps", True),
        ("Small caps=true", "small_caps", True), ("Superscript=true", "superscript", True),
        ("Subscript=true", "subscript", True), ("Underline=double:", "underline", "double"),
    ]:
        assert paragraph(doc, prefix).inline_structure[0].format[key] == value
    language = paragraph(doc, "Run language/script:").inline_structure[0].format["language"]
    assert (language.latin, language.east_asia, language.bidi) == ("en-US", "ja-JP", "ar-SA")
    assert items(paragraph(doc, "Before explicit page break"))[1] == ("break", "page")
    assert items(paragraph(doc, "Tab left\t"))[1] == ("tab", None)
    link = paragraph(doc, "Internal hyperlink:").inline_structure[0]
    assert isinstance(link, e.ImportedHyperlink)
    assert doc.inline_at(link.anchor).bookmark == "CatalogBookmark"
    assert [(s.id, s.based_on) for s in doc.styles.styles if s.name == "Catalog Child"] == [
        ("CatalogChild", "CatalogBase"),
    ]
    assert (100, 0, 1) in {(x.abstract_id, x.level, x.start) for x in doc.abstract_starts}
    tables = [b for b in doc.body if isinstance(b, e.BodyTable)]
    assert tables[0].layout == "fixed" and tables[0].grid_widths == [1440] * 3
    assert [(r.children[0].grid_span, r.children[0].vertical_merge) for r in tables[0].children[:2]] == [
        (2, "restart"), (2, "continue"),
    ]
    assert any(isinstance(b, e.BodyOpaque) and b.xml == "w:tbl"
               for b in tables[1].children[0].children[0].blocks)
    assert items(paragraph(doc, "Footnote reference;"))[1::2] == [
        ("footnoteReference", "0"), ("endnoteReference", "0"),
    ]
    assert [(r.kind, r.author, r.date, doc.inline_at(r.children[0].anchor).items[0].value)
            for r in paragraph(doc, "Current: ").inline_structure[1:]] == [
        ("insertion", "Catalog editor", "2026-11-20T00:00:00Z", "inserted wording"),
        ("deletion", "Catalog editor", None, "old wording"),
    ]
    # Field nodes are still opaque in the sequence; dedicated inspection
    # decodes the supported instruction and flags.
    field = paragraph(doc, "Field simple PAGE:")
    assert field.has_opaque_content and field.inline_structure[1].xml == "w:fldSimple"
    assert doc.field_at(field.anchor, 1).instruction == " PAGE "
    fraction = paragraph(doc, "Inline equation:").inline_structure[1]
    assert isinstance(fraction, e.ImportedFraction)
    assert (fraction.numerator, fraction.denominator) == ("1", "2")
    assert ("opaque", "w:bookmarkStart") in items(paragraph(doc, "Bookmark CatalogBookmark starts"))
    assert {(c.context, c.id, c.tag) for c in doc.controls if c.id is not None} == {
        ("inline", "5001", "review-caption"), ("block", "5002", "review-block"),
        ("row", "5003", "review-row"), ("cell", "5004", "review-cell"),
    }


def rejected_batches(doc):
    caption, = (c for c in doc.controls if c.id == "5001")
    assert (caption.alias, caption.lock, caption.bound) == ("Review caption", "contentLocked", False)
    # A rich field's recognition can evolve; the locked control still must
    # reject the entire mixed batch and leave the original anchor usable.
    title = e.ResourceEdit.core_title("Catalog retry")
    with pytest.raises(ValueError, match="locked"):
        doc.preview([], resources=[title], controls=[e.ControlEdit(caption.anchor, text="forbidden")])
    assert doc.core_title == "All features visual inspection"
    retry = e.ImportedDocument.open(doc.preview(
        [], resources=[title], controls=[e.ControlEdit(caption.anchor, alias="Allowed alias")],
    ).to_docx())
    assert retry.core_title == "Catalog retry"
    assert next(c.alias for c in retry.controls if c.id == "5001") == "Allowed alias"
    with pytest.raises(ValueError, match="snapshot"):
        retry.preview([], controls=[e.ControlEdit(caption.anchor, alias="Foreign anchor")])


def check_story(doc, kind, variant, *, local, drawings, text=None):
    first, second = (s for s in doc.stories if (s.kind, s.variant) == (kind, variant))
    assert (first.section_index, second.section_index) == (0, 1)
    assert not first.inherited and second.inherited != local
    assert (first.part_name != second.part_name) == local
    for slot, count in zip((first, second), drawings, strict=True):
        flat = [item for block in slot.blocks for item in items(block)]
        assert flat.count(("opaque", "w:drawing")) == count
        if text is not None:
            assert ("text", text) in flat


def verify_catalog(stem, suffixes):
    docs = {s: e.ImportedDocument.open(stem.with_suffix(s).read_bytes()) for s in suffixes}
    final = docs.pop(".docx")
    authored(final)
    rejected_batches(final)
    assert docs.pop(".template.docx").body[0].text == "Template {{replace}}: cross-run replacement."

    edited = docs.pop(".edited.docx")
    p = paragraph(edited, "Imported editor: this paragraph changes")
    assert p.alignment == "center"
    index, = (i for i, b in enumerate(edited.body) if isinstance(b, e.BodyParagraph) and b.text == p.text)
    table = edited.body[index + 1]
    assert isinstance(table, e.BodyTable)
    label, = table.children[0].children[0].blocks[0].inline_structure
    assert label.format["bold"] is True
    assert [(i.kind, i.value) for i in label.items] == [("text", "Imported editor table: bold label")]

    changed = docs.pop(".run-text.docx")
    ranged = paragraph(changed, "Ranged 📝 + point editor:")
    assert changed.core_subject == "Imported resource edits" and changed.default_alignment == "center"
    assert changed.metadata_field("manager") == "Review lead" and changed.custom_property("Reviewed") is False
    before, after = final.document_defaults, changed.document_defaults
    assert "bold" not in before and after["bold"] is False
    assert before["afterTwips"] == 80 and "afterTwips" not in after
    assert (before["footnoteStart"], after["footnoteStart"]) == (3, 5)
    assert changed.paragraph_numbering(ranged.anchor) == (9049, 0)
    assert (100, 0, 7) in {(x.abstract_id, x.level, x.start) for x in changed.abstract_starts}

    objects = docs.pop(".objects.docx")
    runs = [r for p in objects.body if isinstance(p, e.BodyParagraph)
            for r in p.inline_structure if isinstance(r, e.ImportedRun) and r.drawing]
    picture, = [r.drawing for r in runs if r.drawing.kind == "image" and r.drawing.description == "Edited picture"]
    assert picture.relationship_id
    shape, = [r.drawing for r in runs if r.drawing.kind == "shape" and r.drawing.shape_text == "Edited shape"]
    assert shape.geometry == "ellipse"
    chart, = [r for r in runs if r.drawing.kind == "chart"]
    assert objects.chart_series(chart.anchor) == (["Final", "Revised"], [12, 8])
    fraction = paragraph(objects, "Inline equation:").inline_structure[1]
    assert isinstance(fraction, e.ImportedFraction)
    assert (fraction.numerator, fraction.denominator) == ("3", "7")
    images = [r.drawing.relationship_id for r in entry(objects, "footnote", "0").blocks[0].inline_structure
              if isinstance(r, e.ImportedRun) and r.drawing and r.drawing.kind == "image"]
    assert len(images) == 2 and images[0] and images[0] == images[1]
    assert any(c.tag == "object-sibling" and c.context == "block" for c in objects.controls)

    controls = docs.pop(".controls.docx")
    status, = (c for c in controls.controls if c.tag == "review-status")
    assert (status.kind, status.selected) == ("dropDownList", "pending")
    assert ("Pending", "pending") in status.choices
    plain, = (c for c in controls.controls if c.alias == "Imported plain-text sample")
    assert (plain.context, plain.kind, plain.multi_line, plain.tag) == ("block", "text", "0", None)
    controls.preview_unwrap_control(plain.anchor)
    with pytest.raises(ValueError, match="snapshot"):
        objects.preview_unwrap_control(plain.anchor)

    linked = docs.pop(".entry-text.docx")
    for kind, id in [("footnote", "0"), ("endnote", "0"), ("comment", "1")]:
        block = entry(linked, kind, id).blocks[0]
        link = block.inline_structure[-1]
        assert isinstance(link, e.ImportedHyperlink) and linked.inline_at(link.anchor).relationship_id
        child, = link.children
        assert child.format["size"] == 13.5
        assert items(block)[-2:] == [("text", f"Edited {kind} entry"), ("text", "Review link")]
    lifecycle = docs.pop(".entry-edits.docx")
    assert entry(lifecycle, "comment", "1").initials == "ED"
    assert [items(b)[-1] for b in entry(lifecycle, "footnote", "47").blocks] == [
        ("text", "Lifecycle note"), ("text", "Second note paragraph"),
    ]
    assert [it for it in items(paragraph(lifecycle, "Footnote reference;")) if it[0].endswith("Reference")] == [
        ("footnoteReference", "47"), ("footnoteReference", "0"), ("endnoteReference", "0"),
    ]

    source = docs.pop(".header-source.docx")
    check_story(source, "header", "default", local=False, drawings=(0, 0))
    for suffix, kind, variant, local, drawings, text in [
        (".header-shared.docx", "header", "default", False, (1, 1), None),
        (".header-local.docx", "header", "default", True, (0, 1), None),
        (".header-first-shared.docx", "header", "first", False, (1, 1), None),
        (".header-first-local.docx", "header", "first", True, (0, 1), None),
        (".footer-even.docx", "footer", "even", True, (0, 1), "Edited shared even footer"),
    ]:
        check_story(docs.pop(suffix), kind, variant, local=local, drawings=drawings, text=text)

    story_blocks = docs.pop(".story-blocks.docx")
    first, second = (s for s in story_blocks.stories if (s.kind, s.variant) == ("header", "default"))
    original = next(s for s in source.stories if (s.kind, s.variant) == ("header", "default"))
    assert first.part_name != second.part_name == original.part_name
    assert not first.inherited and not second.inherited
    assert [b.xml for b in first.blocks if isinstance(b, e.BodyOpaque)] == ["w:tbl"]
    assert len(second.blocks) == len(original.blocks)
    footer, = (s for s in story_blocks.stories if (s.section_index, s.kind, s.variant) == (1, "footer", "first"))
    for story in (first, footer):
        link, = story.blocks[-1].inline_structure
        assert isinstance(link, e.ImportedHyperlink) and link.relationship_id
        assert items(story.blocks[-1]) == [("text", "Story block link")]
    assert not docs, f"Add typed readback for {list(docs)}"
