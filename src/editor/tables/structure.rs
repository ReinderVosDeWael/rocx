//! Structural edits share the alignment editor's validated grid. Geometry
//! edits are exclusive per table; cell replacements may be grouped.
use super::{trailing, validate};
use crate::editor::{
    ALTERNATE, CellAlignmentEdit, TableAnchor, insertion::BoundaryState, unsupported,
};
use crate::{
    Result,
    opc::Package,
    xml::{Content, Node},
};
use std::collections::{BTreeMap, BTreeSet};

/// Structural operation at an original snapshot coordinate. Insertions are before
/// the supplied index (the count appends). Column operations edit every row.
#[derive(Clone, Debug)]
pub enum TableOperation {
    /// Set or clear direct table layout without recalculating widths.
    Layout(Option<crate::TableLayout>),
    InsertRow(usize),
    RemoveRow(usize),
    InsertCell {
        row: usize,
        cell: usize,
    },
    RemoveCell {
        row: usize,
        cell: usize,
    },
    InsertColumn(usize),
    RemoveColumn(usize),
    ReplaceCell {
        row: usize,
        cell: usize,
        blocks: Vec<crate::ResolvedBlock>,
    },
}

#[derive(Clone, Debug)]
pub struct TableStructureEdit {
    pub target: TableAnchor,
    pub operation: TableOperation,
}

fn slots(parent: &Node, name: &str) -> Vec<usize> {
    parent
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, c)| matches!(c,Content::Element(n) if n.name==name).then_some(i))
        .collect()
}
fn element(c: &mut Content) -> &mut Node {
    let Content::Element(n) = c else {
        unreachable!()
    };
    n
}
fn blank() -> Node {
    Node::new("w:tc").with(Node::new("w:p"))
}
fn prop<'a>(cell: &'a Node, name: &str) -> Option<&'a Node> {
    cell.child("w:tcPr")?.child(name)
}
// Only used after validate() has checked the numeric value and interval.
fn span(c: &Node) -> usize {
    prop(c, "w:gridSpan").map_or(1, |n| n.get("w:val").unwrap().parse().unwrap())
}
fn absent() -> crate::Error {
    unsupported("table structure index out of bounds")
}
fn collision() -> crate::Error {
    unsupported("table structure edit would split or destroy a merge")
}

// Only discard understood plain content, whitespace and basic geometry. This
// whitelist deliberately excludes relationships, revisions and foreign markup.
fn disposable(n: &Node) -> bool {
    let attrs: &[&str] = match n.name.as_str() {
        "w:tr" | "w:trPr" | "w:tc" | "w:tcPr" | "w:p" | "w:r" => &[],
        "w:t" => &["xml:space"],
        "w:tcW" => &["w:w", "w:type"],
        "w:gridCol" => &["w:w"],
        "w:gridAfter" | "w:vAlign" => &["w:val"],
        _ => return false,
    };
    n.attrs
        .iter()
        .all(|(a, _)| a.declaration().is_some() || attrs.contains(&a.as_str()))
        && n.children.iter().all(|c| match c {
            Content::Element(e) => disposable(e),
            Content::Text(t) => n.name == "w:t" || t.trim().is_empty(),
            Content::Raw(_) => false,
        })
}
fn guard_delete(n: &Node) -> Result<()> {
    if disposable(n) {
        Ok(())
    } else {
        Err(unsupported("table edit would discard unsupported markup"))
    }
}

// A short physical row accounts for omitted *logical* columns in trPr/gridAfter.
fn set_trailing(row: &mut Node, value: usize) -> Result<()> {
    if row
        .child("w:trPr")
        .is_some_and(|pr| pr.child("w:wAfter").is_some())
    {
        return Err(unsupported("explicit trailing row width"));
    }
    if row.child("w:trPr").is_none() {
        row.children
            .insert(0, Content::Element(Node::new("w:trPr")))
    }
    row.child_mut("w:trPr")
        .unwrap()
        .ensure("w:gridAfter")
        .set("w:val", value);
    Ok(())
}

