//! Inspect generated DOCX via the public imported-document model, not OOXML.
use rocx::editor::{
    BodyBlock, ControlContext, ControlEdit, CustomValue, DefaultField, DefaultValue, EditBatch,
    EntryKind, ImportedCellBlock, ImportedDocument, ImportedEntry, ImportedInline,
    ImportedInlineNode, ImportedParagraph, ImportedRowChild, ImportedStoryBlock, ImportedTableCell,
    ImportedTableChild, LayoutField, LayoutValue, ParagraphFlag, ResourceEdit, SectionStoryKind,
};
use rocx::{Alignment, HeaderFooterType, NoteKind, SectionType, TableLayout};
use std::path::Path;

fn open(path: &Path, suffix: &str) -> ImportedDocument {
    ImportedDocument::open(&std::fs::read(path.with_extension(suffix)).unwrap()).unwrap()
}

fn paragraph<'a>(doc: &'a ImportedDocument, prefix: &str) -> &'a ImportedParagraph {
    let matches: Vec<_> = doc
        .body()
        .iter()
        .filter_map(|b| match b {
            BodyBlock::Paragraph(p) if p.text.starts_with(prefix) => Some(p),
            _ => None,
        })
        .collect();
    let [p] = matches[..] else {
        panic!("expected one paragraph starting with {prefix:?}");
    };
    p
}

fn first_cell(row: &ImportedTableChild) -> &ImportedTableCell {
    let ImportedTableChild::Row(row) = row else {
        panic!("expected physical row");
    };
    let ImportedRowChild::Cell(cell) = &row.children[0] else {
        panic!("expected physical cell");
    };
    cell
}

fn entry<'a>(doc: &'a ImportedDocument, kind: EntryKind, id: &str) -> &'a ImportedEntry {
    doc.entry_stories()
        .unwrap()
        .iter()
        .find(|e| e.kind == kind && e.id == id)
        .unwrap()
}

fn text(items: &[ImportedInline]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|i| match i {
            ImportedInline::Text(s) => Some(s.as_str()),
            _ => None,
        })
        .collect()
}

