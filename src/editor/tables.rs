//! Conservative edits to physical cells in direct body tables.
mod properties;
mod structure;
pub(super) use properties::inspect;
pub use properties::{DirectTableProperties, TableSite};
pub(super) use structure::apply as structure;
pub use structure::{TableOperation, TableStructureEdit};

use super::{ALTERNATE, TableAnchor, unsupported};
use crate::xml::{Content, Node};
use crate::{Result, VerticalAlignment};
use std::collections::{BTreeMap, BTreeSet};

/// Patch one physical cell. Indices include opaque row and cell children.
#[derive(Clone, Debug)]
pub struct CellAlignmentEdit {
    pub target: TableAnchor,
    pub row_index: usize,
    pub cell_index: usize,
    pub alignment: VerticalAlignment,
}

/// Direct value changed on a physical cell; no inherited value is inferred.
#[derive(Clone, Debug)]
pub struct CellAlignmentChange {
    pub body_index: usize,
    pub row_index: usize,
    pub cell_index: usize,
    pub before: Option<String>,
    pub after: VerticalAlignment,
}

fn unique<'a>(node: &'a Node, name: &str) -> Result<Option<&'a Node>> {
    let mut found = node.elements().filter(|n| n.name == name);
    let first = found.next();
    if found.next().is_some() {
        return Err(unsupported("ambiguous table properties"));
    }
    Ok(first)
}

fn validate(table: &Node, structural: bool) -> Result<()> {
    let bad = || unsupported("unsupported or malformed table grid or merge");
    if table.name != "w:tbl" || table.child(ALTERNATE).is_some() {
        return Err(bad());
    }
    unique(table, "w:tblPr")?;
    let grid = unique(table, "w:tblGrid")?.ok_or_else(bad)?;
    let columns = grid.elements().filter(|n| n.name == "w:gridCol").count();
    if columns == 0 || grid.elements().any(|n| n.name != "w:gridCol") {
        return Err(bad());
    }
    let mut active = BTreeSet::new();
    for row in table
        .elements()
        .filter(|n| n.name != "w:tblPr" && n.name != "w:tblGrid")
    {
        if row.name != "w:tr" || row.child(ALTERNATE).is_some() {
            return Err(bad());
        }
        if let Some(pr) = unique(row, "w:trPr")?
            && pr.elements().any(|n| {
                matches!(n.name.as_str(), "w:gridBefore" | "w:trPrChange")
                    || n.name == ALTERNATE
                    || (structural && matches!(n.name.as_str(), "w:ins" | "w:del"))
                    || (n.name == "w:gridAfter" && !structural)
            })
        {
            return Err(bad());
        }
        let mut next = BTreeSet::new();
        let mut column = 0usize;
        for cell in row.elements().filter(|n| n.name != "w:trPr") {
            if cell.name != "w:tc" || cell.child(ALTERNATE).is_some() {
                return Err(bad());
            }
            let pr = unique(cell, "w:tcPr")?;
            if let Some(pr) = pr
                && pr.elements().any(|n| {
                    matches!(
                        n.name.as_str(),
                        "w:hMerge" | "w:tcPrChange" | "w:cellMerge" | "w:cellIns" | "w:cellDel"
                    ) || n.name == ALTERNATE
                })
            {
                return Err(bad());
            }
            let span = match pr.map(|p| unique(p, "w:gridSpan")).transpose()?.flatten() {
                None => 1,
                Some(n) => n
                    .get("w:val")
                    .and_then(|v| v.parse::<usize>().ok())
                    .filter(|v| *v > 0)
                    .ok_or_else(bad)?,
            };
            let end = column
                .checked_add(span)
                .filter(|end| *end <= columns)
                .ok_or_else(bad)?;
            if let Some(pr) = pr {
                unique(pr, "w:vAlign")?;
                if structural {
                    unique(pr, "w:tcW")?;
                }
                if let Some(merge) = unique(pr, "w:vMerge")? {
                    match merge.get("w:val") {
                        Some("restart") => {
                            next.insert((column, end));
                        }
                        None | Some("continue") if active.contains(&(column, end)) => {
                            next.insert((column, end));
                        }
                        _ => return Err(bad()),
                    }
                }
            }
            column = end;
        }
        if column == 0 || column.checked_add(trailing(row)?) != Some(columns) {
            return Err(bad());
        }
        active = next;
    }
    Ok(())
}

pub(super) fn apply(
    body: &mut Node,
    edits: &[CellAlignmentEdit],
) -> Result<Vec<CellAlignmentChange>> {
    if edits.is_empty() {
        return Ok(Vec::new());
    }
    let mut targets = BTreeMap::new();
    for edit in edits {
        if targets
            .insert(
                (edit.target._body_index, edit.row_index, edit.cell_index),
                edit.alignment,
            )
            .is_some()
        {
            return Err(unsupported("duplicate cell alignment target"));
        }
    }
    let mut changes = Vec::new();
    for (body_index, table) in body
        .elements_mut()
        .filter(|n| n.name != "w:sectPr")
        .enumerate()
    {
        let Some((&(next_body, _, _), _)) = targets.first_key_value() else {
            break;
        };
        if next_body != body_index {
            continue;
        }
        validate(table, false)?;
        for (row_index, row) in table
            .elements_mut()
            .filter(|n| n.name != "w:tblPr" && n.name != "w:tblGrid")
            .enumerate()
        {
            for (cell_index, cell) in row
                .elements_mut()
                .filter(|n| n.name != "w:trPr")
                .enumerate()
            {
                let Some(after) = targets.remove(&(body_index, row_index, cell_index)) else {
                    continue;
                };
                if cell.child("w:tcPr").is_none() {
                    cell.children
                        .insert(0, Content::Element(Node::new("w:tcPr")));
                }
                let props = cell.child_mut("w:tcPr").unwrap();
                let align = props.ensure("w:vAlign");
                let value = match after {
                    VerticalAlignment::Top => "top",
                    VerticalAlignment::Center => "center",
                    VerticalAlignment::Bottom => "bottom",
                    VerticalAlignment::Both => "both",
                };
                if align.get("w:val") != Some(value) {
                    let before = align.get("w:val").map(str::to_owned);
                    align.set("w:val", value);
                    changes.push(CellAlignmentChange {
                        body_index,
                        row_index,
                        cell_index,
                        before,
                        after,
                    });
                }
            }
        }
    }
    if !targets.is_empty() {
        return Err(unsupported("cell alignment target not found"));
    }
    Ok(changes)
}

fn trailing(row: &Node) -> Result<usize> {
    let Some(pr) = row.child("w:trPr") else {
        return Ok(0);
    };
    unique(pr, "w:gridAfter")?.map_or(Ok(0), |n| {
        n.get("w:val")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| unsupported("invalid trailing table grid"))
    })
}
