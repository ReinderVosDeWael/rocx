use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read};

#[test]
fn deferred_legal_numbering_preserves_three_states_in_abstract_and_concrete_levels() {
    let mut document = Document::new(vec![Section::new(vec![]).into()]);
    document.numbering = Value::future(async {
        vec![NumberingDefinition {
            id: 80,
            levels: [None, Some(false), Some(true)]
                .into_iter()
                .enumerate()
                .map(|(i, is_legal)| NumberingLevel {
                    label_format: None,
                    paragraph_style: None,
                    paragraph_format: None,
                    level: i as u8,
                    start: 1,
                    format: "upperRoman".into(),
                    text: "%1.%2.%3".into(),
                    is_legal,
                    suffix: None,
                    restart: None,
                    alignment: None,
                })
                .collect(),
            level_overrides: vec![NumberingLevel {
                label_format: None,
                paragraph_style: None,
                paragraph_format: None,
                level: 2,
                start: 1,
                format: "lowerLetter".into(),
                text: "%3".into(),
                is_legal: Some(false),
                suffix: None,
                restart: None,
                alignment: None,
            }],
            ..Default::default()
        }]
    });
    let snapshot = block_on(document.resolve()).unwrap();
    let bytes = snapshot.to_docx().unwrap();
    assert_eq!(bytes, snapshot.to_docx().unwrap());
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("word/numbering.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let levels: Vec<_> = tree
        .descendants()
        .filter(|n| n.has_tag_name((w, "lvl")))
        .collect();
    let levels = &levels[levels.len() - 4..];
    let states: Vec<_> = levels
        .iter()
        .map(|n| {
            n.children()
                .find(|n| n.has_tag_name((w, "isLgl")))
                .and_then(|n| n.attribute((w, "val")))
        })
        .collect();
    assert_eq!(states, [None, Some("0"), Some("1"), Some("0")]);
    for level in levels {
        let names: Vec<_> = level
            .children()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name())
            .collect();
        assert!(
            names == ["start", "numFmt", "lvlText"]
                || names == ["start", "numFmt", "isLgl", "lvlText"]
        );
    }
}