fn geometry(table: &mut Node, op: &TableOperation) -> Result<()> {
    let rows = slots(table, "w:tr");
    let cols = table.child("w:tblGrid").unwrap().elements().count();
    if rows.is_empty() {
        return Err(unsupported("empty source table"));
    }
    match *op {
        TableOperation::Layout(_) => unreachable!(),
        TableOperation::InsertCell { row: r, cell: i }
        | TableOperation::RemoveCell { row: r, cell: i } => {
            let insert = matches!(op, TableOperation::InsertCell { .. });
            let target = *rows.get(r).ok_or_else(absent)?;
            let row = element(&mut table.children[target]);
            let cells = slots(row, "w:tc");
            if i > cells.len() || (!insert && i == cells.len()) {
                return Err(absent());
            }
            if !insert && cells.len() == 1 {
                return Err(unsupported("cannot remove final row cell"));
            }
            // Only the suffix shifts relative to the grid. Never move or remove
            // one end of a vertical merge independently of the other rows.
            for &s in &cells[i..] {
                if prop(element(&mut row.children[s]), "w:vMerge").is_some() {
                    return Err(collision());
                }
            }
            let slot = cells.get(i).copied().unwrap_or(row.children.len());
            let after = trailing(row)?;
            if insert {
                row.children.insert(slot, Content::Element(blank()));
                if after > 0 {
                    set_trailing(row, after - 1)?
                } else {
                    table
                        .child_mut("w:tblGrid")
                        .unwrap()
                        .push(Node::new("w:gridCol"));
                    for other in rows.into_iter().filter(|&s| s != target) {
                        let row = element(&mut table.children[other]);
                        set_trailing(row, trailing(row)? + 1)?
                    }
                }
            } else {
                let c = element(&mut row.children[slot]);
                if prop(c, "w:gridSpan").is_some() {
                    return Err(collision());
                }
                guard_delete(c)?;
                row.children.remove(slot);
                set_trailing(row, after + 1)?;
            }
        }
        TableOperation::InsertRow(i) => {
            if i > rows.len() {
                return Err(absent());
            }
            // At this cut, the cells continuing in the *following* row
            // identify exactly which grid intervals must continue through ours.
            let mut row = Node::new("w:tr");
            let mut column = 0;
            if let Some(&s) = rows.get(i) {
                for c in element(&mut table.children[s])
                    .elements()
                    .filter(|c| c.name == "w:tc")
                {
                    let span = span(c);
                    if prop(c, "w:vMerge").is_some_and(|m| m.get("w:val") != Some("restart")) {
                        let mut pr = Node::new("w:tcPr");
                        if span > 1 {
                            pr.push(Node::new("w:gridSpan").attr("w:val", span))
                        }
                        pr.push(Node::new("w:vMerge"));
                        row.push(Node::new("w:tc").with(pr).with(Node::new("w:p")));
                    } else {
                        for _ in 0..span {
                            row.push(blank())
                        }
                    }
                    column += span;
                }
            }
            for _ in column..cols {
                row.push(blank())
            }
            table.children.insert(
                rows.get(i).copied().unwrap_or(table.children.len()),
                Content::Element(row),
            );
        }
        TableOperation::RemoveRow(i) => {
            let slot = *rows.get(i).ok_or_else(absent)?;
            if rows.len() == 1 {
                return Err(unsupported("cannot remove final table row"));
            }
            guard_delete(element(&mut table.children[slot]))?;
            table.children.remove(slot);
        }
        TableOperation::InsertColumn(i) | TableOperation::RemoveColumn(i) => {
            let insert = matches!(op, TableOperation::InsertColumn(_));
            if i > cols || (!insert && i == cols) {
                return Err(absent());
            }
            if !insert && cols == 1 {
                return Err(unsupported("cannot remove final table column"));
            }
            for r in rows {
                let row = element(&mut table.children[r]);
                if trailing(row)? != 0 {
                    return Err(unsupported("whole-column edits require full rows"));
                }
                let mut logical = 0;
                let mut slot = row.children.len();
                for s in slots(row, "w:tc") {
                    let cell = element(&mut row.children[s]);
                    let span = span(cell);
                    if logical == i {
                        if !insert && (span != 1 || prop(cell, "w:vMerge").is_some()) {
                            return Err(collision());
                        }
                        slot = s;
                        break;
                    }
                    logical += span;
                    if logical > i {
                        return Err(collision());
                    }
                }
                if insert {
                    row.children.insert(slot, Content::Element(blank()))
                } else {
                    guard_delete(element(&mut row.children[slot]))?;
                    row.children.remove(slot);
                }
            }
            let grid = table.child_mut("w:tblGrid").unwrap();
            let slot = slots(grid, "w:gridCol")
                .get(i)
                .copied()
                .unwrap_or(grid.children.len());
            if insert {
                grid.children
                    .insert(slot, Content::Element(Node::new("w:gridCol")))
            } else {
                guard_delete(element(&mut grid.children[slot]))?;
                grid.children.remove(slot);
            }
        }
        TableOperation::ReplaceCell { .. } => unreachable!(),
    }
    validate(table, true)
}