fn authored(doc: &ImportedDocument) {
    use ImportedInlineNode::{Hyperlink, Opaque, Revision, Run};
    assert_eq!(
        doc.core_title().unwrap().as_deref(),
        Some("All features visual inspection")
    );
    assert_eq!(
        doc.custom_property("Published").unwrap(),
        Some(CustomValue::Boolean(false))
    );
    let sections = doc.sections().unwrap();
    assert_eq!(
        sections.iter().map(|s| s.section_type).collect::<Vec<_>>(),
        [
            None,
            Some(SectionType::NextPage),
            Some(SectionType::NextColumn),
            Some(SectionType::Continuous),
            Some(SectionType::EvenPage),
            Some(SectionType::OddPage),
        ]
    );
    assert_eq!(sections[0].body_start, 0);
    assert_eq!(sections.last().unwrap().body_end, doc.body().len());
    assert!(
        sections
            .windows(2)
            .all(|s| s[0].body_end == s[1].body_start)
    );
    let layout = &sections[1].layout;
    assert_eq!(layout[&LayoutField::PageWidth], LayoutValue::Integer(15840));
    assert_eq!(layout[&LayoutField::ColumnCount], LayoutValue::Integer(2));
    assert_eq!(
        layout[&LayoutField::PageFormat],
        LayoutValue::Token("lowerRoman".into())
    );

    let p = paragraph(doc, "First-line indent=");
    assert_eq!(
        p.layout[&LayoutField::FirstLineIndent],
        LayoutValue::Integer(360)
    );
    assert_eq!(p.flags.get(&ParagraphFlag::WidowControl), Some(&false));
    let p = paragraph(doc, "Builder bold unspecified");
    let [Run(unset), Run(set), Run(cleared)] = p.inline_structure.as_slice() else {
        panic!("expected three runs with separate direct properties");
    };
    assert_eq!(
        [unset.format.bold, set.format.bold, cleared.format.bold],
        [None, Some(true), Some(false)]
    );
    assert_eq!(cleared.format.italic, Some(true));
    let Hyperlink(link) = &paragraph(doc, "Internal hyperlink:").inline_structure[0] else {
        panic!("expected internal hyperlink");
    };
    assert!(
        matches!(doc.inline_at(&link.anchor).unwrap(), Hyperlink(h) if h.bookmark.as_deref() == Some("CatalogBookmark"))
    );

    let styles = doc.styles().unwrap().unwrap();
    let child = styles
        .styles
        .iter()
        .find(|s| s.name.as_deref() == Some("Catalog Child"))
        .unwrap();
    assert_eq!(child.id.as_deref(), Some("CatalogChild"));
    assert_eq!(child.based_on.as_deref(), Some("CatalogBase"));

    let mut tables = doc.body().iter().filter_map(|b| match b {
        BodyBlock::Table(t) => Some(t),
        _ => None,
    });
    let table = tables.next().unwrap();
    assert_eq!(table.layout, Some(TableLayout::Fixed));
    assert_eq!(table.grid_widths, [Some(1440); 3]);
    for (row, merge) in table.children.iter().zip(["restart", "continue"]) {
        let cell = first_cell(row);
        assert_eq!(cell.grid_span, Some(2));
        assert_eq!(cell.vertical_merge.as_deref(), Some(merge));
    }
    let nested = first_cell(&tables.next().unwrap().children[0]);
    assert!(
        nested
            .blocks
            .iter()
            .any(|b| matches!(b, ImportedCellBlock::Opaque(n) if n == "w:tbl"))
    );
    let p = paragraph(doc, "Current: ");
    let [Run(_), Revision(ins), Revision(del)] = p.inline_structure.as_slice() else {
        panic!("expected insertion and deletion wrappers");
    };
    for (revision, kind, date, wording) in [
        (
            ins,
            "insertion",
            Some("2026-11-20T00:00:00Z"),
            "inserted wording",
        ),
        (del, "deletion", None, "old wording"),
    ] {
        assert_eq!(revision.kind, kind);
        assert_eq!(revision.author.as_deref(), Some("Catalog editor"));
        assert_eq!(revision.date.as_deref(), date);
        let [Run(child)] = revision.children.as_slice() else {
            panic!("expected one revision run");
        };
        assert!(
            matches!(doc.inline_at(&child.anchor).unwrap(), Run(r) if text(&r.items) == [wording])
        );
    }
    let field = paragraph(doc, "Field simple PAGE:");
    assert!(field.has_opaque_content);
    assert!(matches!(&field.inline_structure[1], Opaque(n) if n == "w:fldSimple"));
    assert_eq!(
        doc.field_at(&field.anchor, 1).unwrap().instruction,
        " PAGE "
    );
    assert!(matches!(
        &paragraph(doc, "Inline equation:").inline_structure[1],
        ImportedInlineNode::Fraction { numerator, denominator, .. }
            if numerator == "1" && denominator == "2"
    ));
}

