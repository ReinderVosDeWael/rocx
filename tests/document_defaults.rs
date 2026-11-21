#[path = "support/document_defaults.rs"]
mod fixture;

use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read};

#[test]
fn defaults_resolve_as_an_immutable_aggregate_without_changing_direct_properties() {
    let snapshot = block_on(fixture::document().resolve()).unwrap();
    assert_eq!(
        snapshot.defaults().unwrap().run.as_ref().unwrap().bold,
        Some(true)
    );
    let paragraph = snapshot.sections()[0].paragraphs()[0];
    assert_eq!(paragraph.format().left_indent, None);
    let bytes = snapshot.to_docx().unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("word/styles.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let defaults = tree
        .descendants()
        .find(|n| n.has_tag_name((w, "docDefaults")))
        .unwrap();
    for (element, attr, expected) in [
        ("sz", "val", "27"),
        ("lang", "bidi", "ar-SA"),
        ("spacing", "before", "0"),
        ("spacing", "line", "280"),
        ("ind", "left", "240"),
        ("ind", "hanging", "120"),
    ] {
        let node = defaults
            .descendants()
            .find(|n| n.has_tag_name((w, element)))
            .unwrap();
        assert_eq!(node.attribute((w, attr)), Some(expected));
    }
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn defaults_reject_style_references_and_heading_selection() {
    for defaults in [
        DocumentDefaults {
            run: Some(RunFormat {
                style: Some("Character".into()),
                ..Default::default()
            }),
            ..Default::default()
        },
        DocumentDefaults {
            paragraph: Some(ParagraphFormat {
                style: Some("Normal".into()),
                ..Default::default()
            }),
            ..Default::default()
        },
        DocumentDefaults {
            paragraph: Some(ParagraphFormat {
                heading: Some(0),
                ..Default::default()
            }),
            ..Default::default()
        },
    ] {
        let document = Document {
            defaults: Some(defaults).into(),
            ..Default::default()
        };
        assert!(
            block_on(document.resolve())
                .unwrap_err()
                .to_string()
                .contains("defaults")
        );
    }
}

#[test]
fn defaults_reject_invalid_measurements_during_resolution() {
    let document = Document {
        defaults: Some(DocumentDefaults {
            run: Some(RunFormat {
                size: Some(f64::NAN),
                ..Default::default()
            }),
            ..Default::default()
        })
        .into(),
        ..Default::default()
    };
    assert!(block_on(document.resolve()).is_err());
}

#[test]
fn named_default_properties_share_values_without_cloning_authoring_futures() {
    let source = Property::new(
        "defaults",
        Value::future(async {
            Some(DocumentDefaults {
                run: Some(RunFormat {
                    bold: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            })
        }),
    );
    let document = Document {
        defaults: source.reference().into(),
        ..Default::default()
    };
    let snapshot = block_on(document.resolve()).unwrap();
    assert_eq!(
        snapshot.defaults().unwrap().run.as_ref().unwrap().bold,
        Some(false)
    );
}
