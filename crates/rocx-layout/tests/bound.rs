use rocx::editor::{BodyBlock, EditBatch, ImportedDocument, TextRangeEdit};
use rocx_layout::{Affinity, LayoutSession, TextPoint, plain_section};

const DOCX: &[u8] = include_bytes!("fixtures/physical-plain.docx");

fn p(paragraph: usize, offset: u32) -> TextPoint {
    TextPoint { paragraph, offset }
}

#[test]
fn previews_target_original_docx_even_after_opening_another_revision() {
    let mut session =
        LayoutSession::new(include_bytes!("fonts/DejaVuSans-subset.ttf").to_vec(), 768).unwrap();
    let original = session.open_pages(DOCX).unwrap();
    let caret = original
        .geometry()
        .caret_box(p(3, 2), Affinity::Upstream)
        .unwrap();
    let bytes = original
        .preview_insert(caret.page, caret.x, caret.top, "!")
        .unwrap();
    let newer = session.open_pages(&bytes).unwrap();
    let selected = original.preview_replace(p(3, 2), p(3, 1), "X").unwrap();
    let current = ImportedDocument::open(DOCX).unwrap();
    let BodyBlock::Paragraph(target) = &current.body()[3] else {
        unreachable!()
    };
    let direct = current
        .preview_edits(EditBatch {
            ranges: &[TextRangeEdit {
                target: target.anchor.clone(),
                start: 1,
                end: 2,
                text: "X".into(),
            }],
            ..Default::default()
        })
        .unwrap()
        .to_docx()
        .unwrap();
    assert_eq!(selected, direct);
    assert_eq!(
        plain_section(&ImportedDocument::open(&bytes).unwrap())
            .unwrap()
            .0[3],
        "a😀!a"
    );
    assert_eq!(newer.geometry().layout.reuse.line_hits, 3);
    assert!(
        original
            .geometry()
            .caret_box(p(3, 4), Affinity::Upstream)
            .is_none()
    );
    assert!(
        newer
            .geometry()
            .caret_box(p(3, 4), Affinity::Upstream)
            .is_some()
    );
}

#[test]
fn visible_carets_are_not_edit_permissions() {
    let mut session =
        LayoutSession::new(include_bytes!("fonts/DejaVuSans-subset.ttf").to_vec(), 768).unwrap();
    let source = session.open_pages(DOCX).unwrap();
    let at_break = source
        .geometry()
        .caret_box(p(2, 1), Affinity::Upstream)
        .unwrap();
    assert!(
        source
            .preview_insert(at_break.page, at_break.x, at_break.top, "x")
            .is_err()
    );
    assert!(source.preview_insert(0, 0, 0, "x").is_err());
    for (start, end) in [(p(0, 0), p(1, 0)), (p(0, 0), p(0, 0)), (p(0, 1), p(0, 20))] {
        assert!(source.preview_replace(start, end, "").is_err());
    }
}
