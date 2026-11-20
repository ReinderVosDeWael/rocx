use rv_docx_rs::*;

pub fn document() -> Document {
    let mut table = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![
                Paragraph::new("table-look request, not computed formatting").into(),
            ])
            .into(),
        ])
        .into(),
    ]);
    table.properties = Value::future(async {
        TableProperties {
            look: Some(TableLook {
                first_column: Some(false),
                last_row: Some(true),
                last_column: Some(true),
                horizontal_banding: Some(false),
                vertical_banding: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }
    });
    Document::new(vec![Section::new(vec![table.into()]).into()])
}
