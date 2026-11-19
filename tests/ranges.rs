#[path = "support/ranges.rs"]
mod fixture;
mod support;
use futures::executor::block_on;
use rv_docx_rs::*;
use support::{W, part};

fn doc(runs: InlineChildren) -> Document {
    Document::new(vec![
        Section::new(vec![Paragraph::from_children(runs).into()]).into(),
    ])
}
#[test]
fn overlapping_cross_paragraph_ranges_and_internal_links_have_paired_ids() {
    let snapshot = block_on(fixture::document().resolve()).unwrap();
    assert_eq!(
        snapshot.sections()[0].paragraphs()[0].text(),
        "jump alpha beta"
    );
    let bytes = snapshot.to_docx().unwrap();
    let xml = part(&bytes, "word/document.xml");
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let mut ids = std::collections::BTreeSet::new();
    for name in ["alpha", "beta"] {
        let start = tree
            .descendants()
            .find(|n| {
                n.has_tag_name((W, "bookmarkStart")) && n.attribute((W, "name")) == Some(name)
            })
            .unwrap();
        let id = start.attribute((W, "id")).unwrap();
        assert!(ids.insert(id));
        assert_eq!(tree.descendants().filter(|n| n.has_tag_name((W,"bookmarkEnd")) && n.attribute((W,"id")) == Some(id)).count(), 1);
    }
    let link = tree
        .descendants()
        .find(|n| n.has_tag_name((W, "hyperlink")))
        .unwrap();
    assert_eq!(link.attribute((W, "anchor")), Some("beta"));
    assert!(
        link.attribute((
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
            "id"
        ))
        .is_none()
    );
    assert!(!part(&bytes, "word/_rels/document.xml.rels").contains("/hyperlink"));
    let comments = part(&bytes, "word/comments.xml");
    let comments = roxmltree::Document::parse(&comments).unwrap();
    assert_eq!(
        comments
            .descendants()
            .filter(|n| n.has_tag_name((W, "comment")))
            .count(),
        2
    );
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}
#[test]
fn dangling_reversed_duplicate_cross_story_and_empty_comment_ranges_fail_with_locations() {
    let name = BookmarkName::new("name").unwrap();
    let cases = vec![
        doc(vec![RangeStart::bookmark(name.clone()).into()]),
        doc(vec![
            RangeEnd::bookmark(&name).into(),
            RangeStart::bookmark(name.clone()).into(),
        ]),
        doc(vec![
            RangeStart::bookmark(name.clone()).into(),
            RangeStart::bookmark(name.clone()).into(),
            RangeEnd::bookmark(&name).into(),
        ]),
        doc(vec![
            RangeStart::comment(
                RangeId::new("empty").unwrap(),
                ResolvedComment {
                    text: "empty".into(),
                    author: None,
                },
            )
            .into(),
            RangeEnd::new(RangeId::new("empty").unwrap()).into(),
        ]),
    ];
    for document in cases {
        let error = block_on(document.resolve()).unwrap_err();
        assert_eq!(error.rule(), Some(Rule::InvalidContent));
        assert!(
            error
                .path()
                .unwrap()
                .starts_with("sections[0].children[0].children[")
        );
    }
    let mut section = Section::new(vec![
        Paragraph::from_children(vec![RangeStart::bookmark(name.clone()).into()]).into(),
    ]);
    section.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![
            Paragraph::from_children(vec![RangeEnd::bookmark(&name).into()]).into(),
        ])
        .into(),
    );
    assert!(block_on(Document::new(vec![section.into()]).resolve()).is_err());
    let missing = block_on(
        doc(vec![
            Hyperlink::new(
                HyperlinkTarget::bookmark(name),
                vec![TextRun::new("missing").into()],
            )
            .into(),
        ])
        .resolve(),
    )
    .unwrap()
    .to_docx()
    .unwrap_err();
    assert_eq!(missing.rule(), Some(Rule::InvalidRelationship));
    assert_eq!(
        missing.path(),
        Some("sections[0].children[0].children[0].target")
    );
}
#[test]
fn ranges_can_cross_sections_but_comments_still_cannot_live_in_headers() {
    let name = BookmarkName::new("across_sections").unwrap();
    let document = Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![
                RangeStart::bookmark(name.clone()).into(),
                TextRun::new("first").into(),
            ])
            .into(),
        ])
        .into(),
        Section::new(vec![
            Paragraph::from_children(vec![
                TextRun::new("second").into(),
                RangeEnd::bookmark(&name).into(),
            ])
            .into(),
        ])
        .into(),
    ]);
    assert!(block_on(document.resolve()).unwrap().to_docx().is_ok());
    let id = RangeId::new("header_review").unwrap();
    let mut section = Section::default();
    section.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![
            Paragraph::from_children(vec![
                RangeStart::comment(
                    id.clone(),
                    ResolvedComment {
                        text: "note".into(),
                        author: None,
                    },
                )
                .into(),
                TextRun::new("header").into(),
                RangeEnd::new(id).into(),
            ])
            .into(),
        ])
        .into(),
    );
    assert_eq!(
        block_on(Document::new(vec![section.into()]).resolve())
            .unwrap_err()
            .rule(),
        Some(Rule::InvalidContent)
    );
}
#[test]
fn logical_stories_share_a_physical_owner_without_aliasing() {
    let mut stories = ooxml::StoryRegistry::default();
    let owner = opc::PartName::new("word/comments.xml").unwrap();
    let a = stories
        .register(
            StoryKind::Comment,
            owner.clone(),
            ooxml::StoryContent::CommentText,
        )
        .unwrap();
    let b = stories
        .register(
            StoryKind::Comment,
            owner.clone(),
            ooxml::StoryContent::CommentText,
        )
        .unwrap();
    assert_ne!(a, b);
    assert_eq!(stories.get(a).unwrap().owner, owner);
    assert_eq!(stories.get(b).unwrap().owner, owner);
    assert_eq!(stories.stories().len(), 2);
}

