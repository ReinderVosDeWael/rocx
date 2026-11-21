use rocx::*;

fn main() -> Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("image-dependencies.docx".into());
    let transformation = Property::new(
        "image.transformation",
        Value::future(async {
            Some(ImageTransformation {
                width: Some(36.),
                height: None,
                ..Default::default()
            })
        }),
    );
    let alt = Property::new(
        "image.alt_text",
        Value::future(async {
            Some(AltText {
                title: Some("Shared image options λ".into()),
                ..Default::default()
            })
        }),
    );
    let mut image = ImageRun::new(include_bytes!("../tests/fixtures/images/sample.png").to_vec());
    image.transformation = transformation.reference().into();
    image.alt_text = alt.reference().into();
    let snapshot = futures::executor::block_on(
        Document::new(vec![
            Section::new(vec![Paragraph::from_children(vec![image.into()]).into()]).into(),
        ])
        .resolve(),
    )?;
    snapshot.save(output)
}
