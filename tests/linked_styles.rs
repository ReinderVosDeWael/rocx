#[path = "support/linked_styles.rs"]
mod fixture;

use futures::executor::block_on;
use rv_docx_rs::*;
use std::io::{Cursor, Read};

#[test]
fn linked_style_references_are_not_inheritance_cycles_or_effective_formatting() {
    let snapshot = block_on(fixture::document().resolve()).unwrap();
    let bytes = snapshot.to_docx().unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("word/styles.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    for (id, target) in [
        ("PairParagraph", "PairCharacter"),
        ("PairCharacter", "PairParagraph"),
    ] {
        let style = tree
            .descendants()
            .find(|n| n.attribute((w, "styleId")) == Some(id))
            .unwrap();
        assert_eq!(
            style
                .children()
                .find(|n| n.has_tag_name((w, "link")))
                .unwrap()
                .attribute((w, "val")),
            Some(target)
        );
    }
    assert_eq!(
        snapshot.sections()[0].paragraphs()[0]
            .format()
            .style
            .as_deref(),
        Some("Pair Paragraph")
    );
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn authored_link_targets_must_exist_and_have_the_opposite_kind() {
    for target in ["Missing", "Normal", "Source"] {
        let mut document = Document::new(vec![]);
        document.styles = vec![
            ParagraphStyleDefinition {
                name: "Source".into(),
                linked_style: Some(target.into()),
                ..Default::default()
            }
            .into(),
        ]
        .into();
        let error = block_on(document.resolve())
            .unwrap()
            .to_docx()
            .unwrap_err()
            .to_string();
        assert!(error.contains("linked_style"), "{error}");
    }
}

#[test]
fn character_links_require_a_paragraph_target() {
    let mut document = Document::new(vec![]);
    document.styles = vec![
        CharacterStyleDefinition {
            name: "Source".into(),
            linked_style: Some("Source".into()),
            ..Default::default()
        }
        .into(),
    ]
    .into();
    assert!(block_on(document.resolve()).unwrap().to_docx().is_err());
}

#[test]
fn character_styles_cannot_be_applied_as_paragraph_styles() {
    let mut document = Document::new(vec![
        Section::new(vec![
            Paragraph::new("wrong")
                .with_format(ParagraphFormat {
                    style: Some("Character".into()),
                    ..Default::default()
                })
                .into(),
        ])
        .into(),
    ]);
    document.styles = vec![
        CharacterStyleDefinition {
            name: "Character".into(),
            ..Default::default()
        }
        .into(),
    ]
    .into();
    assert!(block_on(document.resolve()).unwrap().to_docx().is_err());
}
