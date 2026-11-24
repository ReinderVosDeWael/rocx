//! Synthetic WordprocessingML and independently generated python-docx fixtures.
use futures::executor::block_on;
use rocx::{
    Alignment, Document, Paragraph, Section,
    editor::{BodyBlock, ImportedDocument, ParagraphAlignmentEdit},
};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const BODY: &str = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>
<w:p><w:pPr><w:keepNext/><w:jc w:val="left"/><w:textDirection w:val="lrTb"/></w:pPr><w:bookmarkStart w:id="7" w:name="spot"/><w:r><w:rPr><w:b/></w:rPr><w:t>Hello </w:t></w:r><w:r><w:t>world</w:t></w:r><w:bookmarkEnd w:id="7"/></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>in table</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
<w:sdt><w:sdtPr/><w:sdtContent><w:p><w:r><w:t>hidden</w:t></w:r></w:p></w:sdtContent></w:sdt>
<w:p><w:pPr><w:keepNext/><w:textDirection w:val="lrTb"/></w:pPr><w:r><w:t>Second</w:t></w:r></w:p>
<w:sectPr/></w:body></w:document>"#;

fn parts(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut entry = archive.by_index(i).unwrap();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            (entry.name().unwrap().into_owned(), data)
        })
        .collect()
}

fn fixture() -> Vec<u8> {
    fixture_with_body(BODY)
}

#[test]
fn mixed_new_edits_preserve_original_coordinates_across_parts() {
    use rocx::editor::{
        BlockEdit, EditBatch, ImportedInlineNode::Run, InlineAction, InlineEdit, ResourceEdit,
    };
    let input = include_bytes!("fixtures/imported-editor.docx");
    let source = ImportedDocument::open(input).unwrap();
    let original = parts(input);
    let target = paragraph(&source, 3);
    let Run(run) = &target.inline_structure[0] else {
        panic!("run")
    };
    let replacement = InlineAction::Replace(rocx::ResolvedInline::Text {
        text: "Edited tail".into(),
        format: Default::default(),
        comment: None,
        revision: None,
    });
    let inline = [InlineEdit {
        target: run.anchor.clone(),
        action: replacement,
    }];
    let title = [ResourceEdit::CoreTitle(Some("Mixed transaction".into()))];
    let remove = [BlockEdit::Remove(paragraph(&source, 1).anchor.clone())];
    let batch = |blocks| EditBatch {
        blocks,
        inline_edits: &inline,
        resources: &title,
        ..Default::default()
    };
    let overlap = [BlockEdit::Remove(target.anchor.clone())];
    assert!(source.preview_edits(batch(&overlap)).is_err());
    assert_eq!(
        parts(&source.preview(&[]).unwrap().to_docx().unwrap()),
        original
    );

    let preview = source.preview_edits(batch(&remove)).unwrap();
    assert_eq!(preview.block_changes(), &[1]);
    assert_eq!(preview.inline_changes(), &[(3, 0)]);
    let bytes = preview.to_docx().unwrap();
    let result = parts(&bytes);
    assert_eq!(
        original.keys().collect::<Vec<_>>(),
        result.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        original
            .iter()
            .filter(|(k, v)| result.get(*k) != Some(*v))
            .map(|(k, _)| k.as_str())
            .collect::<Vec<_>>(),
        ["docProps/core.xml", "word/document.xml"]
    );
    let reopened = ImportedDocument::open(&bytes).unwrap();
    assert_eq!(
        reopened.core_title().unwrap().as_deref(),
        Some("Mixed transaction")
    );
    assert_eq!(reopened.body().len(), 3);
    assert_eq!(paragraph(&reopened, 0).text, "Patch me");
    assert_eq!(paragraph(&reopened, 1).text, "");
    assert_eq!(paragraph(&reopened, 2).text, "Edited tail");
    assert!(
        reopened
            .preview_edits(EditBatch {
                inline_edits: &inline,
                resources: &[ResourceEdit::CoreTitle(None)],
                ..Default::default()
            })
            .is_err()
    );
    assert_eq!(
        parts(&reopened.preview(&[]).unwrap().to_docx().unwrap()),
        result
    );
    let Run(fresh) = &paragraph(&reopened, 2).inline_structure[0] else {
        panic!("run")
    };
    assert_ne!(fresh.anchor, run.anchor);
    assert!(
        matches!(reopened.inline_at(&fresh.anchor).unwrap(), Run(r) if r.anchor == fresh.anchor)
    );
    assert_eq!(
        source
            .preview_edits(batch(&remove))
            .unwrap()
            .to_docx()
            .unwrap(),
        bytes
    );
}

#[test]
fn text_ranges_use_snapshot_coordinates_at_run_boundaries() {
    use rocx::editor::TextRangeEdit;
    let source = fixture_with_body(&BODY.replace(
        "<w:t>Second</w:t>",
        "<w:t>😀</w:t></w:r><w:r><w:t/></w:r><w:r><w:t>é</w:t>",
    ));
    let doc = ImportedDocument::open(&source).unwrap();
    let BodyBlock::Paragraph(p) = &doc.body()[3] else {
        panic!("paragraph")
    };
    let edit = |start, end, text: &str| TextRangeEdit {
        target: p.anchor.clone(),
        start,
        end,
        text: text.into(),
    };
    let preview = |ranges: &[TextRangeEdit]| {
        doc.preview_with_ranges(&[], &[], &[], &[], &[], &[], &[], ranges)
    };
    assert!(preview(&[edit(1, usize::MAX, "out of bounds")]).is_err());
    let point = rocx::editor::TextInsertion {
        target: p.anchor.clone(),
        position: 1,
        text: "+".into(),
    };
    let inserted = doc
        .preview_edits(rocx::editor::EditBatch {
            text_insertions: &[point],
            ..Default::default()
        })
        .unwrap();
    let changes = inserted.run_text_changes();
    assert_eq!(changes.len(), 1);
    assert_eq!(
        (changes[0].inline_index, changes[0].after.as_str()),
        (2, "+é")
    );
    let changes = preview(&[edit(1, 2, "new"), edit(0, 1, "first")]).unwrap();
    assert_eq!(
        changes
            .run_text_changes()
            .iter()
            .map(|c| (c.inline_index, c.after.as_str()))
            .collect::<Vec<_>>(),
        [(0, "first"), (2, "neẃ")]
    );
}

