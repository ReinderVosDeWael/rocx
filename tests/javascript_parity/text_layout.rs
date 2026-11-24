use rocx::*;
use std::path::Path;

pub fn write(out: &Path) -> Result<()> {
    let run = TextRun::new("Deterministic\tlayout\n😀").with_format(RunFormat {
        bold: Some(false),
        color: Some(Color::Rgb([16, 32, 48])),
        ..Default::default()
    });
    let paragraph = Paragraph::from_children(vec![run.into()]).with_format(ParagraphFormat {
        alignment: Some(Alignment::Center),
        spacing_after: Some(6.),
        bidi: Some(false),
        ..Default::default()
    });
    let section = Section::new(vec![paragraph.into()]).with_properties(Some(SectionProperties {
        page_margins: Some(PageMargins::default().with_left(914400)),
        line_numbering: Some(
            LineNumbering::default()
                .with_count_by(2)
                .with_restart(LineNumberRestart::NewSection),
        ),
        ..Default::default()
    }));
    let document = Document::new(vec![section.into()])
        .with_metadata(
            Metadata::default()
                .with_title("Text layout")
                .with_custom_boolean_properties([("Reviewed".into(), false)].into()),
        )
        .with_defaults(Some(
            DocumentDefaults::default()
                .with_run(RunFormat::default().with_size(11.))
                .with_mirror_margins(true),
        ))
        .with_timestamp(Some("2021-04-03T02:01:00Z".into()));
    let bytes = futures::executor::block_on(document.resolve())?.to_docx()?;
    std::fs::write(out.join("text-layout.docx"), bytes)?;
    Ok(())
}
