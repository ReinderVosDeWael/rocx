#[path = "support/table_alignment.rs"]
mod fixture;

use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read};

#[test]
fn table_alignment_preserves_source_omission_and_all_tokens_through_deferred_properties() {
    let snapshot = block_on(fixture::document().resolve()).unwrap();
    let expected = [
        None,
        Some(TableAlignment::Left),
        Some(TableAlignment::Center),
        Some(TableAlignment::Right),
        Some(TableAlignment::Start),
        Some(TableAlignment::End),
    ];
    for (block, alignment) in snapshot.sections()[0].blocks().iter().zip(expected) {
        let ResolvedBlock::Table(table) = block else {
            panic!("expected table")
        };
        assert_eq!(table.alignment(), alignment);
        assert_eq!(table.column_widths(), Some([1440].as_slice()));
    }
    let bytes = snapshot.to_docx().unwrap();
    assert_eq!(bytes, snapshot.to_docx().unwrap());
    let mut package = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut xml = String::new();
    package
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let properties: Vec<_> = tree
        .descendants()
        .filter(|n| n.has_tag_name((w, "tblPr")))
        .collect();
    let requests: Vec<_> = properties
        .iter()
        .map(|props| {
            props
                .children()
                .find(|n| n.has_tag_name((w, "jc")))
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
            Some("end")
        ]
    );
    for props in &properties[1..] {
        let names: Vec<_> = props
            .children()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name())
            .collect();
        assert_eq!(names, ["tblW", "jc", "tblLayout", "tblLook"]);
    }
}
