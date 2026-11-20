#[path = "support/cell_layout.rs"]
mod fixture;

use futures::executor::block_on;
use rv_docx_rs::*;
use std::io::{Cursor, Read};

#[test]
fn cell_layout_uses_checked_twips_and_immutable_deferred_properties() {
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
    let margins = tree
        .descendants()
        .find(|n| n.has_tag_name((w, "tcMar")))
        .unwrap();
    assert_eq!(
        margins
            .children()
            .filter(|n| n.is_element())
            .map(|n| (
                n.tag_name().name(),
                n.attribute((w, "w")).unwrap(),
                n.attribute((w, "type")).unwrap()
            ))
            .collect::<Vec<_>>(),
        [
            ("top", "63", "dxa"),
            ("start", "0", "dxa"),
            ("bottom", "120", "dxa"),
            ("end", "180", "dxa")
        ]
    );
    assert_eq!(
        tree.descendants()
            .find(|n| n.has_tag_name((w, "textDirection")))
            .unwrap()
            .attribute((w, "val")),
        Some("tbRl")
    );
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn invalid_cell_margins_fail_in_the_pure_domain() {
    for value in [-1., f64::INFINITY, f64::NAN, f64::MAX] {
        let error = TableCellProperties {
            margins: Some(CellMargins {
                start: Some(value),
                ..Default::default()
            }),
            ..Default::default()
        }
        .validate()
        .unwrap_err();
        assert_eq!(error.rule, Rule::InvalidMeasurement);
        assert_eq!(error.location.as_deref(), Some("margins.start"));
    }
}
