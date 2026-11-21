#[path = "support/numbering_replacements.rs"]
mod fixture;

use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read};

#[test]
fn concrete_level_replacements_preserve_abstract_levels_and_combine_start_overrides() {
    let snapshot = block_on(fixture::document().resolve()).unwrap();
    let bytes = snapshot.to_docx().unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("word/numbering.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let num = tree
        .descendants()
        .find(|n| n.has_tag_name((w, "num")) && n.attribute((w, "numId")) == Some("42"))
        .unwrap();
    let overrides: Vec<_> = num
        .children()
        .filter(|n| n.has_tag_name((w, "lvlOverride")))
        .collect();
    assert_eq!(
        overrides
            .iter()
            .map(|n| n.attribute((w, "ilvl")))
            .collect::<Vec<_>>(),
        [Some("0"), Some("1"), Some("2")]
    );
    assert_eq!(
        overrides[0]
            .children()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name())
            .collect::<Vec<_>>(),
        ["startOverride", "lvl"]
    );
    assert_eq!(
        overrides[0]
            .descendants()
            .find(|n| n.has_tag_name((w, "start")))
            .unwrap()
            .attribute((w, "val")),
        Some("4")
    );
    assert_eq!(
        overrides[0]
            .descendants()
            .find(|n| n.has_tag_name((w, "startOverride")))
            .unwrap()
            .attribute((w, "val")),
        Some("7")
    );
    assert_eq!(
        overrides[1]
            .descendants()
            .find(|n| n.has_tag_name((w, "numFmt")))
            .unwrap()
            .attribute((w, "val")),
        Some("bullet")
    );
    let abstract_id = num
        .children()
        .find(|n| n.has_tag_name((w, "abstractNumId")))
        .unwrap()
        .attribute((w, "val"));
    let abstract_node = tree
        .descendants()
        .find(|n| {
            n.has_tag_name((w, "abstractNum")) && n.attribute((w, "abstractNumId")) == abstract_id
        })
        .unwrap();
    assert!(
        abstract_node
            .descendants()
            .filter(|n| n.has_tag_name((w, "start")))
            .all(|n| n.attribute((w, "val")) == Some("1"))
    );
    assert!(
        abstract_node
            .descendants()
            .filter(|n| n.has_tag_name((w, "numFmt")))
            .all(|n| n.attribute((w, "val")) == Some("decimal"))
    );
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn replacements_must_target_unique_defined_levels() {
    for targets in [vec![1], vec![9], vec![0, 0]] {
        let definition = NumberingDefinition {
            id: 42,
            levels: vec![NumberingLevel::default()],
            level_overrides: targets
                .into_iter()
                .map(|level| NumberingLevel {
                    level,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let error = definition.validate().unwrap_err();
        assert_eq!(error.rule, Rule::InvalidContent);
        assert!(error.to_string().contains("level_overrides"));
    }
}

#[test]
fn empty_replacements_do_not_change_start_only_serialization() {
    let mut document = Document::new(vec![]);
    document.numbering = vec![NumberingDefinition {
        abstract_id: None,
        inherit_style: None,
        id: 42,
        levels: vec![NumberingLevel {
            label_format: None,
            paragraph_style: None,
            paragraph_format: None,
            level: 0,
            start: 1,
            format: "decimal".into(),
            text: "%1.".into(),
            is_legal: None,
            suffix: None,
            restart: None,
            alignment: None,
        }],
        start_overrides: vec![NumberingStartOverride { level: 0, start: 7 }],
        level_overrides: Vec::new(),
    }]
    .into();
    let bytes = block_on(document.resolve()).unwrap().to_docx().unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("word/numbering.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let num = tree
        .descendants()
        .find(|n| n.has_tag_name((w, "num")) && n.attribute((w, "numId")) == Some("42"))
        .unwrap();
    assert!(!num.descendants().any(|n| n.has_tag_name((w, "lvl"))));
}
