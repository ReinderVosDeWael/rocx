use rocx::*;

pub fn document() -> Document {
    let mut paragraph = Paragraph::new("replacement request, not a calculated label");
    paragraph.numbering = Some(NumberingReference { id: 42, level: 0 }).into();
    let mut document = Document::new(vec![Section::new(vec![paragraph.into()]).into()]);
    document.numbering = Value::future(async {
        vec![NumberingDefinition {
            id: 42,
            levels: (0..3)
                .map(|level| NumberingLevel {
                    level,
                    start: 1,
                    format: "decimal".into(),
                    text: format!("%{}.", level + 1),
                    is_legal: None,
                    suffix: None,
                    restart: None,
                    alignment: None,
                })
                .collect(),
            start_overrides: vec![
                NumberingStartOverride { level: 2, start: 0 },
                NumberingStartOverride { level: 0, start: 7 },
            ],
            level_overrides: vec![
                NumberingLevel {
                    level: 1,
                    start: 0,
                    format: "bullet".into(),
                    text: "•".into(),
                    is_legal: None,
                    suffix: None,
                    restart: None,
                    alignment: None,
                },
                NumberingLevel {
                    level: 0,
                    start: 4,
                    format: "upperRoman".into(),
                    text: "Replacement-%1".into(),
                    is_legal: None,
                    suffix: None,
                    restart: None,
                    alignment: None,
                },
            ],
        }]
    });
    document
}
