use rv_docx_rs::*;

pub fn document() -> Document {
    let properties = Value::future(async {
        TableCellProperties {
            margins: Some(CellMargins {
                top: Some(3.126),
                start: Some(0.),
                bottom: Some(6.),
                end: Some(9.),
            }),
            text_direction: Some(CellTextDirection::TopToBottomRightToLeft),
            ..Default::default()
        }
    });
    let mut cell = TableCell::new(vec![Paragraph::new("cell").into()]);
    cell.properties = properties;
    Document::new(vec![
        Section::new(vec![
            Table::new(vec![TableRow::new(vec![cell.into()]).into()]).into(),
        ])
        .into(),
    ])
}
