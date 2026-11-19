use rv_docx_rs::*;

fn image() -> Value<Inline> {
    ImageRun::new(include_bytes!("../fixtures/images/sample.png").to_vec()).into()
}
fn link(text: &str) -> Value<Inline> {
    Hyperlink::new(
        HyperlinkTarget::external("https://example.invalid/λ?q=a%20b&x=2#frag").unwrap(),
        vec![TextRun::new(text).into()],
    )
    .into()
}
fn table(text: &str) -> Value<Block> {
    let nested = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![Paragraph::new("nested table").into()]).into(),
        ])
        .into(),
    ]);
    Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![Paragraph::new(text).into()]).into(),
            TableCell::new(vec![nested.into()]).into(),
        ])
        .into(),
    ])
    .into()
}
pub fn document() -> Document {
    let comment = Comment::new(vec![
        Paragraph::from_children(vec![
            TextRun::new("Rich λ")
                .with_format(RunFormat {
                    bold: Some(true),
                    ..Default::default()
                })
                .into(),
            link("comment link"),
            image(),
        ])
        .into(),
        table("comment table"),
    ])
    .with_metadata(CommentMetadata {
        author: Some("Ada".into()),
        date: Some("2026-11-18T01:02:03Z".into()),
        initials: Some("AL".into()),
    });
    let range = RangeId::new("structured review").unwrap();
    let mut document = Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![
                TextRun::new("body λ").into(),
                NoteReference::footnote(NoteId::new(0)).into(),
                NoteReference::footnote(NoteId::new(2)).into(),
                NoteReference::endnote(NoteId::new(0)).into(),
            ])
            .into(),
            Paragraph::from_children(vec![
                RangeStart::structured_comment(range.clone(), comment).into(),
                TextRun::new("reviewed").into(),
                RangeEnd::new(range).into(),
            ])
            .into(),
        ])
        .into(),
    ]);
    document.notes = vec![
        Note::footnote(
            NoteId::new(0),
            vec![
                Paragraph::from_children(vec![
                    TextRun::new("Foot λ")
                        .with_format(RunFormat {
                            bold: Some(true),
                            ..Default::default()
                        })
                        .into(),
                    link("note link"),
                    image(),
                ])
                .into(),
                table("note table"),
            ],
        )
        .into(),
        Note::footnote(NoteId::new(2), vec![Paragraph::new("Other foot").into()]).into(),
        Note::endnote(
            NoteId::new(0),
            vec![Paragraph::from_children(vec![TextRun::new("End Ω").into(), image()]).into()],
        )
        .into(),
    ]
    .into();
    document.timestamp = Some("2026-11-18T00:00:00Z".into()).into();
    document
}
