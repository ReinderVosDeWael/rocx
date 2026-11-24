use super::{ALTERNATE, unsupported};
use crate::xml::{Content, Node, W};
use crate::{ResolvedTable, Result, TableLayout};
use std::collections::{BTreeMap, BTreeSet};

/// Validate body insertion points without imposing table geometry constraints.
pub(super) fn paragraph_sites(
    body: &Node,
    targets: &BTreeSet<usize>,
) -> Result<Vec<(usize, usize)>> {
    let mut boundaries = BoundaryState::default();
    let mut sites = Vec::new();
    let mut index = 0;
    let mut at_body_end = false;
    for (slot, child) in body.children.iter().enumerate() {
        let Content::Element(node) = child else {
            continue;
        };
        if at_body_end {
            return Err(unsupported("body section properties are not terminal"));
        }
        if node.name == "w:sectPr" {
            at_body_end = true;
            continue;
        }
        boundaries.advance(node);
        if targets.contains(&index) {
            if node.name != "w:p" || !boundaries.safe() {
                return Err(unsupported(
                    "paragraph insertion point is inside an active or ambiguous range",
                ));
            }
            sites.push((index, slot + 1));
        }
        index += 1;
    }
    if sites.len() != targets.len() {
        return Err(unsupported("paragraph target not found"));
    }
    Ok(sites)
}

/// Use the same element order as the body view. The direct section terminator
/// after an insertion slot governs the slot.
pub(super) fn table_sites(
    body: &Node,
    targets: &BTreeMap<usize, &ResolvedTable>,
) -> Result<Vec<(usize, usize, i64)>> {
    let mut boundaries = BoundaryState::default();
    let mut blocks = Vec::new();
    let mut at_body_end = false;
    for (slot, child) in body.children.iter().enumerate() {
        let Content::Element(node) = child else {
            continue;
        };
        if at_body_end {
            return Err(unsupported("body section properties are not terminal"));
        }
        if node.name == "w:sectPr" {
            at_body_end = true;
            continue;
        }
        let index = blocks.len();
        boundaries.advance(node);
        if targets.contains_key(&index) && (node.name != "w:p" || !boundaries.safe()) {
            return Err(unsupported(
                "table insertion point is inside an active or ambiguous range",
            ));
        }
        blocks.push((slot, node));
    }
    let mut next_section = unique_child(body, "w:sectPr");
    let mut sites = Vec::with_capacity(targets.len());
    for (index, (slot, node)) in blocks.into_iter().enumerate().rev() {
        if let Some(table) = targets.get(&index) {
            let width = table_width(table, next_section)?;
            sites.push((index, slot + 1, width));
        }
        if node.name == "w:p" {
            let section = unique_child(node, "w:pPr")
                .and_then(|props| props.map_or(Ok(None), |props| unique_child(props, "w:sectPr")));
            if !matches!(section, Ok(None)) {
                next_section = section;
            }
        } else if node.name == ALTERNATE {
            next_section = Err("section geometry is ambiguous at alternate content");
        }
    }
    if sites.len() != targets.len() {
        return Err(unsupported("table target paragraph not found"));
    }
    sites.reverse();
    Ok(sites)
}

fn unique_child<'a>(
    node: &'a Node,
    name: &str,
) -> std::result::Result<Option<&'a Node>, &'static str> {
    let mut matches = node.elements().filter(|child| child.name == name);
    let first = matches.next();
    if matches.next().is_some() || node.child(ALTERNATE).is_some() {
        Err("section geometry is ambiguous at the insertion point")
    } else {
        Ok(first)
    }
}

pub(super) fn table_width(
    table: &ResolvedTable,
    section: std::result::Result<Option<&Node>, &'static str>,
) -> Result<i64> {
    if table.columns() == 0 {
        return Err(unsupported("an inserted table needs at least one column"));
    }
    if let Some(widths) = table.column_widths()
        && table.layout() != Some(TableLayout::Autofit)
    {
        return Ok(widths.iter().copied().map(i64::from).sum());
    }
    width_for(section.map_err(unsupported)?)
}

