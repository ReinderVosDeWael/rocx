mod support;
use futures::executor::block_on;
use rocx::*;
use support::{W, part};
const TARGET: &str = "https://example.invalid/λ?q=a%20b&lang=日本#résumé";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn link() -> Hyperlink {
    Hyperlink::new(
        HyperlinkTarget::external(TARGET).unwrap(),
        vec![
            TextRun::new("Hello ")
                .with_format(RunFormat {
                    bold: Some(true),
                    ..Default::default()
                })
                .into(),
            TextRun::new("λ").into(),
            Tab::default().into(),
            Break::default().into(),
        ],
    )
    .with_properties(HyperlinkProperties {
        tooltip: Some("A <&> λ".into()),
        history: Some(false),
    })
}
#[test]
fn rich_links_project_display_text_and_use_owner_local_external_relationships() {
    let mut section = Section::new(vec![
        Paragraph::from_children(vec![link().into(), link().into()]).into(),
        Table::new(vec![
            TableRow::new(vec![
                TableCell::new(vec![Paragraph::from_children(vec![link().into()]).into()]).into(),
            ])
            .into(),
        ])
        .into(),
    ]);
    section.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![Paragraph::from_children(vec![link().into()]).into()]).into(),
    );
    section.footers.insert(
        HeaderFooterType::Default,
        Footer::new(vec![Paragraph::from_children(vec![link().into()]).into()]).into(),
    );
    let snapshot = block_on(Document::new(vec![section.into()]).resolve()).unwrap();
    assert_eq!(
        snapshot.sections()[0].paragraphs()[0].text(),
        "Hello λ\t\nHello λ\t\n"
    );
    let bytes = snapshot.to_docx().unwrap();
    for (owner, count) in [("document", 3), ("header1", 1), ("footer1", 1)] {
        let xml = part(&bytes, &format!("word/{owner}.xml"));
        let tree = roxmltree::Document::parse(&xml).unwrap();
        let links: Vec<_> = tree
            .descendants()
            .filter(|n| n.has_tag_name((W, "hyperlink")))
            .collect();
        assert_eq!(links.len(), count);
        let rels = part(&bytes, &format!("word/_rels/{owner}.xml.rels"));
        let relationships = roxmltree::Document::parse(&rels).unwrap();
        for link in links {
            assert_eq!(link.attribute((W, "tooltip")), Some("A <&> λ"));
            assert_eq!(link.attribute((W, "history")), Some("0"));
            assert!(link.descendants().any(|n| n.has_tag_name((W, "b"))));
            let id = link.attribute((R, "id")).unwrap();
            let relation = relationships
                .descendants()
                .find(|n| n.attribute("Id") == Some(id))
                .unwrap();
            assert_eq!(relation.attribute("Target"), Some(TARGET));
            assert_eq!(relation.attribute("TargetMode"), Some("External"));
            assert_eq!(
                relation.attribute("Type"),
                Some(format!("{R}/hyperlink").as_str())
            );
        }
        assert_eq!(
            relationships
                .descendants()
                .filter(|n| n.attribute("Target") == Some(TARGET))
                .count(),
            1
        );
    }
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}
#[test]
fn deferred_link_fields_share_generic_properties_and_hidden_links_do_not_run() {
    let target = Property::new(
        "target",
        Value::future(async { HyperlinkTarget::external(TARGET).unwrap() }),
    );
    let children = Property::new(
        "children",
        Value::try_future(async {
            Ok::<_, Error>(vec![TextRun::new("async λ").resolve().await?.unwrap()])
        }),
    );
    let properties = Property::new(
        "properties",
        Value::future(async {
            HyperlinkProperties {
                tooltip: Some("async".into()),
                history: None,
            }
        }),
    );
    let link = Hyperlink::new(
        target.reference(),
        Value::from(children.reference()).map(|children: Vec<ResolvedInline>| {
            children
                .into_iter()
                .map(Into::into)
                .collect::<InlineChildren>()
        }),
    )
    .with_properties(properties.reference());
    let resolved = block_on(link.resolve()).unwrap().unwrap();
    assert_eq!(resolved.text(), "async λ");
    let hidden = Hyperlink::new(
        Value::lazy(|| panic!("target ran")),
        Value::lazy(|| panic!("children ran")),
    )
    .with_properties(Value::lazy(|| panic!("properties ran")))
    .when(false);
    assert!(block_on(hidden.resolve()).unwrap().is_none());
}
#[test]
fn invalid_targets_and_nested_links_are_rejected() {
    for target in [
        "",
        "#bookmark",
        "relative/path",
        "https://a b",
        "https://a/%GG",
        "https://a/%",
        "1bad:test",
        "https://a/\u{0}",
    ] {
        assert_eq!(
            HyperlinkTarget::external(target).unwrap_err().rule,
            Rule::InvalidRelationship
        );
    }
    let nested = Hyperlink::new(
        HyperlinkTarget::external(TARGET).unwrap(),
        vec![link().into()],
    );
    let error = block_on(nested.resolve()).unwrap_err();
    assert_eq!(error.rule(), Some(Rule::InvalidContent));
    assert_eq!(error.path(), Some("hyperlink.children[0]"));
    let resolved = block_on(link().resolve()).unwrap().unwrap();
    assert!(
        ResolvedHyperlink::new(
            HyperlinkTarget::external(TARGET).unwrap(),
            vec![resolved],
            HyperlinkProperties::default()
        )
        .is_err()
    );
}

