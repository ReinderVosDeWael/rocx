#[path = "support/numbering_suffix.rs"]
mod fixture;

use futures::executor::block_on;
use std::io::{Cursor, Read};

#[test]
fn suffix_requests_preserve_omission_and_all_tokens_in_both_level_locations() {
    let snapshot = block_on(fixture::document().resolve()).unwrap();
    let bytes = snapshot.to_docx().unwrap();
    assert_eq!(bytes, snapshot.to_docx().unwrap());
    let mut package = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut xml = String::new();
    package
        .by_name("word/numbering.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let levels: Vec<_> = tree
        .descendants()
        .filter(|n| n.has_tag_name((w, "lvl")))
        .collect();
    let levels = &levels[levels.len() - 8..];
    let tokens: Vec<_> = levels
        .iter()
        .map(|level| {
            level
                .children()
                .find(|n| n.has_tag_name((w, "suff")))
                .and_then(|n| n.attribute((w, "val")))
        })
        .collect();
    assert_eq!(
        tokens,
        [
            None,
            Some("tab"),
            Some("space"),
            Some("nothing"),
            Some("nothing"),
            Some("space"),
            Some("tab"),
            None
        ]
    );
    for level in levels {
        let names: Vec<_> = level
            .children()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name())
            .collect();
        assert!(
            names == ["start", "numFmt", "isLgl", "lvlText"]
                || names == ["start", "numFmt", "isLgl", "suff", "lvlText"]
        );
    }
}