fn width_for(section: Option<&Node>) -> Result<i64> {
    let Some(section) = section else {
        return Ok(8640);
    };
    let child = |name| unique_child(section, name).map_err(unsupported);
    let number = |node: Option<&Node>, name, default| -> Result<i64> {
        node.and_then(|node| node.get(name))
            .map_or(Ok(default), |value| {
                value.trim().parse::<u32>().map(i64::from).map_err(|_| {
                    unsupported("unrecognized section geometry; supply fixed table widths")
                })
            })
    };
    let columns = child("w:cols")?;
    let margins = child("w:pgMar")?;
    if number(columns, "w:num", 1)? != 1
        || columns.is_some_and(|node| node.child("w:col").is_some())
        || number(margins, "w:gutter", 0)? != 0
    {
        return Err(unsupported(
            "section has columns or a gutter; supply fixed table widths",
        ));
    }
    let width = number(child("w:pgSz")?, "w:w", 12240)?
        - number(margins, "w:left", 1800)?
        - number(margins, "w:right", 1800)?;
    if width <= 0 {
        return Err(unsupported("section has no positive usable table width"));
    }
    Ok(width)
}

/// Known body ranges must not silently gain content. Foreign branches are not
/// resolved in this editor; if an alternate may carry body markers, refuse.
#[derive(Default)]
pub(crate) struct BoundaryState {
    fields: usize,
    proof_ranges: [bool; 2],
    ranges: BTreeSet<(&'static str, String)>,
    uncertain: bool,
}

impl BoundaryState {
    pub(crate) fn safe(&self) -> bool {
        self.fields == 0
            && !self.proof_ranges.contains(&true)
            && self.ranges.is_empty()
            && !self.uncertain
    }

    pub(crate) fn advance(&mut self, node: &Node) {
        if self.enter(node) {
            for child in node.elements() {
                self.advance(child);
            }
        }
    }

    /// Observe this element alone; return whether its descendants share this state.
    pub(crate) fn enter(&mut self, node: &Node) -> bool {
        let name = node.name.as_str();
        if name == ALTERNATE {
            self.uncertain = true;
        }
        // Extension-defined range markers (for example conflict ranges) have
        // no pairing model here, even when their lexical prefix is familiar.
        if node.name.namespace() != Some(W)
            && (node.name.local().ends_with("RangeStart")
                || node.name.local().ends_with("RangeEnd"))
        {
            self.uncertain = true;
        }
        // Office Math belongs to the enclosing story: a Word range can open
        // or close inside an equation and continue across ordinary runs.
        if !matches!(
            node.name.namespace(),
            Some(W | "http://schemas.openxmlformats.org/officeDocument/2006/math")
        ) || matches!(name, "w:pPr" | "w:rPr" | "w:txbxContent")
        {
            return false;
        }
        if name == "w:fldChar" {
            match node.get("w:fldCharType") {
                Some("begin") => self.fields += 1,
                Some("end") if self.fields > 0 => self.fields -= 1,
                Some("separate") if self.fields > 0 => (),
                _ => self.uncertain = true,
            }
        }
        if name == "w:proofErr" {
            let (index, open) = match node.get("w:type") {
                Some("spellStart") => (0, true),
                Some("spellEnd") => (0, false),
                Some("gramStart") => (1, true),
                Some("gramEnd") => (1, false),
                _ => {
                    self.uncertain = true;
                    return false;
                }
            };
            self.uncertain |= std::mem::replace(&mut self.proof_ranges[index], open) == open;
        }
        if let Some((range, start)) = range_marker(name) {
            // Permission IDs are strings; the other marker families use the
            // arbitrary-precision ST_DecimalNumber value space.
            if let Some(id) = node.get("w:id").and_then(|id| {
                if range == "w:perm" {
                    Some(id.to_owned())
                } else {
                    crate::ooxml::decimal_id(id)
                }
            }) {
                let key = (range, id);
                let matched = if start {
                    self.ranges.insert(key)
                } else {
                    self.ranges.remove(&key)
                };
                self.uncertain |= !matched;
            } else {
                self.uncertain = true;
            }
        }
        true
    }
}

pub(super) fn can_append(story: &Node) -> bool {
    let mut boundaries = BoundaryState::default();
    boundaries.advance(story);
    boundaries.safe()
}

fn range_marker(name: &str) -> Option<(&'static str, bool)> {
    let (prefix, start) = if let Some(prefix) = name.strip_suffix("Start") {
        (prefix, true)
    } else {
        (name.strip_suffix("End")?, false)
    };
    [
        "w:bookmark",
        "w:commentRange",
        "w:perm",
        "w:moveFromRange",
        "w:moveToRange",
        "w:customXmlInsRange",
        "w:customXmlDelRange",
        "w:customXmlMoveFromRange",
        "w:customXmlMoveToRange",
    ]
    .into_iter()
    .find(|range| *range == prefix)
    .map(|range| (range, start))
}
