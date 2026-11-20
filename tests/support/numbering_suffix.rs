use rv_docx_rs::*;

pub fn document() -> Document {
    let mut document = Document::new(vec![
        Section::new(vec![
            Paragraph::new("suffix request, not inserted text").into(),
        ])
        .into(),
    ]);
    document.numbering = Value::future(async {
        let suffixes = [
            None,
            Some(NumberingSuffix::Tab),
            Some(NumberingSuffix::Space),
            Some(NumberingSuffix::Nothing),
        ];
        let level = |(index, suffix): (usize, Option<NumberingSuffix>)| NumberingLevel {
            level: index as u8,
            start: 1,
            format: "upperRoman".into(),
            text: "%1.".into(),
            is_legal: Some(true),
            suffix,
            restart: None,
            alignment: None,
        };
        vec![NumberingDefinition {
            id: 80,
            levels: suffixes.into_iter().enumerate().map(level).collect(),
            level_overrides: suffixes.into_iter().rev().enumerate().map(level).collect(),
            ..Default::default()
        }]
    });
    document
}
