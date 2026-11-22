use futures::executor::block_on;
use rocx::*;
use std::io::{Cursor, Read};

fn render(properties: ShapeProperties) -> Result<Vec<u8>> {
    let document = Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![
                ShapeRun::new(Value::future(async move { properties })).into(),
            ])
            .into(),
        ])
        .into(),
    ])
    .with_timestamp(Some("2026-11-21T00:00:00Z".into()));
    block_on(document.resolve())?.to_docx()
}

#[test]
fn drawingml_text_style_applies_to_runs_and_empty_paragraphs_in_schema_order() {
    let format = ShapeTextFormat::default()
        .with_font_size(14.25)
        .with_bold(false)
        .with_italic(false)
        .with_font("A & B")
        .with_color([16, 48, 32])
        .with_alignment(ShapeTextAlignment::Center)
        .with_vertical_alignment(ShapeTextVerticalAlignment::Bottom);
    let bytes = render(
        ShapeProperties::default()
            .with_text("First\n\n Last ")
            .with_text_format(format),
    )
    .unwrap();
    let mut package = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    package
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let a = "http://schemas.openxmlformats.org/drawingml/2006/main";
    let body = tree
        .descendants()
        .find(|n| n.has_tag_name((a, "txBody")))
        .unwrap();
    assert_eq!(
        body.first_element_child().unwrap().attribute("anchor"),
        Some("b")
    );
    let paragraphs: Vec<_> = body
        .children()
        .filter(|n| n.has_tag_name((a, "p")))
        .collect();
    assert_eq!(paragraphs.len(), 3);
    for (index, paragraph) in paragraphs.iter().enumerate() {
        let children: Vec<_> = paragraph.children().filter(|n| n.is_element()).collect();
        assert_eq!(children[0].tag_name().name(), "pPr");
        assert_eq!(children[0].attribute("algn"), Some("ctr"));
        assert_eq!(children.last().unwrap().tag_name().name(), "endParaRPr");
        assert_eq!(children.len(), if index == 1 { 2 } else { 3 });
    }
    let properties: Vec<_> = body
        .descendants()
        .filter(|n| n.has_tag_name((a, "rPr")) || n.has_tag_name((a, "endParaRPr")))
        .collect();
    assert_eq!(properties.len(), 5);
    for properties in properties {
        assert_eq!(properties.attribute("sz"), Some("1425"));
        assert_eq!(properties.attribute("b"), Some("0"));
        assert_eq!(properties.attribute("i"), Some("0"));
        let children: Vec<_> = properties.children().filter(|n| n.is_element()).collect();
        assert_eq!(
            children
                .iter()
                .map(|n| n.tag_name().name())
                .collect::<Vec<_>>(),
            ["solidFill", "latin"]
        );
        assert_eq!(children[1].attribute("typeface"), Some("A & B"));
        let color = properties
            .descendants()
            .find(|n| n.has_tag_name((a, "srgbClr")))
            .unwrap();
        assert_eq!(color.attribute("val"), Some("103020"));
    }
}

#[test]
fn empty_format_preserves_bytes_and_font_size_validation_uses_raw_bounds() {
    let plain = ShapeProperties::default().with_text("plain");
    assert_eq!(
        render(plain.clone()).unwrap(),
        render(plain.with_text_format(ShapeTextFormat::default())).unwrap()
    );
    for size in [0., 0.999, 4000.001, f64::NAN, f64::INFINITY] {
        assert!(
            render(
                ShapeProperties::default()
                    .with_text_format(ShapeTextFormat::default().with_font_size(size))
            )
            .is_err()
        );
    }
    for size in [1., 4000.] {
        assert!(
            render(
                ShapeProperties::default()
                    .with_text("size")
                    .with_text_format(ShapeTextFormat::default().with_font_size(size))
            )
            .is_ok()
        );
    }
}

#[test]
fn standalone_text_box_has_a_direct_text_shape_and_no_geometric_properties() {
    let bytes = render(
        ShapeProperties::default()
            .with_geometry(ShapeGeometry::TextBox)
            .with_width(144.)
            .with_height(48.)
            .with_text("Standalone\nText box")
            .with_text_format(ShapeTextFormat::default().with_bold(false)),
    )
    .unwrap();
    let mut package = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    package
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let a = "http://schemas.openxmlformats.org/drawingml/2006/main";
    assert!(
        !tree
            .descendants()
            .any(|n| n.has_tag_name((a, "sp")) || n.has_tag_name((a, "spPr")))
    );
    let text_shape = tree
        .descendants()
        .find(|n| n.has_tag_name((a, "txSp")))
        .unwrap();
    let children: Vec<_> = text_shape.children().filter(|n| n.is_element()).collect();
    assert_eq!(
        children
            .iter()
            .map(|n| n.tag_name().name())
            .collect::<Vec<_>>(),
        ["txBody", "xfrm"]
    );
    let extent = children[1]
        .children()
        .find(|n| n.has_tag_name((a, "ext")))
        .unwrap();
    assert_eq!(extent.attribute("cx"), Some("1828800"));
    assert_eq!(extent.attribute("cy"), Some("609600"));
    assert!(
        render(
            ShapeProperties::default()
                .with_geometry(ShapeGeometry::TextBox)
                .with_fill([0; 3])
        )
        .is_err()
    );
    assert!(
        render(
            ShapeProperties::default()
                .with_geometry(ShapeGeometry::TextBox)
                .with_outline(ShapeOutline {
                    width: 1.,
                    color: [0; 3]
                })
        )
        .is_err()
    );
}
