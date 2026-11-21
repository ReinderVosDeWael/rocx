#[path = "support/tab_stops.rs"]
mod fixture;

use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read};

#[test]
fn custom_tabs_sort_signed_point_positions_and_keep_tab_characters_separate() {
    let snapshot = block_on(fixture::document().resolve()).unwrap();
    assert_eq!(
        snapshot.sections()[0].paragraphs()[0].text(),
        "label\tvalue"
    );
    let bytes = snapshot.to_docx().unwrap();
    let mut package = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut xml = String::new();
    package
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let tabs: Vec<_> = tree
        .descendants()
        .filter(|n| n.has_tag_name((w, "tabs")))
        .collect();
    let stops: Vec<_> = tabs[0].children().filter(|n| n.is_element()).collect();
    assert_eq!(
        stops
            .iter()
            .map(|n| n.attribute((w, "pos")).unwrap())
            .collect::<Vec<_>>(),
        ["-240", "1440", "2160", "2880", "3600"]
    );
    assert_eq!(stops[2].attribute((w, "leader")), Some("dot"));
    assert_eq!(tabs.len(), 2);
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn custom_tab_validation_checks_wire_position_collisions_and_measurement_errors() {
    for value in [f64::NAN, f64::INFINITY, f64::MAX] {
        let error = ParagraphFormat {
            tab_stops: vec![TabStop {
                position: value,
                ..Default::default()
            }],
            ..Default::default()
        }
        .validate()
        .unwrap_err();
        assert_eq!(error.rule, Rule::InvalidMeasurement);
        assert_eq!(error.location.as_deref(), Some("tab_stops[0].position"));
    }
    let error = ParagraphFormat {
        tab_stops: vec![
            TabStop {
                position: 72.,
                ..Default::default()
            },
            TabStop {
                position: 72.01,
                ..Default::default()
            },
        ],
        ..Default::default()
    }
    .validate()
    .unwrap_err();
    assert_eq!(error.rule, Rule::InvalidContent);
    assert_eq!(error.location.as_deref(), Some("tab_stops[1].position"));
    assert!(
        ParagraphFormat {
            tab_stops: vec![TabStop {
                position: -12.,
                ..Default::default()
            }],
            ..Default::default()
        }
        .validate()
        .is_ok()
    );
}
