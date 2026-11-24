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
    /// Copy the named direct property from a resolved table (first row/cell
    /// for those sites), or clear it. Unrelated properties are not copied.
    Property {
        site: super::TableSite,
        field: String,
        source: Option<Box<crate::ResolvedTable>>,
    },
    /// Replace a table, physical row, or physical cell from a resolved table.
    /// Row and cell replacements require a one-row or one-cell payload.
    ReplaceNode {
        site: super::TableSite,
        source: Box<crate::ResolvedTable>,
    },
    RemoveTable,
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

// Additional modeled elements a whole-node replacement may deliberately
// discard. Never discard arbitrary attributes, descendants or raw content.
fn discard_node(n: &Node) -> bool {
    if n.name == "w:p" {
        return crate::editor::blocks::local(n, "body");
    }
    if disposable(n) {
        return true;
    }
    let attrs: &[&str] = match n.name.as_str() {
        "w:tbl" | "w:tblGrid" | "w:tblPr" | "w:tr" | "w:trPr" | "w:tc" | "w:tcPr"
        | "w:tblBorders" | "w:tcBorders" | "w:tcMar" => &[],
        "w:tblLook" => &[
            "w:firstRow",
            "w:lastRow",
            "w:firstColumn",
            "w:lastColumn",
            "w:noHBand",
            "w:noVBand",
            "w:val",
        ],
        "w:tblW" | "w:tcW" => &["w:w", "w:type"],
        "w:tblLayout" => &["w:type"],
        "w:shd" => &["w:val", "w:fill", "w:color"],
        "w:tblStyle" | "w:jc" | "w:gridSpan" | "w:vMerge" | "w:textDirection" | "w:cantSplit"
        | "w:tblHeader" => &["w:val"],
        "w:trHeight" => &["w:val", "w:hRule"],
        "w:top" | "w:bottom" | "w:left" | "w:right" | "w:start" | "w:end" | "w:insideH"
        | "w:insideV" => &["w:w", "w:type", "w:val", "w:sz", "w:color"],
        _ => return false,
    };
    n.attrs
        .iter()
        .all(|(k, _)| k.declaration().is_some() || attrs.contains(&k.as_str()))
        && n.children.iter().all(|c| match c {
            Content::Element(e) => e.name != "w:tbl" && discard_node(e),
            Content::Text(s) => s.trim().is_empty(),
            Content::Raw(_) => false,
        })
}

