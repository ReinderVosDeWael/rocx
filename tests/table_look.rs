#[path = "support/table_look.rs"]
mod fixture;

use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read};

#[test]
fn table_look_resolves_as_a_deferred_property_without_materializing_omitted_fields() {
    let snapshot = block_on(fixture::document().resolve()).unwrap();
    let ResolvedBlock::Table(table) = &snapshot.sections()[0].blocks()[0] else {
        panic!("expected table")
    };
    assert_eq!(table.look().unwrap().first_row, None);
    assert_eq!(table.look().unwrap().last_row, Some(true));
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
    let node = tree
        .descendants()
        .find(|n| n.has_tag_name((w, "tblLook")))
        .unwrap();
    for (name, value) in [
        ("firstRow", "1"),
        ("lastRow", "1"),
        ("firstColumn", "0"),
        ("lastColumn", "1"),
        ("noHBand", "1"),
        ("noVBand", "0"),
        ("val", "0360"),
    ] {
        assert_eq!(node.attribute((w, name)), Some(value));
    }
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn empty_look_has_the_same_serialization_as_omitting_the_record() {
    let generate = |look| {
        let mut table = Table::new(vec![
            TableRow::new(vec![
                TableCell::new(vec![Paragraph::new("same").into()]).into(),
            ])
            .into(),
        ]);
        table.properties = TableProperties {
            look,
            ..Default::default()
        }
        .into();
        let mut document = Document::new(vec![Section::new(vec![table.into()]).into()]);
        document.timestamp = Some("2026-11-19T00:00:00Z".into()).into();
        block_on(document.resolve()).unwrap().to_docx().unwrap()
    };
    assert_eq!(generate(None), generate(Some(TableLook::default())));
}
