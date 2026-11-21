use rocx::*;

pub fn document() -> Document {
    let mut first = Section::new(vec![
        Paragraph::from_children(vec![
            TextRun::new("first العربية last")
                .with_format(RunFormat {
                    rtl: Some(false),
                    ..Default::default()
                })
                .into(),
        ])
        .with_format(ParagraphFormat {
            bidi: Some(false),
            ..Default::default()
        })
        .into(),
    ]);
    first.properties = Value::future(async {
        Some(SectionProperties {
            bidi: Some(true),
            columns: Some(Columns {
                count: Some(2),
                ..Default::default()
            }),
            ..Default::default()
        })
    });
    let mut last = Section::new(vec![Paragraph::new("off").into()]);
    last.properties = Some(SectionProperties {
        bidi: Some(false),
        ..Default::default()
    })
    .into();
    Document::new(vec![
        first.into(),
        Section::new(vec![Paragraph::new("inherited").into()]).into(),
        last.into(),
    ])
}
