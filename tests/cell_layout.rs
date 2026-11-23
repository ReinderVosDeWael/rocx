#[path = "support/cell_layout.rs"]
mod fixture;

use futures::executor::block_on;
use rocx::*;
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

#[test]
fn nested_auto_tables_use_the_grid_width_of_their_authored_cell() {
    for (column_widths, layout, expected) in [
        (vec![300, 900, 1500], TableLayout::Fixed, "1200"),
        (vec![300, 900, 1500], TableLayout::Autofit, "2880"),
    ] {
        let nested = Table::new(vec![
            TableRow::new(vec![
                TableCell::new(vec![Paragraph::new("nested A").into()]).into(),
                TableCell::new(vec![Paragraph::new("nested B").into()]).into(),
            ])
            .into(),
        ]);
        let outer = Table::new(vec![
            TableRow::new(vec![
                TableCell::new(vec![Paragraph::new("first column").into()]).into(),
                TableCell::new(vec![nested.into()])
                    .with_properties(TableCellProperties::default().with_grid_span(2))
                    .into(),
            ])
            .into(),
        ])
        .with_properties(
            TableProperties::default()
                .with_column_widths(column_widths)
                .with_layout(layout),
        );
        let bytes =
            block_on(Document::new(vec![Section::new(vec![outer.into()]).into()]).resolve())
                .unwrap()
                .to_docx()
                .unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut xml = String::new();
        zip.by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        let tree = roxmltree::Document::parse(&xml).unwrap();
        let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
        let nested_table = tree
            .descendants()
            .filter(|n| n.has_tag_name((w, "tbl")))
            .nth(1)
            .unwrap();
        let widths: Vec<_> = nested_table
            .descendants()
            .filter(|n| n.has_tag_name((w, "gridCol")))
            .map(|n| n.attribute((w, "w")).unwrap())
            .collect();
        assert_eq!(widths, [expected, expected], "{layout:?}");
    }
}
