#[path = "support/section_gutter.rs"]
mod fixture;

use futures::executor::block_on;
use rv_docx_rs::*;
use std::io::{Cursor, Read};

#[test]
fn section_gutter_inherits_until_an_explicit_false_without_changing_bidi() {
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
    let sections: Vec<_> = tree
        .descendants()
        .filter(|n| n.has_tag_name((w, "sectPr")))
        .collect();
    let flags: Vec<_> = sections
        .iter()
        .map(|n| {
            n.children()
                .find(|n| n.has_tag_name((w, "rtlGutter")))
                .unwrap()
                .attribute((w, "val"))
        })
        .collect();
    assert_eq!(flags, [Some("1"), Some("1"), Some("0")]);
    assert!(sections.iter().all(|n| {
        n.children()
            .find(|n| n.has_tag_name((w, "bidi")))
            .unwrap()
            .attribute((w, "val"))
            == Some("0")
    }));
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn omitted_gutter_request_does_not_author_an_override() {
    let bytes = block_on(Document::new(vec![Section::new(vec![]).into()]).resolve())
        .unwrap()
        .to_docx()
        .unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    assert!(tree.descendants().all(|n| !n.has_tag_name((
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
        "rtlGutter"
    ))));
}