fn has(n: &Node, name: &str) -> bool {
    n.name == name || n.elements().any(|e| has(e, name))
}
fn guard_remove_node(n: &Node) -> Result<()> {
    if !discard_node(n) || has(n, "w:vMerge") {
        return Err(unsupported(
            "table edit would discard unsupported markup or merge",
        ));
    }
    Ok(())
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
        TableOperation::Layout(_)
        | TableOperation::Property { .. }
        | TableOperation::ReplaceNode { .. }
        | TableOperation::RemoveTable => unreachable!(),
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
                guard_remove_node(c)?;
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
            guard_remove_node(element(&mut table.children[slot]))?;
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
                    guard_remove_node(element(&mut row.children[slot]))?;
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

fn validate_rendered(node: &Node) -> Result<()> {
    if node.name == "w:tbl" {
        validate(node, true)?;
    }
    for child in node.elements() {
        validate_rendered(child)?;
    }
    Ok(())
}

fn replace_node(
    package: Package,
    main: &str,
    table: &mut Node,
    site: super::TableSite,
    source: &crate::ResolvedTable,
    section_width: Option<i64>,
) -> Result<Package> {
    use super::TableSite;
    let cell_widths = table
        .child("w:tblGrid")
        .unwrap()
        .elements()
        .map(|c| {
            c.get("w:w")
                .and_then(|v| v.parse::<u32>().ok())
                .map(i64::from)
        })
        .collect::<Vec<_>>();
    let target = super::properties::site_node(table, site)?;
    if !discard_node(target) || (site != TableSite::Table && has(target, "w:vMerge")) {
        return Err(unsupported(
            "replacement would discard unsupported or merged content",
        ));
    }
    let (donor, width) = match site {
        TableSite::Table => (TableSite::Table, section_width.unwrap_or(0)),
        TableSite::Row(_) if source.rows().len() == 1 && source.columns() == cell_widths.len() => (
            TableSite::Row(0),
            cell_widths
                .iter()
                .copied()
                .sum::<Option<i64>>()
                .unwrap_or(0),
        ),
        TableSite::Cell(..)
            if source.rows().len() == 1
                && source.rows()[0].cells().len() == 1
                && source.columns() == span(target) =>
        {
            (
                TableSite::Cell(0, 0),
                prop(target, "w:tcW")
                    .filter(|p| p.get("w:type") == Some("dxa"))
                    .and_then(|p| p.get("w:w")?.parse::<u32>().ok())
                    .map(i64::from)
                    .unwrap_or(0),
            )
        }
        _ => {
            return Err(unsupported(
                "replacement payload does not match target grid",
            ));
        }
    };
    let width = if let Some(w) = source
        .column_widths()
        .filter(|_| source.layout() != Some(crate::TableLayout::Autofit))
    {
        w.iter().copied().map(i64::from).sum()
    } else {
        width
    };
    if width <= 0 {
        return Err(unsupported("replacement needs explicit positive widths"));
    }
    let (next, mut nodes) =
        crate::ooxml::encode_editor_tables(package, main, vec![(source.clone(), width)])?;
    validate_rendered(&nodes[0])?;
    let replacement = super::properties::site_node(&mut nodes[0], donor)?;
    if site != TableSite::Table && has(replacement, "w:vMerge") {
        return Err(collision());
    }
    *target = replacement.clone();
    Ok(next)
}

pub(in crate::editor) fn apply(
    body: &mut Node,
    mut package: Package,
    main: &str,
    edits: &[TableStructureEdit],
    alignments: &[CellAlignmentEdit],
    deletes: &mut BTreeSet<usize>,
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
    use super::TableSite;
    let whole = edits
        .iter()
        .filter_map(|e| match &e.operation {
            TableOperation::ReplaceNode {
                site: TableSite::Table,
                source,
            } => Some((e.target._body_index, source.as_ref())),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let widths: BTreeMap<_, _> = if whole.is_empty() {
        BTreeMap::new()
    } else {
        crate::editor::insertion::table_positions(body, &whole, "w:tbl")?
            .into_iter()
            .map(|(i, _, w)| (i, w))
            .collect()
    };
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
        let Some(mut ops) = grouped.remove(&i) else {
            continue;
        };
        if !safe || !ranges.safe() || contains_range(table) {
            return Err(unsupported("table intersects protected range"));
        }
        validate(table, true)?;
        let mut seen = BTreeSet::new();
        let exclusive = ops.len() != 1 || alignments.iter().any(|e| e.target._body_index == i);
        let properties_only = ops
            .iter()
            .all(|op| matches!(op, TableOperation::Property { .. }));
        if properties_only {
            ops.sort_by_key(|op| match op {
                TableOperation::Property { site, field, .. } => (*site, field),
                _ => unreachable!(),
            });
        }
        let mut patched = BTreeSet::new();
        for op in ops {
            match op {
                TableOperation::Property {
                    site,
                    field,
                    source,
                } => {
                    if !properties_only
                        || !patched.insert((*site, field))
                        || alignments.iter().any(|e| {
                            e.target._body_index == i
                                && *site == TableSite::Cell(e.row_index, e.cell_index)
                                && field == "verticalAlignment"
                        })
                    {
                        return Err(unsupported("duplicate or conflicting table property edit"));
                    }
                    super::properties::patch(
                        table,
                        *site,
                        field,
                        source.as_deref(),
                        &package,
                        main,
                    )?;
                    changed = true;
                }
                TableOperation::ReplaceNode { site, source } => {
                    if exclusive {
                        return Err(unsupported("replacement requires exclusive table target"));
                    }
                    package =
                        replace_node(package, main, table, *site, source, widths.get(&i).copied())?;
                    changed = true;
                }
                TableOperation::RemoveTable => {
                    if exclusive {
                        return Err(unsupported("removal requires exclusive table target"));
                    }
                    if !discard_node(table) {
                        return Err(unsupported("removal would discard unsupported content"));
                    }
                    deletes.insert(body_slots[i]);
                    changed = true;
                }
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
                    guard_cell_content(c)?;
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
        validate(table, true)?;
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
        package = write_cell(package, main, (crate::StoryKind::Body, main), cell, blocks)?;
    }
    Ok((package, changed))
}

fn guard_cell_content(c: &Node) -> Result<()> {
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
    Ok(())
}

fn write_cell(
    package: Package,
    main: &str,
    owner: (crate::StoryKind, &str),
    cell: &mut Node,
    blocks: &[crate::ResolvedBlock],
) -> Result<Package> {
    let width = prop(cell, "w:tcW")
        .filter(|n| n.get("w:type") == Some("dxa"))
        .and_then(|n| n.get("w:w")?.parse::<u32>().ok())
        .filter(|w| *w > 0)
        .map(i64::from);
    let (next, nodes) = crate::ooxml::encode_editor_owner_blocks(
        package,
        main,
        owner,
        blocks,
        width,
        "editor.table",
    )?;
    cell.remove("w:p");
    let needs_paragraph = nodes.last().is_none_or(|n| n.name != "w:p");
    for n in nodes {
        validate_rendered(&n)?;
        cell.push(n)
    }
    if needs_paragraph {
        cell.push(Node::new("w:p"))
    }
    Ok(next)
}
pub(super) fn contains_range(n: &Node) -> bool {
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

/// Physical table geometry, properties and cell content all keep the
/// addressed table in place, so non-overlapping paths remain stable.
pub(super) fn physical(
    table: &mut Node,
    ops: &[&TableOperation],
    mut package: Package,
    main: &str,
    owner: (crate::StoryKind, &str),
) -> Result<(Package, bool)> {
    validate(table, true)?;
    let mut fields = BTreeMap::new();
    let mut replacements = BTreeMap::new();
    for op in ops {
        match op {
            TableOperation::Property {
                site,
                field,
                source,
            } => {
                if fields.insert((*site, field), source.as_deref()).is_some() {
                    return Err(unsupported("duplicate physical table property"));
                }
            }
            TableOperation::ReplaceCell { row, cell, blocks } => {
                if replacements.insert((*row, *cell), blocks).is_some() {
                    return Err(unsupported("duplicate physical cell replacement"));
                }
            }
            TableOperation::Layout(v) if ops.len() == 1 => {
                return Ok((package, layout(table, *v)?));
            }
            TableOperation::InsertRow(_)
            | TableOperation::RemoveRow(_)
            | TableOperation::InsertColumn(_)
            | TableOperation::RemoveColumn(_)
            | TableOperation::InsertCell { .. }
            | TableOperation::RemoveCell { .. }
                if ops.len() == 1 =>
            {
                geometry(table, op)?;
                return Ok((package, true));
            }
            _ => {
                return Err(unsupported(
                    "unsupported or conflicting physical table operations",
                ));
            }
        }
    }
    if !fields.is_empty() && !replacements.is_empty() {
        return Err(unsupported(
            "physical cell replacements conflict with property edits",
        ));
    }
    for ((site, field), source) in fields {
        super::properties::patch(table, site, field, source, &package, main)?;
    }
    validate(table, true)?;
    for ((r, c), blocks) in replacements {
        let cell = super::properties::site_node(table, super::TableSite::Cell(r, c))?;
        guard_cell_content(cell)?;
        package = write_cell(package, main, owner, cell, blocks)?;
    }
    Ok((package, true))
}
