#[path = "support/script_formatting.rs"]
mod fixture;

use futures::executor::block_on;
use rv_docx_rs::*;
use std::io::{Cursor, Read};

#[test]
fn complex_script_properties_and_languages_are_independent_of_latin_formatting() {
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
    for (name, expected) in [
        ("b", "0"),
        ("bCs", "1"),
        ("i", "1"),
        ("iCs", "0"),
        ("sz", "22"),
        ("szCs", "29"),
        ("rtl", "1"),
        ("cs", "0"),
    ] {
        assert_eq!(
            tree.descendants()
                .find(|n| n.has_tag_name((w, name)))
                .unwrap()
                .attribute((w, "val")),
            Some(expected)
        );
    }
    let lang = tree
        .descendants()
        .find(|n| n.has_tag_name((w, "lang")))
        .unwrap();
    assert_eq!(lang.attribute((w, "val")), Some("en-GB"));
    assert_eq!(lang.attribute((w, "eastAsia")), Some("ja-JP"));
    assert_eq!(lang.attribute((w, "bidi")), Some("ar-SA"));
    let fonts = tree
        .descendants()
        .find(|n| n.has_tag_name((w, "rFonts")))
        .unwrap();
    assert_eq!(fonts.attribute((w, "ascii")), Some("Latin"));
    assert_eq!(fonts.attribute((w, "hAnsi")), Some("Latin"));
    assert_eq!(fonts.attribute((w, "eastAsia")), Some("East Asian"));
    assert_eq!(fonts.attribute((w, "cs")), Some("Arabic"));
    assert_eq!(
        tree.descendants()
            .find(|n| n.has_tag_name((w, "bidi")))
            .unwrap()
            .attribute((w, "val")),
        Some("1")
    );
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn complex_script_font_sizes_use_checked_half_point_conversion() {
    for value in [-1., f64::NAN, f64::INFINITY, f64::MAX] {
        let error = RunFormat {
            size_complex_script: Some(value),
            ..Default::default()
        }
        .validate()
        .unwrap_err();
        assert_eq!(error.rule, Rule::InvalidMeasurement);
        assert_eq!(error.location.as_deref(), Some("size_complex_script"));
    }
}
