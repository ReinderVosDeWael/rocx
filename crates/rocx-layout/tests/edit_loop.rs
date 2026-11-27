use rocx::editor::{
    BodyBlock, EditBatch, ImportedDocument, ImportedInline, ImportedInlineNode, TextInsertion,
};
use rocx_layout::LayoutSession;

// Fixture-only projection. Each item must be ordinary run text; do not
// accidentally qualify tables or opaque content by dropping it.
fn paragraphs(doc: &ImportedDocument) -> Vec<String> {
    doc.body()
        .iter()
        .map(|block| {
            let BodyBlock::Paragraph(p) = block else {
                panic!("expected paragraph")
            };
            let mut text = String::new();
            for node in &p.inline_structure {
                let ImportedInlineNode::Run(run) = node else {
                    panic!("expected direct run")
                };
                for item in &run.items {
                    let ImportedInline::Text(s) = item else {
                        panic!("expected text")
                    };
                    text.push_str(s);
                }
            }
            text
        })
        .collect()
}

fn font() -> Vec<u8> {
    include_bytes!("fonts/DejaVuSans-subset.ttf").to_vec()
}

#[test]
fn page_capacity_and_width_invalidate_different_products() {
    let input = vec!["first".into(), "second".into()];
    let mut s = LayoutSession::new(font(), 12 * 64).unwrap();
    let old = s.layout(&input, 20_000, 1).unwrap();
    let resized = s.layout(&input, 20_000, 2).unwrap();
    assert_eq!(old.pages.len(), 2);
    assert_eq!(resized.pages.len(), 1);
    assert_eq!(resized.reuse.line_hits, 2);
    assert_eq!(resized.reuse.page_hits, 0);
    let narrow = s.layout(&input, 512, 2).unwrap();
    assert_eq!(narrow.reuse.shaped_hits, 2);
    assert_eq!(narrow.reuse.line_hits, 0);
    let mut no_cache = LayoutSession::with_cache_budget(font(), 12 * 64, 0).unwrap();
    for _ in 0..2 {
        let cold = no_cache.layout(&input, 512, 2).unwrap();
        assert_eq!(cold.reuse, Default::default());
        assert_eq!(narrow.paragraphs, cold.paragraphs);
        assert_eq!(narrow.pages, cold.pages);
    }
}

#[test]
fn hit_second_occurrence_edit_original_docx_and_reflow() {
    let doc = ImportedDocument::open(include_bytes!("fixtures/split-runs.docx")).unwrap();
    let mut session = LayoutSession::new(font(), 12 * 64).unwrap();
    let content = paragraphs(&doc);
    let before = session.layout(&content, 20_000, 1).unwrap();
    assert_eq!(before.pages.len(), 2);
    assert_eq!(before.reuse.line_hits, 1);
    let width = before.paragraphs[0][0].width;
    let (index, offset) = before.hit_row(1, 0, width).unwrap();
    assert_eq!((index, offset), (1, 5));
    let BodyBlock::Paragraph(target) = &doc.body()[index] else {
        unreachable!()
    };
    let point = TextInsertion {
        target: target.anchor.clone(),
        position: offset as usize,
        text: " hello".into(),
    };
    let batch = || EditBatch {
        text_insertions: std::slice::from_ref(&point),
        ..Default::default()
    };
    let edited =
        ImportedDocument::open(&doc.preview_edits(batch()).unwrap().to_docx().unwrap()).unwrap();
    let next = paragraphs(&edited);
    assert_eq!(next, ["hello", "hello hello"]);
    assert!(edited.preview_edits(batch()).is_err());

    // First change the page width, then edit the second paragraph.
    let at_width = session.layout(&content, width, 1).unwrap();
    let warm = session.layout(&next, width, 1).unwrap();
    let cold = LayoutSession::with_cache_budget(font(), 12 * 64, 0)
        .unwrap()
        .layout(&next, width, 1)
        .unwrap();
    assert!(warm.pages.len() > at_width.pages.len());
    assert!(warm.reuse.line_hits >= 1);
    assert!(warm.reuse.page_hits >= 1);
    assert_eq!(warm.pages, cold.pages);
    assert_eq!(warm.paragraphs, cold.paragraphs);
    assert_eq!(before.hit_row(1, 0, width), Some((1, 5)));
}
