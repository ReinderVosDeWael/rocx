//! Fields request consumer calculation; cached results below are placeholders.
use rv_docx_rs::*;

fn main() -> Result<()> {
    let output = std::env::args().nth(1).unwrap_or("fields.docx".into());
    let name = BookmarkName::new("Introduction")?;
    let instruction = Property::new(
        "reference.instruction",
        Value::future({
            let name = name.clone();
            async move { FieldInstruction::Reference(name) }
        }),
    );
    let mut section = Section::new(vec![
        Paragraph::from_children(vec![
            RangeStart::bookmark(name.clone()).into(),
            TextRun::new("Introduction").into(),
            RangeEnd::bookmark(&name).into(),
        ])
        .into(),
        Paragraph::from_children(vec![
            Field::new(
                instruction.reference(),
                vec![TextRun::new("Introduction (cached)").into()],
            )
            .with_properties(FieldProperties {
                dirty: Some(true),
                ..Default::default()
            })
            .into(),
        ])
        .into(),
        Paragraph::from_children(vec![
            Field::new(
                FieldInstruction::TableOfContents,
                vec![TextRun::new("Contents pending update").into()],
            )
            .with_properties(FieldProperties {
                form: FieldForm::Complex,
                dirty: Some(true),
                locked: None,
            })
            .into(),
        ])
        .into(),
    ]);
    section.footers.insert(
        HeaderFooterType::Default,
        Footer::new(vec![
            Paragraph::from_children(vec![
                TextRun::new("Page ").into(),
                Field::new(FieldInstruction::Page, vec![TextRun::new("1").into()]).into(),
                TextRun::new(" of ").into(),
                Field::new(FieldInstruction::NumPages, vec![TextRun::new("1").into()]).into(),
            ])
            .into(),
        ])
        .into(),
    );
    futures::executor::block_on(Document::new(vec![section.into()]).resolve())?.save(output)
}