#[test]
fn resource_numbering_starts_reject_duplicate_identities_and_preserve_lexical_noop() {
    use rocx::editor::{AbstractStart, ResourceEdit};
    let fixture = include_bytes!("fixtures/imported-editor.docx");
    let mut entries = parts(fixture);
    let name = "word/numbering.xml";
    for (levels, expected_ok) in [
        (r#"<w:lvl w:ilvl="00"><w:start w:val="007"/></w:lvl>"#, true),
        (r#"<w:lvl w:ilvl="0"/><w:lvl w:ilvl="00"/>"#, false),
        (r#"<w:lvl w:ilvl="0"><w:start w:val="-3"/></w:lvl>"#, false),
    ] {
        entries.insert(name.into(), format!(r#"<w:numbering xmlns:w="{W}"><w:abstractNum w:abstractNumId="0">{levels}</w:abstractNum></w:numbering>"#).into_bytes());
        let doc = ImportedDocument::open(&package_bytes(entries.clone())).unwrap();
        let start = AbstractStart {
            abstract_id: 0,
            level: 0,
            start: Some(7),
        };
        let read = doc.abstract_starts();
        let preview = doc.preview_with_resources(
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[ResourceEdit::AbstractStart(start.clone())],
        );
        if expected_ok {
            assert_eq!(read.unwrap(), [start]);
            assert_eq!(entries, parts(&preview.unwrap().to_docx().unwrap()));
        } else {
            assert!(read.is_err());
            assert!(preview.is_err());
        }
    }
}

#[test]
fn cell_alignment_patch_validates_grid_and_preserves_other_content() {
    use rocx::{VerticalAlignment, editor::CellAlignmentEdit};
    let xml = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:tbl><w:tblPr/><w:tblGrid><w:gridCol/><w:gridCol/></w:tblGrid>
<w:tr><w:tc><w:tcPr><w:gridSpan w:val="2"/><w:vMerge w:val="restart"/><w:shd w:fill="FF0000"/></w:tcPr><w:p/></w:tc></w:tr>
<w:tr><w:tc><w:tcPr><w:gridSpan w:val="2"/><w:vMerge/></w:tcPr><w:p/></w:tc></w:tr></w:tbl><w:sectPr/></w:body></w:document>"#
    );
    let source = fixture_with_body(&xml);
    let doc = ImportedDocument::open(&source).unwrap();
    let BodyBlock::Table(table) = &doc.body()[0] else {
        panic!("table")
    };
    let edit = CellAlignmentEdit {
        target: table.anchor.clone(),
        row_index: 0,
        cell_index: 0,
        alignment: VerticalAlignment::Center,
    };
    let preview = doc
        .preview_with_cells(&[], &[], &[], &[], std::slice::from_ref(&edit))
        .unwrap();
    assert_eq!(preview.cell_alignment_changes().len(), 1);
    let result = parts(&preview.to_docx().unwrap());
    let source_parts = parts(&source);
    let before_xml = std::str::from_utf8(&source_parts["word/document.xml"]).unwrap();
    let xml = String::from_utf8(result["word/document.xml"].clone()).unwrap();
    let before = roxmltree::Document::parse(before_xml).unwrap();
    let after = roxmltree::Document::parse(&xml).unwrap();
    let preserved = |document: &roxmltree::Document<'_>| {
        document
            .descendants()
            .filter(|n| n.has_tag_name((W, "tcPr")))
            .map(|props| {
                props
                    .children()
                    .filter(|n| !n.has_tag_name((W, "vAlign")))
                    .map(|n| tree(n, false))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(preserved(&before), preserved(&after));
    assert_eq!(
        after
            .descendants()
            .filter(|n| n.has_tag_name((W, "vAlign")))
            .map(|n| n.attribute((W, "val")))
            .collect::<Vec<_>>(),
        [Some("center")]
    );
    for (name, data) in parts(&source) {
        if name != "word/document.xml" {
            assert_eq!(result[&name], data);
        }
    }
    assert!(
        doc.preview_with_cells(&[], &[], &[], &[], &[edit.clone(), edit.clone()])
            .is_err()
    );
    let other = ImportedDocument::open(&source).unwrap();
    assert!(
        other
            .preview_with_cells(&[], &[], &[], &[], &[edit])
            .is_err()
    );
    let broken = xml.replace("w:val=\"restart\"", "w:val=\"continue\"");
    let broken_doc = ImportedDocument::open(&fixture_with_body(&broken)).unwrap();
    let BodyBlock::Table(table) = &broken_doc.body()[0] else {
        panic!("table")
    };
    let invalid = CellAlignmentEdit {
        target: table.anchor.clone(),
        row_index: 0,
        cell_index: 0,
        alignment: VerticalAlignment::Top,
    };
    assert!(
        broken_doc
            .preview_with_cells(&[], &[], &[], &[], &[invalid])
            .is_err()
    );
    let reopened = ImportedDocument::open(&preview.to_docx().unwrap()).unwrap();
    let BodyBlock::Table(table) = &reopened.body()[0] else {
        panic!("table")
    };
    let noop = CellAlignmentEdit {
        target: table.anchor.clone(),
        row_index: 0,
        cell_index: 0,
        alignment: VerticalAlignment::Center,
    };
    assert!(
        reopened
            .preview_with_cells(&[], &[], &[], &[], std::slice::from_ref(&noop))
            .unwrap()
            .cell_alignment_changes()
            .is_empty()
    );
    let continuation = CellAlignmentEdit {
        row_index: 1,
        alignment: VerticalAlignment::Bottom,
        ..noop.clone()
    };
    assert_eq!(
        reopened
            .preview_with_cells(&[], &[], &[], &[], &[continuation])
            .unwrap()
            .cell_alignment_changes()
            .len(),
        1
    );
    let missing = CellAlignmentEdit {
        row_index: 8,
        ..noop
    };
    assert!(
        reopened
            .preview_with_cells(&[], &[], &[], &[], &[missing])
            .is_err()
    );
}

#[test]
fn cell_alignment_rejects_unsafe_table_layouts_atomically() {
    use rocx::{VerticalAlignment, editor::CellAlignmentEdit};
    let base = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:p><w:r><w:t>before</w:t></w:r></w:p><w:tbl><w:tblGrid><w:gridCol/><w:gridCol/></w:tblGrid><w:tr><w:tc><w:tcPr><w:gridSpan w:val="2"/><w:vMerge w:val="restart"/></w:tcPr><w:p/></w:tc></w:tr><w:tr><w:tc><w:tcPr><w:gridSpan w:val="2"/><w:vMerge/></w:tcPr><w:p/></w:tc></w:tr></w:tbl><w:sectPr/></w:body></w:document>"#
    );
    let cases = [
        base.replace("w:val=\"restart\"", "w:val=\"continue\""),
        base.replace("w:val=\"2\"", "w:val=\"3\""),
        base.replace("<w:vMerge/>", "<w:hMerge/>"),
        base.replace(
            "<w:vMerge/>",
            "<w:vMerge/><w:vAlign w:val=\"top\"/><w:vAlign w:val=\"bottom\"/>",
        ),
        base.replace(
            "<w:tr><w:tc>",
            "<w:tr><w:trPr><w:gridBefore w:val=\"1\"/></w:trPr><w:tc>",
        ),
        base.replace("<w:tr><w:tc>", "<w:tr><w:sdt/><w:tc>"),
        base.replace("<w:vMerge/>", "<w:tcPrChange w:id=\"1\"/>"),
    ];
    for xml in cases {
        let source = fixture_with_body(&xml);
        let doc = ImportedDocument::open(&source).unwrap();
        let BodyBlock::Table(table) = &doc.body()[1] else {
            panic!("table")
        };
        let edit = CellAlignmentEdit {
            target: table.anchor.clone(),
            row_index: 0,
            cell_index: 0,
            alignment: VerticalAlignment::Center,
        };
        let paragraph = paragraph(&doc, 0);
        assert!(
            doc.preview_with_cells(
                &[ParagraphAlignmentEdit {
                    target: paragraph.anchor.clone(),
                    alignment: Alignment::Right
                }],
                &[],
                &[],
                &[],
                &[edit],
            )
            .is_err()
        );
        assert_eq!(
            parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
            parts(&source)
        );
    }
}

fn fixture_with_body(xml: &str) -> Vec<u8> {
    let original = block_on(
        Document::new(vec![
            Section::new(vec![Paragraph::new("seed").into()]).into(),
        ])
        .resolve(),
    )
    .unwrap()
    .to_docx()
    .unwrap();
    let mut entries = parts(&original);
    entries.insert("word/document.xml".into(), xml.as_bytes().to_vec());
    package_bytes(entries)
}

#[test]
fn block_edit_guards_validate_discarded_markup_and_section_ownership() {
    use rocx::editor::BlockEdit;
    let outer = |middle: &str| {
        fixture_with_body(&format!(
            r#"<w:document xmlns:w="{W}"><w:body><w:p/>{middle}<w:p/><w:sectPr/></w:body></w:document>"#
        ))
    };
    for middle in [
        r#"<w:p><w:bookmarkStart w:id="1" w:name="target"/><w:bookmarkEnd w:id="1"/></w:p>"#,
        "<w:p><w:r><w:footnoteReference/></w:r></w:p>",
        "<w:p><w:hyperlink/></w:p>",
        "<w:p><w:pPr/><w:pPr/></w:p>",
        "<w:p><w:pPr><w:sectPr/></w:pPr></w:p>",
        r#"<w:p customId="x"/>"#,
        "<w:p><w:r><w:t>visible<!--opaque--></w:t></w:r></w:p>",
    ] {
        let source = outer(middle);
        let doc = ImportedDocument::open(&source).unwrap();
        let keep = BlockEdit::KeepWithNext(paragraph(&doc, 0).anchor.clone(), true);
        let remove = BlockEdit::Remove(paragraph(&doc, 1).anchor.clone());
        assert!(
            doc.preview_blocks(&[keep.clone(), remove]).is_err(),
            "{middle}"
        );
        assert_eq!(doc.preview_blocks(&[keep]).unwrap().block_changes(), &[0]);
        assert_eq!(
            parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
            parts(&source)
        );
    }
    for middle in [
        "<w:tbl/>",
        "<w:p><w:pPr><w:sectPr><w:headerReference w:type=\"default\"/></w:sectPr></w:pPr></w:p>",
        "<w:sdt><w:sdtContent><w:p><w:pPr><w:sectPr/></w:pPr></w:p></w:sdtContent></w:sdt>",
        "<!--opaque body content-->",
        "<w:p><w:r><w:fldChar w:fldCharType=\"begin\"/></w:r></w:p>",
    ] {
        let doc = ImportedDocument::open(&outer(middle)).unwrap();
        assert!(
            doc.preview_blocks(&[BlockEdit::ReplaceSection(
                paragraph(&doc, 0).anchor.clone(),
                vec![]
            )])
            .is_err(),
            "{middle}"
        );
    }
    for markers in [
        (
            "<w:proofErr w:type=\"gramStart\"/>",
            "<w:proofErr w:type=\"gramEnd\"/>",
        ),
        (
            "<x:customXmlConflictInsRangeStart xmlns:x=\"urn:future\"/>",
            "<x:customXmlConflictInsRangeEnd xmlns:x=\"urn:future\"/>",
        ),
    ] {
        let doc = ImportedDocument::open(&outer(&format!(
            "<w:p>{}</w:p><w:p/><w:p>{}</w:p>",
            markers.0, markers.1
        )))
        .unwrap();
        assert!(
            doc.preview_blocks(&[BlockEdit::Remove(paragraph(&doc, 2).anchor.clone())])
                .is_err()
        );
    }
}

#[test]
fn tracked_text_is_explicit_and_reviews_only_one_original_wrapper() {
    use rocx::{
        TextRevisionAction::*,
        editor::{ImportedInlineNode::*, RunTextEdit, TrackedText},
    };
    let bytes = fixture();
    let source = ImportedDocument::open(&bytes).unwrap();
    let Run(run) = &paragraph(&source, 3).inline_structure[0] else {
        panic!()
    };
    let tracked = |edit| source.preview_tracked_text(&run.anchor, edit, "Reviewer", None);
    let on = tracked(TrackedText::Replace("new".into()))
        .unwrap()
        .to_docx()
        .unwrap();
    let ordinary = source
        .preview_with_text(
            &[],
            &[],
            &[],
            &[RunTextEdit {
                target: run.anchor.clone(),
                text: "new".into(),
            }],
        )
        .unwrap()
        .to_docx()
        .unwrap();
    assert_eq!(
        paragraph(&ImportedDocument::open(&ordinary).unwrap(), 3).text,
        "new"
    );
    let changed = ImportedDocument::open(&on).unwrap();
    let p = paragraph(&changed, 3);
    assert_eq!(p.text, "");
    let [Revision(del), Revision(ins)] = p.inline_structure.as_slice() else {
        panic!()
    };
    assert_ne!(del.id, ins.id);
    assert_eq!(ins.author.as_deref(), Some("Reviewer"));
    assert_eq!(ins.date, None);
    for (anchor, action, text) in [
        (&ins.anchor, Accept, "new"),
        (&ins.anchor, Reject, ""),
        (&del.anchor, Accept, ""),
        (&del.anchor, Reject, "Second"),
    ] {
        assert!(source.preview_revision(anchor, action, None).is_err());
        let out = changed
            .preview_revision(anchor, action, None)
            .unwrap()
            .to_docx()
            .unwrap();
        assert_eq!(
            paragraph(&ImportedDocument::open(&out).unwrap(), 3).text,
            text
        );
    }
    let Run(protected) = &paragraph(&source, 0).inline_structure[1] else {
        panic!()
    };
    assert!(
        source
            .preview_tracked_text(&protected.anchor, TrackedText::Delete, "Reviewer", None)
            .is_err()
    );
    assert_eq!(
        parts(
            &tracked(TrackedText::Replace("Second".into()))
                .unwrap()
                .to_docx()
                .unwrap()
        ),
        parts(&bytes)
    );
    assert_eq!(
        tracked(TrackedText::Replace("new".into()))
            .unwrap()
            .to_docx()
            .unwrap(),
        on
    );
}

fn package_bytes(entries: BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn tracked_ids_reserve_relocated_xml_and_fail_closed_on_unreadable_xml() {
    use rocx::editor::{ImportedInlineNode, TrackedText};
    let mut entries = parts(&fixture());
    let types = String::from_utf8(entries["[Content_Types].xml"].clone())
        .unwrap()
        .replace(
            "</Types>",
            r#"<Override PartName="/extra/reviewer.data" ContentType="application/xml"/></Types>"#,
        );
    entries.insert("[Content_Types].xml".into(), types.into_bytes());
    // Valid XML without a leading '<', BOM or conventional extension.
    let payload = format!(r#"  <extra xmlns:w="{W}"><w:ins w:id="+0001"/></extra>"#);
    entries.insert("extra/reviewer.data".into(), payload.as_bytes().to_vec());
    let preview = |doc: &ImportedDocument| {
        let ImportedInlineNode::Run(r) = &paragraph(doc, 3).inline_structure[0] else {
            panic!()
        };
        doc.preview_tracked_text(&r.anchor, TrackedText::Delete, "Reviewer", None)
    };
    let source = ImportedDocument::open(&package_bytes(entries.clone())).unwrap();
    let bytes = preview(&source).unwrap().to_docx().unwrap();
    assert_eq!(parts(&bytes)["extra/reviewer.data"], payload.as_bytes());
    let reopened = ImportedDocument::open(&bytes).unwrap();
    let ImportedInlineNode::Revision(r) = &paragraph(&reopened, 3).inline_structure[0] else {
        panic!()
    };
    assert_ne!(r.id.as_deref(), Some("1"));
    entries.insert("extra/reviewer.data".into(), b"<invalid".to_vec());
    let source = ImportedDocument::open(&package_bytes(entries.clone())).unwrap();
    assert!(preview(&source).is_err());
    assert_eq!(
        parts(&source.preview(&[]).unwrap().to_docx().unwrap()),
        entries
    );
}

#[test]
fn inspect_style_metadata_and_preserve_source_part() {
    use rocx::editor::StyleKind;
    let mut entries = parts(include_bytes!("fixtures/imported-editor.docx"));
    let xml = format!(
        r#"<w:styles xmlns:w="{W}"><w:style w:type="paragraph" w:styleId="Custom"><w:name w:val="Custom Name"/><w:basedOn w:val="Normal"/><w:rPr><w:b/></w:rPr></w:style><w:style w:type="future" w:styleId="Unknown"/><w:style/><w:style w:type="character"/><w:style w:type="table"/><w:style w:type="numbering"/></w:styles>"#
    );
    entries.insert("word/styles.xml".into(), xml.as_bytes().to_vec());
    let bytes = package_bytes(entries);
    let doc = ImportedDocument::open(&bytes).unwrap();
    let styles = doc.styles().unwrap().unwrap();
    assert_eq!(styles.part_name, "word/styles.xml");
    assert_eq!(styles.styles.len(), 6);
    let style = &styles.styles[0];
    assert_eq!(style.id.as_deref(), Some("Custom"));
    assert_eq!(style.kind, StyleKind::Paragraph);
    assert_eq!(style.name.as_deref(), Some("Custom Name"));
    assert_eq!(style.based_on.as_deref(), Some("Normal"));
    assert_eq!(
        styles.styles[1].kind,
        StyleKind::Unsupported("future".into())
    );
    assert_eq!(styles.styles[2].kind, StyleKind::Missing);
    assert_eq!(styles.styles[2].id, None);
    assert_eq!(styles.styles[3].kind, StyleKind::Character);
    assert_eq!(styles.styles[4].kind, StyleKind::Table);
    assert_eq!(styles.styles[5].kind, StyleKind::Numbering);
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap())["word/styles.xml"],
        xml.as_bytes()
    );
}

#[test]
fn styles_absence_and_invalid_part_are_distinct() {
    let mut entries = parts(include_bytes!("fixtures/imported-editor.docx"));
    let rels = String::from_utf8(entries["word/_rels/document.xml.rels"].clone()).unwrap();
    let start = rels[..rels.find("/styles\"").unwrap()]
        .rfind("<Relationship ")
        .unwrap();
    let end = start + rels[start..].find("/>").unwrap() + 2;
    let mut without = rels.clone();
    without.replace_range(start..end, "");
    entries.insert("word/_rels/document.xml.rels".into(), without.into_bytes());
    let missing = ImportedDocument::open(&package_bytes(entries.clone())).unwrap();
    assert!(missing.styles().unwrap().is_none());
    entries.insert("word/_rels/document.xml.rels".into(), rels.into_bytes());
    entries.insert(
        "word/styles.xml".into(),
        format!(r#"<w:notStyles xmlns:w="{W}"/>"#).into_bytes(),
    );
    let invalid = ImportedDocument::open(&package_bytes(entries)).unwrap();
    assert!(invalid.styles().is_err());

    let mut wrong_type = parts(include_bytes!("fixtures/imported-editor.docx"));
    let types = String::from_utf8(wrong_type["[Content_Types].xml"].clone()).unwrap();
    wrong_type.insert(
        "[Content_Types].xml".into(),
        types
            .replace(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml",
                "application/xml",
            )
            .into_bytes(),
    );
    let invalid = ImportedDocument::open(&package_bytes(wrong_type)).unwrap();
    assert!(invalid.styles().is_err());
}

#[test]
fn new_styles_resolve_in_same_transaction_and_failed_insertions_do_not_publish() {
    use rocx::{
        Component, ParagraphFormat, ParagraphStyleDefinition,
        editor::{ParagraphInsertion, StyleEdit},
    };
    let source = fixture();
    let doc = ImportedDocument::open(&source).unwrap();
    let insert = |name| ParagraphInsertion {
        target: paragraph(&doc, 0).anchor.clone(),
        paragraph: block_on(Component::resolve(
            Paragraph::new("Styled insertion")
                .with_format(ParagraphFormat::default().with_style(name)),
        ))
        .unwrap()
        .unwrap(),
    };
    let add = StyleEdit::Add(
        ParagraphStyleDefinition::default()
            .with_name("Imported Accent")
            .with_base_style("Normal")
            .into(),
    );
    let stage = |p| {
        doc.preview_with_styles(
            &[],
            &[],
            &[],
            &[],
            &[p],
            &[],
            &[],
            std::slice::from_ref(&add),
        )
    };
    assert!(stage(insert("Absent")).is_err());
    assert!(doc.style_anchor("ImportedAccent").is_err());
    let out = parts(&stage(insert("Imported Accent")).unwrap().to_docx().unwrap());
    assert!(
        std::str::from_utf8(&out["word/document.xml"])
            .unwrap()
            .contains(r#"<w:pStyle w:val="ImportedAccent""#)
    );
    for (name, bytes) in parts(&source) {
        if name != "word/styles.xml" && name != "word/document.xml" {
            assert_eq!(out[&name], bytes);
        }
    }
}

#[test]
fn styles_follow_relocated_relationship_and_reject_ambiguity() {
    let mut entries = parts(include_bytes!("fixtures/imported-editor.docx"));
    let rels = String::from_utf8(entries["word/_rels/document.xml.rels"].clone()).unwrap();
    let relocated = rels.replace("Target=\"styles.xml\"", "Target=\"resources/custom.xml\"");
    entries.insert(
        "word/_rels/document.xml.rels".into(),
        relocated.clone().into_bytes(),
    );
    let content_types = String::from_utf8(entries["[Content_Types].xml"].clone()).unwrap();
    entries.insert(
        "[Content_Types].xml".into(),
        content_types
            .replace("/word/styles.xml", "/word/resources/custom.xml")
            .into_bytes(),
    );
    let style_xml = entries.remove("word/styles.xml").unwrap();
    entries.insert("word/resources/custom.xml".into(), style_xml.clone());
    let doc = ImportedDocument::open(&package_bytes(entries.clone())).unwrap();
    assert_eq!(
        doc.styles().unwrap().unwrap().part_name,
        "word/resources/custom.xml"
    );
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap())["word/resources/custom.xml"],
        style_xml
    );
    let duplicate = r#"<Relationship Id="rStyleExtra" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="resources/custom.xml"/>"#;
    entries.insert(
        "word/_rels/document.xml.rels".into(),
        relocated
            .replace("</Relationships>", &format!("{duplicate}</Relationships>"))
            .into_bytes(),
    );
    let doc = ImportedDocument::open(&package_bytes(entries)).unwrap();
    assert!(doc.styles().is_err());
}

fn paragraph(doc: &ImportedDocument, index: usize) -> &rocx::editor::ImportedParagraph {
    match &doc.body()[index] {
        BodyBlock::Paragraph(p) => p,
        _ => panic!("expected paragraph at {index}"),
    }
}

#[test]
fn removes_original_run_targets_and_rejects_unsafe_or_conflicting_batches() {
    use rocx::editor::{ImportedInlineNode::Run, RunTextEdit};
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p>
        <w:r><w:rPr><w:b/></w:rPr><w:t>first</w:t></w:r>
        <w:r><w:t>keep</w:t></w:r><w:r><w:t>last</w:t></w:r>
        <w:r><w:t>bad</w:t><w:tab/></w:r>
        <w:r><w:rPr><w:rPrChange w:id="1"/></w:rPr><w:t>revision</w:t></w:r>
        <w:r w:rsidR="12345678"><w:t>attributed</w:t></w:r>
        <w:r><w:rPr><w:b w:id="3"/></w:rPr><w:t>unknown attribute</w:t></w:r>
        <w:bookmarkStart w:id="7" w:name="guard"/><w:r><w:t>in range</w:t></w:r><w:bookmarkEnd w:id="7"/>
        </w:p><w:sectPr/></w:body></w:document>"#;
    let data = fixture_with_body(xml);
    let doc = ImportedDocument::open(&data).unwrap();
    let nodes = &paragraph(&doc, 0).inline_structure;
    let anchor = |i| match &nodes[i] {
        Run(run) => run.anchor.clone(),
        _ => panic!("run"),
    };
    let preview = doc
        .preview_with_run_removals(&[], &[], &[], &[], &[anchor(2), anchor(0)])
        .unwrap();
    assert_eq!(
        preview
            .run_removals()
            .iter()
            .map(|c| (c.inline_index, c.before.as_str()))
            .collect::<Vec<_>>(),
        [(0, "first"), (2, "last")]
    );
    let output = preview.to_docx().unwrap();
    assert_eq!(
        paragraph(&ImportedDocument::open(&output).unwrap(), 0).text,
        "keepbad\trevisionattributedunknown attributein range"
    );
    for (name, value) in parts(&data) {
        if name != "word/document.xml" {
            assert_eq!(parts(&output)[&name], value, "{name}");
        }
    }
    let stale = ImportedDocument::open(&data).unwrap();
    let Run(stale_run) = &paragraph(&stale, 0).inline_structure[0] else {
        panic!("run")
    };
    for targets in [
        vec![anchor(0), anchor(0)],
        vec![anchor(0), anchor(3)],
        vec![anchor(0), anchor(4)],
        vec![anchor(0), anchor(5)],
        vec![anchor(0), anchor(6)],
        vec![anchor(0), anchor(8)],
        vec![anchor(0), stale_run.anchor.clone()],
    ] {
        assert!(
            doc.preview_with_run_removals(&[], &[], &[], &[], &targets)
                .is_err()
        );
    }
    assert!(
        doc.preview_with_run_removals(
            &[],
            &[],
            &[],
            &[RunTextEdit {
                target: anchor(0),
                text: "x".into()
            }],
            &[anchor(0)]
        )
        .is_err()
    );
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        parts(&data)
    );
}

#[test]
fn whole_inline_edits_respect_cross_paragraph_fields_and_keep_opaque_formatting() {
    use rocx::editor::{ImportedInlineNode::Run, InlineAction, InlineEdit};
    let xml = format!(
        r#"<w:document xmlns:w="{W}" xmlns:ext="urn:fixture"><w:body>
        <w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r></w:p>
        <w:p><w:r><w:t>cache</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r>
        <w:r><w:rPr><w:b ext:token="keep"/><ext:decoration ext:version="7">opaque</ext:decoration></w:rPr><w:t>visible</w:t></w:r><w:r><w:cr/></w:r></w:p>
        <w:sectPr/></w:body></w:document>"#
    );
    let doc = ImportedDocument::open(&fixture_with_body(&xml)).unwrap();
    let node = |index: usize, action| {
        let Run(run) = &paragraph(&doc, 1).inline_structure[index] else {
            panic!("expected a run")
        };
        InlineEdit {
            target: run.anchor.clone(),
            action,
        }
    };
    let preview =
        |edits: &[InlineEdit]| doc.preview_with_inlines(&[], &[], &[], &[], &[], &[], &[], edits);
    let line = rocx::ResolvedInline::Break {
        kind: rocx::BreakType::Line,
        format: Default::default(),
        comment: None,
    };
    assert!(
        preview(&[
            node(2, InlineAction::SetBold(false)),
            node(0, InlineAction::InsertAfter(line.clone()))
        ])
        .is_err()
    );
    let carriage_return = preview(&[node(3, InlineAction::Replace(line.clone()))]).unwrap();
    assert_eq!(carriage_return.inline_changes(), &[(1, 3)]);
    assert!(preview(&[node(2, InlineAction::Replace(line))]).is_err());
    let bold = preview(&[node(2, InlineAction::SetBold(false))]).unwrap();
    assert_eq!(bold.inline_changes(), &[(1, 2)]);
    let package = parts(&bold.to_docx().unwrap());
    let result =
        roxmltree::Document::parse(std::str::from_utf8(&package["word/document.xml"]).unwrap())
            .unwrap();
    let prop = result
        .descendants()
        .find(|n| n.has_tag_name((W, "b")))
        .unwrap();
    assert_eq!(prop.attribute((W, "val")), Some("0"));
    assert_eq!(prop.attribute(("urn:fixture", "token")), Some("keep"));
    let opaque = prop.next_sibling_element().unwrap();
    assert!(opaque.has_tag_name(("urn:fixture", "decoration")));
    assert_eq!(
        (opaque.attribute(("urn:fixture", "version")), opaque.text()),
        (Some("7"), Some("opaque"))
    );
}

// Compare XML structure rather than serialization; the edited part may be reserialized.
fn tree(node: roxmltree::Node<'_, '_>, skip_jc: bool) -> String {
    if node.is_text() {
        let text = node.text().unwrap_or("");
        return if text.trim().is_empty() && !node.parent().is_some_and(|n| n.has_tag_name((W, "t")))
        {
            String::new()
        } else {
            text.to_owned()
        };
    }
    if !node.is_element() || (skip_jc && node.has_tag_name((W, "jc"))) {
        return String::new();
    }
    let mut attrs: Vec<_> = node
        .attributes()
        .map(|a| (a.namespace().unwrap_or(""), a.name(), a.value()))
        .collect();
    attrs.sort_unstable();
    format!(
        "<{:?}{attrs:?}>{}</>",
        node.tag_name(),
        node.children()
            .map(|n| tree(n, skip_jc))
            .collect::<String>()
    )
}

#[test]
fn reads_direct_body_blocks_and_paragraph_properties() {
    let doc = ImportedDocument::open(&fixture()).unwrap();
    assert!(doc.body().len() >= 4);
    let first = paragraph(&doc, 0);
    assert_eq!(first.text, "Hello world");
    assert_eq!(first.alignment, Some(Alignment::Left));
    assert_eq!(first.alignment_xml.as_deref(), Some("left"));
    assert!(first.has_opaque_content);
    let BodyBlock::Table(table) = &doc.body()[1] else {
        panic!("expected table")
    };
    let [rocx::editor::ImportedTableChild::Row(row)] = table.children.as_slice() else {
        panic!("expected row")
    };
    let [rocx::editor::ImportedRowChild::Cell(cell)] = row.children.as_slice() else {
        panic!("expected cell")
    };
    let [rocx::editor::ImportedCellBlock::Paragraph(items)] = cell.blocks.as_slice() else {
        panic!("expected paragraph")
    };
    assert_eq!(
        items,
        &[rocx::editor::ImportedInline::Text("in table".into())]
    );
    let BodyBlock::ContentControl(control) = &doc.body()[2] else {
        panic!("expected content control")
    };
    assert_eq!(
        (&control.tag, &control.alias, &control.lock),
        (&None, &None, &None)
    );
    let second = paragraph(&doc, 3);
    assert_eq!(second.text, "Second");
    assert_eq!(second.alignment, None);
    assert_eq!(second.alignment_xml, None);
    assert!(!second.has_opaque_content);
}

#[test]
fn control_targets_refuse_active_ranges_nested_wrappers_and_ambiguous_properties() {
    use rocx::editor::ControlEdit;
    let inline = "<w:sdt><w:sdtPr><w:tag w:val='copy'/></w:sdtPr><w:sdtContent><w:r><w:t>keep</w:t></w:r></w:sdtContent></w:sdt>";
    let document = |contents: &str| {
        let body = format!(
            "<w:document xmlns:w='{W}'><w:body>{contents}<w:sectPr/></w:body></w:document>"
        );
        ImportedDocument::open(&fixture_with_body(&body)).unwrap()
    };
    let nested =
        format!("<w:sdt><w:sdtPr/><w:sdtContent><w:p>{inline}</w:p></w:sdtContent></w:sdt>");
    for (xml, i, reason) in [
        (
            format!(
                "<w:p><w:bookmarkStart w:id='1' w:name='a'/>{inline}<w:bookmarkEnd w:id='1'/></w:p>"
            ),
            0,
            "protected",
        ),
        (
            format!("<w:p><w:r><w:fldChar w:fldCharType='begin'/></w:r>{inline}</w:p>"),
            0,
            "protected",
        ),
        (nested.clone(), 0, "structure"),
        (nested, 1, "nested"),
        (
            format!(
                "<w:p>{}</w:p>",
                inline.replace("</w:sdtPr>", "<w:tag w:val='other'/></w:sdtPr>")
            ),
            0,
            "ambiguous",
        ),
        (
            format!(
                "<w:p>{}</w:p>",
                inline.replace("</w:sdtPr>", "<w:lock w:val='unknown'/></w:sdtPr>")
            ),
            0,
            "locked",
        ),
        (
            format!(
                "<w:p>{}</w:p>",
                inline.replace(
                    "</w:sdtPr>",
                    "<x:dataBinding xmlns:x='urn:future'/></w:sdtPr>"
                )
            ),
            0,
            "XML-bound",
        ),
        (
            format!(
                "<w:p>{}</w:p>",
                inline.replace("</w:sdtPr>", "<w:dropDownList/></w:sdtPr>")
            ),
            0,
            "properties",
        ),
    ] {
        let doc = document(&xml);
        let bad = ControlEdit {
            target: doc.controls()[i].anchor.clone(),
            tag: None,
            alias: Some("after".into()),
            text: None,
        };
        let error = doc
            .preview_with_controls(&[], &[], &[], &[], &[], &[], &[], &[bad])
            .unwrap_err();
        assert!(error.to_string().contains(reason), "{error}");
    }
}

#[test]
fn direct_control_metadata_is_read_only_and_preserved() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>
<w:sdt><w:sdtPr><w:tag w:val="customer &amp; id"/><w:alias w:val="Name"/><w:lock w:val="futureLock"/></w:sdtPr><w:sdtContent><w:p><w:r><w:t>secret</w:t></w:r></w:p></w:sdtContent></w:sdt>
<w:sdt><w:sdtPr><w:tag/><w:alias w:val=""/></w:sdtPr><w:sdtContent/></w:sdt>
<w:p><w:r><w:t>outside</w:t></w:r></w:p><w:sectPr/></w:body></w:document>"#;
    let input = fixture_with_body(xml);
    let doc = ImportedDocument::open(&input).unwrap();
    let BodyBlock::ContentControl(first) = &doc.body()[0] else {
        panic!("control")
    };
    assert_eq!(first.tag.as_deref(), Some("customer & id"));
    assert_eq!(first.alias.as_deref(), Some("Name"));
    assert_eq!(first.lock.as_deref(), Some("futureLock"));
    let BodyBlock::ContentControl(second) = &doc.body()[1] else {
        panic!("control")
    };
    assert_eq!(second.tag, None);
    assert_eq!(second.alias.as_deref(), Some(""));
    assert_eq!(second.lock, None);
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        parts(&input)
    );
    let output = doc
        .preview(&[ParagraphAlignmentEdit {
            target: paragraph(&doc, 2).anchor.clone(),
            alignment: Alignment::Center,
        }])
        .unwrap()
        .to_docx()
        .unwrap();
    let before = parts(&input);
    let after = parts(&output);
    for (name, bytes) in &before {
        if name != "word/document.xml" {
            assert_eq!(&after[name], bytes);
        }
    }
    let before_xml = std::str::from_utf8(&before["word/document.xml"]).unwrap();
    let after_xml = std::str::from_utf8(&after["word/document.xml"]).unwrap();
    let before_tree = roxmltree::Document::parse(before_xml).unwrap();
    let after_tree = roxmltree::Document::parse(after_xml).unwrap();
    let controls = |doc: &roxmltree::Document<'_>| {
        doc.descendants()
            .filter(|n| n.has_tag_name((W, "sdt")))
            .map(|n| tree(n, false))
            .collect::<Vec<_>>()
    };
    assert_eq!(controls(&before_tree), controls(&after_tree));
}

#[test]
fn table_inspection_preserves_unknown_children_in_order() {
    use rocx::editor::{ImportedCellBlock, ImportedRowChild, ImportedTableChild};
    let xml = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:tbl><w:tblPr/><w:customXml/><w:tr><w:trPr/><w:sdt/><w:tc><w:tcPr/><w:p><w:r><w:t>x</w:t><w:drawing/></w:r></w:p><w:tbl/><w:sdt/></w:tc></w:tr></w:tbl></w:body></w:document>"#
    );
    let doc = ImportedDocument::open(&fixture_with_body(&xml)).unwrap();
    let BodyBlock::Table(table) = &doc.body()[0] else {
        panic!()
    };
    assert!(
        matches!(&table.children[0], ImportedTableChild::Opaque(name) if name == "w:customXml")
    );
    let ImportedTableChild::Row(row) = &table.children[1] else {
        panic!()
    };
    assert!(matches!(&row.children[0], ImportedRowChild::Opaque(name) if name == "w:sdt"));
    let ImportedRowChild::Cell(cell) = &row.children[1] else {
        panic!()
    };
    assert!(
        matches!(&cell.blocks[0], ImportedCellBlock::Paragraph(items) if matches!(&items[1], rocx::editor::ImportedInline::Opaque(name) if name == "w:drawing"))
    );
    assert!(matches!(&cell.blocks[1], ImportedCellBlock::Opaque(name) if name == "w:tbl"));
    assert!(matches!(&cell.blocks[2], ImportedCellBlock::Opaque(name) if name == "w:sdt"));
    let [node @ rocx::editor::ImportedInlineNode::Run(run)] = cell.inline_structure(0).unwrap()
    else {
        panic!("expected run in cell after opaque table and row children");
    };
    assert!(std::ptr::eq(doc.inline_at(&run.anchor).unwrap(), node));
    assert!(cell.inline_structure(1).is_none());
    assert!(cell.inline_structure(2).is_none());
}

