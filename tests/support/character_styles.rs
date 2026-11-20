use rv_docx_rs::*;

pub fn document() -> Document {
    let mut document = Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![
                TextRun::new("styled")
                    .with_format(RunFormat {
                        style: Some("Child Character".into()),
                        bold: Some(false),
                        ..Default::default()
                    })
                    .into(),
            ])
            .into(),
        ])
        .into(),
    ]);
    document.styles = vec![
        CharacterStyleDefinition {
            name: "Child Character".into(),
            base_style: Some("Later Character".into()),
            linked_style: None,
            format: RunFormat {
                italic: Some(true),
                size_complex_script: Some(14.5),
                ..Default::default()
            },
        }
        .into(),
        CharacterStyleDefinition {
            name: "Later Character".into(),
            format: RunFormat {
                bold: Some(true),
                font: Some("Arial".into()),
                ..Default::default()
            },
            ..Default::default()
        }
        .into(),
    ]
    .into();
    document
}
