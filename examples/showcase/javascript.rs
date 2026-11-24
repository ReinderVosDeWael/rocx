//! Native Rust counterpart of the named and legacy JavaScript reports.
use rocx::{
    Alignment, AltText, Block, Break, Hyperlink, HyperlinkTarget, ImageRun, ImageTransformation,
    Paragraph, ParagraphFormat, RunFormat, Tab, Table, TableCell, TableRow, TextRun, Value,
};

pub fn blocks() -> Vec<Value<Block>> {
    let paragraph = Paragraph::from_children(vec![
        TextRun::new("JavaScript authoring: ")
            .with_format(RunFormat::default().with_bold(true).with_italic(false))
            .into(),
        TextRun::new("Café 東京")
            .with_format(RunFormat::default().with_italic(true))
            .into(),
    ])
    .with_format(ParagraphFormat::default().with_alignment(Alignment::Center));
    let text_cell = |text: &str| TableCell::new(vec![Paragraph::new(text).into()]).into();
    let details = Paragraph::from_children(vec![
        TextRun::new("2")
            .with_format(RunFormat::default().with_bold(true))
            .into(),
        Tab::default().into(),
        Hyperlink::new(
            HyperlinkTarget::external("https://example.org/report").expect("valid fixture URL"),
            vec![
                TextRun::new("Report details")
                    .with_format(RunFormat::default().with_italic(true))
                    .into(),
            ],
        )
        .into(),
        Break::default().into(),
        TextRun::new("Verified").into(),
    ]);
    let nested = Table::new(vec![TableRow::new(vec![text_cell("Nested detail")]).into()]);
    let table = Table::new(vec![
        TableRow::new(vec![text_cell("Item"), text_cell("Details")]).into(),
        TableRow::new(vec![
            text_cell("Pages"),
            TableCell::new(vec![details.into(), nested.into()]).into(),
        ])
        .into(),
    ]);
    let image = ImageRun::new(include_bytes!("../../tests/fixtures/images/sample.png").to_vec())
        .with_transformation(Some(
            ImageTransformation::default()
                .with_width(72.0)
                .with_height(36.0),
        ))
        .with_alt_text(Some(
            AltText::default().with_description("JavaScript authoring sample image"),
        ));
    vec![
        paragraph.into(),
        table.into(),
        Paragraph::from_children(vec![image.into()]).into(),
    ]
}
