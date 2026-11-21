use rocx::*;

pub fn document() -> Document {
    let format = Value::future(async {
        RunFormat {
            language: Some(RunLanguage {
                latin: Some("en-GB".into()),
                east_asia: Some("ja-JP".into()),
                bidi: Some("ar-SA".into()),
            }),
            bold: Some(false),
            bold_complex_script: Some(true),
            italic: Some(true),
            italic_complex_script: Some(false),
            size: Some(11.),
            size_complex_script: Some(14.5),
            rtl: Some(true),
            complex_script: Some(false),
            font: Some("Latin".into()),
            font_east_asia: Some("East Asian".into()),
            font_complex_script: Some("Arabic".into()),
            ..Default::default()
        }
    });
    let mut document = Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![
                TextRun::new("العربية 日本語").with_format(format).into(),
            ])
            .with_format(ParagraphFormat {
                bidi: Some(true),
                ..Default::default()
            })
            .into(),
        ])
        .into(),
    ]);
    document.styles = vec![
        ParagraphStyleDefinition {
            name: "Script".into(),
            bidi: Some(false),
            language: Some(RunLanguage {
                bidi: Some("he-IL".into()),
                ..Default::default()
            }),
            bold_complex_script: Some(false),
            size_complex_script: Some(13.),
            ..Default::default()
        }
        .into(),
    ]
    .into();
    document
}
