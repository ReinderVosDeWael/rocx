use rocx::editor::{
    BodyBlock, ImportedRowChild, ImportedTable, ImportedTableChild, TableStructureEdit,
};
use serde_json::{Value, json};

use super::{
    Snapshot,
    block_readback::table_fields,
    body_inlines::{InlineHandles, typed_inline_page},
};
use crate::schema::tables::BodyTableEdit;

const PREFIX: &str = "body-table:";

fn rows(table: &ImportedTable) -> impl Iterator<Item = &rocx::editor::ImportedTableRow> {
    table.children.iter().filter_map(|node| match node {
        ImportedTableChild::Row(row) => Some(row),
        _ => None,
    })
}

fn cells(
    row: &rocx::editor::ImportedTableRow,
) -> impl Iterator<Item = &rocx::editor::ImportedTableCell> {
    row.children.iter().filter_map(|node| match node {
        ImportedRowChild::Cell(cell) => Some(cell),
        _ => None,
    })
}

impl Snapshot {
    pub fn body_table_index(local: &str) -> Option<usize> {
        local.strip_prefix(PREFIX)?.parse().ok()
    }

    pub fn table_cell_inline_page(
        &self,
        table_index: usize,
        (row_index, cell_index): (usize, usize),
        block_index: usize,
        start: usize,
        limit: usize,
        handles: &mut InlineHandles<'_>,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        let Some(BodyBlock::Table(table)) = self.document.body().get(table_index) else {
            return Err("missing body table".into());
        };
        let cell = rows(table)
            .nth(row_index)
            .and_then(|row| cells(row).nth(cell_index))
            .ok_or("row_index or cell_index does not identify a direct cell")?;
        let nodes = cell
            .inline_structure(block_index)
            .ok_or("block_index does not identify a direct cell paragraph")?;
        let source = format!("inline:table:{table_index}:{row_index}:{cell_index}:{block_index}");
        Ok(typed_inline_page(nodes, start, limit, &source, handles))
    }

    pub fn body_table_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        let mut tables = self
            .document
            .body()
            .iter()
            .enumerate()
            .filter_map(|(index, node)| match node {
                BodyBlock::Table(table) => Some((index, table)),
                _ => None,
            })
            .skip(start);
        let rows = tables
            .by_ref()
            .take(limit)
            .map(|(index, table)| {
                // Structural operations count only direct `tr`/`tc` elements.
                // Do not reuse compact alignment indices that include opaque siblings.
                let rows = rows(table)
                    .map(|row| {
                        let cells = cells(row).map(|cell| &cell.properties).collect::<Vec<_>>();
                        json!({"properties": row.properties, "cells": cells})
                    })
                    .collect::<Vec<_>>();
                let mut value = json!({
                    "anchor": format!("{PREFIX}{index}"),
                    "body_index": index,
                    "rows": rows,
                });
                table_fields(table, &mut value);
                value
            })
            .collect::<Vec<_>>();
        let next = tables.next().map(|_| start + rows.len());
        Ok((rows, next))
    }

    pub fn body_table_edit(
        &self,
        local: &str,
        edit: BodyTableEdit,
    ) -> Result<TableStructureEdit, String> {
        let table = Self::body_table_index(local).and_then(|index| self.document.body().get(index));
        let Some(BodyBlock::Table(table)) = table else {
            return Err("wrong_anchor_type: expected a body-tables anchor".into());
        };
        Ok(TableStructureEdit {
            target: table.anchor.clone(),
            operation: edit.resolve()?,
        })
    }
}
