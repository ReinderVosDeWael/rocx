use futures::executor::block_on;
use rocx::*;

#[test]
fn revision_projection_keeps_inserted_text_and_hides_deleted_text() {
    let revision = TextRevision {
        kind: RevisionKind::Insertion,
        author: "Reviewer".into(),
        date: Some("2024-02-29T12:34:56Z".into()),
    };
    let document = Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![
                TextRun::new("new")
                    .with_revision(Some(revision.clone()))
                    .into(),
                TextRun::new("old")
                    .with_revision(Value::future(async move {
                        Some(revision.with_kind(RevisionKind::Deletion))
                    }))
                    .into(),
                TextRun::new("hidden")
                    .with_revision(Value::future(async {
                        panic!("a hidden revision producer must remain unevaluated")
                    }))
                    .when(false)
                    .into(),
            ])
            .into(),
        ])
        .into(),
    ]);
    let snapshot = block_on(document.resolve()).unwrap();
    let paragraph = snapshot.sections()[0].paragraphs()[0];
    assert_eq!(paragraph.text(), "new");
    assert_eq!(paragraph.runs().len(), 2);
    assert_eq!(
        paragraph.runs()[1]
            .revision()
            .map(|revision| &revision.kind),
        Some(&RevisionKind::Deletion),
    );
}

#[test]
fn invalid_revision_date_fails_resolution() {
    let run = TextRun::new("text").with_revision(Some(TextRevision {
        kind: RevisionKind::Insertion,
        author: "Reviewer".into(),
        date: Some("2025-02-29T12:34:56Z".into()),
    }));
    let document = Document::new(vec![
        Section::new(vec![Paragraph::from_children(vec![run.into()]).into()]).into(),
    ]);
    let error = block_on(document.resolve()).unwrap_err().to_string();
    assert!(error.contains("revision.date"), "{error}");
}
