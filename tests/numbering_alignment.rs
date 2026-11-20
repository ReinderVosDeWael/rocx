#[path = "support/numbering_alignment.rs"]
mod fixture;

use futures::executor::block_on;
use std::io::{Cursor, Read};

#[test]
fn label_alignment_preserves_omission_all_tokens_and_level_property_order() {
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
    let num = tree
        .descendants()
        .find(|n| n.has_tag_name((w, "num")) && n.attribute((w, "numId")) == Some("80"))
        .unwrap();
    let identity = num
        .children()
        .find(|n| n.has_tag_name((w, "abstractNumId")))
        .unwrap()
        .attribute((w, "val"));
    let abstract_node = tree
        .descendants()
        .find(|n| {
            n.has_tag_name((w, "abstractNum")) && n.attribute((w, "abstractNumId")) == identity
        })
        .unwrap();
    let levels: Vec<_> = abstract_node
        .descendants()
        .chain(num.descendants())
        .filter(|n| n.has_tag_name((w, "lvl")))
        .collect();
    let requests: Vec<_> = levels
        .iter()
        .map(|level| {
            level
                .children()
                .find(|n| n.has_tag_name((w, "lvlJc")))
                .and_then(|n| n.attribute((w, "val")))
        })
        .collect();
    assert_eq!(
        requests,
        [
            None,
            Some("left"),
            Some("center"),
            Some("right"),
            Some("start"),
            Some("end"),
            Some("end"),
            Some("start"),
            Some("right"),
            Some("center"),
            Some("left"),
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
            names == ["start", "numFmt", "lvlRestart", "isLgl", "suff", "lvlText"]
                || names
                    == [
                        "start",
                        "numFmt",
                        "lvlRestart",
                        "isLgl",
                        "suff",
                        "lvlText",
                        "lvlJc"
                    ]
        );
    }
}
