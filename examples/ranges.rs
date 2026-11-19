//! Overlapping bookmarks and a cross-paragraph body comment, with an internal link.
use rv_docx_rs::*;

fn main() -> Result<()> {
    let output = std::env::args().nth(1).unwrap_or("ranges.docx".into());
    let bookmark = BookmarkName::new("chapter")?;
    let review = RangeId::new("review")?;
    let first = Paragraph::from_children(vec![
        Hyperlink::new(
            HyperlinkTarget::bookmark(bookmark.clone()),
            vec![TextRun::new("Jump to chapter. ").into()],
        )
        .into(),
        RangeStart::bookmark(bookmark.clone()).into(),
        TextRun::new("Chapter begins. ").into(),
        RangeStart::comment(
            review.clone(),
            ResolvedComment {
                text: "Review this range".into(),
                author: Some("Ada".into()),
            },
        )
        .into(),
        TextRun::new("First paragraph.").into(),
    ]);
    let second = Paragraph::from_children(vec![
        TextRun::new("Second paragraph. ").into(),
        RangeEnd::bookmark(&bookmark).into(),
        TextRun::new("Review ends here.").into(),
        RangeEnd::new(review).into(),
    ]);
    let snapshot = futures::executor::block_on(
        Document::new(vec![Section::new(vec![first.into(), second.into()]).into()]).resolve(),
    )?;
    snapshot.save(output)
}