#[test]
fn independent_table_cell_inline_anchors_identify_physical_locations() {
    use rocx::editor::ImportedInline::{self, Break, Tab, Text};
    use rocx::editor::ImportedInlineNode::{Hyperlink, Opaque, Run};
    use rocx::editor::{ImportedCellBlock, ImportedRowChild, ImportedTableChild};

    let input = include_bytes!("fixtures/imported-table-cells.docx");
    let doc = ImportedDocument::open(input).unwrap();
    let cell_at = |table: usize, row: usize, cell: usize| {
        let BodyBlock::Table(table) = &doc.body()[table] else {
            panic!("expected table");
        };
        let ImportedTableChild::Row(row) = &table.children[row] else {
            panic!("expected row");
        };
        let ImportedRowChild::Cell(cell) = &row.children[cell] else {
            panic!("expected cell");
        };
        cell
    };
    let first = cell_at(1, 0, 0);
    let [
        label_node @ Run(label),
        Opaque(start),
        link_node @ Hyperlink(link),
        Opaque(end),
    ] = first.inline_structure(0).unwrap()
    else {
        panic!("expected a run and a link bracketed by opaque bookmark endpoints");
    };
    assert_eq!(
        label.items,
        [
            Text("Cell label".into()),
            Tab,
            Break(rocx::BreakType::TextWrapping)
        ]
    );
    assert_eq!(
        (start.as_str(), end.as_str()),
        ("w:bookmarkStart", "w:bookmarkEnd")
    );
    assert_eq!(link.bookmark, None);
    let [Run(prefix), last_node @ Run(suffix)] = link.children.as_slice() else {
        panic!("expected two hyperlink runs");
    };
    assert_eq!(prefix.items, [Text("cell ".into())]);
    assert_eq!(suffix.items, [Text("link".into())]);
    let ImportedCellBlock::Paragraph(items) = &first.blocks[0] else {
        panic!("expected paragraph");
    };
    assert_eq!(
        items,
        &[
            Text("Cell label".into()),
            Tab,
            Break(rocx::BreakType::TextWrapping),
            ImportedInline::Opaque("w:bookmarkStart".into()),
            Text("cell ".into()),
            Text("link".into()),
            ImportedInline::Opaque("w:bookmarkEnd".into()),
        ]
    );
    assert!(matches!(&first.blocks[1], ImportedCellBlock::Opaque(name) if name == "w:tbl"));
    assert!(first.inline_structure(1).is_none());
    assert!(first.inline_structure(3).unwrap().is_empty());
    assert!(first.inline_structure(4).is_none());

    for (node, anchor) in [
        (label_node, &label.anchor),
        (link_node, &link.anchor),
        (last_node, &suffix.anchor),
    ] {
        assert!(std::ptr::eq(doc.inline_at(anchor).unwrap(), node));
    }
    let mut seen = vec![&label.anchor];
    for node in [
        &paragraph(&doc, 0).inline_structure[0],
        &first.inline_structure(2).unwrap()[0],
        &cell_at(1, 0, 1).inline_structure(0).unwrap()[0],
        &cell_at(1, 1, 0).inline_structure(0).unwrap()[0],
        &cell_at(2, 0, 0).inline_structure(0).unwrap()[0],
    ] {
        let Run(run) = node else {
            panic!("expected run")
        };
        assert_eq!(run.items, [Text("Cell label".into())]);
        assert!(!seen.contains(&&run.anchor));
        seen.push(&run.anchor);
        assert!(std::ptr::eq(doc.inline_at(&run.anchor).unwrap(), node));
    }

    let original = parts(input);
    let rels = std::str::from_utf8(&original["word/_rels/document.xml.rels"]).unwrap();
    let rels = roxmltree::Document::parse(rels).unwrap();
    let id = link.relationship_id.as_deref().expect("explicit r:id");
    let relationship = rels
        .descendants()
        .find(|n| n.attribute("Id") == Some(id))
        .unwrap();
    assert_eq!(
        relationship.attribute("Target"),
        Some("https://example.invalid/cell")
    );
    assert_eq!(relationship.attribute("TargetMode"), Some("External"));
    let output = doc.preview(&[]).unwrap().to_docx().unwrap();
    assert_eq!(parts(&output), original);
    for bytes in [input.as_slice(), &output] {
        let reopened = ImportedDocument::open(bytes).unwrap();
        let error = reopened.inline_at(&suffix.anchor).unwrap_err();
        assert!(error.to_string().contains("different document snapshot"));
    }
}

