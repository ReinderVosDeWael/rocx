use rocx::*;
use std::path::Path;

pub fn write(out: &Path, png: &Path) -> Result<()> {
    let alt = AltText::default().with_title("Path probe");
    let transformation = ImageTransformation::default().with_width(15.);
    let run = ImageRun::new(ImageData::Path(png.to_owned()))
        .with_alt_text(Value::ready(Some(alt.clone())))
        .with_transformation(Value::ready(Some(transformation.clone())));
    let document = Document::new(vec![
        Section::new(vec![Paragraph::from_children(vec![run.into()]).into()]).into(),
    ])
    .with_timestamp(Some("2024-02-03T04:05:06Z".into()));
    let document = futures::executor::block_on(document.resolve())?;
    std::fs::write(out.join("image-path.docx"), document.to_docx()?)?;
    Ok(())
}
