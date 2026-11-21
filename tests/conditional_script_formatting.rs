#[path = "support/conditional_script_formatting.rs"]
mod fixture;

use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read};

#[test]
fn conditional_script_requests_use_the_shared_run_codec() {
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
        .find(|n| n.attribute((w, "styleId")) == Some("Scripts"))
        .unwrap();
    let region = style
        .children()
        .find(|n| n.attribute((w, "type")) == Some("firstRow"))
        .unwrap();
    for (element, expected) in [
        ("bCs", "0"),
        ("iCs", "1"),
        ("szCs", "27"),
        ("rtl", "0"),
        ("cs", "1"),
    ] {
        let node = region
            .descendants()
            .find(|n| n.has_tag_name((w, element)))
            .unwrap();
        assert_eq!(node.attribute((w, "val")), Some(expected));
    }
    let language = region
        .descendants()
        .find(|n| n.has_tag_name((w, "lang")))
        .unwrap();
    assert_eq!(language.attribute((w, "bidi")), Some("ar-SA"));
    let fonts = region
        .descendants()
        .find(|n| n.has_tag_name((w, "rFonts")))
        .unwrap();
    assert_eq!(fonts.attribute((w, "eastAsia")), Some("East"));
    assert_eq!(fonts.attribute((w, "cs")), Some("Complex"));
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn conditional_complex_script_size_rejects_invalid_measurements() {
    for size in [-1., f64::NAN, f64::INFINITY, 1e30] {
        let mut document = Document::new(vec![]);
        document.styles = vec![
            TableStyleDefinition {
                name: "Bad".into(),
                first_row: Some(TableSectionFormat {
                    size_complex_script: Some(size),
                    ..Default::default()
                }),
                ..Default::default()
            }
            .into(),
        ]
        .into();
        assert!(block_on(document.resolve()).unwrap().to_docx().is_err());
    }
}