#[test]
fn inspects_ordered_inline_items_without_changing_the_text_projection() {
    use rocx::BreakType;
    use rocx::editor::ImportedInline::{Break, Opaque, Tab, Text};
    use rocx::editor::ImportedInlineNode::{Hyperlink, Run};
    let xml = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:p><w:r><w:t xml:space="preserve">a </w:t><w:tab/><w:br/><w:br w:type="page"/><w:cr/><w:br w:type="column"/><w:drawing/></w:r><w:hyperlink w:anchor="bookmark"><w:r><w:t>b</w:t></w:r></w:hyperlink></w:p></w:body></w:document>"#
    );
    let doc = ImportedDocument::open(&fixture_with_body(&xml)).unwrap();
    let first = paragraph(&doc, 0);
    assert_eq!(first.text, "a \t\n\nb");
    assert!(first.has_opaque_content);
    assert_eq!(
        first.inline_items,
        [
            Text("a ".into()),
            Tab,
            Break(BreakType::TextWrapping),
            Break(BreakType::Page),
            Break(BreakType::Line),
            Break(BreakType::Column),
            Opaque("w:drawing".into()),
            Text("b".into()),
        ]
    );
    let [Run(run), Hyperlink(link)] = first.inline_structure.as_slice() else {
        panic!("expected direct run followed by hyperlink");
    };
    assert_eq!(run.items, first.inline_items[..7]);
    assert_eq!(link.bookmark.as_deref(), Some("bookmark"));
    assert_eq!(link.relationship_id, None);
}

#[test]
fn independent_body_inline_anchors_are_snapshot_scoped_and_read_only() {
    use rocx::editor::ImportedInline::Text;
    use rocx::editor::ImportedInlineNode::{Hyperlink, Opaque, Run};

    let input = include_bytes!("fixtures/imported-editor.docx");
    let doc = ImportedDocument::open(input).unwrap();
    let p = paragraph(&doc, 0);
    assert_eq!(p.text, "Patch me");
    let [
        Run(label),
        Opaque(start),
        Run(empty),
        Hyperlink(link),
        Opaque(end),
    ] = p.inline_structure.as_slice()
    else {
        panic!("expected direct runs, hyperlink, and opaque bookmark endpoints");
    };
    assert_eq!(label.items, [Text("Patch".into())]);
    assert_eq!(start, "w:bookmarkStart");
    assert!(empty.items.is_empty());
    assert_eq!(end, "w:bookmarkEnd");
    assert_eq!(link.bookmark, None);
    let relationship_id = link.relationship_id.as_deref().expect("explicit r:id");
    let original = parts(input);
    let relationships = std::str::from_utf8(&original["word/_rels/document.xml.rels"]).unwrap();
    let relationships = roxmltree::Document::parse(relationships).unwrap();
    let relationship = relationships
        .descendants()
        .find(|node| node.attribute("Id") == Some(relationship_id))
        .unwrap();
    assert_eq!(
        relationship.attribute("Target"),
        Some("https://example.invalid/body")
    );
    assert_eq!(relationship.attribute("TargetMode"), Some("External"));
    let [Run(space), Run(word)] = link.children.as_slice() else {
        panic!("expected distinct runs inside the hyperlink");
    };
    assert_eq!(space.items, [Text(" ".into())]);
    assert_eq!(word.items, [Text("me".into())]);
    assert_ne!(label.anchor, empty.anchor);
    assert_ne!(link.anchor, space.anchor);
    assert_ne!(space.anchor, word.anchor);
    assert!(
        matches!(doc.inline_at(&empty.anchor).unwrap(), Run(found) if found.anchor == empty.anchor)
    );
    assert!(
        matches!(doc.inline_at(&link.anchor).unwrap(), Hyperlink(found) if found.anchor == link.anchor)
    );
    assert!(
        matches!(doc.inline_at(&word.anchor).unwrap(), Run(found) if found.anchor == word.anchor)
    );
    let other = ImportedDocument::open(input).unwrap();
    let error = other.inline_at(&word.anchor).unwrap_err();
    assert!(error.to_string().contains("different document snapshot"));
    let saved = doc.preview(&[]).unwrap().to_docx().unwrap();
    let reopened = ImportedDocument::open(&saved).unwrap();
    assert!(
        reopened
            .inline_at(&word.anchor)
            .unwrap_err()
            .to_string()
            .contains("different document snapshot")
    );
    assert!(matches!(doc.inline_at(&word.anchor).unwrap(), Run(_)));
    assert_eq!(parts(&saved), original);
}

#[test]
fn patch_preserves_other_markup_and_parts_and_preview_is_non_mutating() {
    let input = fixture();
    let doc = ImportedDocument::open(&input).unwrap();
    let edit = ParagraphAlignmentEdit {
        target: paragraph(&doc, 0).anchor.clone(),
        alignment: Alignment::Right,
    };
    let preview = doc.preview(&[edit]).unwrap();
    assert_eq!(preview.changes().len(), 1);
    assert_eq!(preview.changes()[0].body_index, 0);
    assert_eq!(preview.changes()[0].before.as_deref(), Some("left"));
    assert_eq!(preview.changes()[0].after, Alignment::Right);
    assert_eq!(paragraph(&doc, 0).alignment, Some(Alignment::Left));
    let output = preview.to_docx().unwrap();
    let original_parts = parts(&input);
    let output_parts = parts(&output);
    assert_eq!(original_parts.len(), output_parts.len());
    for (name, payload) in &original_parts {
        if name != "word/document.xml" {
            assert_eq!(output_parts.get(name), Some(payload), "part {name}");
        }
    }
    let new_xml = std::str::from_utf8(&output_parts["word/document.xml"]).unwrap();
    let old = roxmltree::Document::parse(BODY).unwrap();
    let new = roxmltree::Document::parse(new_xml).unwrap();
    let old_body = old
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    let new_body = new
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    assert_eq!(tree(old_body, true), tree(new_body, true));
    let jc = new
        .descendants()
        .find(|n| n.has_tag_name((W, "jc")))
        .unwrap();
    assert_eq!(jc.attribute((W, "val")), Some("right"));
}

#[test]
fn retained_previews_write_independent_edits_after_source_drop() {
    let input = include_bytes!("fixtures/imported-editor.docx");
    let original = parts(input);
    let alignments = [Alignment::Right, Alignment::Center];
    let (unchanged, previews) = {
        let doc = ImportedDocument::open(input).unwrap();
        let target = &paragraph(&doc, 0).anchor;
        let previews = alignments.map(|alignment| {
            doc.preview(&[ParagraphAlignmentEdit {
                target: target.clone(),
                alignment,
            }])
            .unwrap()
        });
        (doc.preview(&[]).unwrap(), previews)
    };

    assert_eq!(parts(&unchanged.to_docx().unwrap()), original);
    for (preview, alignment) in previews.iter().zip(alignments) {
        let output = preview.to_docx().unwrap();
        let reopened = ImportedDocument::open(&output).unwrap();
        assert_eq!(paragraph(&reopened, 0).alignment, Some(alignment));

        let output_parts = parts(&output);
        assert_eq!(output_parts.len(), original.len());
        for (name, payload) in &original {
            if name != "word/document.xml" {
                assert_eq!(output_parts.get(name), Some(payload), "part {name}");
            }
        }
    }
}

#[test]
fn inspects_explicit_and_implicit_sections_and_refuses_ambiguity() {
    let xml = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:p><w:pPr><w:sectPr><w:type w:val="continuous"/></w:sectPr></w:pPr></w:p><w:p><w:pPr><w:sectPr><w:type w:val="unknown"/></w:sectPr></w:pPr></w:p><w:tbl/></w:body></w:document>"#
    );
    let doc = ImportedDocument::open(&fixture_with_body(&xml)).unwrap();
    let sections = doc.sections().unwrap();
    assert_eq!(
        sections
            .iter()
            .map(|s| (s.body_start, s.body_end, s.section_type_xml.as_deref()))
            .collect::<Vec<_>>(),
        [
            (0, 1, Some("continuous")),
            (1, 2, Some("unknown")),
            (2, 3, None),
        ]
    );
    assert_eq!(
        sections[0].section_type,
        Some(rocx::SectionType::Continuous)
    );
    assert_eq!(sections[1].section_type, None);
    let duplicate = xml.replace(
        r#"<w:type w:val="continuous"/>"#,
        r#"<w:type w:val="continuous"/><w:type w:val="nextPage"/>"#,
    );
    let doc = ImportedDocument::open(&fixture_with_body(&duplicate)).unwrap();
    assert!(doc.sections().is_err());
}

#[test]
fn inserts_alignment_in_schema_order() {
    let input = fixture();
    let doc = ImportedDocument::open(&input).unwrap();
    let preview = doc
        .preview(&[ParagraphAlignmentEdit {
            target: paragraph(&doc, 3).anchor.clone(),
            alignment: Alignment::Center,
        }])
        .unwrap();
    assert_eq!(preview.changes()[0].body_index, 3);
    assert_eq!(preview.changes()[0].before, None);
    let output = parts(&preview.to_docx().unwrap());
    let xml = std::str::from_utf8(&output["word/document.xml"]).unwrap();
    let parsed = roxmltree::Document::parse(xml).unwrap();
    let p = parsed
        .descendants()
        .rfind(|n| n.has_tag_name((W, "p")))
        .unwrap();
    let ppr = p.children().find(|n| n.has_tag_name((W, "pPr"))).unwrap();
    let names: Vec<_> = ppr
        .children()
        .filter(|n| n.is_element())
        .map(|n| n.tag_name().name())
        .collect();
    assert_eq!(names, ["keepNext", "jc", "textDirection"]);
    assert_eq!(
        ppr.children()
            .find(|n| n.has_tag_name((W, "jc")))
            .unwrap()
            .attribute((W, "val")),
        Some("center")
    );
}

#[test]
fn rejects_foreign_and_duplicate_anchors_without_mutation() {
    let input = fixture();
    let doc = ImportedDocument::open(&input).unwrap();
    let other = ImportedDocument::open(&input).unwrap();
    let foreign = ParagraphAlignmentEdit {
        target: paragraph(&other, 0).anchor.clone(),
        alignment: Alignment::Center,
    };
    assert!(doc.preview(&[foreign]).is_err());
    let duplicate = || ParagraphAlignmentEdit {
        target: paragraph(&doc, 0).anchor.clone(),
        alignment: Alignment::Right,
    };
    assert!(doc.preview(&[duplicate(), duplicate()]).is_err());
    assert_eq!(paragraph(&doc, 0).alignment, Some(Alignment::Left));
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        parts(&input)
    );
}

#[test]
fn no_op_preserves_all_payloads() {
    let input = fixture();
    let doc = ImportedDocument::open(&input).unwrap();
    let empty = doc.preview(&[]).unwrap();
    assert!(empty.changes().is_empty());
    assert_eq!(parts(&empty.to_docx().unwrap()), parts(&input));
    let same = doc
        .preview(&[ParagraphAlignmentEdit {
            target: paragraph(&doc, 0).anchor.clone(),
            alignment: Alignment::Left,
        }])
        .unwrap();
    assert!(same.changes().is_empty());
    assert_eq!(parts(&same.to_docx().unwrap()), parts(&input));
}

#[test]
fn rejects_ambiguous_alignment_without_staging_other_changes() {
    let xml = BODY.replacen(
        "<w:keepNext/><w:textDirection",
        r#"<w:keepNext/><mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><mc:Fallback><w:jc w:val="left"/></mc:Fallback></mc:AlternateContent><w:textDirection"#,
        1,
    );
    let input = fixture_with_body(&xml);
    let doc = ImportedDocument::open(&input).unwrap();
    let result = doc.preview(&[
        ParagraphAlignmentEdit {
            target: paragraph(&doc, 0).anchor.clone(),
            alignment: Alignment::Right,
        },
        ParagraphAlignmentEdit {
            target: paragraph(&doc, 3).anchor.clone(),
            alignment: Alignment::Center,
        },
    ]);
    assert!(result.unwrap_err().to_string().contains("ambiguous"));
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        parts(&input)
    );
}

fn insertion_table() -> rocx::ResolvedTable {
    use rocx::{Table, TableCell, TableRow};
    let table = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![Paragraph::new("inserted A").into()]).into(),
            TableCell::new(vec![Paragraph::new("inserted B").into()]).into(),
        ])
        .into(),
    ]);
    resolve_insertion_table(table)
}

fn resolve_insertion_table(table: rocx::Table) -> rocx::ResolvedTable {
    use rocx::ResolvedBlock;
    let resolved =
        block_on(Document::new(vec![Section::new(vec![table.into()]).into()]).resolve()).unwrap();
    match &resolved.sections()[0].blocks()[0] {
        ResolvedBlock::Table(table) => table.clone(),
        _ => panic!("expected table"),
    }
}

