//! A09–A10: table authoring, standalone resolution and physical grid views.
mod properties;

use crate::{
    boundary::{self as b, Failure, Pass, Paths, Record},
    host::Wire,
    program::{self, Hooks, Projection, Resolve},
};
use futures_util::future::BoxFuture;
use js_sys::Array;
use rocx::{
    Block, ResolvedBlock, ResolvedTable, ResolvedTableCell, ResolvedTableRow, Result, Table,
    TableCell, TableRow,
};
use wasm_bindgen::JsValue;

pub(crate) const HOOKS: Hooks = Hooks {
    components: &[
        ("table", None, resolve_table as Resolve),
        ("tableRow", None, resolve_row),
        ("tableCell", None, resolve_cell),
    ],
    blocks: &[("table", block)],
    block_views: &[block_view],
    operations: &[
        (
            "validateValue:tableRowProperties",
            (properties::validate_row, Paths::Declaration),
        ),
        (
            "validateValue:tableCellProperties",
            (properties::validate_cell, Paths::Declaration),
        ),
        (
            "validateValue:cellMargins",
            (properties::validate_margins, Paths::Declaration),
        ),
    ],
    ..Hooks::EMPTY
};

fn component_record(wire: &Wire, path: &str, tag: &str, children: &str) -> Result<Record> {
    let record = Record::new(wire, path, &["kind", children, "properties", "condition"])?;
    if b::string(&record.get("kind")?, &record.path("kind"))? != tag {
        return Err(Failure::invalid(
            &record.path("kind"),
            format!("expected `{tag}`"),
        ));
    }
    Ok(record)
}

fn table(wire: &Wire, pass: &Pass, path: &str) -> Result<Table> {
    let record = component_record(wire, path, "table", "rows")?;
    Ok(Table {
        condition: pass.condition(record.get("condition")?, record.path("condition"))?,
        rows: program::required(&record, pass, "rows", |wire, pass, path| {
            program::values(wire, pass, path, row)
        })?,
        properties: program::optional(&record, pass, "properties", properties::table)?
            .unwrap_or_default(),
    })
}

fn row(wire: &Wire, pass: &Pass, path: &str) -> Result<TableRow> {
    let record = component_record(wire, path, "tableRow", "children")?;
    Ok(TableRow {
        condition: pass.condition(record.get("condition")?, record.path("condition"))?,
        children: program::required(&record, pass, "children", |wire, pass, path| {
            program::values(wire, pass, path, cell)
        })?,
        properties: program::optional(&record, pass, "properties", properties::row)?
            .unwrap_or_default(),
    })
}

fn cell(wire: &Wire, pass: &Pass, path: &str) -> Result<TableCell> {
    let record = component_record(wire, path, "tableCell", "children")?;
    Ok(TableCell {
        condition: pass.condition(record.get("condition")?, record.path("condition"))?,
        children: program::required(&record, pass, "children", program::blocks)?,
        properties: program::optional(&record, pass, "properties", properties::cell)?
            .unwrap_or_default(),
    })
}

fn block(wire: &Wire, pass: &Pass, path: &str) -> Result<Block> {
    table(wire, pass, path).map(Block::from)
}

fn block_view(block: &ResolvedBlock) -> Option<JsValue> {
    if let ResolvedBlock::Table(table) = block {
        Some(table_handle(table.clone()))
    } else {
        None
    }
}

fn resolve_table(wire: Wire, pass: Pass) -> BoxFuture<'static, Result<Option<Projection>>> {
    Box::pin(async move { program::resolve(table(&wire, &pass, "table")?, table_handle).await })
}

fn resolve_row(wire: Wire, pass: Pass) -> BoxFuture<'static, Result<Option<Projection>>> {
    Box::pin(async move { program::resolve(row(&wire, &pass, "tableRow")?, row_handle).await })
}

fn resolve_cell(wire: Wire, pass: Pass) -> BoxFuture<'static, Result<Option<Projection>>> {
    Box::pin(async move { program::resolve(cell(&wire, &pass, "tableCell")?, cell_handle).await })
}

fn table_handle(table: ResolvedTable) -> JsValue {
    let view = b::record(&[
        ("columns", (table.columns() as u32).into()),
        ("properties", properties::table_view(&table)),
        (
            "rows",
            table
                .rows()
                .iter()
                .cloned()
                .map(row_handle)
                .collect::<Array>()
                .into(),
        ),
    ]);
    // Also borrowed as ResolvedTable by the editor's insert-table operation.
    b::handle("table", table, view)
}

fn row_handle(row: ResolvedTableRow) -> JsValue {
    let view = b::record(&[
        ("properties", properties::row_view(row.properties())),
        (
            "cells",
            row.cells()
                .iter()
                .cloned()
                .map(cell_handle)
                .collect::<Array>()
                .into(),
        ),
    ]);
    b::handle("tableRow", row, view)
}

fn cell_handle(cell: ResolvedTableCell) -> JsValue {
    let view = properties::cell_view(&cell);
    b::set(
        &view,
        "blocks",
        &cell
            .blocks()
            .iter()
            .cloned()
            .map(program::block_handle)
            .collect::<Array>()
            .into(),
    );
    b::handle("tableCell", cell, view)
}
