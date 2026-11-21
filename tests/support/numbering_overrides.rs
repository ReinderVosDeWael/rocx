use rocx::*;

pub fn document() -> Document {
    let mut paragraph = Paragraph::new("restart");
    paragraph.numbering = Some(NumberingReference { id: 42, level: 0 }).into();
    let mut document = Document::new(vec![Section::new(vec![paragraph.into()]).into()]);
    document.numbering = vec![NumberingDefinition {
        abstract_id: None,
        inherit_style: None,
        id: 42,
        levels: vec![NumberingLevel {
            label_format: None,
            paragraph_style: None,
            paragraph_format: None,
            level: 0,
            start: 1,
            format: "decimal".into(),
            text: "%1.".into(),
            is_legal: None,
            suffix: None,
            restart: None,
            alignment: None,
        }],
        start_overrides: vec![NumberingStartOverride { level: 0, start: 7 }],
        level_overrides: Vec::new(),
    }]
    .into();
    document
}
