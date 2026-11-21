use rocx::*;

pub fn document() -> Document {
    let mut document = Document::new(vec![
        Section::new(vec![
            Paragraph::new("restart request, not a calculated counter").into(),
        ])
        .into(),
    ]);
    document.numbering = Value::future(async {
        let requests = [None, Some(0), Some(1), Some(2), Some(u32::MAX)];
        let level = |(index, restart): (usize, Option<u32>)| NumberingLevel {
            level: index as u8,
            start: 3,
            format: "upperRoman".into(),
            text: "%1.".into(),
            is_legal: Some(false),
            suffix: Some(NumberingSuffix::Space),
            restart,
            alignment: None,
        };
        vec![NumberingDefinition {
            id: 80,
            levels: requests.into_iter().enumerate().map(level).collect(),
            level_overrides: requests.into_iter().rev().enumerate().map(level).collect(),
            start_overrides: vec![NumberingStartOverride { level: 2, start: 7 }],
        }]
    });
    document
}
