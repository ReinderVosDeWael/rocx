//! Native Rust equivalent of the JavaScript quickstart.
use rocx::{
    AltText, Block, ImageRun, Paragraph, RunFormat, Table, TableCell, TableRow, TextRun, Value,
};

pub fn blocks() -> Vec<Value<Block>> {
    let paragraph = Paragraph::from_children(vec![
        TextRun::new("JavaScript authoring: ")
            .with_format(RunFormat::default().with_bold(true).with_italic(false))
            .into(),
        TextRun::new("Café 東京")
            .with_format(RunFormat::default().with_italic(true))
            .into(),
    ]);
    let table = Table::new(
        [["Item", "Count"], ["Pages", "2"]]
            .into_iter()
            .map(|row| {
                TableRow::new(
                    row.into_iter()
                        .map(|text| TableCell::new(vec![Paragraph::new(text).into()]).into())
                        .collect(),
                )
                .into()
            })
            .collect(),
    );
    let image = ImageRun::new(include_bytes!("../../tests/fixtures/images/sample.png").to_vec())
        .with_alt_text(Some(
            AltText::default().with_description("JavaScript authoring sample image"),
        ));
    vec![
        paragraph.into(),
        table.into(),
        Paragraph::from_children(vec![image.into()]).into(),
    ]
}
