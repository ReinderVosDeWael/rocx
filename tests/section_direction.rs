#[path = "support/section_direction.rs"]
mod fixture;

use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read};

#[test]
fn section_direction_inherits_until_an_explicit_false_override() {
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
    let directions: Vec<_> = tree
        .descendants()
        .filter(|n| n.has_tag_name((w, "sectPr")))
        .map(|n| {
            n.children()
                .find(|n| n.has_tag_name((w, "bidi")))
                .unwrap()
                .attribute((w, "val"))
        })
        .collect();
    assert_eq!(directions, [Some("1"), Some("1"), Some("0")]);
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn absent_section_direction_does_not_materialize_an_override() {
    let document = Document::new(vec![
        Section::new(vec![Paragraph::new("plain").into()]).into(),
    ]);
    let bytes = block_on(document.resolve()).unwrap().to_docx().unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    assert!(tree.descendants().all(|n| !n.has_tag_name((
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
        "bidi"
    ))));
}
