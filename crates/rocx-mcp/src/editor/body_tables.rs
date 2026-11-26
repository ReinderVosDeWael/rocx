use rocx::editor::{BodyBlock, ImportedRowChild, ImportedTableChild, TableStructureEdit};
use serde_json::{Value, json};

use super::Snapshot;
use crate::schema::tables::BodyTableEdit;

const PREFIX: &str = "body-table:";

impl Snapshot {
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
                let rows = table
                    .children
                    .iter()
                    .filter_map(|node| {
                        let ImportedTableChild::Row(row) = node else {
                            return None;
                        };
                        let cells = row
                            .children
                            .iter()
                            .filter_map(|node| {
                                let ImportedRowChild::Cell(cell) = node else {
                                    return None;
                                };
                                Some(&cell.properties)
                            })
                            .collect::<Vec<_>>();
                        Some(json!({"properties": row.properties, "cells": cells}))
                    })
                    .collect::<Vec<_>>();
                json!({
                    "anchor": format!("{PREFIX}{index}"),
                    "body_index": index,
                    "grid_widths": table.grid_widths,
                    "properties": table.properties,
                    "rows": rows,
                })
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
        let table = local
            .strip_prefix(PREFIX)
            .and_then(|s| s.parse::<usize>().ok())
            .and_then(|index| self.document.body().get(index));
        let Some(BodyBlock::Table(table)) = table else {
            return Err("wrong_anchor_type: expected a body-tables anchor".into());
        };
        Ok(TableStructureEdit {
            target: table.anchor.clone(),
            operation: edit.resolve()?,
        })
    }
}
