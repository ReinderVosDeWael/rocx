//! Direct Rust oracle for deeply nested rich tables; no MCP DTOs.
use rocx::{
    BlockChildren, Component, Paragraph, ResolvedBlock, ResolvedTable, RowHeight, RunFormat, Table,
    TableAlignment, TableCell, TableCellProperties, TableLayout, TableProperties, TableRow,
    TableRowProperties, TextRun,
    editor::{
        BodyBlock, EditBatch, HeaderEditMode, ImportedDocument, TableInsertion, TableOperation,
        TableSite, TableStructureEdit, WrapperEdit,
    },
};
use std::{env, fs};

const WIDE_SPAN: u32 = 8193;
const CHAIN_SPAN: u32 = 1024;

fn table(depth: usize, mode: &str) -> ResolvedTable {
    let mut nested = None;
    for level in (0..depth).rev() {
        let span = match mode {
            "wide" if level + 1 == depth => Some(WIDE_SPAN),
            "wide-chain" => Some(CHAIN_SPAN),
            _ => None,
        };
        let rows = (0..if span == Some(WIDE_SPAN) { 2 } else { 1 })
            .map(|_| {
                let mut blocks: BlockChildren = vec![
                    Paragraph::from_children(vec![
                        TextRun::new(format!("Level {level} & <text>"))
                            .with_format(RunFormat::default().with_bold(true))
                            .into(),
                    ])
                    .into(),
                ];
                if let Some(child) = nested.take() {
                    blocks.push(ResolvedBlock::Table(child).into());
                    blocks.push(Paragraph::new("After nested table").into());
                } else if mode == "header" {
                    blocks.clear();
                }
                let cell = TableCell::new(blocks).with_properties(TableCellProperties {
                    grid_span: span,
                    background_color: Some([17, 34, 51]),
                    ..TableCellProperties::default()
                });
                TableRow::new(vec![cell.into()])
                    .with_properties(
                        TableRowProperties::default().with_height(RowHeight::Exact(14.0)),
                    )
                    .into()
            })
            .collect();
        let widths = span.map_or_else(|| vec![1440], |span| vec![1; span as usize]);
        nested = futures_executor::block_on(
            Table::new(rows)
                .with_properties(
                    TableProperties::default()
                        .with_layout(TableLayout::Fixed)
                        .with_alignment(TableAlignment::Right)
                        .with_column_widths(widths),
                )
                .resolve(),
        )
        .unwrap();
    }
    nested.unwrap()
}

fn empty_row_donor(doc: &ImportedDocument) -> Vec<u8> {
    let donor = futures_executor::block_on(
        Table::new(vec![
            TableRow::new(vec![])
                .with_properties(TableRowProperties::default().with_height(RowHeight::Exact(18.0)))
                .into(),
        ])
        .resolve(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(donor.columns(), 0);
    let [BodyBlock::Paragraph(paragraph), BodyBlock::Table(target)] = doc.body() else {
        panic!("expected a paragraph and a table");
    };
    let insertion = doc.preview_edits(EditBatch {
        tables: &[TableInsertion {
            target: paragraph.anchor.clone(),
            table: donor.clone(),
        }],
        ..EditBatch::default()
    });
    let Err(error) = insertion else {
        panic!("inserting a zero-column table unexpectedly succeeded");
    };
    assert!(
        error
            .to_string()
            .contains("an inserted table needs at least one column")
    );
    doc.preview_edits(EditBatch {
        structural: &[TableStructureEdit {
            target: target.anchor.clone(),
            operation: TableOperation::Property {
                site: TableSite::Row(0),
                field: "height".into(),
                source: Some(Box::new(donor)),
            },
        }],
        ..EditBatch::default()
    })
    .unwrap()
    .to_docx()
    .unwrap()
}

fn zero_column_control_probe(doc: &ImportedDocument) -> Vec<u8> {
    let nested =
        futures_executor::block_on(Table::new(vec![TableRow::new(vec![]).into()]).resolve())
            .unwrap()
            .unwrap();
    assert_eq!(nested.columns(), 0);
    let [control] = doc.controls() else {
        panic!("expected one rich block control");
    };
    let replace = |table| {
        doc.preview_edits(EditBatch {
            wrappers: &[WrapperEdit::ReplaceBlocks(
                control.anchor.clone(),
                vec![ResolvedBlock::Table(table)],
            )],
            ..EditBatch::default()
        })
    };
    let Err(error) = replace(nested.clone()) else {
        panic!("direct zero-column replacement unexpectedly succeeded");
    };
    assert!(error.to_string().contains("empty replacement table"));
    let outer = futures_executor::block_on(
        Table::new(vec![
            TableRow::new(vec![
                TableCell::new(vec![ResolvedBlock::Table(nested).into()]).into(),
            ])
            .into(),
        ])
        .with_properties(TableProperties::default().with_column_widths(vec![1440]))
        .resolve(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(outer.columns(), 1);
    replace(outer).unwrap().to_docx().unwrap()
}

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let (source, output, depth, mode) = match args.as_slice() {
        [source, output, mode] if matches!(mode.as_str(), "donor" | "zero-column-control") => {
            (source, output, 0, mode.as_str())
        }
        [source, output, depth] => (source, output, depth.parse().unwrap(), "body"),
        [source, output, depth, mode]
            if matches!(mode.as_str(), "header" | "wide" | "wide-chain") =>
        {
            (source, output, depth.parse().unwrap(), mode.as_str())
        }
        _ => panic!("expected SOURCE OUTPUT (donor | zero-column-control | DEPTH [MODE])"),
    };
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let bytes = if mode == "donor" {
        empty_row_donor(&doc)
    } else if mode == "zero-column-control" {
        zero_column_control_probe(&doc)
    } else if mode == "header" {
        let story = doc.default_headers().unwrap().into_iter().next().unwrap();
        doc.edit_story_blocks(
            &story.anchor,
            HeaderEditMode::Shared,
            1,
            0,
            &[ResolvedBlock::Table(table(depth, mode))],
        )
        .unwrap()
    } else {
        let BodyBlock::Paragraph(target) = &doc.body()[0] else {
            panic!("expected paragraph");
        };
        doc.preview_edits(EditBatch {
            tables: &[TableInsertion {
                target: target.anchor.clone(),
                table: table(depth, mode),
            }],
            ..EditBatch::default()
        })
        .unwrap()
        .to_docx()
        .unwrap()
    };
    let reopened = ImportedDocument::open(&bytes).expect("native output must reopen");
    if mode == "header" {
        reopened
            .default_headers()
            .expect("native header must reopen");
    }
    fs::write(output, bytes).unwrap();
}