#[test]
fn table_cells_retain_the_enclosing_range_story() {
    let name = BookmarkName::new("table_range").unwrap();
    let first = TableCell::new(vec![
        Paragraph::from_children(vec![
            RangeStart::bookmark(name.clone()).into(),
            TextRun::new("first").into(),
        ])
        .into(),
    ]);
    let second = TableCell::new(vec![
        Paragraph::from_children(vec![
            TextRun::new("second").into(),
            RangeEnd::bookmark(&name).into(),
        ])
        .into(),
    ]);
    let document = Document::new(vec![
        Section::new(vec![
            Table::new(vec![
                TableRow::new(vec![first.into(), second.into()]).into(),
            ])
            .into(),
        ])
        .into(),
    ]);
    let bytes = block_on(document.resolve()).unwrap().to_docx().unwrap();
    let xml = part(&bytes, "word/document.xml");
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let start = tree
        .descendants()
        .find(|n| n.has_tag_name((W, "bookmarkStart")))
        .unwrap();
    let end = tree
        .descendants()
        .find(|n| n.has_tag_name((W, "bookmarkEnd")))
        .unwrap();
    assert_eq!(start.attribute((W, "id")), end.attribute((W, "id")));
    assert_ne!(
        start.ancestors().find(|n| n.has_tag_name((W, "tc"))),
        end.ancestors().find(|n| n.has_tag_name((W, "tc")))
    );
}
#[test]
fn ranges_resolve_deferred_keys_and_payloads_and_skip_hidden_markers() {
    let name = BookmarkName::new("deferred").unwrap();
    let id = Property::new(
        "range.id",
        Value::future({
            let name = name.clone();
            async move { name.range_id() }
        }),
    );
    let start = RangeStart::new(
        id.reference(),
        Value::future(async move { RangeKind::Bookmark(name) }),
    );
    let end = RangeEnd::new(id.reference());
    let hidden = RangeEnd::new(Value::lazy(|| panic!("hidden key ran"))).when(false);
    assert!(
        block_on(doc(vec![start.into(), end.into(), hidden.into()]).resolve())
            .unwrap()
            .to_docx()
            .is_ok()
    );
}

#[test]
fn imported_bookmarks_reserve_names_and_ids_without_rewriting_existing_links() {
    let source = block_on(fixture::document().resolve())
        .unwrap()
        .to_docx()
        .unwrap();
    let name = BookmarkName::new("gamma").unwrap();
    let addition = block_on(
        doc(vec![
            RangeStart::bookmark(name.clone()).into(),
            TextRun::new("new").into(),
            RangeEnd::bookmark(&name).into(),
            Hyperlink::new(
                HyperlinkTarget::bookmark(BookmarkName::new("beta").unwrap()),
                vec![TextRun::new("imported target").into()],
            )
            .into(),
        ])
        .resolve(),
    )
    .unwrap();
    let output = application::compile(&addition, &source, &Default::default())
        .unwrap()
        .to_bytes()
        .unwrap();
    let xml = part(&output, "word/document.xml");
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let ids: Vec<_> = tree
        .descendants()
        .filter(|n| n.has_tag_name((W, "bookmarkStart")))
        .map(|n| n.attribute((W, "id")).unwrap())
        .collect();
    assert_eq!(ids.len(), 3);
    assert_eq!(
        ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
        3
    );
    assert_eq!(
        tree.descendants()
            .filter(
                |n| n.has_tag_name((W, "hyperlink")) && n.attribute((W, "anchor")) == Some("beta")
            )
            .count(),
        2
    );
    let name = BookmarkName::new("alpha").unwrap();
    let collision = block_on(
        doc(vec![
            RangeStart::bookmark(name.clone()).into(),
            RangeEnd::bookmark(&name).into(),
        ])
        .resolve(),
    )
    .unwrap();
    let error = application::compile(&collision, &source, &Default::default()).unwrap_err();
    assert_eq!(error.rule(), Some(Rule::InvalidContent));
    assert!(error.path().is_some());
}

#[test]
fn template_replacement_preserves_marker_positions_and_inserts_at_the_first_run() {
    let name = BookmarkName::new("template_range").unwrap();
    let source = block_on(
        doc(vec![
            RangeStart::bookmark(name.clone()).into(),
            TextRun::new("{{NA").into(),
            RangeEnd::bookmark(&name).into(),
            TextRun::new("ME}}").into(),
        ])
        .resolve(),
    )
    .unwrap()
    .to_docx()
    .unwrap();
    let empty = block_on(Document::default().resolve()).unwrap();
    let edits = TemplateEdits {
        replacements: vec![("{{NAME}}".into(), "New λ".into())],
        ..Default::default()
    };
    let output = application::compile_with_edits(&empty, &source, &edits, &Default::default())
        .unwrap()
        .to_bytes()
        .unwrap();
    let xml = part(&output, "word/document.xml");
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let start = tree
        .descendants()
        .find(|n| n.has_tag_name((W, "bookmarkStart")))
        .unwrap();
    let run = start.next_sibling_element().unwrap();
    assert_eq!(
        run.descendants()
            .find(|n| n.has_tag_name((W, "t")))
            .unwrap()
            .text(),
        Some("New λ")
    );
    assert!(
        run.next_sibling_element()
            .unwrap()
            .has_tag_name((W, "bookmarkEnd"))
    );
}