fn layout(table: &mut Node, value: Option<crate::TableLayout>) -> Result<bool> {
    if table.child("w:tblPr").is_none() {
        if value.is_none() {
            return Ok(false);
        }
        table
            .children
            .insert(0, Content::Element(Node::new("w:tblPr")));
    }
    let pr = table.child_mut("w:tblPr").unwrap();
    if pr.child("w:tblPrChange").is_some() {
        return Err(unsupported("tracked table properties"));
    }
    let existing = super::unique(pr, "w:tblLayout")?;
    if let Some(layout) = value {
        if existing.is_some_and(|n| n.get("w:type") == Some(layout.xml())) {
            return Ok(false);
        }
        pr.ensure("w:tblLayout").set("w:type", layout.xml());
    } else if let Some(existing) = existing {
        if existing
            .attrs
            .iter()
            .any(|(k, _)| k.declaration().is_none() && k != "w:type")
            || existing
                .children
                .iter()
                .any(|c| !matches!(c, Content::Text(t) if t.trim().is_empty()))
        {
            return Err(unsupported(
                "clearing layout would discard unsupported markup",
            ));
        }
        pr.remove("w:tblLayout");
    } else {
        return Ok(false);
    }
    Ok(true)
}

pub(in crate::editor) fn apply(
    body: &mut Node,
    mut package: Package,
    main: &str,
    edits: &[TableStructureEdit],
    alignments: &[CellAlignmentEdit],
) -> Result<(Package, bool)> {
    if edits.is_empty() {
        return Ok((package, false));
    }
    let mut grouped = BTreeMap::<usize, Vec<&TableOperation>>::new();
    for e in edits {
        grouped
            .entry(e.target._body_index)
            .or_default()
            .push(&e.operation)
    }
    let mut replacements = BTreeMap::new();
    let mut changed = false;
    let body_slots: Vec<_> = body
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, c)| matches!(c,Content::Element(n) if n.name!="w:sectPr").then_some(i))
        .collect();
    let mut ranges = BoundaryState::default();
    for (i, table) in body
        .elements_mut()
        .filter(|n| n.name != "w:sectPr")
        .enumerate()
    {
        let safe = ranges.safe();
        ranges.advance(table);
        let Some(ops) = grouped.remove(&i) else {
            continue;
        };
        if !safe || !ranges.safe() || contains_range(table) {
            return Err(unsupported("table intersects protected range"));
        }
        validate(table, true)?;
        let mut seen = BTreeSet::new();
        let exclusive = ops.len() != 1 || alignments.iter().any(|e| e.target._body_index == i);
        for op in ops {
            match op {
                TableOperation::Layout(value) => {
                    if exclusive {
                        return Err(unsupported("layout edit requires exclusive table target"));
                    }
                    changed |= layout(table, *value)?;
                }
                TableOperation::ReplaceCell { row, cell, blocks } => {
                    if !seen.insert((*row, *cell)) {
                        return Err(unsupported("duplicate cell replacement"));
                    }
                    let r_slot = *slots(table, "w:tr").get(*row).ok_or_else(absent)?;
                    let r = element(&mut table.children[r_slot]);
                    let c_slot = *slots(r, "w:tc").get(*cell).ok_or_else(absent)?;
                    let c = element(&mut r.children[c_slot]);
                    if prop(c, "w:vMerge").is_some_and(|m| m.get("w:val") != Some("restart")) {
                        return Err(collision());
                    }
                    for b in &c.children {
                        match b {
                            Content::Element(p) if p.name == "w:tcPr" => (),
                            Content::Element(p) if p.name == "w:p" => guard_delete(p)?,
                            Content::Text(s) if s.trim().is_empty() => (),
                            _ => return Err(unsupported("unsupported existing cell content")),
                        }
                    }
                    replacements.insert((body_slots[i], r_slot, c_slot), blocks);
                    changed = true;
                }
                _ => {
                    if exclusive {
                        return Err(unsupported("geometry edit requires exclusive table target"));
                    }
                    geometry(table, op)?;
                    changed = true;
                }
            }
        }
    }
    if !grouped.is_empty() {
        return Err(absent());
    }
    // Validate all geometry and all destructive targets before allocating any
    // relationships. Source order also makes allocation independent of batch order.
    for ((t, r, c), blocks) in replacements {
        let table = element(&mut body.children[t]);
        let row = element(&mut table.children[r]);
        let cell = element(&mut row.children[c]);
        let width = prop(cell, "w:tcW")
            .filter(|n| n.get("w:type") == Some("dxa"))
            .and_then(|n| n.get("w:w")?.parse::<u32>().ok())
            .filter(|w| *w > 0)
            .map(i64::from);
        let (next, nodes) =
            crate::ooxml::encode_editor_blocks(package, main, blocks, width, "editor.table")?;
        package = next;
        cell.remove("w:p");
        let needs_paragraph = nodes.last().is_none_or(|n| n.name != "w:p");
        for n in nodes {
            cell.push(n)
        }
        if needs_paragraph {
            cell.push(Node::new("w:p"))
        }
    }
    Ok((package, changed))
}
fn contains_range(n: &Node) -> bool {
    let s = n.name.as_str();
    s == "w:fldChar"
        // A proof range can open and close in different cells while leaving
        // BoundaryState safe at both edges of the table.
        || s == "w:proofErr"
        || s == ALTERNATE
        || s.ends_with("Start")
        || s.ends_with("End")
        || n.elements().any(contains_range)
}
