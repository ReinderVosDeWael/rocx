use rv_docx_rs::*;

pub fn document() -> Document {
    let mut document = Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![
                TextRun::new("unchanged")
                    .with_format(RunFormat {
                        bold: Some(false),
                        ..Default::default()
                    })
                    .into(),
            ])
            .into(),
        ])
        .into(),
    ]);
    document.defaults = Value::future(async {
        Some(DocumentDefaults {
            run: Some(RunFormat {
                font: Some("Literal".into()),
                size: Some(13.5),
                bold: Some(true),
                rtl: Some(false),
                language: Some(RunLanguage {
                    bidi: Some("ar-SA".into()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            paragraph: Some(ParagraphFormat {
                spacing_before: Some(0.),
                line_height: Some(LineHeight::Exact(14.)),
                left_indent: Some(12.),
                first_line_indent: Some(-6.),
                bidi: Some(true),
                tab_stops: vec![TabStop {
                    position: 36.,
                    ..Default::default()
                }],
                ..Default::default()
            }),
        })
    });
    document
}
