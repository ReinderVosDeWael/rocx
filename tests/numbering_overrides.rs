#[path = "support/numbering_overrides.rs"]
mod fixture;

use futures::executor::block_on;
use rv_docx_rs::*;
use std::io::{Cursor, Read};

#[test]
fn concrete_numbering_start_override_does_not_rewrite_abstract_start() {
    let bytes = block_on(fixture::document().resolve())
        .unwrap()
        .to_docx()
        .unwrap();
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
    assert_eq!(
        num.descendants()
            .find(|n| n.has_tag_name((w, "startOverride")))
            .unwrap()
            .attribute((w, "val")),
        Some("7")
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
    assert_eq!(
        abstract_node
            .descendants()
            .find(|n| n.has_tag_name((w, "start")))
            .unwrap()
            .attribute((w, "val")),
        Some("1")
    );
}

#[test]
fn numbering_start_override_requires_a_unique_defined_level() {
    let mut definition = NumberingDefinition {
        id: 42,
        levels: vec![NumberingLevel::default()],
        start_overrides: vec![NumberingStartOverride { level: 1, start: 7 }],
        level_overrides: Vec::new(),
    };
    assert_eq!(
        definition.validate().unwrap_err().rule,
        Rule::InvalidContent
    );
    definition.start_overrides[0].level = 0;
    assert!(definition.validate().is_ok());
    definition
        .start_overrides
        .push(NumberingStartOverride { level: 0, start: 9 });
    assert_eq!(
        definition.validate().unwrap_err().rule,
        Rule::InvalidContent
    );
}
