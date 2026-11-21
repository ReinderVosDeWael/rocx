//! Deferred note keys and a structured base-schema comment.
use rocx::*;

fn main() -> Result<()> {
    let output = std::env::args().nth(1).unwrap_or("notes.docx".into());
    let target = Property::new(
        "note.target",
        Value::future(async {
            NoteTarget {
                kind: NoteKind::Footnote,
                id: NoteId::new(0),
            }
        }),
    );
    let comment = Comment::new(vec![
        Paragraph::from_children(vec![
            TextRun::new("Structured review λ")
                .with_format(RunFormat {
                    bold: Some(true),
                    ..Default::default()
                })
                .into(),
        ])
        .into(),
    ])
    .with_metadata(CommentMetadata {
        author: Some("Ada".into()),
        date: Some("2026-11-18T00:00:00Z".into()),
        initials: Some("AL".into()),
    });
    let id = RangeId::new("review")?;
    let mut document = Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![
                TextRun::new("Text with a footnote").into(),
                NoteReference::new(target.reference()).into(),
                TextRun::new(" and endnote").into(),
                NoteReference::endnote(NoteId::new(0)).into(),
            ])
            .into(),
            Paragraph::from_children(vec![
                RangeStart::structured_comment(id.clone(), comment).into(),
                TextRun::new("Reviewed text").into(),
                RangeEnd::new(id).into(),
            ])
            .into(),
        ])
        .into(),
    ]);
    document.notes = vec![
        Note::new(
            target.reference(),
            vec![Paragraph::new("Footnote body λ").into()],
        )
        .into(),
        Note::endnote(
            NoteId::new(0),
            vec![Paragraph::new("Endnote body Ω").into()],
        )
        .into(),
    ]
    .into();
    futures::executor::block_on(document.resolve())?.save(output)
}
