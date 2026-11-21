use rocx::*;

pub fn document() -> Document {
    let mut document = Document::new(vec![]);
    document.styles = Value::future(async {
        vec![
            TableStyleDefinition {
                name: "Conditional".into(),
                first_row: Some(TableSectionFormat {
                    line_height: Some(LineHeight::Exact(14.5)),
                    tab_stops: vec![TabStop {
                        position: 72.,
                        alignment: TabAlignment::Right,
                        leader: Some(TabLeader::Dot),
                    }],
                    bidi: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            }
            .into(),
        ]
    });
    document
}
