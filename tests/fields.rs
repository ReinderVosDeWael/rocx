use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read};

fn part(bytes: &[u8]) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    xml
}

#[test]
fn simple_and_nested_complex_fields_preserve_instructions_flags_and_cached_text() {
    let mut outer = Field::new(
        FieldInstruction::raw("  IF 1 = 1 \"yes\" \"no\"  ").unwrap(),
        vec![
            TextRun::new("cached ").into(),
            Field::new(FieldInstruction::Page, vec![TextRun::new("7").into()]).into(),
        ],
    );
    outer.properties = FieldProperties {
        form: FieldForm::Complex,
        dirty: Some(true),
        locked: Some(false),
    }
    .into();
    let snapshot = block_on(
        Document::new(vec![
            Section::new(vec![Paragraph::from_children(vec![outer.into()]).into()]).into(),
        ])
        .resolve(),
    )
    .unwrap();
    assert_eq!(snapshot.sections()[0].paragraphs()[0].text(), "cached 7");
    let bytes = snapshot.to_docx().unwrap();
    let xml = part(&bytes);
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let instruction = tree
        .descendants()
        .find(|n| n.has_tag_name((w, "instrText")))
        .unwrap();
    assert_eq!(instruction.text(), Some("  IF 1 = 1 \"yes\" \"no\"  "));
    assert_eq!(
        instruction.attribute(("http://www.w3.org/XML/1998/namespace", "space")),
        Some("preserve")
    );
    let marks: Vec<_> = tree
        .descendants()
        .filter(|n| n.has_tag_name((w, "fldChar")))
        .collect();
    assert_eq!(
        marks
            .iter()
            .map(|n| n.attribute((w, "fldCharType")).unwrap())
            .collect::<Vec<_>>(),
        ["begin", "separate", "end"]
    );
    assert_eq!(marks[0].attribute((w, "dirty")), Some("1"));
    assert_eq!(marks[0].attribute((w, "fldLock")), Some("0"));
    assert!(
        tree.descendants().any(
            |n| n.has_tag_name((w, "fldSimple")) && n.attribute((w, "instr")) == Some(" PAGE ")
        )
    );
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}

#[test]
fn typed_references_require_unique_bookmarks_and_illegal_cached_ranges_fail() {
    let name = BookmarkName::new("target").unwrap();
    let reference = || {
        Field::new(
            FieldInstruction::Reference(name.clone()),
            vec![TextRun::new("cached").into()],
        )
    };
    let missing = block_on(
        Document::new(vec![
            Section::new(vec![
                Paragraph::from_children(vec![reference().into()]).into(),
            ])
            .into(),
        ])
        .resolve(),
    )
    .unwrap();
    assert!(missing.to_docx().is_err());
    let valid = Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![
                RangeStart::bookmark(name.clone()).into(),
                TextRun::new("Target").into(),
                RangeEnd::bookmark(&name).into(),
                reference().into(),
            ])
            .into(),
        ])
        .into(),
    ]);
    assert!(block_on(valid.resolve()).unwrap().to_docx().is_ok());
    let illegal = Field::new(
        FieldInstruction::Page,
        vec![RangeStart::bookmark(name).into()],
    );
    assert!(block_on(illegal.resolve()).is_err());
    assert!(FieldInstruction::raw(" \t ").is_err());
    assert!(FieldInstruction::raw("PAGE\0").is_err());
}
