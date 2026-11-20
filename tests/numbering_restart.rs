#[path = "support/numbering_restart.rs"]
mod fixture;

use futures::executor::block_on;
use std::io::{Cursor, Read};

#[test]
fn restart_requests_preserve_omission_zero_one_based_and_ignored_values() {
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
    let levels = &levels[levels.len() - 10..];
    let requests: Vec<_> = levels
        .iter()
        .map(|level| {
            level
                .children()
                .find(|n| n.has_tag_name((w, "lvlRestart")))
                .and_then(|n| n.attribute((w, "val")))
        })
        .collect();
    assert_eq!(
        requests,
        [
            None,
            Some("0"),
            Some("1"),
            Some("2"),
            Some("4294967295"),
            Some("4294967295"),
            Some("2"),
            Some("1"),
            Some("0"),
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
            names == ["start", "numFmt", "isLgl", "suff", "lvlText"]
                || names == ["start", "numFmt", "lvlRestart", "isLgl", "suff", "lvlText"]
        );
    }
    assert_eq!(
        tree.descendants()
            .find(|n| n.has_tag_name((w, "startOverride")))
            .unwrap()
            .attribute((w, "val")),
        Some("7")
    );
}