#[test]
fn mixed_cell_paragraph_and_run_edits_use_original_indices_atomically() {
    use rocx::{
        ResolvedBlock, VerticalAlignment,
        editor::{CellAlignmentEdit, ImportedInlineNode::Run, ParagraphInsertion, RunTextEdit},
    };
    let source = fixture_with_body(
        &BODY
            .replace(
                "<w:tbl><w:tr>",
                "<w:tbl><w:tblGrid><w:gridCol/></w:tblGrid><w:tr>",
            )
            .replace(
                "<w:t>Second</w:t></w:r>",
                "<w:t>keep</w:t></w:r><w:r><w:t>remove</w:t></w:r>",
            ),
    );
    let doc = ImportedDocument::open(&source).unwrap();
    let payload = block_on(
        Document::new(vec![
            Section::new(vec![Paragraph::new("inserted").into()]).into(),
        ])
        .resolve(),
    )
    .unwrap();
    let ResolvedBlock::Paragraph(content) = &payload.sections()[0].blocks()[0] else {
        panic!("paragraph")
    };
    let BodyBlock::Table(table) = &doc.body()[1] else {
        panic!("table")
    };
    let Run(run) = &paragraph(&doc, 3).inline_structure[1] else {
        panic!("run")
    };
    let paragraphs = [ParagraphInsertion {
        target: paragraph(&doc, 0).anchor.clone(),
        paragraph: content.clone(),
    }];
    let cells = [CellAlignmentEdit {
        target: table.anchor.clone(),
        row_index: 0,
        cell_index: 0,
        alignment: VerticalAlignment::Bottom,
    }];
    let removals = [run.anchor.clone()];
    let batch = |text: &[RunTextEdit]| {
        doc.preview_mixed(&[], &[], &[], text, &paragraphs, &cells, &removals)
    };
    let preview = batch(&[]).unwrap();
    assert_eq!(preview.paragraph_insertions(), &[0]);
    let [cell] = preview.cell_alignment_changes() else {
        panic!("one cell patch")
    };
    assert_eq!(
        (cell.body_index, cell.row_index, cell.cell_index),
        (1, 0, 0)
    );
    let [removed] = preview.run_removals() else {
        panic!("one removal")
    };
    assert_eq!(
        (
            removed.body_index,
            removed.inline_index,
            removed.before.as_str()
        ),
        (3, 1, "remove")
    );
    let output = preview.to_docx().unwrap();
    let reopened = ImportedDocument::open(&output).unwrap();
    assert_eq!(paragraph(&reopened, 1).text, "inserted");
    assert!(matches!(reopened.body()[2], BodyBlock::Table(_)));
    assert_eq!(paragraph(&reopened, 4).text, "keep");
    let output_parts = parts(&output);
    let xml = roxmltree::Document::parse(
        std::str::from_utf8(&output_parts["word/document.xml"]).unwrap(),
    )
    .unwrap();
    let cell_align = xml
        .descendants()
        .find(|n| n.has_tag_name((W, "vAlign")))
        .unwrap();
    assert_eq!(cell_align.attribute((W, "val")), Some("bottom"));
    assert!(
        batch(&[RunTextEdit {
            target: run.anchor.clone(),
            text: "conflict".into()
        }])
        .is_err()
    );
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        parts(&source)
    );
    assert_eq!(batch(&[]).unwrap().to_docx().unwrap(), output);
}

#[test]
fn paragraph_insertions_are_atomic_and_preserve_other_parts() {
    use rocx::{
        ResolvedBlock,
        editor::{ParagraphInsertion, TableInsertion},
    };
    let resolved = block_on(
        Document::new(vec![
            Section::new(vec![Paragraph::new("new paragraph").into()]).into(),
        ])
        .resolve(),
    )
    .unwrap();
    let ResolvedBlock::Paragraph(content) = &resolved.sections()[0].blocks()[0] else {
        panic!("expected paragraph");
    };
    let source = fixture();
    let doc = ImportedDocument::open(&source).unwrap();
    let insertion = ParagraphInsertion {
        target: paragraph(&doc, 0).anchor.clone(),
        paragraph: content.clone(),
    };
    let preview = doc
        .preview_with_paragraphs(
            &[ParagraphAlignmentEdit {
                target: paragraph(&doc, 3).anchor.clone(),
                alignment: Alignment::Center,
            }],
            &[TableInsertion {
                target: paragraph(&doc, 3).anchor.clone(),
                table: insertion_table(),
            }],
            &[],
            &[],
            std::slice::from_ref(&insertion),
        )
        .unwrap();
    assert_eq!(preview.paragraph_insertions(), &[0]);
    let output = preview.to_docx().unwrap();
    let reopened = ImportedDocument::open(&output).unwrap();
    assert_eq!(paragraph(&reopened, 1).text, "new paragraph");
    assert_eq!(paragraph(&reopened, 4).alignment, Some(Alignment::Center));
    assert!(matches!(&reopened.body()[5], BodyBlock::Table(_)));
    for (name, original) in parts(&source) {
        if name != "word/document.xml" {
            assert_eq!(parts(&output)[&name], original, "{name}");
        }
    }
    assert!(
        doc.preview_with_paragraphs(&[], &[], &[], &[], &[insertion.clone(), insertion])
            .is_err()
    );
    let other = ImportedDocument::open(&source).unwrap();
    let stale = ParagraphInsertion {
        target: paragraph(&other, 0).anchor.clone(),
        paragraph: content.clone(),
    };
    assert!(
        doc.preview_with_paragraphs(&[], &[], &[], &[], &[stale])
            .is_err()
    );
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        parts(&source)
    );
}

#[test]
fn paragraph_insertion_rejects_active_ranges() {
    use rocx::{ResolvedBlock, editor::ParagraphInsertion};
    let xml = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:p><w:bookmarkStart w:id="1" w:name="open"/></w:p><w:p><w:bookmarkEnd w:id="1"/></w:p><w:sectPr/></w:body></w:document>"#
    );
    let doc = ImportedDocument::open(&fixture_with_body(&xml)).unwrap();
    let resolved = block_on(
        Document::new(vec![Section::new(vec![Paragraph::new("x").into()]).into()]).resolve(),
    )
    .unwrap();
    let ResolvedBlock::Paragraph(content) = &resolved.sections()[0].blocks()[0] else {
        panic!()
    };
    let insertion = ParagraphInsertion {
        target: paragraph(&doc, 0).anchor.clone(),
        paragraph: content.clone(),
    };
    assert!(
        doc.preview_with_paragraphs(&[], &[], &[], &[], &[insertion])
            .unwrap_err()
            .to_string()
            .contains("range")
    );
}

#[test]
fn inserts_formatted_content_nested_table_and_fraction() {
    use rocx::editor::TableInsertion;
    use rocx::{InlineFraction, RunFormat, Table, TableCell, TableProperties, TableRow, TextRun};
    let nested = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![Paragraph::new("nested").into()]).into(),
            TableCell::new(vec![Paragraph::new("neighbor").into()]).into(),
        ])
        .into(),
    ]);
    let content = Paragraph::from_children(vec![
        TextRun::new("bold")
            .with_format(RunFormat {
                bold: Some(true),
                ..Default::default()
            })
            .into(),
        InlineFraction::new("1", "2").into(),
    ]);
    let table = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![content.into(), nested.into()]).into(),
        ])
        .into(),
    ])
    .with_properties(TableProperties::default().with_column_widths(vec![1800]));
    let doc = ImportedDocument::open(&fixture()).unwrap();
    let preview = doc
        .preview_batch(
            &[],
            &[TableInsertion {
                target: paragraph(&doc, 3).anchor.clone(),
                table: resolve_insertion_table(table),
            }],
        )
        .unwrap();
    let output = parts(&preview.to_docx().unwrap());
    let parsed =
        roxmltree::Document::parse(std::str::from_utf8(&output["word/document.xml"]).unwrap())
            .unwrap();
    let body = parsed
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    let table = body
        .children()
        .rfind(|n| n.has_tag_name((W, "tbl")))
        .unwrap();
    assert_eq!(
        table
            .descendants()
            .filter(|n| n.has_tag_name((W, "tbl")))
            .count(),
        2
    );
    let grid_widths: Vec<_> = table
        .descendants()
        .filter(|n| n.has_tag_name((W, "gridCol")))
        .map(|n| n.attribute((W, "w")).unwrap())
        .collect();
    assert_eq!(grid_widths, ["1800", "900", "900"]);
    assert!(table.descendants().any(|n| n.has_tag_name((W, "b"))));
    assert!(table.descendants().any(|n| n.tag_name().namespace()
        == Some("http://schemas.openxmlformats.org/officeDocument/2006/math")
        && n.tag_name().name() == "f"));
    assert!(
        table
            .descendants()
            .any(|n| n.has_tag_name((W, "t")) && n.text() == Some("nested"))
    );
}

#[test]
fn inserted_external_hyperlink_uses_document_relationship() {
    use rocx::editor::TableInsertion;
    use rocx::{Hyperlink, HyperlinkTarget, Table, TableCell, TableRow, TextRun};
    const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
    let url = "https://example.invalid/inserted";
    let link = Hyperlink::new(
        HyperlinkTarget::external(url).unwrap(),
        vec![TextRun::new("linked").into()],
    );
    let table = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![Paragraph::from_children(vec![link.into()]).into()]).into(),
        ])
        .into(),
    ]);
    let input = fixture();
    let doc = ImportedDocument::open(&input).unwrap();
    let preview = doc
        .preview_batch(
            &[],
            &[TableInsertion {
                target: paragraph(&doc, 3).anchor.clone(),
                table: resolve_insertion_table(table),
            }],
        )
        .unwrap();
    let output = parts(&preview.to_docx().unwrap());
    let document =
        roxmltree::Document::parse(std::str::from_utf8(&output["word/document.xml"]).unwrap())
            .unwrap();
    let id = document
        .descendants()
        .find(|n| n.has_tag_name((W, "hyperlink")))
        .unwrap()
        .attribute((R, "id"))
        .unwrap();
    let relationships = roxmltree::Document::parse(
        std::str::from_utf8(&output["word/_rels/document.xml.rels"]).unwrap(),
    )
    .unwrap();
    let relationship = relationships
        .descendants()
        .find(|n| n.has_tag_name((REL, "Relationship")) && n.attribute("Id") == Some(id))
        .unwrap();
    assert_eq!(relationship.attribute("Target"), Some(url));
    assert_eq!(relationship.attribute("TargetMode"), Some("External"));
    assert!(
        relationship
            .attribute("Type")
            .unwrap()
            .ends_with("/hyperlink")
    );
    let original = parts(&input);
    for (name, payload) in &original {
        if name != "word/document.xml" && name != "word/_rels/document.xml.rels" {
            assert_eq!(output.get(name), Some(payload), "part {name}");
        }
    }
}

#[test]
fn inserts_exactly_after_anchor_in_mixed_batch_and_preserves_untouched_parts() {
    use rocx::editor::TableInsertion;
    let input = fixture();
    let doc = ImportedDocument::open(&input).unwrap();
    let preview = doc
        .preview_batch(
            &[ParagraphAlignmentEdit {
                target: paragraph(&doc, 3).anchor.clone(),
                alignment: Alignment::Right,
            }],
            &[TableInsertion {
                target: paragraph(&doc, 0).anchor.clone(),
                table: insertion_table(),
            }],
        )
        .unwrap();
    assert_eq!(preview.changes().len(), 1);
    assert_eq!(preview.insertions().len(), 1);
    assert_eq!(preview.insertions()[0].after_body_index, 0);
    assert_eq!(preview.insertions()[0].rows, 1);
    assert_eq!(preview.insertions()[0].columns, 2);
    assert_eq!(paragraph(&doc, 3).alignment, None);
    let original = parts(&input);
    let output = parts(&preview.to_docx().unwrap());
    for (name, payload) in &original {
        if name != "word/document.xml" {
            assert_eq!(output.get(name), Some(payload), "part {name}");
        }
    }
    let parsed =
        roxmltree::Document::parse(std::str::from_utf8(&output["word/document.xml"]).unwrap())
            .unwrap();
    let body = parsed
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    let children: Vec<_> = body.children().filter(|n| n.is_element()).collect();
    assert_eq!(
        children
            .iter()
            .map(|n| n.tag_name().name())
            .collect::<Vec<_>>(),
        ["p", "tbl", "tbl", "sdt", "p", "sectPr"]
    );
    assert!(
        children[1]
            .descendants()
            .any(|n| n.has_tag_name((W, "t")) && n.text() == Some("inserted A"))
    );
    assert!(
        children[2]
            .descendants()
            .any(|n| n.has_tag_name((W, "t")) && n.text() == Some("in table"))
    );
    assert!(
        children[4]
            .descendants()
            .any(|n| n.has_tag_name((W, "jc")) && n.attribute((W, "val")) == Some("right"))
    );
}

#[test]
fn rejects_stale_duplicate_and_crossing_range_insertions_atomically() {
    use rocx::editor::TableInsertion;
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:bookmarkStart w:id="1" w:name="cross"/><w:r><w:t>first</w:t></w:r></w:p><w:p><w:bookmarkEnd w:id="1"/><w:r><w:t>second</w:t></w:r></w:p><w:sectPr/></w:body></w:document>"#;
    let input = fixture_with_body(xml);
    let doc = ImportedDocument::open(&input).unwrap();
    let other = ImportedDocument::open(&input).unwrap();
    let insert = |target| TableInsertion {
        target,
        table: insertion_table(),
    };
    let edit = || ParagraphAlignmentEdit {
        target: paragraph(&doc, 1).anchor.clone(),
        alignment: Alignment::Center,
    };
    assert!(
        doc.preview_batch(&[edit()], &[insert(paragraph(&other, 1).anchor.clone())])
            .is_err()
    );
    assert!(
        doc.preview_batch(
            &[edit()],
            &[
                insert(paragraph(&doc, 1).anchor.clone()),
                insert(paragraph(&doc, 1).anchor.clone())
            ]
        )
        .is_err()
    );
    assert!(
        doc.preview_batch(&[edit()], &[insert(paragraph(&doc, 0).anchor.clone())])
            .is_err()
    );
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        parts(&input)
    );
}

#[test]
fn table_after_section_terminator_uses_next_section_width() {
    use rocx::editor::TableInsertion;
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:sectPr><w:pgSz w:w="10000" w:h="15840"/><w:pgMar w:left="1000" w:right="1000" w:top="1440" w:bottom="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:pPr><w:r><w:t>section one</w:t></w:r></w:p><w:p><w:r><w:t>section two</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="14000" w:h="15840"/><w:pgMar w:left="2000" w:right="2000" w:top="1440" w:bottom="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:body></w:document>"#;
    let doc = ImportedDocument::open(&fixture_with_body(xml)).unwrap();
    let preview = doc
        .preview_batch(
            &[],
            &[TableInsertion {
                target: paragraph(&doc, 0).anchor.clone(),
                table: insertion_table(),
            }],
        )
        .unwrap();
    let output = parts(&preview.to_docx().unwrap());
    let parsed =
        roxmltree::Document::parse(std::str::from_utf8(&output["word/document.xml"]).unwrap())
            .unwrap();
    let body = parsed
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    let children: Vec<_> = body.children().filter(|n| n.is_element()).collect();
    assert_eq!(children[1].tag_name().name(), "tbl");
    let widths: Vec<_> = children[1]
        .descendants()
        .filter(|n| n.has_tag_name((W, "gridCol")))
        .map(|n| n.attribute((W, "w")).unwrap())
        .collect();
    assert_eq!(widths, ["5000", "5000"]);
}

