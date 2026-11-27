//! Direct native byte oracle for the independent MCP physical-table fixture.
use std::{env, fs};

use rocx::{
    BorderLine, BorderSide, CellBorder, CellBorderSide, CellMargins, CellTextDirection, Component,
    ContentControlProperties, Hyperlink, HyperlinkTarget, Paragraph, ResolvedBlock, ResolvedTable,
    RowHeight, RunFormat, Table, TableAlignment, TableBorder, TableCell, TableCellProperties,
    TableLayout, TableLook, TableProperties, TableRow, TableRowProperties, TextRun,
    VerticalAlignment, VerticalMerge,
    editor::{
        BodyBlock, EditBatch, ImportedDocument, ImportedInlineNode, RunTextEdit, TableInsertion,
        TableOperation, TableSite, TableStructureEdit,
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

fn body_replace_cell(doc: &ImportedDocument, duplicate: bool, output: &str) {
    let table = doc
        .body()
        .iter()
        .filter_map(|block| match block {
            BodyBlock::Table(table) => Some(table),
            _ => None,
        })
        .nth(1)
        .unwrap();
    let BodyBlock::Paragraph(tail) = doc.body().last().unwrap() else {
        panic!("expected tail paragraph");
    };
    let ImportedInlineNode::Run(run) = &tail.inline_structure[0] else {
        panic!("expected tail run");
    };
    let mut edits = vec![TableStructureEdit {
        target: table.anchor.clone(),
        operation: TableOperation::ReplaceCell {
            row: 0,
            cell: 0,
            blocks: rich_blocks(),
        },
    }];
    if duplicate {
        edits.push(edits[0].clone());
    }
    let result = doc.preview_edits(EditBatch {
        structural: &edits,
        run_text: &[RunTextEdit {
            target: run.anchor.clone(),
            text: "Edited tail 😀".into(),
        }],
        ..EditBatch::default()
    });
    if duplicate {
        fs::write(output, result.err().unwrap().to_string()).unwrap();
    } else {
        fs::write(output, result.unwrap().to_docx().unwrap()).unwrap();
    }
}

fn property_donor() -> ResolvedTable {
    let color = [1, 2, 3];
    let cell = TableCell::new(vec![
        Paragraph::from_children(vec![
            Hyperlink::new(
                HyperlinkTarget::external("https://example.org/donor-only").unwrap(),
                vec![TextRun::new("not copied").into()],
            )
            .into(),
        ])
        .into(),
    ])
    .with_properties(TableCellProperties {
        grid_span: Some(2),
        vmerge: Some(VerticalMerge::Restart),
        background_color: Some(color),
        vertical_alignment: Some(VerticalAlignment::Bottom),
        text_direction: Some(CellTextDirection::TopToBottomRightToLeft),
        margins: Some(CellMargins {
            top: Some(4.5),
            start: Some(3.0),
            bottom: Some(5.0),
            end: Some(8.0),
        }),
        borders: [
            CellBorderSide::Top,
            CellBorderSide::Bottom,
            CellBorderSide::Start,
            CellBorderSide::End,
        ]
        .map(|side| CellBorder {
            side,
            sz: 8,
            color,
            val: BorderLine::Dashed,
        })
        .into(),
        content_control: None,
    });
    resolve(
        Table::new(vec![
            TableRow::new(vec![cell.into()])
                .with_properties(TableRowProperties {
                    height: Some(RowHeight::AtLeast(16.0)),
                    repeat_header: Some(false),
                    cant_split: Some(true),
                    content_control: None,
                })
                .into(),
        ])
        .with_properties(TableProperties {
            column_widths: Some(vec![2100, 2300]),
            layout: Some(TableLayout::Fixed),
            style: Some("Table Grid".into()),
            alignment: Some(TableAlignment::Center),
            look: Some(TableLook {
                first_row: Some(false),
                last_row: Some(true),
                first_column: Some(false),
                last_column: Some(true),
                horizontal_banding: Some(false),
                vertical_banding: Some(true),
            }),
            borders: [
                BorderSide::Top,
                BorderSide::Left,
                BorderSide::Bottom,
                BorderSide::Right,
                BorderSide::InsideH,
                BorderSide::InsideV,
            ]
            .map(|side| TableBorder {
                side,
                sz: 8,
                color,
                val: BorderLine::Single,
            })
            .into(),
        }),
    )
}

fn properties(doc: &ImportedDocument, case: &str, owner: &str, output: &str) {
    let anchors: Vec<_> = if owner == "ordinary" {
        doc.body()
            .iter()
            .filter_map(|block| match block {
                BodyBlock::Table(table) => Some(table.anchor.clone()),
                _ => None,
            })
            .collect()
    } else {
        doc.part_tables(Some(owner))
            .unwrap()
            .into_iter()
            .map(|table| table.anchor)
            .collect()
    };
    assert_eq!(anchors.len(), 2);
    let donor = property_donor();
    let mut edits = Vec::new();
    let mut push = |table: usize, site, field: &str, set: bool| {
        edits.push(TableStructureEdit {
            target: anchors[table].clone(),
            operation: TableOperation::Property {
                site,
                field: field.into(),
                source: set.then(|| Box::new(donor.clone())),
            },
        });
    };
    let set = case == "donors-set";
    match case {
        "donors-opaque-border" => push(0, TableSite::Cell(0, 1), "border.top", false),
        "donors-opaque-look" => push(1, TableSite::Table, "look.firstRow", false),
        "donors-set" | "donors-clear" => {
            for (site, fields) in [
                (
                    TableSite::Table,
                    &[
                        "style",
                        "alignment",
                        "layout",
                        "look.firstRow",
                        "look.lastRow",
                        "look.firstColumn",
                        "look.lastColumn",
                        "look.horizontalBanding",
                        "look.verticalBanding",
                        "border.top",
                        "border.left",
                        "border.bottom",
                        "border.right",
                        "border.insideH",
                        "border.insideV",
                    ][..],
                ),
                (TableSite::Row(0), &["height", "repeatHeader", "cantSplit"]),
                (
                    TableSite::Cell(0, 0),
                    &[
                        "verticalMerge",
                        "backgroundColor",
                        "verticalAlignment",
                        "textDirection",
                        "margin.top",
                        "margin.start",
                        "margin.bottom",
                        "margin.end",
                        "border.top",
                        "border.bottom",
                        "border.start",
                        "border.end",
                    ],
                ),
                (TableSite::Column(0), &["width"]),
            ] {
                for field in fields {
                    push(0, site, field, set);
                }
            }
            // Only one span can grow while the other shrinks on this row.
            push(1, TableSite::Cell(0, 0), "gridSpan", set);
            push(1, TableSite::Cell(0, 1), "gridSpan", !set);
            if set {
                push(0, TableSite::Cell(0, 1), "border.top", true);
            }
        }
        _ => panic!("unknown donor case"),
    }
    let result = if owner == "ordinary" {
        doc.preview_edits(EditBatch {
            structural: &edits,
            ..EditBatch::default()
        })
        .and_then(|preview| preview.to_docx())
    } else {
        doc.edit_part_tables(owner, &edits)
    };
    if case.starts_with("donors-opaque-") {
        fs::write(output, result.unwrap_err().to_string()).unwrap();
    } else {
        fs::write(output, result.unwrap()).unwrap();
    }
}

fn declaration_edges(doc: &ImportedDocument) -> Vec<u8> {
    use CellTextDirection::*;
    let cells = [
        LeftToRight,
        BottomToTopLeftToRight,
        LeftToRightVertical,
        TopToBottomRightToLeftVertical,
        TopToBottomLeftToRightVertical,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, direction)| {
        let mut properties = TableCellProperties::default().with_text_direction(direction);
        if index == 0 {
            properties = properties.with_margins(CellMargins::default().with_start(0.0));
        }
        TableCell::new(vec![]).with_properties(properties).into()
    })
    .collect();
    let first = Table::new(vec![
        TableRow::new(cells)
            .with_properties(
                TableRowProperties::default()
                    .with_height(RowHeight::Exact(0.0))
                    .with_repeat_header(false)
                    .with_cant_split(false),
            )
            .into(),
        TableRow::new(vec![]).into(),
    ])
    .with_properties(
        TableProperties::default()
            .with_layout(TableLayout::Autofit)
            .with_column_widths(vec![0])
            .with_alignment(TableAlignment::Start)
            .with_look(TableLook::default().with_first_row(false)),
    );
    let single = |alignment| {
        Table::new(vec![
            TableRow::new(vec![TableCell::new(vec![]).into()]).into(),
        ])
        .with_properties(TableProperties::default().with_alignment(alignment))
    };
    let tables = [
        first,
        single(TableAlignment::End),
        single(TableAlignment::Left),
    ]
    .into_iter()
    .zip(doc.body())
    .map(|(table, block)| {
        let BodyBlock::Paragraph(target) = block else {
            panic!("expected paragraph");
        };
        TableInsertion {
            target: target.anchor.clone(),
            table: resolve(table),
        }
    })
    .collect::<Vec<_>>();
    doc.preview_edits(EditBatch {
        tables: &tables,
        ..EditBatch::default()
    })
    .unwrap()
    .to_docx()
    .unwrap()
}

fn property_edges(case: &str) -> Vec<TableOperation> {
    let donor = |cell: TableCell, properties| {
        resolve(
            Table::new(vec![TableRow::new(vec![cell.into()]).into()]).with_properties(properties),
        )
    };
    let put = |site, field: &str, source| TableOperation::Property {
        site,
        field: field.into(),
        source: Some(Box::new(source)),
    };
    let continuation = || {
        donor(
            TableCell::new(vec![]).with_properties(
                TableCellProperties::default().with_vmerge(VerticalMerge::Continue),
            ),
            TableProperties::default(),
        )
    };
    match case {
        "edge-properties" => vec![
            put(
                TableSite::Table,
                "look.firstRow",
                donor(
                    TableCell::new(vec![]),
                    TableProperties::default().with_look(TableLook::default().with_first_row(true)),
                ),
            ),
            put(TableSite::Cell(1, 0), "verticalMerge", continuation()),
        ],
        "edge-guard-merge" => vec![put(TableSite::Cell(1, 1), "verticalMerge", continuation())],
        "edge-guard-look" => vec![put(
            TableSite::Table,
            "look.firstRow",
            donor(TableCell::new(vec![]), TableProperties::default()),
        )],
        "edge-empty-row-donor" | "edge-control-donor" => {
            let mut row = TableRowProperties::default().with_height(RowHeight::Exact(0.0));
            let cells = if case == "edge-control-donor" {
                let control = ContentControlProperties::default().with_tag("donor-only");
                row = row.with_content_control(control.clone());
                vec![
                    TableCell::new(vec![])
                        .with_properties(
                            TableCellProperties::default().with_content_control(control),
                        )
                        .into(),
                ]
            } else {
                vec![]
            };
            let source = resolve(Table::new(vec![
                TableRow::new(cells).with_properties(row).into(),
            ]));
            vec![put(TableSite::Row(0), "height", source)]
        }
        _ => unreachable!(),
    }
}

fn main() {
    let [source, output, case, owner]: [String; 4] =
        env::args().skip(1).collect::<Vec<_>>().try_into().unwrap();
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    if case.starts_with("donors-") {
        properties(&doc, &case, &owner, &output);
        return;
    }
    if case == "body-mixed" {
        fs::write(output, body_mixed(&doc)).unwrap();
        return;
    }
    if matches!(
        case.as_str(),
        "body-replace-cell" | "body-replace-cell-duplicate"
    ) {
        body_replace_cell(&doc, case.ends_with("-duplicate"), &output);
        return;
    }
    if case == "declaration-edges" {
        fs::write(output, declaration_edges(&doc)).unwrap();
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
        case if case.starts_with("edge-") => property_edges(case),
        _ => panic!("unknown case"),
    };
    let edits: Vec<_> = operations
        .into_iter()
        .map(|operation| TableStructureEdit {
            target: target.anchor.clone(),
            operation,
        })
        .collect();
    let result = doc.edit_part_tables(&target.part_name, &edits);
    if case.starts_with("edge-guard-") {
        fs::write(output, result.unwrap_err().to_string()).unwrap();
    } else {
        fs::write(output, result.unwrap()).unwrap();
    }
}
