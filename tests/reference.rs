mod support;

use futures::executor::block_on;
use rocx::{Document, Paragraph, Section, Value};
use support::{body, part};

fn section(text: &[&str]) -> Value<Section> {
    Section::new(
        text.iter()
            .map(|text| Paragraph::new(*text).into())
            .collect(),
    )
    .into()
}

#[test]
fn paragraph_content_and_section_structure_match_pinned_cmi_fixtures() {
    let cases = [
        (Document::default(), include_str!("fixtures/empty.xml")),
        (
            Document::new(vec![Section::default().into()]),
            include_str!("fixtures/empty_section.xml"),
        ),
        (
            Document::new(vec![section(&[
                "Hello, world!",
                "",
                "café Ελληνικά 日本語 🌍",
            ])]),
            include_str!("fixtures/plain.xml"),
        ),
        (
            Document::new(vec![section(&[
                "  Leading  and trailing  ",
                "<tag> & \"quotes\" 'apostrophes' ]]> ",
                "\tfirst\t\tlast\t",
                "a\nb\rc\r\nd\n",
            ])]),
            include_str!("fixtures/whitespace.xml"),
        ),
        (
            Document::new(vec![
                section(&["one"]),
                section(&[]),
                section(&["two", "three"]),
            ]),
            include_str!("fixtures/sections.xml"),
        ),
        (
            Document::new(vec![
                Section::default()
                    .with_children(Value::lazy(|| panic!("Hidden children")))
                    .when(false)
                    .into(),
                Section::new(vec![
                    Paragraph::new("visible").into(),
                    Paragraph::new(Value::lazy(|| panic!("Hidden text")))
                        .when(false)
                        .into(),
                    Paragraph::new("also visible").into(),
                ])
                .into(),
            ]),
            include_str!("fixtures/conditional.xml"),
        ),
        (
            Document::new(vec![
                Section::default()
                    .with_children(Value::lazy(|| {
                        vec![
                            Paragraph::new(Value::future(async { "Deferred text".into() })).into(),
                            Paragraph::new(Value::lazy(|| "Lazy text".into())).into(),
                        ]
                    }))
                    .into(),
            ]),
            include_str!("fixtures/deferred.xml"),
        ),
    ];
    for (index, (declaration, reference)) in cases.into_iter().enumerate() {
        let bytes = block_on(declaration.resolve()).unwrap().to_docx().unwrap();
        assert_eq!(
            body(&part(&bytes, "word/document.xml")),
            body(reference),
            "fixture {index}"
        );
    }
}