// These packages are synthetic; they do not claim Word-authored provenance.
fn header_fixture(content: &str, hyperlink: bool) -> Vec<u8> {
    let mut entries = parts(&fixture());
    let body = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body>
<w:p><w:pPr><w:sectPr><w:headerReference w:type="default" r:id="headerTest"/></w:sectPr></w:pPr></w:p>
<w:p><w:pPr><w:sectPr/></w:pPr></w:p>
<w:p/><w:sectPr/></w:body></w:document>"#;
    entries.insert("word/document.xml".into(), body.as_bytes().to_vec());
    let rel_name = "word/_rels/document.xml.rels";
    let rels = String::from_utf8(entries[rel_name].clone()).unwrap();
    let rels = rels.replace("</Relationships>", r#"<Relationship Id="headerTest" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="headerTest.xml"/></Relationships>"#);
    entries.insert(rel_name.into(), rels.into_bytes());
    let types_name = "[Content_Types].xml";
    let types = String::from_utf8(entries[types_name].clone()).unwrap();
    let types = types.replace("</Types>", r#"<Override PartName="/word/headerTest.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#);
    entries.insert(types_name.into(), types.into_bytes());
    let header = format!(
        r#"<w:hdr xmlns:w="{W}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">{content}</w:hdr>"#
    );
    entries.insert("word/headerTest.xml".into(), header.into_bytes());
    if hyperlink {
        entries.insert("word/_rels/headerTest.xml.rels".into(), br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="link1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.invalid/header" TargetMode="External"/></Relationships>"#.to_vec());
    }
    package_bytes(entries)
}

fn header_image() -> rocx::ResolvedImage {
    resolve_header_image(None)
}

fn resolve_header_image(anchor: Option<rocx::ImageAnchor>) -> rocx::ResolvedImage {
    use rocx::{Component, ImageRun};
    let mut image = ImageRun::new(include_bytes!("fixtures/images/sample.png").to_vec());
    image.transformation = anchor
        .map(|anchor| rocx::ImageTransformation::default().with_anchor(anchor))
        .into();
    block_on(Component::resolve(image))
        .unwrap()
        .unwrap()
        .image()
        .unwrap()
        .clone()
}

fn assert_header_image_relationship(output: &BTreeMap<String, Vec<u8>>, part: &str) {
    const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
    const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
    let part = part.trim_start_matches('/');
    let (dir, filename) = part.rsplit_once('/').unwrap();
    let rel_name = format!("{dir}/_rels/{filename}.rels");
    let header = roxmltree::Document::parse(std::str::from_utf8(&output[part]).unwrap()).unwrap();
    let id = header
        .descendants()
        .find(|n| n.has_tag_name((A, "blip")))
        .unwrap()
        .attribute((R, "embed"))
        .unwrap();
    let rels =
        roxmltree::Document::parse(std::str::from_utf8(&output[&rel_name]).unwrap()).unwrap();
    let rel = rels
        .descendants()
        .find(|n| n.has_tag_name((REL, "Relationship")) && n.attribute("Id") == Some(id))
        .unwrap();
    assert!(rel.attribute("Type").unwrap().ends_with("/image"));
    let target = rel.attribute("Target").unwrap();
    let image_part = rocx::opc::PartName::resolve(part, target).unwrap();
    assert_eq!(
        output[image_part.zip_name()],
        include_bytes!("fixtures/images/sample.png")
    );
}

#[test]
fn mixed_transaction_preserves_source_and_uses_original_slots() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit, TableInsertion};
    let input = include_bytes!("fixtures/imported-editor.docx");
    let doc = ImportedDocument::open(input).unwrap();
    let headers = doc.default_headers().unwrap();
    let original = parts(input);
    let align = ParagraphAlignmentEdit {
        target: paragraph(&doc, 0).anchor.clone(),
        alignment: Alignment::Center,
    };
    let table = TableInsertion {
        target: paragraph(&doc, 1).anchor.clone(),
        table: insertion_table(),
    };
    for mode in [HeaderEditMode::Shared, HeaderEditMode::SectionLocal] {
        let edit = HeaderImageEdit {
            target: headers[0].anchor.clone(),
            mode,
            image: header_image(),
        };
        let preview = doc
            .preview_transaction(
                std::slice::from_ref(&align),
                std::slice::from_ref(&table),
                std::slice::from_ref(&edit),
            )
            .unwrap();
        let local = mode == HeaderEditMode::SectionLocal;
        let [change] = preview.header_changes() else {
            panic!("expected one header change")
        };
        assert_eq!(
            change.affected_sections,
            if local { &[0][..] } else { &[0, 1] }
        );
        assert_eq!(change.restored_section, local.then_some(1));
        let bytes = preview.to_docx().unwrap();
        let reopened = ImportedDocument::open(&bytes).unwrap();
        assert_eq!(paragraph(&reopened, 0).alignment, Some(Alignment::Center));
        assert!(matches!(reopened.body()[2], BodyBlock::Table(_)));
        let after = reopened.default_headers().unwrap();
        assert_eq!(after[0].part_name, change.output_part);
        assert_eq!(after[1].part_name, change.source_part);
        let output = parts(&bytes);
        assert_header_image_relationship(&output, &change.output_part);
        let header = headers[0].part_name.trim_start_matches('/');
        let (dir, filename) = header.rsplit_once('/').unwrap();
        let rels = format!("{dir}/_rels/{filename}.rels");
        let variant_changes = if local {
            vec!["word/_rels/document.xml.rels"]
        } else {
            vec![header, rels.as_str()]
        };
        for (name, old) in &original {
            if !["word/document.xml", "[Content_Types].xml"].contains(&name.as_str())
                && !variant_changes.contains(&name.as_str())
            {
                assert_eq!(&output[name], old, "{name}");
            }
        }
        assert_eq!(
            doc.preview_transaction(&[], &[], std::slice::from_ref(&edit))
                .unwrap()
                .to_docx()
                .unwrap(),
            doc.preview_header_image(&edit).unwrap().to_docx().unwrap()
        );
    }
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        original
    );
}

#[test]
fn adjacent_local_forks_are_order_independent_and_failure_leaks_no_staged_work() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    let input = header_fixture("<w:p/>", false);
    let doc = ImportedDocument::open(&input).unwrap();
    let headers = doc.default_headers().unwrap();
    let edits = [0, 1].map(|section| HeaderImageEdit {
        target: headers[section].anchor.clone(),
        mode: HeaderEditMode::SectionLocal,
        image: header_image(),
    });
    let align = [ParagraphAlignmentEdit {
        target: paragraph(&doc, 0).anchor.clone(),
        alignment: Alignment::Right,
    }];
    let preview = doc.preview_transaction(&align, &[], &edits).unwrap();
    let [first, second] = preview.header_changes() else {
        panic!("expected two local forks")
    };
    assert_eq!(first.affected_sections, [0]);
    assert_eq!(second.affected_sections, [1]);
    assert_eq!(first.restored_section, None);
    assert_eq!(second.restored_section, Some(2));
    let bytes = preview.to_docx().unwrap();
    let reversed = doc
        .preview_transaction(&align, &[], &[edits[1].clone(), edits[0].clone()])
        .unwrap();
    assert_eq!(reversed.to_docx().unwrap(), bytes);
    let after = ImportedDocument::open(&bytes)
        .unwrap()
        .default_headers()
        .unwrap();
    assert_ne!(first.output_part, second.output_part);
    assert_eq!(after[0].part_name, first.output_part);
    assert_eq!(after[1].part_name, second.output_part);
    assert_eq!(after[2].part_name, first.source_part);
    let output = parts(&bytes);
    let drawing_id = |part: &str| {
        let xml = std::str::from_utf8(&output[part.trim_start_matches('/')]).unwrap();
        roxmltree::Document::parse(xml)
            .unwrap()
            .descendants()
            .find(|node| node.tag_name().name() == "docPr")
            .unwrap()
            .attribute("id")
            .unwrap()
            .parse::<u32>()
            .unwrap()
    };
    assert!(drawing_id(&first.output_part) < drawing_id(&second.output_part));

    let mut invalid = edits.clone();
    invalid[1].image = resolve_header_image(Some(rocx::ImageAnchor::default()));
    let error = doc.preview_transaction(&align, &[], &invalid).unwrap_err();
    assert!(
        error.to_string().contains("requires an inline image"),
        "{error}"
    );
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        parts(&input)
    );
    assert_eq!(
        doc.preview_transaction(&align, &[], &edits)
            .unwrap()
            .to_docx()
            .unwrap(),
        bytes
    );
}

#[test]
fn shared_header_conflicts_follow_physical_parts_not_relationship_ids() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    let mut entries = parts(&header_fixture("<w:p/>", false));
    let xml = entries.get_mut("word/document.xml").unwrap();
    *xml = std::str::from_utf8(xml)
        .unwrap()
        .replace(
            "<w:sectPr/></w:body>",
            r#"<w:sectPr><w:headerReference w:type="default" r:id="alias"/></w:sectPr></w:body>"#,
        )
        .into_bytes();
    let rels = entries.get_mut("word/_rels/document.xml.rels").unwrap();
    *rels = std::str::from_utf8(rels)
        .unwrap()
        .replace("</Relationships>", r#"<Relationship Id="alias" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="headerTest.xml"/></Relationships>"#)
        .into_bytes();
    let input = package_bytes(entries);
    let doc = ImportedDocument::open(&input).unwrap();
    let headers = doc.default_headers().unwrap();
    assert!(!headers[0].inherited && !headers[2].inherited);
    assert_eq!(headers[0].part_name, headers[2].part_name);
    let align = [ParagraphAlignmentEdit {
        target: paragraph(&doc, 0).anchor.clone(),
        alignment: Alignment::Right,
    }];
    let edit = HeaderImageEdit {
        target: headers[0].anchor.clone(),
        mode: HeaderEditMode::Shared,
        image: header_image(),
    };
    for mode in [HeaderEditMode::Shared, HeaderEditMode::SectionLocal] {
        let aliased = HeaderImageEdit {
            target: headers[2].anchor.clone(),
            mode,
            image: header_image(),
        };
        for edits in [[edit.clone(), aliased.clone()], [aliased, edit.clone()]] {
            let error = doc.preview_transaction(&align, &[], &edits).unwrap_err();
            assert!(error.to_string().contains("same physical part"), "{error}");
        }
    }
    let error = doc
        .preview_transaction(&align, &[], &[edit.clone(), edit])
        .unwrap_err();
    assert!(error.to_string().contains("target one section"), "{error}");
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        parts(&input)
    );
}

#[test]
fn shared_header_edit_reports_all_consumers_and_owns_image_relationship() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    let input = header_fixture("<w:p><w:r><w:t>original</w:t></w:r></w:p>", false);
    let doc = ImportedDocument::open(&input).unwrap();
    let headers = doc.default_headers().unwrap();
    assert_eq!(headers.len(), 3);
    assert_eq!(
        headers.iter().map(|h| h.section_index).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(
        headers.iter().map(|h| h.inherited).collect::<Vec<_>>(),
        [false, true, true]
    );
    assert!(headers.iter().all(|h| h.part_name == headers[0].part_name));
    assert!(matches!(&headers[1].blocks[..],
        [rocx::editor::ImportedHeaderBlock::Paragraph(items)]
        if matches!(&items[..], [rocx::editor::ImportedInline::Text(text)] if text == "original")));
    let edit = HeaderImageEdit {
        target: headers[1].anchor.clone(),
        mode: HeaderEditMode::Shared,
        image: header_image(),
    };
    let preview = doc.preview_header_image(&edit).unwrap();
    let change = preview.change();
    assert_eq!(change.source_part, headers[0].part_name);
    assert_eq!(change.output_part, change.source_part);
    assert_eq!(change.affected_sections, [0, 1, 2]);
    assert_eq!(change.restored_section, None);
    assert_eq!(change.mode, HeaderEditMode::Shared);
    let output = parts(&preview.to_docx().unwrap());
    assert_header_image_relationship(&output, &change.output_part);
    let original = parts(&input);
    for (name, bytes) in &original {
        if name != "word/headerTest.xml" && name != "[Content_Types].xml" {
            assert_eq!(output.get(name), Some(bytes), "{name}");
        }
    }
}

#[test]
fn header_story_preserves_order_and_opaque_children() {
    use rocx::editor::{ImportedHeaderBlock, ImportedInline, ImportedInlineNode};
    let input = header_fixture(
        concat!(
            "<w:p><w:r><w:t>A</w:t><w:tab/><w:t>B</w:t></w:r></w:p><w:tbl/>",
            r#"<w:p><w:hyperlink w:anchor="spot"><w:r><w:drawing/></w:r>"#,
            r#"<w:bookmarkStart w:id="7" w:name="spot"/>"#,
            r#"<w:ins w:id="9" w:author="Reviewer"><w:r><w:t>hidden</w:t></w:r></w:ins>"#,
            r#"<w:r/><w:bookmarkEnd w:id="7"/>"#,
            "</w:hyperlink></w:p><w:p/>"
        ),
        false,
    );
    let doc = ImportedDocument::open(&input).unwrap();
    let headers = doc.default_headers().unwrap();
    assert!(matches!(&headers[0].blocks[..], [
        ImportedHeaderBlock::Paragraph(first),
        ImportedHeaderBlock::Opaque(table),
        ImportedHeaderBlock::Paragraph(last),
        ImportedHeaderBlock::Paragraph(empty),
    ] if matches!(&first[..], [ImportedInline::Text(a), ImportedInline::Tab, ImportedInline::Text(b)] if a == "A" && b == "B")
        && table == "w:tbl"
        && last == &["w:drawing", "w:bookmarkStart", "w:ins", "w:bookmarkEnd"].map(|name| ImportedInline::Opaque(name.into()))
        && empty.is_empty()));
    assert!(headers[0].inline_structure(1).is_none());
    assert!(headers[0].inline_structure(3).unwrap().is_empty());
    assert!(headers[0].inline_structure(4).is_none());
    let [ImportedInlineNode::Hyperlink(link)] = headers[0].inline_structure(2).unwrap() else {
        panic!("expected hyperlink")
    };
    assert_eq!(link.bookmark.as_deref(), Some("spot"));
    assert!(link.relationship_id.is_none());
    assert!(matches!(link.children.as_slice(), [
        ImportedInlineNode::Run(drawing),
        ImportedInlineNode::Opaque(start),
        ImportedInlineNode::Revision(revision),
        ImportedInlineNode::Run(empty),
        ImportedInlineNode::Opaque(end),
    ] if drawing.items == [ImportedInline::Opaque("w:drawing".into())]
        && start == "w:bookmarkStart" && revision.kind == "insertion"
        && revision.id.as_deref() == Some("9")
        && revision.author.as_deref() == Some("Reviewer")
        && matches!(revision.children.as_slice(), [ImportedInlineNode::Run(run)]
            if run.items == [ImportedInline::Text("hidden".into())]
            && matches!(doc.inline_at(&run.anchor), Ok(ImportedInlineNode::Run(_))))
        && empty.items.is_empty() && end == "w:bookmarkEnd"
        && matches!(doc.inline_at(&empty.anchor).unwrap(), ImportedInlineNode::Run(found)
            if found.anchor == empty.anchor)));
}