#[test]
fn deferred_target_validation_reports_the_field_once() {
    let target = Value::try_future(async {
        HyperlinkTarget::external("relative/path").map_err(Error::from)
    });
    let error = block_on(Hyperlink::new(target, Vec::new()).resolve()).unwrap_err();
    assert_eq!(error.rule(), Some(Rule::InvalidRelationship));
    assert_eq!(error.path(), Some("hyperlink.target"));
}

#[test]
fn template_edits_visit_display_runs_but_never_cross_link_boundaries_or_edit_targets() {
    let target = HyperlinkTarget::external("https://example.invalid/{{NAME}}?x=1&y=2#frag");
    // Braces must be percent-encoded in a URI; text replacements do not touch them.
    assert!(target.is_err());
    let link = Hyperlink::new(
        HyperlinkTarget::external("https://example.invalid/%7B%7BNAME%7D%7D#frag").unwrap(),
        vec![TextRun::new("{{NA").into(), TextRun::new("ME}}").into()],
    );
    let source = block_on(
        Document::new(vec![
            Section::new(vec![
                Paragraph::from_children(vec![
                    TextRun::new("before ").into(),
                    link.into(),
                    TextRun::new(" after").into(),
                ])
                .into(),
            ])
            .into(),
        ])
        .resolve(),
    )
    .unwrap()
    .to_docx()
    .unwrap();
    let empty = block_on(Document::default().resolve()).unwrap();
    let edits = TemplateEdits {
        replacements: vec![("{{NAME}}".into(), "λ <&>".into())],
        ..Default::default()
    };
    let output = application::compile_with_edits(&empty, &source, &edits, &Default::default())
        .unwrap()
        .to_bytes()
        .unwrap();
    assert_eq!(support::texts(&output), ["before λ <&> after"]);
    assert_eq!(
        part(&source, "word/_rels/document.xml.rels"),
        part(&output, "word/_rels/document.xml.rels")
    );
    let crossing = TemplateEdits {
        replacements: vec![("before {{NAME}}".into(), "bad".into())],
        ..Default::default()
    };
    assert_eq!(
        application::compile_with_edits(&empty, &source, &crossing, &Default::default())
            .unwrap_err()
            .rule(),
        Some(Rule::UnsupportedEdit)
    );
}

#[test]
fn link_images_and_comments_keep_their_existing_story_rules() {
    let mut run = TextRun::new("commented link");
    run.comment_text = Some("child note".into());
    let link = Hyperlink::new(
        HyperlinkTarget::external(TARGET).unwrap(),
        vec![
            run.into(),
            ImageRun::new(include_bytes!("fixtures/images/sample.png").to_vec()).into(),
        ],
    );
    let mut paragraph = Paragraph::from_children(vec![link.into()]);
    paragraph.comment_text = Some("paragraph note".into());
    let body = block_on(Document::new(vec![Section::new(vec![paragraph.into()]).into()]).resolve())
        .unwrap()
        .to_docx()
        .unwrap();
    let xml = part(&body, "word/document.xml");
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let link = tree
        .descendants()
        .find(|n| n.has_tag_name((W, "hyperlink")))
        .unwrap();
    assert!(link.descendants().any(|n| n.has_tag_name((W, "drawing"))));
    assert!(
        link.children()
            .any(|n| n.has_tag_name((W, "commentRangeStart")))
    );
    assert!(part(&body, "word/comments.xml").contains("child note"));
    let mut run = TextRun::new("header link");
    run.comment_text = Some("unsupported".into());
    let mut section = Section::default();
    section.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![
            Paragraph::from_children(vec![
                Hyperlink::new(HyperlinkTarget::external(TARGET).unwrap(), vec![run.into()]).into(),
            ])
            .into(),
        ])
        .into(),
    );
    assert_eq!(
        block_on(Document::new(vec![section.into()]).resolve())
            .unwrap()
            .to_docx()
            .unwrap_err()
            .rule(),
        Some(Rule::InvalidContent)
    );
}
