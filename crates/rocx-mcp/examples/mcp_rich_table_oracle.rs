//! Direct Rust oracle for rich table insertion; no MCP DTOs.
use rocx::{
    BorderLine, BorderSide, CellMargins, Component, Paragraph, RowHeight, RunFormat, Table,
    TableAlignment, TableBorder, TableCell, TableCellProperties, TableLayout, TableProperties,
    TableRow, TableRowProperties, TextRun, VerticalAlignment,
    editor::{BodyBlock, EditBatch, ImportedDocument, TableInsertion},
};
use std::{env, fs};

fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let [output, source] = args.as_slice() else {
        panic!("expected OUTPUT SOURCE");
    };
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let BodyBlock::Paragraph(target) = &doc.body()[0] else {
        panic!("expected paragraph");
    };
    let nested = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![Paragraph::new("Nested").into()]).into(),
        ])
        .into(),
    ])
    .with_properties(TableProperties::default().with_column_widths(vec![900]));
    let first = TableCell::new(vec![
        Paragraph::from_children(vec![
            TextRun::new("Styled cell")
                .with_format(RunFormat::default().with_bold(true))
                .into(),
        ])
        .into(),
        nested.into(),
        Paragraph::new("Trailing").into(),
    ])
    .with_properties(
        TableCellProperties::default()
            .with_background_color([0xfa, 0x80, 0x60])
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_margins(CellMargins::default().with_start(3.0)),
    );
    let row = TableRow::new(vec![
        first.into(),
        TableCell::new(vec![Paragraph::new("Second").into()]).into(),
    ])
    .with_properties(
        TableRowProperties::default()
            .with_repeat_header(true)
            .with_cant_split(false)
            .with_height(RowHeight::AtLeast(12.0)),
    );
    let table = Table::new(vec![row.into()]).with_properties(
        TableProperties::default()
            .with_column_widths(vec![1440, 1800])
            .with_layout(TableLayout::Fixed)
            .with_alignment(TableAlignment::Right)
            .with_borders(vec![TableBorder {
                side: BorderSide::Top,
                sz: 8,
                color: [0x11, 0x22, 0x33],
                val: BorderLine::Single,
            }]),
    );
    let table = futures_executor::block_on(table.resolve())
        .unwrap()
        .unwrap();
    let preview = doc
        .preview_edits(EditBatch {
            tables: &[TableInsertion {
                target: target.anchor.clone(),
                table,
            }],
            ..EditBatch::default()
        })
        .unwrap();
    fs::write(output, preview.to_docx().unwrap()).unwrap();
}
