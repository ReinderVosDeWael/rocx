use rocx::*;

pub fn document() -> Document {
    let rows = [
        TableRowProperties {
            height: Some(RowHeight::Exact(14.25)),
            repeat_header: Some(true),
            cant_split: Some(true),
        },
        TableRowProperties {
            height: Some(RowHeight::AtLeast(18.)),
            repeat_header: Some(false),
            cant_split: Some(false),
        },
        TableRowProperties::default(),
    ]
    .into_iter()
    .map(|properties| {
        TableRow::new(vec![
            TableCell::new(vec![Paragraph::new("cell").into()]).into(),
        ])
        .with_properties(properties)
        .into()
    })
    .collect();
    Document::new(vec![Section::new(vec![Table::new(rows).into()]).into()])
}
