use rv_docx_rs::*;

pub fn document() -> Document {
    let format = Value::future(async {
        ParagraphFormat {
            tab_stops: vec![
                TabStop {
                    position: 108.,
                    alignment: TabAlignment::Right,
                    leader: Some(TabLeader::Dot),
                },
                TabStop {
                    position: -12.,
                    ..Default::default()
                },
                TabStop {
                    position: 72.,
                    alignment: TabAlignment::Center,
                    leader: Some(TabLeader::Hyphen),
                },
                TabStop {
                    position: 144.,
                    alignment: TabAlignment::Decimal,
                    leader: Some(TabLeader::Underscore),
                },
                TabStop {
                    position: 180.,
                    alignment: TabAlignment::Bar,
                    leader: None,
                },
            ],
            ..Default::default()
        }
    });
    let mut document = Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![
                TextRun::new("label").into(),
                Tab::default().into(),
                TextRun::new("value").into(),
            ])
            .with_format(format)
            .into(),
            Paragraph::new("inherited")
                .with_format(ParagraphFormat {
                    style: Some("Tabbed".into()),
                    ..Default::default()
                })
                .into(),
            Paragraph::new("clear")
                .with_format(ParagraphFormat {
                    style: Some("Tabbed".into()),
                    tab_stops: vec![TabStop {
                        position: 72.,
                        alignment: TabAlignment::Clear,
                        leader: None,
                    }],
                    ..Default::default()
                })
                .into(),
        ])
        .into(),
    ]);
    document.styles = vec![
        ParagraphStyleDefinition {
            name: "Tabbed".into(),
            tab_stops: vec![TabStop {
                position: 72.,
                ..Default::default()
            }],
            ..Default::default()
        }
        .into(),
    ]
    .into();
    document
}