#[test]
fn revision_inspection_keeps_boundaries_metadata_and_read_only_anchors() {
    use rocx::editor::{ImportedInline, ImportedInlineNode, RunTextEdit};
    let xml = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:p>
        <w:ins w:id="4" w:author="Ada" w:date="2024-01-01T00:00:00Z"><w:r><w:t>new</w:t></w:r><w:proofErr/></w:ins>
        <w:del w:id="5"><w:r><w:delText>old</w:delText><w:drawing/></w:r></w:del>
        <w:moveFrom w:id="6"><w:r><w:delText>move</w:delText></w:r></w:moveFrom>
        </w:p><w:sectPr/></w:body></w:document>"#
    );
    let bytes = fixture_with_body(&xml);
    let doc = ImportedDocument::open(&bytes).unwrap();
    let p = paragraph(&doc, 0);
    let [
        ImportedInlineNode::Revision(ins),
        ImportedInlineNode::Revision(del),
        ImportedInlineNode::Opaque(moved),
    ] = p.inline_structure.as_slice()
    else {
        panic!("expected two revisions and an opaque move");
    };
    assert_eq!(
        (
            ins.kind,
            ins.id.as_deref(),
            ins.author.as_deref(),
            ins.date.as_deref()
        ),
        (
            "insertion",
            Some("4"),
            Some("Ada"),
            Some("2024-01-01T00:00:00Z")
        )
    );
    assert_eq!(
        (del.kind, del.id.as_deref(), del.author.as_deref()),
        ("deletion", Some("5"), None)
    );
    assert_eq!(moved, "w:moveFrom");
    let [
        ImportedInlineNode::Run(inserted),
        ImportedInlineNode::Opaque(unknown),
    ] = ins.children.as_slice()
    else {
        panic!("insertion children");
    };
    assert_eq!(unknown, "w:proofErr");
    assert_eq!(inserted.items, [ImportedInline::Text("new".into())]);
    let [ImportedInlineNode::Run(deleted)] = del.children.as_slice() else {
        panic!("deletion children");
    };
    assert_eq!(
        deleted.items,
        [
            ImportedInline::Text("old".into()),
            ImportedInline::Opaque("w:drawing".into())
        ]
    );
    assert!(matches!(
        doc.inline_at(&del.anchor),
        Ok(ImportedInlineNode::Revision(_))
    ));
    assert!(matches!(
        doc.inline_at(&deleted.anchor),
        Ok(ImportedInlineNode::Run(_))
    ));
    assert!(
        ImportedDocument::open(&bytes)
            .unwrap()
            .inline_at(&deleted.anchor)
            .is_err()
    );
    assert!(
        doc.preview_with_text(
            &[],
            &[],
            &[],
            &[RunTextEdit {
                target: inserted.anchor.clone(),
                text: "unsafe".into()
            }]
        )
        .is_err()
    );
}

#[test]
fn story_inspection_tracks_slots_but_anchors_follow_physical_parts() {
    use rocx::HeaderFooterType::{Default, Even, First};
    use rocx::editor::{
        ImportedInline::Text,
        ImportedInlineNode::{Hyperlink, Run},
        ImportedStoryBlock::Opaque,
        SectionStoryKind::{Footer, Header},
    };
    let content = r#"<w:p><w:hyperlink r:id="link1"><w:r><w:t>same</w:t></w:r></w:hyperlink></w:p><w:tbl/><w:p/>"#;
    let mut entries = parts(&header_fixture(content, true));
    let document = String::from_utf8(entries["word/document.xml"].clone())
        .unwrap()
        .replace(
            "r:id=\"headerTest\"/>",
            concat!(
                "r:id=\"headerTest\"/>",
                "<w:headerReference w:type=\"first\" r:id=\"samePart\"/>",
                "<w:footerReference w:type=\"even\" r:id=\"footerTest\"/>"
            ),
        )
        .replace(
            "<w:p/><w:sectPr/>",
            "<w:p/><w:sectPr><w:footerReference w:type=\"even\" r:id=\"footerTest\"/></w:sectPr>",
        );
    entries.insert("word/document.xml".into(), document.as_bytes().to_vec());
    let rels_name = "word/_rels/document.xml.rels";
    let rels = String::from_utf8(entries[rels_name].clone()).unwrap().replace(
        "</Relationships>",
        concat!(
            r#"<Relationship Id="samePart" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="./headerTest.xml"/>"#,
            r#"<Relationship Id="footerTest" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footerTest.xml"/>"#,
            "</Relationships>"
        ),
    );
    entries.insert(rels_name.into(), rels.into_bytes());
    let types_name = "[Content_Types].xml";
    let types = String::from_utf8(entries[types_name].clone()).unwrap().replace(
        "</Types>",
        r#"<Override PartName="/word/footerTest.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/></Types>"#,
    );
    entries.insert(types_name.into(), types.into_bytes());
    let footer = String::from_utf8(entries["word/headerTest.xml"].clone())
        .unwrap()
        .replace("w:hdr", "w:ftr");
    let footer_rels = entries["word/_rels/headerTest.xml.rels"].clone();
    // Identical local relationship ID, but owned by another physical part.
    entries.insert("word/_rels/footerTest.xml.rels".into(), footer_rels);
    entries.insert("word/footerTest.xml".into(), footer.as_bytes().to_vec());
    let input = package_bytes(entries.clone());
    let doc = ImportedDocument::open(&input).unwrap();
    let stories = doc.stories().unwrap();
    assert_eq!(
        stories
            .iter()
            .map(|s| (s.section_index, s.kind, s.variant, s.inherited))
            .collect::<Vec<_>>(),
        [
            (0, Header, Default, false),
            (0, Header, First, false),
            (0, Footer, Even, false),
            (1, Header, Default, true),
            (1, Header, First, true),
            (1, Footer, Even, true),
            (2, Header, Default, true),
            (2, Header, First, true),
            (2, Footer, Even, false),
        ]
    );
    let mut anchors = Vec::new();
    for story in &stories {
        assert!(matches!(&story.blocks[1], Opaque(name) if name == "w:tbl"));
        assert!(story.inline_structure(1).is_none());
        assert!(story.inline_structure(2).unwrap().is_empty());
        assert!(story.inline_structure(3).is_none());
        let [Hyperlink(link)] = story.inline_structure(0).unwrap() else {
            panic!("expected hyperlink");
        };
        let [Run(run)] = link.children.as_slice() else {
            panic!("expected hyperlink run");
        };
        assert_eq!(link.relationship_id.as_deref(), Some("link1"));
        assert!(matches!(doc.inline_at(&run.anchor).unwrap(), Run(found)
            if found.items == [Text("same".into())]));
        anchors.push(&link.anchor);
    }
    for (story, anchor) in stories.iter().zip(&anchors) {
        let first_alias = if story.kind == Header { 0 } else { 2 };
        assert_eq!(story.part_name, stories[first_alias].part_name);
        assert_eq!(*anchor, anchors[first_alias]);
    }
    assert_ne!(stories[0].part_name, stories[2].part_name);
    assert_ne!(anchors[0], anchors[2]);
    let headers = doc.default_headers().unwrap();
    assert!(
        matches!(headers[0].inline_structure(0).unwrap(), [Hyperlink(link)]
        if &link.anchor == anchors[0])
    );
    assert!(
        ImportedDocument::open(&input)
            .unwrap()
            .inline_at(anchors[2])
            .is_err()
    );
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        entries
    );

    // Broader validation must not change the legacy API's inspection scope.
    for (bad_footer, message) in [
        (footer.replace("w:ftr", "w:hdr"), "invalid footer root"),
        (footer, "duplicate footer reference type"),
    ] {
        let mut malformed = entries.clone();
        malformed.insert("word/footerTest.xml".into(), bad_footer.into_bytes());
        if message.starts_with("duplicate") {
            let reference = r#"<w:footerReference w:type="even" r:id="footerTest"/>"#;
            malformed.insert(
                "word/document.xml".into(),
                document
                    .replace(reference, &reference.repeat(2))
                    .into_bytes(),
            );
        }
        let doc = ImportedDocument::open(&package_bytes(malformed)).unwrap();
        assert_eq!(doc.default_headers().unwrap().len(), 3);
        let error = doc.stories().unwrap_err().to_string();
        assert!(error.contains(message), "{error}");
        assert_eq!(doc.default_headers().unwrap().len(), 3);
    }
}

#[test]
fn independent_default_header_inline_anchors_identify_the_physical_part() {
    use rocx::editor::{
        HeaderEditMode, HeaderImageEdit,
        ImportedInline::Text,
        ImportedInlineNode::{Hyperlink, Run},
    };
    let source = include_bytes!("fixtures/imported-editor.docx");
    let doc = ImportedDocument::open(source).unwrap();
    let headers = doc.default_headers().unwrap();
    let [first, second] = headers.as_slice() else {
        panic!("expected two default header aliases")
    };
    assert_eq!(first.part_name, second.part_name);
    assert!(!first.inherited && second.inherited);
    let [Run(label), Hyperlink(link)] = first.inline_structure(0).unwrap() else {
        panic!("expected direct run and hyperlink")
    };
    let [Run(alias), Hyperlink(alias_link)] = second.inline_structure(0).unwrap() else {
        panic!("expected inherited inline nodes")
    };
    assert_eq!(label.items, [Text("Shared header ".into())]);
    assert_eq!(label.anchor, alias.anchor);
    assert_eq!(link.anchor, alias_link.anchor);
    let [Run(li), Run(nk)] = link.children.as_slice() else {
        panic!("expected two hyperlink runs")
    };
    let [Run(alias_li), Run(_)] = alias_link.children.as_slice() else {
        panic!("expected inherited hyperlink runs")
    };
    assert_eq!(li.anchor, alias_li.anchor);
    assert_ne!(li.anchor, nk.anchor);
    assert_ne!(li.anchor, link.anchor);
    assert!(
        matches!(doc.inline_at(&li.anchor).unwrap(), Run(run) if run.items == [Text("li".into())])
    );
    assert!(
        matches!(doc.inline_at(&link.anchor).unwrap(), Hyperlink(found)
        if found.relationship_id == link.relationship_id)
    );
    let BodyBlock::Paragraph(body) = &doc.body()[0] else {
        panic!("expected body paragraph")
    };
    let Run(body_label) = &body.inline_structure[0] else {
        panic!("expected direct body run")
    };
    assert_ne!(label.anchor, body_label.anchor);
    let reopened = ImportedDocument::open(source).unwrap();
    assert!(
        reopened
            .inline_at(&li.anchor)
            .unwrap_err()
            .to_string()
            .contains("different document snapshot")
    );

    // An empty batch after inspection does not alter any payload, including the
    // header and its owner-local relationships.
    assert_eq!(
        parts(&doc.preview(&[]).unwrap().to_docx().unwrap()),
        parts(source)
    );
    let output = doc
        .preview_header_image(&HeaderImageEdit {
            target: second.anchor.clone(),
            mode: HeaderEditMode::SectionLocal,
            image: header_image(),
        })
        .unwrap()
        .to_docx()
        .unwrap();
    let local = ImportedDocument::open(&output).unwrap();
    let headers = local.default_headers().unwrap();
    let [_, Hyperlink(original)] = headers[0].inline_structure(0).unwrap() else {
        panic!("expected original hyperlink")
    };
    let [_, Hyperlink(copy)] = headers[1].inline_structure(0).unwrap() else {
        panic!("expected copied hyperlink")
    };
    assert_ne!(headers[0].part_name, headers[1].part_name);
    assert_ne!(original.anchor, copy.anchor);
    assert_eq!(original.relationship_id, copy.relationship_id);
}

#[test]
fn invalid_default_header_does_not_block_body_inline_lookup() {
    use rocx::editor::ImportedInlineNode::Run;
    let mut entries = parts(include_bytes!("fixtures/imported-editor.docx"));
    *entries.get_mut("word/header1.xml").unwrap() =
        format!(r#"<w:ftr xmlns:w="{W}"><w:p/></w:ftr>"#).into_bytes();
    let doc = ImportedDocument::open(&package_bytes(entries)).unwrap();
    let BodyBlock::Paragraph(paragraph) = &doc.body()[0] else {
        panic!("expected body paragraph")
    };
    let Run(run) = &paragraph.inline_structure[0] else {
        panic!("expected body run")
    };
    assert!(matches!(doc.inline_at(&run.anchor).unwrap(), Run(_)));
    assert!(
        doc.default_headers()
            .unwrap_err()
            .to_string()
            .contains("invalid header root")
    );
    assert!(matches!(doc.inline_at(&run.anchor).unwrap(), Run(_)));
}

#[test]
fn local_header_edit_restores_later_inheritors_and_preserves_link() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    let input = header_fixture(
        r#"<w:p><w:hyperlink r:id="link1"><w:r><w:t>linked</w:t></w:r></w:hyperlink></w:p>"#,
        true,
    );
    for section in [0, 1] {
        let doc = ImportedDocument::open(&input).unwrap();
        let headers = doc.default_headers().unwrap();
        let edit = HeaderImageEdit {
            target: headers[section].anchor.clone(),
            mode: HeaderEditMode::SectionLocal,
            image: header_image(),
        };
        let preview = doc.preview_header_image(&edit).unwrap();
        let change = preview.change();
        assert_eq!(change.source_part, headers[section].part_name);
        assert_ne!(change.output_part, change.source_part);
        assert_eq!(change.affected_sections, [section]);
        assert_eq!(change.restored_section, Some(section + 1));
        assert_eq!(change.mode, HeaderEditMode::SectionLocal);
        let bytes = preview.to_docx().unwrap();
        let output = parts(&bytes);
        assert_header_image_relationship(&output, &change.output_part);
        assert_eq!(
            output["word/headerTest.xml"],
            parts(&input)["word/headerTest.xml"]
        );
        assert_eq!(
            output["word/_rels/headerTest.xml.rels"],
            parts(&input)["word/_rels/headerTest.xml.rels"]
        );
        let reopened = ImportedDocument::open(&bytes).unwrap();
        let after = reopened.default_headers().unwrap();
        for (index, header) in after.iter().enumerate() {
            assert_eq!(
                header.part_name,
                if index == section {
                    &change.output_part
                } else {
                    &change.source_part
                }
                .as_str(),
                "section {index}"
            );
        }
        let part = change.output_part.trim_start_matches('/');
        let (dir, filename) = part.rsplit_once('/').unwrap();
        let rels = format!("{dir}/_rels/{filename}.rels");
        let header = std::str::from_utf8(&output[part]).unwrap();
        let header = roxmltree::Document::parse(header).unwrap();
        let id = header
            .descendants()
            .find(|n| n.has_tag_name((W, "hyperlink")))
            .unwrap()
            .attribute((
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
                "id",
            ))
            .unwrap();
        let rels =
            roxmltree::Document::parse(std::str::from_utf8(&output[&rels]).unwrap()).unwrap();
        let link = rels
            .descendants()
            .find(|n| n.attribute("Id") == Some(id))
            .unwrap();
        assert_eq!(
            link.attribute("Target"),
            Some("https://example.invalid/header")
        );
        assert_eq!(link.attribute("TargetMode"), Some("External"));
    }
}

