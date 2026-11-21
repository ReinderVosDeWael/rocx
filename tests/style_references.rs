use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read};

#[test]
fn forward_style_references_use_reserved_ids_and_allow_next_self_links() {
    let mut document = Document::new(vec![]);
    document.styles = vec![
        ParagraphStyleDefinition {
            name: "Child".into(),
            base_style: Some("Later Base".into()),
            next_paragraph_style: Some("Later Next".into()),
            ..Default::default()
        }
        .into(),
        ParagraphStyleDefinition {
            name: "Later Next".into(),
            next_paragraph_style: Some("Later Next".into()),
            ..Default::default()
        }
        .into(),
        ParagraphStyleDefinition {
            name: "Later Base".into(),
            ..Default::default()
        }
        .into(),
    ]
    .into();
    let bytes = block_on(document.resolve()).unwrap().to_docx().unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("word/styles.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let child = tree
        .descendants()
        .find(|n| n.attribute((w, "styleId")) == Some("Child"))
        .unwrap();
    assert_eq!(
        child
            .children()
            .find(|n| n.has_tag_name((w, "basedOn")))
            .unwrap()
            .attribute((w, "val")),
        Some("LaterBase")
    );
    assert_eq!(
        child
            .children()
            .find(|n| n.has_tag_name((w, "next")))
            .unwrap()
            .attribute((w, "val")),
        Some("LaterNext")
    );
}

#[test]
fn based_on_cycles_and_wrong_target_kinds_are_rejected() {
    for styles in [
        vec![
            ParagraphStyleDefinition {
                name: "A".into(),
                base_style: Some("B".into()),
                ..Default::default()
            }
            .into(),
            ParagraphStyleDefinition {
                name: "B".into(),
                base_style: Some("A".into()),
                ..Default::default()
            }
            .into(),
        ],
        vec![
            ParagraphStyleDefinition {
                name: "A".into(),
                base_style: Some("Normal Table".into()),
                ..Default::default()
            }
            .into(),
        ],
    ] {
        let mut document = Document::new(vec![]);
        document.styles = styles.into();
        assert!(block_on(document.resolve()).unwrap().to_docx().is_err());
    }
}