fn rich_artifacts(path: &Path) {
    use ImportedInlineNode::{Fraction, Run};
    let objects = open(path, "objects.docx");
    let runs = objects
        .body()
        .iter()
        .filter_map(|b| match b {
            BodyBlock::Paragraph(p) => Some(p.inline_structure.iter()),
            _ => None,
        })
        .flatten()
        .filter_map(|n| match n {
            Run(r) => Some(r),
            _ => None,
        });
    let mut picture = false;
    let mut shape = false;
    let mut charts = 0;
    for run in runs {
        let Some(drawing) = &run.drawing else {
            continue;
        };
        match drawing.kind {
            "image" if drawing.description.as_deref() == Some("Edited picture") => {
                assert!(drawing.relationship_id.is_some());
                picture = true;
            }
            "shape" if drawing.shape_text.as_deref() == Some("Edited shape") => {
                assert_eq!(drawing.geometry.as_deref(), Some("ellipse"));
                shape = true;
            }
            "chart" => {
                assert_eq!(
                    objects.chart_series(&run.anchor).unwrap(),
                    (
                        vec![String::from("Final"), String::from("Revised")],
                        vec![12., 8.]
                    )
                );
                charts += 1;
            }
            _ => {}
        }
    }
    assert!(picture && shape && charts == 1);
    assert!(matches!(
        &paragraph(&objects, "Inline equation:").inline_structure[1],
        Fraction { numerator, denominator, .. } if numerator == "3" && denominator == "7"
    ));
    let images: Vec<_> = entry(&objects, EntryKind::Footnote, "0")
        .inline_structure(0)
        .unwrap()
        .iter()
        .filter_map(|n| match n {
            Run(r) => r.drawing.as_ref().filter(|d| d.kind == "image"),
            _ => None,
        })
        .collect();
    assert_eq!(images.len(), 2);
    assert!(images[0].relationship_id.is_some());
    assert_eq!(images[0].relationship_id, images[1].relationship_id);
    assert!(
        objects
            .controls()
            .iter()
            .any(|c| c.tag.as_deref() == Some("object-sibling")
                && c.context == ControlContext::Block)
    );

    let controls = open(path, "controls.docx");
    let status = controls
        .controls()
        .iter()
        .find(|c| c.tag.as_deref() == Some("review-status"))
        .unwrap();
    assert_eq!(
        (status.kind, status.selected.as_deref()),
        ("dropDownList", Some("pending"))
    );
    assert!(
        status
            .choices
            .iter()
            .any(|c| c.display_text == "Pending" && c.value == "pending")
    );
    let plain = controls
        .controls()
        .iter()
        .find(|c| c.alias.as_deref() == Some("Imported plain-text sample"))
        .unwrap();
    assert_eq!(
        (plain.context, plain.kind, plain.multi_line.as_deref()),
        (ControlContext::Block, "text", Some("0"))
    );
    assert!(plain.tag.is_none());
    assert!(controls.preview_unwrap_control(&plain.anchor).is_ok());
    assert!(
        objects
            .preview_unwrap_control(&plain.anchor)
            .unwrap_err()
            .to_string()
            .contains("snapshot")
    );
}

fn rejected_batches(doc: &ImportedDocument) {
    let caption = doc
        .controls()
        .iter()
        .find(|c| c.id.as_deref() == Some("5001"))
        .unwrap();
    assert_eq!(caption.context, ControlContext::Inline);
    assert_eq!(caption.lock.as_deref(), Some("contentLocked"));
    assert!(!caption.bound);
    let resources = [ResourceEdit::CoreTitle(Some("Catalog retry".into()))];
    let mut control = ControlEdit {
        target: caption.anchor.clone(),
        tag: None,
        alias: None,
        text: Some("forbidden".into()),
    };
    assert!(matches!(doc.preview_edits(EditBatch {
        resources: &resources, controls: &[control.clone()], ..Default::default()
    }), Err(e) if e.to_string().contains("locked")));
    assert_eq!(
        doc.core_title().unwrap().as_deref(),
        Some("All features visual inspection")
    );
    control.text = None;
    control.alias = Some("Allowed alias".into());
    let bytes = doc
        .preview_edits(EditBatch {
            resources: &resources,
            controls: &[control.clone()],
            ..Default::default()
        })
        .unwrap()
        .to_docx()
        .unwrap();
    let retry = ImportedDocument::open(&bytes).unwrap();
    assert_eq!(
        retry.core_title().unwrap().as_deref(),
        Some("Catalog retry")
    );
    assert_eq!(
        retry
            .controls()
            .iter()
            .find(|c| c.id.as_deref() == Some("5001"))
            .unwrap()
            .alias
            .as_deref(),
        Some("Allowed alias")
    );
    assert!(matches!(retry.preview_edits(EditBatch {
        controls: &[control], ..Default::default()
    }), Err(e) if e.to_string().contains("snapshot")));
}

