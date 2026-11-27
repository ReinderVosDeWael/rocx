use rocx::editor::{
    BlockEdit, BodyBlock, EditBatch, ImportedDocument, ImportedInlineNode, ImportedParagraph,
    InlineAction, InlineEdit, LayoutEdit, LayoutField, LayoutValue, NumberingEdit, ResourceEdit,
    TextInsertion,
};
use rocx::{BreakType, NumberingReference, ResolvedInline};
use rocx_layout::{LayoutSession, PageSpec, plain_section};

fn fixture() -> ImportedDocument {
    ImportedDocument::open(include_bytes!("fixtures/physical-plain.docx")).unwrap()
}

fn font() -> Vec<u8> {
    include_bytes!("fonts/DejaVuSans-subset.ttf").to_vec()
}

fn paragraph(doc: &ImportedDocument, index: usize) -> &ImportedParagraph {
    let BodyBlock::Paragraph(p) = &doc.body()[index] else {
        panic!("expected paragraph at body index {index}");
    };
    p
}

#[test]
fn typed_projection_drives_two_pages_and_scalar_edit_reflow() {
    let doc = fixture();
    let (text, page) = plain_section(&doc).unwrap();
    assert_eq!(text, ["hello", "", "one\ntwo", "a😀a"]);
    assert_eq!(
        page,
        PageSpec {
            width: 3203,
            height: 4608,
            left: 643,
            right: 637,
            top: 640,
            bottom: 640,
            space_before: 0,
            space_after: 0,
        }
    );
    let mut session = LayoutSession::new(font(), 12 * 64).unwrap();
    let before = session.layout_pages(&text, page).unwrap();
    assert_eq!(before.rows.len(), 2);
    let placements: Vec<_> = before
        .rows
        .iter()
        .map(|rows| {
            rows.iter()
                .map(|r| (r.paragraph, r.line))
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(
        placements,
        [vec![(0, 0), (1, 0), (2, 0)], vec![(2, 1), (3, 0)]]
    );
    assert_eq!(before.rows[0][0].baseline, before.rows[1][0].baseline);
    assert_eq!(before.rows[1][0].top, page.top);
    assert_eq!(before.rows[1][0].x, page.left);
    assert_eq!(before.hit_test(1, page.left, page.top), Some((2, 4)));
    let empty = &before.rows[0][1];
    assert_eq!(before.hit_test(0, empty.x, empty.baseline), Some((1, 0)));

    let row = &before.rows[1][1];
    let x = row.x + before.layout.paragraphs[3][0].width;
    let hit = before.hit_test(1, x, row.baseline);
    assert_eq!(hit, Some((3, 3))); // a😀a: three scalars, six UTF-8 bytes
    assert_eq!(before.hit_test(1, x + 1, row.baseline), None);
    let (index, offset) = hit.unwrap();
    let insert = [TextInsertion {
        target: paragraph(&doc, index).anchor.clone(),
        position: offset as usize,
        text: " hello hello".into(),
    }];
    let batch = || EditBatch {
        text_insertions: &insert,
        ..Default::default()
    };
    let bytes = doc.preview_edits(batch()).unwrap().to_docx().unwrap();
    let reopened = ImportedDocument::open(&bytes).unwrap();
    assert!(reopened.preview_edits(batch()).is_err());
    let (edited, next_page) = plain_section(&reopened).unwrap();
    assert_eq!(edited, ["hello", "", "one\ntwo", "a😀a hello hello"]);
    assert_eq!(next_page, page);
    let warm = session.layout_pages(&edited, next_page).unwrap();
    let cold = LayoutSession::with_cache_budget(font(), 12 * 64, 0)
        .unwrap()
        .layout_pages(&edited, next_page)
        .unwrap();
    assert!(warm.rows.len() > before.rows.len());
    assert!(warm.layout.reuse.page_hits >= 1);
    assert_eq!(warm.rows, cold.rows);
    assert_eq!(warm.layout.pages, cold.layout.pages);
    assert_eq!(warm.layout.paragraphs, cold.layout.paragraphs);
    assert_eq!(before.hit_test(1, x, row.baseline), hit);
}

#[test]
fn refuses_table_instead_of_moving_following_text() {
    let bytes = include_bytes!("fixtures/physical-table.docx");
    let doc = ImportedDocument::open(bytes).unwrap();
    assert_eq!(
        plain_section(&doc).unwrap_err(),
        "body block 4: unsupported table"
    );
}

#[test]
fn refuses_multiple_sections_and_incompatible_page_metrics() {
    let doc = fixture();
    let anchor = doc.sections().unwrap().remove(0).anchor;
    let cases = [
        (LayoutField::ColumnCount, Some(2), "unsupported columnCount"),
        (
            LayoutField::MarginGutter,
            Some(20),
            "unsupported marginGutter",
        ),
        (LayoutField::MarginTop, Some(-20), "negative marginTop"),
        (
            LayoutField::PageWidth,
            Some(400),
            "margins leave no body area",
        ),
        (
            LayoutField::PageWidth,
            Some(i32::MAX),
            "pageWidth exceeds layout range",
        ),
        (LayoutField::PageWidth, None, "expected explicit pageWidth"),
        (LayoutField::Margins, None, "expected explicit marginLeft"),
    ];
    for (field, value, message) in cases {
        let preview = doc
            .preview_layout(&[LayoutEdit::Section(
                anchor.clone(),
                field,
                value.map(LayoutValue::Integer),
            )])
            .unwrap();
        let changed = ImportedDocument::open(&preview.to_docx().unwrap()).unwrap();
        assert_eq!(
            plain_section(&changed).unwrap_err(),
            format!("section 0: {message}")
        );
    }
    let preview = doc
        .preview_edits(EditBatch {
            blocks: &[BlockEdit::SplitSection(anchor, 2)],
            ..Default::default()
        })
        .unwrap();
    let split = ImportedDocument::open(&preview.to_docx().unwrap()).unwrap();
    assert_eq!(
        plain_section(&split).unwrap_err(),
        "expected exactly one section"
    );
}

#[test]
fn refuses_direct_paragraph_layout_run_formatting_and_manual_page_breaks() {
    let doc = fixture();
    let p = paragraph(&doc, 0);
    let preview = doc
        .preview_layout(&[LayoutEdit::Paragraph(
            p.anchor.clone(),
            LayoutField::SpaceAfter,
            Some(LayoutValue::Integer(20)),
        )])
        .unwrap();
    let spaced = ImportedDocument::open(&preview.to_docx().unwrap()).unwrap();
    assert_eq!(
        plain_section(&spaced).unwrap_err(),
        "body block 0: unsupported paragraph spaceAfter"
    );

    let ImportedInlineNode::Run(run) = &p.inline_structure[0] else {
        panic!("expected run");
    };
    let cases = [
        (InlineAction::SetBold(true), "formatting"),
        (
            InlineAction::Replace(ResolvedInline::Break {
                kind: BreakType::Page,
                format: Default::default(),
                comment: None,
            }),
            "content",
        ),
    ];
    for (action, reason) in cases {
        let preview = doc
            .preview_edits(EditBatch {
                inline_edits: &[InlineEdit {
                    target: run.anchor.clone(),
                    action,
                }],
                ..Default::default()
            })
            .unwrap();
        let changed = ImportedDocument::open(&preview.to_docx().unwrap()).unwrap();
        assert_eq!(
            plain_section(&changed).unwrap_err(),
            format!("body block 0: run 0: unsupported run {reason}")
        );
    }
}

#[test]
fn refuses_direct_list_numbering() {
    let doc = fixture();
    let num_id = doc.numbering_instances().unwrap()[0].num_id;
    let preview = doc
        .preview_edits(EditBatch {
            resources: &[ResourceEdit::Numbering(NumberingEdit::Paragraph {
                target: paragraph(&doc, 3).anchor.clone(),
                value: Some(NumberingReference {
                    id: num_id,
                    level: 0,
                }),
            })],
            ..Default::default()
        })
        .unwrap();
    let numbered = ImportedDocument::open(&preview.to_docx().unwrap()).unwrap();
    assert_eq!(
        plain_section(&numbered).unwrap_err(),
        "body block 3: unsupported paragraph numbering"
    );
}

#[test]
fn refuses_clearing_break_in_relationship_selected_main_part() {
    let bytes = include_bytes!("fixtures/physical-clearing-break.docx");
    let doc = ImportedDocument::open(bytes).unwrap();
    assert_eq!(paragraph(&doc, 2).text, "one\ntwo");
    assert_eq!(
        plain_section(&doc).unwrap_err(),
        "body block 2: run 0: unsupported w:br w:clear value"
    );
}

const FORMATTED: &[u8] = include_bytes!("fixtures/physical-formatted.docx");

#[test]
fn bound_import_aligns_wrapped_rows_and_overlaps_adjacent_spacing() {
    let mut session = LayoutSession::new(font(), 12 * 64).unwrap();
    let opened = session.open_pages(FORMATTED).unwrap();
    let spec = opened.spec();
    let snapshot = opened.geometry();
    assert_eq!(snapshot.rows.len(), 1);
    let [center_first, center_wrapped, right, left] = snapshot.rows[0].as_slice() else {
        panic!("expected two centered rows followed by one right and one left");
    };
    assert_eq!(
        [
            (center_first.paragraph, center_first.line),
            (center_wrapped.paragraph, center_wrapped.line)
        ],
        [(0, 0), (0, 1)]
    );
    let width = spec.width - spec.left - spec.right;
    for row in [center_first, center_wrapped] {
        let line = &snapshot.layout.paragraphs[row.paragraph][row.line];
        assert_eq!(row.x, spec.left + (width - line.width) / 2);
        assert_eq!(snapshot.hit_test(0, row.x, row.top), Some((0, line.start)));
        assert_eq!(snapshot.hit_test(0, row.x - 1, row.top), None);
    }
    assert!(center_wrapped.x > center_first.x);
    let right_width = snapshot.layout.paragraphs[1][0].width;
    assert_eq!(
        (right.paragraph, right.x + right_width),
        (1, spec.width - spec.right)
    );
    assert_eq!((left.paragraph, left.x), (2, spec.left));

    assert_eq!(center_first.top - spec.top, 67); // 21 twips rounded to 1/64 pt
    let leading = center_wrapped.top - center_first.bottom;
    assert_eq!(right.top - center_wrapped.bottom, leading + 11 * 64); // max(7 pt, 11 pt)
    assert_eq!(left.top - right.bottom, leading + 2 * 64); // max(2 pt, 1.05 pt)

    let bytes = opened
        .preview_insert(0, center_wrapped.x, center_wrapped.top, "b")
        .unwrap();
    let edited = ImportedDocument::open(&bytes).unwrap();
    assert_eq!(paragraph(&edited, 0).text, "hello ba");
    assert!(session.open_pages(&bytes).is_ok());

    let doc = ImportedDocument::open(FORMATTED).unwrap();
    assert_eq!(
        plain_section(&doc).unwrap_err(),
        "body block 0: unsupported paragraph alignment (expected left)"
    );
}

#[test]
fn bound_import_rejects_unmodeled_alignment_and_spacing() {
    let doc = ImportedDocument::open(FORMATTED).unwrap();
    let anchor = &paragraph(&doc, 0).anchor;
    let mut session = LayoutSession::new(font(), 12 * 64).unwrap();
    let cases = [
        (
            LayoutField::Alignment,
            LayoutValue::Token("both".into()),
            "unsupported paragraph alignment (expected left, center, or right)",
        ),
        (
            LayoutField::LineRule,
            LayoutValue::Token("auto".into()),
            "unsupported paragraph lineRule",
        ),
        (
            LayoutField::SpaceBefore,
            LayoutValue::Integer(i32::MAX),
            "paragraph spaceBefore exceeds layout range",
        ),
    ];
    for (field, value, expected) in cases {
        let bytes = doc
            .preview_layout(&[LayoutEdit::Paragraph(anchor.clone(), field, Some(value))])
            .unwrap()
            .to_docx()
            .unwrap();
        let error = session
            .open_pages(&bytes)
            .err()
            .expect("unsupported format");
        assert_eq!(error, format!("body block 0: {expected}"));
    }
    let bytes = include_bytes!("fixtures/physical-unsupported-spacing.docx");
    assert_eq!(
        session.open_pages(bytes).err().as_deref(),
        Some("body block 0: unsupported w:spacing attribute w:afterLines")
    );
}

const INDENTS: &[u8] = include_bytes!("fixtures/physical-indents.docx");

#[test]
fn bound_import_places_indented_rows_across_pages_and_retains_indents_on_edit() {
    let mut session = LayoutSession::new(font(), 12 * 64).unwrap();
    let opened = session.open_pages(INDENTS).unwrap();
    let snapshot = opened.geometry();
    let spec = opened.spec();
    assert_eq!(snapshot.rows.len(), 2);
    let actual: Vec<_> = snapshot
        .rows
        .iter()
        .map(|rows| {
            rows.iter()
                .map(|r| (r.paragraph, r.line, r.x))
                .collect::<Vec<_>>()
        })
        .collect();
    let right_x = spec.width - spec.right - 131 - snapshot.layout.paragraphs[2][0].width;
    assert_eq!(
        actual,
        [
            vec![
                (0, 0, spec.left + 256 + 451), // 4 pt left + 141 twip first line
                (0, 1, spec.left + 256),
                (1, 0, spec.left + 512 - 131), // 8 pt left - 41 twip hanging
            ],
            vec![
                (1, 1, spec.left + 512),
                (1, 2, spec.left + 512),
                (2, 0, right_x),
            ],
        ]
    );
    let continuation = &snapshot.rows[1][0];
    assert_eq!(continuation.top, spec.top);
    assert_eq!(
        snapshot.hit_test(1, continuation.x, continuation.top),
        Some((1, 6))
    );
    assert_eq!(
        snapshot.hit_test(1, continuation.x - 1, continuation.top),
        None
    );

    let doc = ImportedDocument::open(INDENTS).unwrap();
    assert_eq!(
        plain_section(&doc).unwrap_err(),
        "body block 0: unsupported paragraph leftIndent"
    );
    let edited_bytes = opened
        .preview_insert(1, continuation.x, continuation.top, "!")
        .unwrap();
    let direct = doc
        .preview_edits(EditBatch {
            text_insertions: &[TextInsertion {
                target: paragraph(&doc, 1).anchor.clone(),
                position: 6,
                text: "!".into(),
            }],
            ..Default::default()
        })
        .unwrap()
        .to_docx()
        .unwrap();
    assert_eq!(edited_bytes, direct);
    let edited = ImportedDocument::open(&edited_bytes).unwrap();
    assert_eq!(paragraph(&edited, 1).text, "hello !hello hello");
    assert_eq!(paragraph(&edited, 1).layout, paragraph(&doc, 1).layout);

    let mut cold = LayoutSession::with_cache_budget(font(), 12 * 64, 0).unwrap();
    for (bytes, warm) in [
        (INDENTS, opened),
        (&edited_bytes, session.open_pages(&edited_bytes).unwrap()),
    ] {
        let reopened = cold.open_pages(bytes).unwrap();
        let (warm, cold) = (warm.geometry(), reopened.geometry());
        assert_eq!(warm.rows, cold.rows);
        assert_eq!(warm.layout.pages, cold.layout.pages);
        assert_eq!(warm.layout.paragraphs, cold.layout.paragraphs);
    }
}

#[test]
fn bound_import_rejects_negative_or_ambiguous_indent_sources() {
    let mut session = LayoutSession::new(font(), 12 * 64).unwrap();
    let ambiguous = include_bytes!("fixtures/physical-ambiguous-indent.docx");
    assert_eq!(
        session.open_pages(ambiguous).err().as_deref(),
        Some("body block 1: w:ind cannot contain both w:firstLine and w:hanging")
    );
    let doc = ImportedDocument::open(INDENTS).unwrap();
    let anchor = &paragraph(&doc, 1).anchor;
    for (field, twips, reason) in [
        (
            LayoutField::LeftIndent,
            40,
            "paragraph hanging indent crosses the content's left edge",
        ),
        (
            LayoutField::RightIndent,
            -1,
            "negative paragraph rightIndent",
        ),
        (
            LayoutField::FirstLineIndent,
            -i32::MAX,
            "paragraph firstLineIndent exceeds layout range",
        ),
    ] {
        let bytes = doc
            .preview_layout(&[LayoutEdit::Paragraph(
                anchor.clone(),
                field,
                Some(LayoutValue::Integer(twips)),
            )])
            .unwrap()
            .to_docx()
            .unwrap();
        let error = session
            .open_pages(&bytes)
            .err()
            .expect("unsupported indent");
        assert_eq!(error, format!("body block 1: {reason}"));
    }
}
