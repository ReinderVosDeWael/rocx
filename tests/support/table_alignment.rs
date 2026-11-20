use rv_docx_rs::*;

pub fn document() -> Document {
    let tables = [
        None,
        Some(TableAlignment::Left),
        Some(TableAlignment::Center),
        Some(TableAlignment::Right),
        Some(TableAlignment::Start),
        Some(TableAlignment::End),
    ]
    .into_iter()
    .map(|alignment| {
        let paragraph =
            Paragraph::new("table alignment, not text alignment").with_format(ParagraphFormat {
                alignment: Some(Alignment::Right),
                ..Default::default()
            });
        let mut table = Table::new(vec![
            TableRow::new(vec![TableCell::new(vec![paragraph.into()]).into()]).into(),
        ]);
        table.properties = Value::future(async move {
            TableProperties {
                alignment,
                column_widths: Some(vec![1440]),
                ..Default::default()
            }
        });
        table.into()
    })
    .collect();
    Document::new(vec![Section::new(tables).into()])
}
