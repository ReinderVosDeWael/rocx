#[path = "support/conditional_paragraph_formatting.rs"]
mod fixture;

use futures::executor::block_on;
use std::io::{Cursor, Read};

#[test]
fn conditional_paragraph_properties_use_the_shared_checked_codec() {
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
    let style = tree
        .descendants()
        .find(|n| n.attribute((w, "styleId")) == Some("Conditional"))
        .unwrap();
    let region = style
        .children()
        .find(|n| n.attribute((w, "type")) == Some("firstRow"))
        .unwrap();
    let spacing = region
        .descendants()
        .find(|n| n.has_tag_name((w, "spacing")))
        .unwrap();
    assert_eq!(spacing.attribute((w, "line")), Some("290"));
    assert_eq!(spacing.attribute((w, "lineRule")), Some("exact"));
    let tab = region
        .descendants()
        .find(|n| n.has_tag_name((w, "tab")))
        .unwrap();
    assert_eq!(tab.attribute((w, "pos")), Some("1440"));
    assert_eq!(tab.attribute((w, "val")), Some("right"));
    assert_eq!(
        region
            .descendants()
            .find(|n| n.has_tag_name((w, "bidi")))
            .unwrap()
            .attribute((w, "val")),
        Some("0")
    );
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}
