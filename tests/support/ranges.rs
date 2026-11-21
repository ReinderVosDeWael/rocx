use rocx::*;

pub fn document() -> Document {
    let a = BookmarkName::new("alpha").unwrap();
    let b = BookmarkName::new("beta").unwrap();
    let review = RangeId::new("review").unwrap();
    let link = Hyperlink::new(
        HyperlinkTarget::bookmark(b.clone()),
        vec![TextRun::new("jump ").into()],
    );
    let first = Paragraph::from_children(vec![
        link.into(),
        RangeStart::bookmark(a.clone()).into(),
        TextRun::new("alpha ").into(),
        RangeStart::comment(
            review.clone(),
            ResolvedComment {
                text: "range note".into(),
                author: Some("Ada".into()),
            },
        )
        .into(),
        TextRun::new("beta").into(),
    ]);
    let second = Paragraph::from_children(vec![
        RangeStart::bookmark(b.clone()).into(),
        TextRun::new("gamma ").into(),
        RangeEnd::bookmark(&a).into(),
        TextRun::new("delta ").into(),
        RangeEnd::new(review).into(),
        TextRun::new("epsilon").into(),
        RangeEnd::bookmark(&b).into(),
    ]);
    let mut legacy = Paragraph::new("legacy");
    legacy.comment_text = Some("legacy note".into());
    Document::new(vec![
        Section::new(vec![first.into(), second.into(), legacy.into()]).into(),
    ])
}
