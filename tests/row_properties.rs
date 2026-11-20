#[path = "support/row_properties.rs"]
mod fixture;

use futures::executor::block_on;
use rv_docx_rs::*;
use std::io::{Cursor, Read};

#[test]
fn row_height_and_pagination_flags_preserve_units_and_absent_false_true() {
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
    let rows: Vec<_> = tree
        .descendants()
        .filter(|n| n.has_tag_name((w, "tr")))
        .collect();
    for (row, (height, rule, flag)) in rows
        .iter()
        .zip([("285", "exact", "1"), ("360", "atLeast", "0")])
    {
        let props = row
            .children()
            .find(|n| n.has_tag_name((w, "trPr")))
            .unwrap();
        let height_node = props
            .children()
            .find(|n| n.has_tag_name((w, "trHeight")))
            .unwrap();
        assert_eq!(height_node.attribute((w, "val")), Some(height));
        assert_eq!(height_node.attribute((w, "hRule")), Some(rule));
        for name in ["tblHeader", "cantSplit"] {
            assert_eq!(
                props
                    .children()
                    .find(|n| n.has_tag_name((w, name)))
                    .unwrap()
                    .attribute((w, "val")),
                Some(flag)
            );
        }
    }
    assert!(rows[2].children().all(|n| !n.has_tag_name((w, "trPr"))));
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn row_height_measurements_fail_with_property_locations() {
    for constructor in [RowHeight::Exact, RowHeight::AtLeast] {
        for value in [-1., f64::NAN, f64::INFINITY, f64::MAX] {
            let properties = TableRowProperties {
                height: Some(constructor(value)),
                ..Default::default()
            };
            let error = properties.validate().unwrap_err();
            assert_eq!(error.rule, Rule::InvalidMeasurement);
            assert_eq!(error.location.as_deref(), Some("height"));
            assert!(block_on(TableRow::new(vec![]).with_properties(properties).resolve()).is_err());
        }
        assert!(
            TableRowProperties {
                height: Some(constructor(0.)),
                ..Default::default()
            }
            .validate()
            .is_ok()
        );
    }
}