#[test]
fn local_header_rejects_identity_bearing_and_unknown_content_and_stale_anchors() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    for content in [
        r#"<w:p><w:r><w:drawing/></w:r></w:p>"#,
        r#"<w:p><w:bookmarkStart w:id="1" w:name="identity"/></w:p>"#,
        r#"<w:p><w:customXml w:element="unknown"/></w:p>"#,
    ] {
        let input = header_fixture(content, false);
        let doc = ImportedDocument::open(&input).unwrap();
        let headers = doc.default_headers().unwrap();
        let edit = HeaderImageEdit {
            target: headers[1].anchor.clone(),
            mode: HeaderEditMode::SectionLocal,
            image: header_image(),
        };
        assert!(doc.preview_header_image(&edit).is_err(), "{content}");
    }
    let input = header_fixture("<w:p/>", false);
    let doc = ImportedDocument::open(&input).unwrap();
    let other = ImportedDocument::open(&input).unwrap();
    let edit = HeaderImageEdit {
        target: other.default_headers().unwrap()[0].anchor.clone(),
        mode: HeaderEditMode::Shared,
        image: header_image(),
    };
    assert!(doc.preview_header_image(&edit).is_err());
}

#[test]
fn local_header_forks_beside_relocated_source_with_prefixed_relationships() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    let original = header_fixture(
        r#"<w:p><w:hyperlink r:id="link1"><w:r><w:t>linked</w:t></w:r></w:hyperlink></w:p>"#,
        true,
    );
    let mut entries = parts(&original);
    let header = entries.remove("word/headerTest.xml").unwrap();
    entries.insert("content/headers/source.xml".into(), header);
    let links = entries.remove("word/_rels/headerTest.xml.rels").unwrap();
    entries.insert("content/headers/_rels/source.xml.rels".into(), links);
    let replace = |xml: &mut Vec<u8>, substitutions: &[(&str, &str)]| {
        let mut text = String::from_utf8(std::mem::take(xml)).unwrap();
        for (from, to) in substitutions {
            text = text.replace(from, to);
        }
        *xml = text.into_bytes();
    };
    for rels in [
        "word/_rels/document.xml.rels",
        "content/headers/_rels/source.xml.rels",
    ] {
        replace(
            entries.get_mut(rels).unwrap(),
            &[
                ("<Relationships xmlns=", "<p:Relationships xmlns:p="),
                ("</Relationships>", "</p:Relationships>"),
                ("<Relationship ", "<p:Relationship "),
                (
                    r#"Target="headerTest.xml""#,
                    r#"Target="../content/headers/source.xml""#,
                ),
            ],
        );
    }
    replace(
        entries.get_mut("[Content_Types].xml").unwrap(),
        &[
            ("<Types xmlns=", "<p:Types xmlns:p="),
            ("</Types>", "</p:Types>"),
            ("<Default ", "<p:Default "),
            ("<Override ", "<p:Override "),
            ("/word/headerTest.xml", "/content/headers/source.xml"),
        ],
    );
    let input = package_bytes(entries);
    let doc = ImportedDocument::open(&input).unwrap();
    let preview = doc
        .preview_header_image(&HeaderImageEdit {
            target: doc.default_headers().unwrap()[1].anchor.clone(),
            mode: HeaderEditMode::SectionLocal,
            image: header_image(),
        })
        .unwrap();
    assert_eq!(preview.change().source_part, "content/headers/source.xml");
    assert!(preview.change().output_part.starts_with("content/headers/"));
    let output = parts(&preview.to_docx().unwrap());
    assert_header_image_relationship(&output, &preview.change().output_part);
    assert_eq!(
        output["content/headers/_rels/source.xml.rels"],
        parts(&input)["content/headers/_rels/source.xml.rels"]
    );
}

#[test]
fn shared_edit_reserves_drawing_ids_from_bomless_utf16_parts() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    let input = header_fixture(
        r#"<w:p xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"><w:r><w:drawing><wp:inline><wp:docPr id="91" name="old"/></wp:inline></w:drawing></w:r></w:p>"#,
        false,
    );
    let mut entries = parts(&input);
    let header = entries.get_mut("word/headerTest.xml").unwrap();
    *header = std::str::from_utf8(header)
        .unwrap()
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let doc = ImportedDocument::open(&package_bytes(entries)).unwrap();
    let preview = doc
        .preview_header_image(&HeaderImageEdit {
            target: doc.default_headers().unwrap()[0].anchor.clone(),
            mode: HeaderEditMode::Shared,
            image: header_image(),
        })
        .unwrap();
    let output = parts(&preview.to_docx().unwrap());
    let header =
        roxmltree::Document::parse(std::str::from_utf8(&output["word/headerTest.xml"]).unwrap())
            .unwrap();
    let ids: Vec<_> = header
        .descendants()
        .filter(|n| n.tag_name().name() == "docPr")
        .map(|n| n.attribute("id").unwrap())
        .collect();
    assert_eq!(ids, ["91", "92"]);
}

#[test]
fn shared_edit_counts_aliases_on_first_page_headers() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    let mut entries = parts(&header_fixture("<w:p/>", false));
    let mut rewrite = |path: &str, old: &str, new: &str| {
        let xml = entries.get_mut(path).unwrap();
        let source = std::str::from_utf8(xml).unwrap();
        assert!(source.contains(old));
        *xml = source.replace(old, new).into_bytes();
    };
    rewrite(
        "word/document.xml",
        "<w:sectPr/></w:pPr>",
        r#"<w:sectPr><w:headerReference w:type="default" r:id="otherHeader"/></w:sectPr></w:pPr>"#,
    );
    rewrite(
        "word/document.xml",
        "<w:sectPr/></w:body>",
        r#"<w:sectPr><w:headerReference w:type="first" r:id="aliasForHeaderTest"/><w:titlePg/></w:sectPr></w:body>"#,
    );
    rewrite(
        "word/_rels/document.xml.rels",
        "</Relationships>",
        r#"<Relationship Id="otherHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="other.xml"/><Relationship Id="aliasForHeaderTest" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="headerTest.xml"/></Relationships>"#,
    );
    rewrite(
        "[Content_Types].xml",
        "</Types>",
        r#"<Override PartName="/word/other.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#,
    );
    entries.insert(
        "word/other.xml".into(),
        entries["word/headerTest.xml"].clone(),
    );
    let doc = ImportedDocument::open(&package_bytes(entries)).unwrap();
    let headers = doc.default_headers().unwrap();
    assert_eq!(headers[1].part_name, headers[2].part_name);
    assert_ne!(headers[0].part_name, headers[1].part_name);
    let preview = doc
        .preview_transaction(
            &[],
            &[],
            &[
                HeaderImageEdit {
                    target: headers[2].anchor.clone(),
                    mode: HeaderEditMode::SectionLocal,
                    image: header_image(),
                },
                HeaderImageEdit {
                    target: headers[0].anchor.clone(),
                    mode: HeaderEditMode::Shared,
                    image: header_image(),
                },
            ],
        )
        .unwrap();
    let [shared, local] = preview.header_changes() else {
        panic!("expected shared and local edits")
    };
    assert_eq!(shared.affected_sections, [0, 2]);
    assert_eq!(local.affected_sections, [2]);
    assert_ne!(shared.source_part, local.source_part);
    let output = parts(&preview.to_docx().unwrap());
    assert_header_image_relationship(&output, &shared.output_part);
    assert_header_image_relationship(&output, &local.output_part);
}

#[test]
fn first_header_image_local_preserves_aliases_and_inheritance() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit, SectionStoryKind};
    let mut entries = parts(&header_fixture("<w:p/>", false));
    let xml = String::from_utf8(entries["word/document.xml"].clone()).unwrap();
    entries.insert(
        "word/document.xml".into(),
        xml.replace(
            r#"<w:headerReference w:type="default" r:id="headerTest"/>"#,
            r#"<w:headerReference w:type="default" r:id="headerTest"/><w:headerReference w:type="first" r:id="headerTest"/>"#,
        )
        .into_bytes(),
    );
    let input = package_bytes(entries);
    let doc = ImportedDocument::open(&input).unwrap();
    let stories = doc.stories().unwrap();
    let first = stories
        .iter()
        .find(|s| {
            s.section_index == 1
                && s.kind == SectionStoryKind::Header
                && s.variant == rocx::HeaderFooterType::First
        })
        .unwrap();
    assert!(first.inherited);
    let default = &doc.default_headers().unwrap()[1].anchor;
    let first_local = HeaderImageEdit {
        target: first.anchor.clone().unwrap(),
        mode: HeaderEditMode::SectionLocal,
        image: header_image(),
    };
    let default_local = HeaderImageEdit {
        target: default.clone(),
        mode: HeaderEditMode::SectionLocal,
        image: header_image(),
    };
    let both = doc
        .preview_transaction(&[], &[], &[first_local.clone(), default_local.clone()])
        .unwrap();
    assert_eq!(both.header_changes().len(), 2);
    assert_ne!(
        both.header_changes()[0].output_part,
        both.header_changes()[1].output_part
    );
    let mut shared = first_local.clone();
    shared.mode = HeaderEditMode::Shared;
    assert!(
        doc.preview_transaction(&[], &[], &[shared, default_local])
            .is_err()
    );
    for mode in [HeaderEditMode::Shared, HeaderEditMode::SectionLocal] {
        let preview = doc
            .preview_header_image(&HeaderImageEdit {
                target: first.anchor.clone().unwrap(),
                mode,
                image: header_image(),
            })
            .unwrap();
        let bytes = preview.to_docx().unwrap();
        let output = parts(&bytes);
        assert_header_image_relationship(&output, &preview.change().output_part);
        if mode == HeaderEditMode::Shared {
            assert_eq!(preview.change().affected_sections, [0, 1, 2]);
            continue;
        }
        assert_eq!(preview.change().affected_sections, [1]);
        assert_eq!(preview.change().restored_section, Some(2));
        assert_eq!(
            parts(&input)["word/headerTest.xml"],
            output["word/headerTest.xml"]
        );
        let reopened = ImportedDocument::open(&bytes).unwrap();
        for story in reopened
            .stories()
            .unwrap()
            .iter()
            .filter(|s| s.kind == SectionStoryKind::Header)
        {
            let expected =
                if story.section_index == 1 && story.variant == rocx::HeaderFooterType::First {
                    &preview.change().output_part
                } else {
                    &preview.change().source_part
                };
            assert_eq!(&story.part_name, expected);
        }
    }
}

#[test]
fn table_structure_rejects_invalid_chains_ranges_and_foreign_anchors_atomically() {
    use rocx::editor::{TableOperation::InsertRow, TableStructureEdit};
    let base = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:p/><w:tbl><w:tblGrid><w:gridCol/></w:tblGrid><w:tr><w:tc><w:p/></w:tc></w:tr></w:tbl><w:sectPr/></w:body></w:document>"#
    );
    let original = ImportedDocument::open(&fixture_with_body(&base)).unwrap();
    let edit = |doc: &ImportedDocument| {
        let BodyBlock::Table(t) = &doc.body()[1] else {
            panic!("expected table")
        };
        TableStructureEdit {
            target: t.anchor.clone(),
            operation: InsertRow(0),
        }
    };
    let apply = |doc: &ImportedDocument, structural: &[TableStructureEdit]| {
        doc.preview_with_table_structure(
            &[ParagraphAlignmentEdit {
                target: paragraph(doc, 0).anchor.clone(),
                alignment: Alignment::Center,
            }],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            structural,
        )
    };
    let good = apply(&original, &[edit(&original)])
        .unwrap()
        .to_docx()
        .unwrap();
    for xml in [
        base.replace("<w:tc>",r#"<w:tc><w:tcPr><w:tcW w:w="500" w:type="dxa"/><w:tcW w:w="1000" w:type="dxa"/></w:tcPr>"#),
        base.replace("<w:tc>", "<w:tc><w:tcPr><w:vMerge/></w:tcPr>"),
        base.replace("<w:tc>", "<w:tc><w:tcPr><w:gridSpan w:val=\"2\"/></w:tcPr>"),
        base.replace(
            "<w:p/><w:tbl>",
            "<w:p><w:bookmarkStart w:id=\"9\" w:name=\"keep\"/></w:p><w:tbl>",
        ),
        base.replace(
            "<w:tr>",
            "<w:tr><w:trPr><w:ins w:id=\"1\" w:author=\"Ada\"/></w:trPr>",
        ),
    ] {
        let bytes = fixture_with_body(&xml);
        let source = ImportedDocument::open(&bytes).unwrap();
        assert!(apply(&source, &[edit(&source)]).is_err());
        assert!(apply(&source, &[edit(&original)]).is_err());
        assert_eq!(
            parts(&source.preview(&[]).unwrap().to_docx().unwrap()),
            parts(&bytes)
        );
    }
    assert_eq!(
        apply(&original, &[edit(&original)])
            .unwrap()
            .to_docx()
            .unwrap(),
        good
    );
}

#[test]
fn physical_header_text_batch_rejects_alias_duplicates_and_protected_ranges() {
    use rocx::editor::{ImportedInlineNode::Run, RunTextEdit};
    let bytes = header_fixture(
        r#"<w:p><w:r><w:t>safe</w:t></w:r><w:bookmarkStart w:id="7" w:name="spot"/><w:r><w:t>protected</w:t></w:r><w:bookmarkEnd w:id="7"/></w:p><w:customXml/>"#,
        false,
    );
    let doc = ImportedDocument::open(&bytes).unwrap();
    let headers = doc.default_headers().unwrap();
    let owner = &headers[0].part_name;
    let edit = |section: usize, index: usize| {
        let Run(run) = &headers[section].inline_structure(0).unwrap()[index] else {
            panic!()
        };
        RunTextEdit {
            target: run.anchor.clone(),
            text: "changed".into(),
        }
    };
    for invalid in [edit(1, 0), edit(1, 2)] {
        assert!(doc.edit_story_text(owner, &[edit(0, 0), invalid]).is_err());
    }
    let mut actual = parts(&doc.edit_story_text(owner, &[edit(1, 0)]).unwrap());
    let mut original = parts(&bytes);
    let changed = String::from_utf8(actual.remove(owner).unwrap()).unwrap();
    assert!(
        changed.contains(">changed</w:t>")
            && changed.contains("protected")
            && changed.contains("w:customXml")
    );
    original.remove(owner);
    assert_eq!(actual, original);
}

#[test]
fn local_story_fork_rejects_unknown_implicit_relationships() {
    use rocx::editor::{HeaderEditMode, HeaderImageEdit};
    let mut input = parts(&header_fixture("<w:p/>", true));
    let rels = "word/_rels/headerTest.xml.rels";
    let xml = String::from_utf8(input[rels].clone())
        .unwrap()
        .replace("relationships/hyperlink", "relationships/unknown");
    input.insert(rels.into(), xml.into_bytes());
    let doc = ImportedDocument::open(&package_bytes(input)).unwrap();
    let target = doc.default_headers().unwrap().remove(1).anchor;
    let mut edit = HeaderImageEdit {
        target,
        mode: HeaderEditMode::SectionLocal,
        image: header_image(),
    };
    assert!(
        doc.preview_header_image(&edit)
            .unwrap_err()
            .to_string()
            .contains("relationships cannot be safely copied")
    );
    edit.mode = HeaderEditMode::Shared;
    assert!(doc.preview_header_image(&edit).is_ok());
}
