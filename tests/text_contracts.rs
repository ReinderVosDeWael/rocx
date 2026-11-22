mod support;

use futures::executor::block_on;
use rocx::{Document, Paragraph, Section, Value};
use support::{Body, body, part};

fn section(text: &[&str]) -> Value<Section> {
    Section::new(
        text.iter()
            .map(|text| Paragraph::new(*text).into())
            .collect(),
    )
    .into()
}

#[test]
fn paragraph_content_and_section_structure_follow_the_native_contract() {
    let cases = [
        (Document::default(), vec![], 1),
        (Document::new(vec![Section::default().into()]), vec![], 1),
        (
            Document::new(vec![section(&[
                "Hello, world!",
                "",
                "café Ελληνικά 日本語 🌍",
            ])]),
            vec![
                ("Hello, world!".into(), false),
                ("".into(), false),
                ("café Ελληνικά 日本語 🌍".into(), false),
            ],
            1,
        ),
        (
            Document::new(vec![section(&[
                "  Leading  and trailing  ",
                "<tag> & \"quotes\" 'apostrophes' ]]> ",
                "\tfirst\t\tlast\t",
                "a\nb\rc\r\nd\n",
            ])]),
            vec![
                ("  Leading  and trailing  ".into(), false),
                ("<tag> & \"quotes\" 'apostrophes' ]]> ".into(), false),
                ("\tfirst\t\tlast\t".into(), false),
                ("a\nb\nc\n\nd\n".into(), false),
            ],
            1,
        ),
        (
            Document::new(vec![
                section(&["one"]),
                section(&[]),
                section(&["two", "three"]),
            ]),
            vec![
                ("one".into(), false),
                ("".into(), true),
                ("".into(), true),
                ("two".into(), false),
                ("three".into(), false),
            ],
            3,
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
            vec![("visible".into(), false), ("also visible".into(), false)],
            1,
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
            vec![("Deferred text".into(), false), ("Lazy text".into(), false)],
            1,
        ),
    ];
    for (index, (declaration, paragraphs, section_count)) in cases.into_iter().enumerate() {
        let bytes = block_on(declaration.resolve()).unwrap().to_docx().unwrap();
        assert_eq!(
            body(&part(&bytes, "word/document.xml")),
            Body {
                paragraphs,
                section_count,
            },
            "fixture {index}"
        );
    }
}