pub fn verify(path: &Path) {
    use HeaderFooterType::{Default as DefaultSlot, Even, First};
    use SectionStoryKind::{Footer, Header};
    let final_doc = open(path, "docx");
    authored(&final_doc);
    rejected_batches(&final_doc);
    rich_artifacts(path);
    let template = open(path, "template.docx");
    assert!(
        matches!(&template.body()[0], BodyBlock::Paragraph(p) if p.text == "Template {{replace}}: cross-run replacement.")
    );

    let edited = open(path, "edited.docx");
    let p = paragraph(&edited, "Imported editor: this paragraph changes");
    assert_eq!(p.alignment, Some(Alignment::Center));
    let index = edited
        .body()
        .iter()
        .position(|b| matches!(b, BodyBlock::Paragraph(q) if q.text == p.text))
        .unwrap();
    let BodyBlock::Table(table) = &edited.body()[index + 1] else {
        panic!("expected inserted table immediately after its anchor");
    };
    assert!(
        matches!(first_cell(&table.children[0]).inline_structure(0).unwrap(),
        [ImportedInlineNode::Run(r)] if r.format.bold == Some(true) && text(&r.items) == ["Imported editor table: bold label"])
    );

    let changed = open(path, "run-text.docx");
    let ranged = paragraph(&changed, "Ranged 📝 + point editor:");
    assert_eq!(
        changed.core_subject().unwrap().as_deref(),
        Some("Imported resource edits")
    );
    assert_eq!(
        changed.default_alignment().unwrap(),
        Some(Alignment::Center)
    );
    let before = final_doc.document_defaults().unwrap();
    let after = changed.document_defaults().unwrap();
    assert!(!before.contains_key(&DefaultField::Bold));
    assert_eq!(after[&DefaultField::Bold], DefaultValue::Bool(false));
    assert_eq!(before[&DefaultField::AfterTwips], DefaultValue::Integer(80));
    assert!(!after.contains_key(&DefaultField::AfterTwips));
    assert_eq!(
        before[&DefaultField::FootnoteStart],
        DefaultValue::Integer(3)
    );
    assert_eq!(
        after[&DefaultField::FootnoteStart],
        DefaultValue::Integer(5)
    );
    let reference = changed
        .paragraph_numbering(&ranged.anchor)
        .unwrap()
        .unwrap();
    assert_eq!((reference.id, reference.level), (9049, 0));
    let instance = changed
        .numbering_instances()
        .unwrap()
        .into_iter()
        .find(|n| n.num_id == reference.id)
        .unwrap();
    assert_eq!(instance.abstract_id, 9048);
    assert_eq!(instance.replacement_levels, [0]);

    let linked = open(path, "entry-text.docx");
    for (kind, id) in [
        (EntryKind::Footnote, "0"),
        (EntryKind::Endnote, "0"),
        (EntryKind::Comment, "1"),
    ] {
        let entry = entry(&linked, kind, id);
        let ImportedStoryBlock::Paragraph(items) = &entry.blocks()[0] else {
            panic!("expected entry paragraph");
        };
        assert_eq!(
            text(items),
            [
                format!("Edited {} entry", kind.as_str()).as_str(),
                "Review link"
            ]
        );
        assert!(matches!(entry.inline_structure(0).unwrap().last(),
            Some(ImportedInlineNode::Hyperlink(h)) if matches!(linked.inline_at(&h.anchor).unwrap(),
                ImportedInlineNode::Hyperlink(found) if found.relationship_id.is_some()
                    && matches!(found.children.as_slice(), [ImportedInlineNode::Run(r)] if r.format.size == Some(13.5)))));
    }
    let lifecycle = open(path, "entry-edits.docx");
    assert_eq!(
        entry(&lifecycle, EntryKind::Comment, "1")
            .initials
            .as_deref(),
        Some("ED")
    );
    let [
        ImportedStoryBlock::Paragraph(first),
        ImportedStoryBlock::Paragraph(second),
    ] = entry(&lifecycle, EntryKind::Footnote, "47").blocks()
    else {
        panic!("expected two paragraphs in new footnote");
    };
    assert_eq!(text(first), ["Lifecycle note"]);
    assert_eq!(text(second), ["Second note paragraph"]);
    let refs: Vec<_> = paragraph(&lifecycle, "Footnote reference;")
        .inline_items
        .iter()
        .filter_map(|i| match i {
            ImportedInline::NoteReference { kind, id } => Some((*kind, id.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        refs,
        [
            (NoteKind::Footnote, "47"),
            (NoteKind::Footnote, "0"),
            (NoteKind::Endnote, "0")
        ]
    );

    for (suffix, kind, variant, local, drawings) in [
        ("header-source.docx", Header, DefaultSlot, false, [0, 0]),
        ("header-shared.docx", Header, DefaultSlot, false, [1, 1]),
        ("header-local.docx", Header, DefaultSlot, true, [0, 1]),
        ("header-first-shared.docx", Header, First, false, [1, 1]),
        ("header-first-local.docx", Header, First, true, [0, 1]),
        ("footer-even.docx", Footer, Even, true, [0, 1]),
    ] {
        let stories = open(path, suffix).stories().unwrap();
        let slots: Vec<_> = stories
            .iter()
            .filter(|s| s.kind == kind && s.variant == variant)
            .collect();
        let [first, second] = slots[..] else {
            panic!("{suffix}: expected two story slots");
        };
        assert_eq!(
            (first.section_index, second.section_index),
            (0, 1),
            "{suffix}"
        );
        assert!(!first.inherited && second.inherited != local, "{suffix}");
        assert_eq!(first.part_name != second.part_name, local, "{suffix}");
        for (slot, count) in [first, second].into_iter().zip(drawings) {
            let mut items = slot.blocks.iter().flat_map(|b| match b {
                ImportedStoryBlock::Paragraph(items) => items.as_slice(),
                _ => &[],
            });
            if kind == Footer {
                assert!(items.clone().any(
                    |i| matches!(i, ImportedInline::Text(s) if s == "Edited shared even footer")
                ));
            }
            assert_eq!(
                items
                    .by_ref()
                    .filter(|i| matches!(i, ImportedInline::Opaque(n) if n == "w:drawing"))
                    .count(),
                count,
                "{suffix}"
            );
        }
    }

    let spliced = open(path, "story-blocks.docx");
    let headers = spliced.default_headers().unwrap();
    assert!(!headers[0].inherited && !headers[1].inherited);
    assert_ne!(headers[0].part_name, headers[1].part_name);
    let original = open(path, "header-source.docx").default_headers().unwrap();
    assert_eq!(headers[1].part_name, original[0].part_name);
    assert!(matches!(&headers[0].blocks[2], ImportedStoryBlock::Opaque(n) if n == "w:tbl"));
    let stories = spliced.stories().unwrap();
    let footer = stories
        .iter()
        .find(|s| s.kind == Footer && s.variant == First && s.section_index == 1)
        .unwrap();
    for (block, inline) in [
        (
            headers[0].blocks.last(),
            headers[0].inline_structure(headers[0].blocks.len() - 1),
        ),
        (
            footer.blocks.last(),
            footer.inline_structure(footer.blocks.len() - 1),
        ),
    ] {
        assert!(
            matches!(block, Some(ImportedStoryBlock::Paragraph(items)) if text(items) == ["Story block link"])
        );
        assert!(
            matches!(inline, Some([ImportedInlineNode::Hyperlink(h)]) if h.relationship_id.is_some())
        );
    }
}
