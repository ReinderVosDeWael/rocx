use rocx::*;

pub fn document() -> Document {
    let mut document = Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![
                TextRun::new("linked")
                    .with_format(RunFormat {
                        style: Some("Pair Character".into()),
                        italic: Some(false),
                        ..Default::default()
                    })
                    .into(),
            ])
            .with_format(ParagraphFormat {
                style: Some("Pair Paragraph".into()),
                ..Default::default()
            })
            .into(),
        ])
        .into(),
    ]);
    document.styles = Value::future(async {
        vec![
            ParagraphStyleDefinition {
                name: "Pair Paragraph".into(),
                linked_style: Some("Pair Character".into()),
                bold: Some(true),
                ..Default::default()
            }
            .into(),
            CharacterStyleDefinition {
                name: "Pair Character".into(),
                linked_style: Some("Pair Paragraph".into()),
                format: RunFormat {
                    italic: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            }
            .into(),
        ]
    });
    document
}
