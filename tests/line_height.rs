use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read};

fn xml(bytes: &[u8], path: &str) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    zip.by_name(path)
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    text
}

#[test]
fn typed_and_legacy_line_heights_encode_distinct_units_and_keep_spacing() {
    let formats = [
        ParagraphFormat::default(),
        ParagraphFormat {
            line_spacing: Some(1.5),
            ..Default::default()
        },
        ParagraphFormat {
            line_height: Some(LineHeight::Multiple(1.5)),
            ..Default::default()
        },
        ParagraphFormat {
            line_height: Some(LineHeight::Exact(14.25)),
            spacing_before: Some(3.),
            spacing_after: Some(4.),
            ..Default::default()
        },
        ParagraphFormat {
            line_height: Some(LineHeight::AtLeast(18.)),
            ..Default::default()
        },
    ];
    let document = Document::new(vec![
        Section::new(
            formats
                .into_iter()
                .map(|format| Paragraph::new("line height").with_format(format).into())
                .collect(),
        )
        .into(),
    ]);
    let bytes = block_on(document.resolve()).unwrap().to_docx().unwrap();
    let text = xml(&bytes, "word/document.xml");
    let tree = roxmltree::Document::parse(&text).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let spacing: Vec<_> = tree
        .descendants()
        .filter(|n| n.has_tag_name((w, "spacing")))
        .collect();
    assert_eq!(spacing.len(), 4);
    for (node, (line, rule)) in spacing.iter().zip([
        ("360", "auto"),
        ("360", "auto"),
        ("285", "exact"),
        ("360", "atLeast"),
    ]) {
        assert_eq!(node.attribute((w, "line")), Some(line));
        assert_eq!(node.attribute((w, "lineRule")), Some(rule));
    }
    assert_eq!(spacing[2].attribute((w, "before")), Some("60"));
    assert_eq!(spacing[2].attribute((w, "after")), Some("80"));
}

#[test]
fn line_height_validation_rejects_nonfinite_negative_overflow_and_conflicting_inputs() {
    for constructor in [LineHeight::Multiple, LineHeight::Exact, LineHeight::AtLeast] {
        for value in [-1., f64::NAN, f64::INFINITY, f64::MAX] {
            let format = ParagraphFormat {
                line_height: Some(constructor(value)),
                ..Default::default()
            };
            let error = format.validate().unwrap_err();
            assert_eq!(error.rule, Rule::InvalidMeasurement);
            assert_eq!(error.location.as_deref(), Some("line_height"));
        }
        assert!(
            ParagraphFormat {
                line_height: Some(constructor(0.)),
                ..Default::default()
            }
            .validate()
            .is_ok()
        );
    }
    let error = ParagraphFormat {
        line_spacing: Some(1.),
        line_height: Some(LineHeight::Exact(12.)),
        ..Default::default()
    }
    .validate()
    .unwrap_err();
    assert_eq!(error.rule, Rule::InvalidContent);
}

#[test]
fn paragraph_style_and_deferred_direct_format_share_the_line_height_codec() {
    let property = Property::new(
        "line.format",
        Value::future(async {
            ParagraphFormat {
                style: Some("Measured".into()),
                line_height: Some(LineHeight::Exact(12.)),
                ..Default::default()
            }
        }),
    );
    let mut document = Document::new(vec![
        Section::new(vec![
            Paragraph::new("override")
                .with_format(property.reference())
                .into(),
        ])
        .into(),
    ]);
    document.styles = vec![
        ParagraphStyleDefinition {
            name: "Measured".into(),
            line_height: Some(LineHeight::AtLeast(20.)),
            spacing_after: Some(5.),
            ..Default::default()
        }
        .into(),
    ]
    .into();
    let snapshot = block_on(document.resolve()).unwrap();
    let bytes = snapshot.to_docx().unwrap();
    let text = xml(&bytes, "word/styles.xml");
    let tree = roxmltree::Document::parse(&text).unwrap();
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let style = tree
        .descendants()
        .find(|n| n.attribute((w, "styleId")) == Some("Measured"))
        .unwrap();
    let spacing = style
        .descendants()
        .find(|n| n.has_tag_name((w, "spacing")))
        .unwrap();
    assert_eq!(spacing.attribute((w, "line")), Some("400"));
    assert_eq!(spacing.attribute((w, "lineRule")), Some("atLeast"));
    assert_eq!(spacing.attribute((w, "after")), Some("100"));
    assert_eq!(bytes, snapshot.to_docx().unwrap());
}
