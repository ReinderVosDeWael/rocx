use rv_docx_rs::*;

pub fn document() -> Document {
    let paragraph =
        Paragraph::new("label alignment, not paragraph alignment").with_format(ParagraphFormat {
            alignment: Some(Alignment::Right),
            ..Default::default()
        });
    let mut document = Document::new(vec![Section::new(vec![paragraph.into()]).into()]);
    document.numbering = Value::future(async {
        let alignments = [
            None,
            Some(NumberingAlignment::Left),
            Some(NumberingAlignment::Center),
            Some(NumberingAlignment::Right),
            Some(NumberingAlignment::Start),
            Some(NumberingAlignment::End),
        ];
        let level = |(index, alignment): (usize, Option<NumberingAlignment>)| NumberingLevel {
            level: index as u8,
            start: 1,
            format: "upperRoman".into(),
            text: "%1.".into(),
            is_legal: Some(false),
            suffix: Some(NumberingSuffix::Space),
            restart: Some(0),
            alignment,
        };
        vec![NumberingDefinition {
            id: 80,
            levels: alignments.into_iter().enumerate().map(level).collect(),
            level_overrides: alignments
                .into_iter()
                .rev()
                .enumerate()
                .map(level)
                .collect(),
            ..Default::default()
        }]
    });
    document
}
