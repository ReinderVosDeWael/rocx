use rv_docx_rs::*;

pub fn document() -> Document {
    let name = BookmarkName::new("target").unwrap();
    let outer = Field::new(
        FieldInstruction::raw("  IF 1 = 1 \"yes\" \"no\"  ").unwrap(),
        vec![
            TextRun::new("cached ").into(),
            Field::new(FieldInstruction::NumPages, vec![TextRun::new("12").into()])
                .with_properties(FieldProperties {
                    form: FieldForm::Complex,
                    ..Default::default()
                })
                .into(),
        ],
    )
    .with_properties(FieldProperties {
        form: FieldForm::Complex,
        dirty: Some(true),
        locked: Some(false),
    });
    Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![
                RangeStart::bookmark(name.clone()).into(),
                TextRun::new("Target").into(),
                RangeEnd::bookmark(&name).into(),
                Field::new(
                    FieldInstruction::Page,
                    vec![
                        TextRun::new("7")
                            .with_format(RunFormat {
                                bold: Some(true),
                                ..Default::default()
                            })
                            .into(),
                    ],
                )
                .into(),
                outer.into(),
                Field::new(
                    FieldInstruction::Reference(name),
                    vec![TextRun::new("Target").into()],
                )
                .into(),
                Field::new(
                    FieldInstruction::TableOfContents,
                    vec![TextRun::new("Contents pending").into()],
                )
                .into(),
            ])
            .into(),
        ])
        .into(),
    ])
}
