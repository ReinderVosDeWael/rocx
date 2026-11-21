#[path = "support/character_styles.rs"]
mod fixture;

use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read};

#[test]
fn character_style_names_resolve_to_ids_without_materializing_inherited_formatting() {
    let snapshot = block_on(fixture::document().resolve()).unwrap();
    let bytes = snapshot.to_docx().unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    assert_eq!(
        tree.descendants()
            .find(|n| n.has_tag_name((w, "rStyle")))
            .unwrap()
            .attribute((w, "val")),
        Some("ChildCharacter")
    );
    assert!(tree.descendants().all(|n| !n.has_tag_name((w, "i"))));
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn character_style_formatting_uses_bases_not_nested_run_style_references() {
    let mut document = Document::new(vec![]);
    document.styles = vec![
        CharacterStyleDefinition {
            name: "Bad".into(),
            format: RunFormat {
                style: Some("Emphasis".into()),
                ..Default::default()
            },
            ..Default::default()
        }
        .into(),
    ]
    .into();
    assert!(block_on(document.resolve()).unwrap().to_docx().is_err());
}
