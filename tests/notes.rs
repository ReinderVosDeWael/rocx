#[path = "support/notes.rs"]
mod fixture;
mod support;
use futures::executor::block_on;
use rocx::*;
use support::{W, part};

#[test]
fn footnotes_and_endnotes_have_distinct_id_spaces_and_real_story_parts() {
    let mut document = Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![
                TextRun::new("body").into(),
                NoteReference::footnote(NoteId::new(0)).into(),
                NoteReference::endnote(NoteId::new(0)).into(),
            ])
            .into(),
        ])
        .into(),
    ]);
    document.notes = vec![
        Note::footnote(NoteId::new(0), vec![Paragraph::new("foot λ").into()]).into(),
        Note::endnote(NoteId::new(0), vec![Paragraph::new("end λ").into()]).into(),
    ]
    .into();
    let snapshot = block_on(document.resolve()).unwrap();
    let bytes = snapshot.to_docx().unwrap();
    assert_eq!(snapshot.notes().len(), 2);
    for (kind, text) in [("footnote", "foot λ"), ("endnote", "end λ")] {
        let xml = part(&bytes, &format!("word/{kind}s.xml"));
        let tree = roxmltree::Document::parse(&xml).unwrap();
        let ordinary = tree
            .descendants()
            .find(|n| n.has_tag_name((W, kind)) && n.attribute((W, "id")) == Some("0"))
            .unwrap();
        assert!(ordinary.descendants().any(|n| n.text() == Some(text)));
        assert!(
            ordinary
                .descendants()
                .any(|n| n.has_tag_name((W, format!("{kind}Ref").as_str())))
        );
        for special in ["separator", "continuationSeparator"] {
            assert!(tree.descendants().any(|n| n.has_tag_name((W,kind)) && n.attribute((W,"type"))==Some(special)));
        }
    }
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn rich_note_and_comment_content_uses_the_physical_owner_for_media_and_links() {
    let snapshot = block_on(fixture::document().resolve()).unwrap();
    let bytes = snapshot.to_docx().unwrap();
    assert_eq!(snapshot.sections()[0].paragraphs()[0].text(), "body λ");
    for owner in ["footnotes", "endnotes", "comments"] {
        let xml = part(&bytes, &format!("word/{owner}.xml"));
        let tree = roxmltree::Document::parse(&xml).unwrap();
        assert!(tree.descendants().any(|n| n.has_tag_name((W, "drawing"))));
        let rels = part(&bytes, &format!("word/_rels/{owner}.xml.rels"));
        assert!(rels.contains("/image"));
        if owner != "endnotes" {
            assert!(rels.contains("/hyperlink"));
            assert!(tree.descendants().any(|n| n.has_tag_name((W, "tbl"))));
        }
    }
    let comments = part(&bytes, "word/comments.xml");
    let tree = roxmltree::Document::parse(&comments).unwrap();
    let comment = tree
        .descendants()
        .find(|n| n.has_tag_name((W, "comment")))
        .unwrap();
    assert_eq!(comment.attribute((W, "author")), Some("Ada"));
    assert_eq!(comment.attribute((W, "date")), Some("2026-11-18T01:02:03Z"));
    assert_eq!(comment.attribute((W, "initials")), Some("AL"));
}

#[test]
fn missing_duplicate_nested_and_cross_note_anchors_are_rejected() {
    let body = || {
        Section::new(vec![
            Paragraph::from_children(vec![NoteReference::footnote(NoteId::new(1)).into()]).into(),
        ])
    };
    assert!(
        block_on(Document::new(vec![body().into()]).resolve())
            .unwrap()
            .to_docx()
            .is_err()
    );
    let mut duplicate = Document::new(vec![body().into()]);
    duplicate.notes = vec![
        Note::footnote(NoteId::new(1), vec![]).into(),
        Note::footnote(NoteId::new(1), vec![]).into(),
    ]
    .into();
    assert_eq!(
        block_on(duplicate.resolve()).unwrap_err().path(),
        Some("notes[1].target")
    );
    let nested = Document {
        notes: vec![
            Note::footnote(
                NoteId::new(1),
                vec![
                    Paragraph::from_children(vec![NoteReference::endnote(NoteId::new(2)).into()])
                        .into(),
                ],
            )
            .into(),
        ]
        .into(),
        ..Default::default()
    };
    let error = block_on(nested.resolve()).unwrap_err();
    assert!(error.path().unwrap().starts_with("notes[0].children"));
    let name = BookmarkName::new("cross_note").unwrap();
    let cross = Document {
        notes: vec![
            Note::footnote(
                NoteId::new(1),
                vec![
                    Paragraph::from_children(vec![RangeStart::bookmark(name.clone()).into()])
                        .into(),
                ],
            )
            .into(),
            Note::footnote(
                NoteId::new(2),
                vec![Paragraph::from_children(vec![RangeEnd::bookmark(&name).into()]).into()],
            )
            .into(),
        ]
        .into(),
        ..Default::default()
    };
    assert!(block_on(cross.resolve()).is_err());
}

#[test]
fn comment_date_validation_uses_an_explicit_utc_seconds_subset() {
    for date in [
        "2026-02-29T00:00:00Z",
        "2026-11-18T24:00:00Z",
        "2026-11-18",
        "0000-01-01T00:00:00Z",
    ] {
        assert!(
            CommentMetadata {
                date: Some(date.into()),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        CommentMetadata {
            date: Some("2024-02-29T23:59:59Z".into()),
            ..Default::default()
        }
        .validate()
        .is_ok()
    );
}
