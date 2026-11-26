//! Direct native byte oracle for the independent MCP physical-table fixture.
use std::{env, fs};

use rocx::{
    Component, Hyperlink, HyperlinkTarget, Paragraph, ResolvedBlock, RunFormat, Table, TableCell,
    TableCellProperties, TableLayout, TableProperties, TableRow, TableRowProperties, TextRun,
    VerticalAlignment,
    editor::{
        BodyBlock, EditBatch, ImportedDocument, ImportedInlineNode, RunTextEdit, TableOperation,
        TableSite, TableStructureEdit,
    },
};

fn resolve<C: Component>(component: C) -> C::Resolved {
    futures_executor::block_on(component.resolve())
        .unwrap()
        .unwrap()
}

fn rich_blocks() -> Vec<ResolvedBlock> {
    let paragraph = Paragraph::from_children(vec![
        TextRun::new("Rich 😀")
            .with_format(RunFormat {
                bold: Some(true),
                ..RunFormat::default()
            })
            .into(),
        Hyperlink::new(
            HyperlinkTarget::external("https://example.org/physical-table").unwrap(),
            vec![TextRun::new(" link").into()],
        )
        .into(),
    ]);
    let table = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![Paragraph::new("nested payload").into()]).into(),
        ])
        .into(),
    ])
    .with_properties(TableProperties {
        column_widths: Some(vec![1440]),
        ..TableProperties::default()
    });
    vec![
        ResolvedBlock::Paragraph(resolve(paragraph)),
        ResolvedBlock::Table(resolve(table)),
    ]
}

fn body_donor(texts: &[&str]) -> rocx::ResolvedTable {
    let cells = texts
        .iter()
        .map(|text| {
            TableCell::new(vec![
                Paragraph::from_children(vec![
                    TextRun::new(*text)
                        .with_format(RunFormat {
                            bold: Some(true),
                            ..RunFormat::default()
                        })
                        .into(),
                ])
                .into(),
            ])
            .with_properties(TableCellProperties {
                vertical_alignment: Some(VerticalAlignment::Bottom),
                ..TableCellProperties::default()
            })
            .into()
        })
        .collect();
    resolve(
        Table::new(vec![
            TableRow::new(cells)
                .with_properties(TableRowProperties {
                    repeat_header: Some(false),
                    ..TableRowProperties::default()
                })
                .into(),
        ])
        .with_properties(TableProperties {
            layout: Some(TableLayout::Fixed),
            column_widths: Some(vec![1800; texts.len()]),
            ..TableProperties::default()
        }),
    )
}

fn body_mixed(doc: &ImportedDocument) -> Vec<u8> {
    let tables: Vec<_> = doc
        .body()
        .iter()
        .filter_map(|block| match block {
            BodyBlock::Table(table) => Some(table),
            _ => None,
        })
        .collect();
    use TableOperation::*;
    let edits = [
        (1, RemoveTable),
        (
            3,
            ReplaceNode {
                site: TableSite::Table,
                source: Box::new(body_donor(&["new table"])),
            },
        ),
        (
            4,
            ReplaceNode {
                site: TableSite::Row(1),
                source: Box::new(body_donor(&["new row left", "new row right"])),
            },
        ),
        (
            5,
            ReplaceNode {
                site: TableSite::Cell(0, 1),
                source: Box::new(body_donor(&["new cell"])),
            },
        ),
        (6, InsertColumn(1)),
    ]
    .map(|(index, operation)| TableStructureEdit {
        target: tables[index].anchor.clone(),
        operation,
    });
    let BodyBlock::Paragraph(tail) = doc.body().last().unwrap() else {
        panic!("expected tail paragraph");
    };
    let ImportedInlineNode::Run(run) = &tail.inline_structure[0] else {
        panic!("expected tail run");
    };
    doc.preview_edits(EditBatch {
        structural: &edits,
        run_text: &[RunTextEdit {
            target: run.anchor.clone(),
            text: "Edited tail 😀".into(),
        }],
        ..EditBatch::default()
    })
    .unwrap()
    .to_docx()
    .unwrap()
}

fn main() {
    let [source, output, case, owner]: [String; 4] =
        env::args().skip(1).collect::<Vec<_>>().try_into().unwrap();
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    if case == "body-mixed" {
        fs::write(output, body_mixed(&doc)).unwrap();
        return;
    }
    let tables = doc
        .part_tables((owner != "body").then_some(owner.as_str()))
        .unwrap();
    if case == "inspect" {
        let rows: Vec<_> = tables
            .iter()
            .map(|table| {
                serde_json::json!({
                    "part": table.part_name, "path": table.path, "depth": table.depth,
                    "entry_id": table.entry_id, "properties": table.properties,
                    "grid_widths": table.grid_widths, "row_properties": table.row_properties,
                    "cell_properties": table.cell_properties,
                })
            })
            .collect();
        fs::write(output, serde_json::to_vec(&rows).unwrap()).unwrap();
        return;
    }
    let index = match (case.as_str(), owner.as_str()) {
        ("rich", "body") => 1,
        ("rich", _) => 0,
        (_, "body") => 2,
        _ => panic!("unexpected fixture case"),
    };
    let target = &tables[index];
    use TableOperation::*;
    let operations = match case.as_str() {
        "insert-row" => vec![InsertRow(2)],
        "remove-row" => vec![RemoveRow(1)],
        "insert-cell" => vec![InsertCell { row: 0, cell: 2 }],
        "remove-cell" => vec![RemoveCell { row: 0, cell: 1 }],
        "insert-column" => vec![InsertColumn(1)],
        "remove-column" => vec![RemoveColumn(1)],
        "fixed" => vec![Layout(Some(TableLayout::Fixed))],
        "unchanged-layout" => vec![Layout(Some(TableLayout::Autofit))],
        "clear-layout" => vec![Layout(None)],
        "clear-properties" => [
            (TableSite::Table, "layout"),
            (TableSite::Row(0), "cantSplit"),
            (TableSite::Cell(0, 0), "verticalAlignment"),
            (TableSite::Column(0), "width"),
        ]
        .map(|(site, field)| Property {
            site,
            field: field.into(),
            source: None,
        })
        .into(),
        "set-properties" => {
            let donor = resolve(
                Table::new(vec![
                    TableRow::new(vec![
                        TableCell::new(vec![Paragraph::new("not copied").into()]).into(),
                    ])
                    .into(),
                ])
                .with_properties(TableProperties {
                    column_widths: Some(vec![2100]),
                    layout: Some(TableLayout::Fixed),
                    ..TableProperties::default()
                }),
            );
            [
                (TableSite::Table, "layout"),
                (TableSite::Column(0), "width"),
            ]
            .map(|(site, field)| Property {
                site,
                field: field.into(),
                source: Some(Box::new(donor.clone())),
            })
            .into()
        }
        "rich" => vec![ReplaceCell {
            row: 0,
            cell: 0,
            blocks: rich_blocks(),
        }],
        _ => panic!("unknown case"),
    };
    let edits: Vec<_> = operations
        .into_iter()
        .map(|operation| TableStructureEdit {
            target: target.anchor.clone(),
            operation,
        })
        .collect();
    fs::write(
        output,
        doc.edit_part_tables(&target.part_name, &edits).unwrap(),
    )
    .unwrap();
}
