//! A closed inventory of editable direct fields. Values come from the same
//! table compiler as authored content. Imported siblings are never re-rendered.
use super::{unique, unsupported};
use crate::{
    ResolvedTable, Result,
    opc::Package,
    xml::{Content, Node},
};
use std::collections::BTreeMap;

/// Physical source coordinate; `Column` addresses an explicit grid column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TableSite {
    Table,
    Row(usize),
    Cell(usize, usize),
    Column(usize),
}

/// Field -> modeled OOXML attributes, retaining lexical values (including
/// explicit false, empty on/off elements, twips, theme-independent RGB).
pub type DirectTableProperties = BTreeMap<String, BTreeMap<String, String>>;

struct Field {
    key: &'static str,
    parent: Option<&'static str>,
    leaf: &'static str,
    attrs: &'static [&'static str],
    bit: u16,
}
const fn field(
    key: &'static str,
    parent: Option<&'static str>,
    leaf: &'static str,
    attrs: &'static [&'static str],
) -> Field {
    Field {
        key,
        parent,
        leaf,
        attrs,
        bit: 0,
    }
}
const fn flag(key: &'static str, attr: &'static [&'static str], bit: u16) -> Field {
    Field {
        key,
        parent: None,
        leaf: "w:tblLook",
        attrs: attr,
        bit,
    }
}
const fn border(key: &'static str, parent: &'static str, side: &'static str) -> Field {
    field(key, Some(parent), side, &["w:val", "w:sz", "w:color"])
}
const fn margin(key: &'static str, side: &'static str) -> Field {
    field(key, Some("w:tcMar"), side, &["w:w", "w:type"])
}
const TABLE: &[Field] = &[
    field("style", None, "w:tblStyle", &["w:val"]),
    field("alignment", None, "w:jc", &["w:val"]),
    field("layout", None, "w:tblLayout", &["w:type"]),
    flag("look.firstRow", &["w:firstRow"], 0x20),
    flag("look.lastRow", &["w:lastRow"], 0x40),
    flag("look.firstColumn", &["w:firstColumn"], 0x80),
    flag("look.lastColumn", &["w:lastColumn"], 0x100),
    flag("look.horizontalBanding", &["w:noHBand"], 0x200),
    flag("look.verticalBanding", &["w:noVBand"], 0x400),
    border("border.top", "w:tblBorders", "w:top"),
    border("border.left", "w:tblBorders", "w:left"),
    border("border.bottom", "w:tblBorders", "w:bottom"),
    border("border.right", "w:tblBorders", "w:right"),
    border("border.insideH", "w:tblBorders", "w:insideH"),
    border("border.insideV", "w:tblBorders", "w:insideV"),
];
const ROW: &[Field] = &[
    field("height", None, "w:trHeight", &["w:val", "w:hRule"]),
    field("repeatHeader", None, "w:tblHeader", &["w:val"]),
    field("cantSplit", None, "w:cantSplit", &["w:val"]),
];
const CELL: &[Field] = &[
    field("gridSpan", None, "w:gridSpan", &["w:val"]),
    field("verticalMerge", None, "w:vMerge", &["w:val"]),
    field("backgroundColor", None, "w:shd", &["w:fill"]),
    field("verticalAlignment", None, "w:vAlign", &["w:val"]),
    field("textDirection", None, "w:textDirection", &["w:val"]),
    margin("margin.top", "w:top"),
    margin("margin.start", "w:start"),
    margin("margin.bottom", "w:bottom"),
    margin("margin.end", "w:end"),
    border("border.top", "w:tcBorders", "w:top"),
    border("border.bottom", "w:tcBorders", "w:bottom"),
    border("border.start", "w:tcBorders", "w:start"),
    border("border.end", "w:tcBorders", "w:end"),
];
fn fields(site: TableSite) -> (&'static str, &'static [Field]) {
    match site {
        TableSite::Table => ("w:tblPr", TABLE),
        TableSite::Row(_) => ("w:trPr", ROW),
        TableSite::Cell(..) => ("w:tcPr", CELL),
        TableSite::Column(_) => ("", &[]),
    }
}
pub(in crate::editor) fn inspect(node: &Node, site: TableSite) -> DirectTableProperties {
    let (pr, fields) = fields(site);
    let mut result = BTreeMap::new();
    if let Some(pr) = node.child(pr) {
        for f in fields {
            let Some(n) = f
                .parent
                .map_or(Some(pr), |p| pr.child(p))
                .and_then(|p| p.child(f.leaf))
            else {
                continue;
            };
            let mut attrs = BTreeMap::new();
            for a in f.attrs {
                if let Some(v) = n.get(a) {
                    attrs.insert(a[2..].to_owned(), v.to_owned());
                }
            }
            if f.bit != 0
                && let Some(mask) = n.get("w:val").and_then(|v| u16::from_str_radix(v, 16).ok())
            {
                attrs.insert("maskBit".into(), ((mask & f.bit != 0) as u8).to_string());
            }
            if !attrs.is_empty() || (f.bit == 0 && f.key != "backgroundColor") {
                result.insert(f.key.to_owned(), attrs);
            }
        }
    }
    result
}
pub(super) fn site_node(table: &mut Node, site: TableSite) -> Result<&mut Node> {
    let absent = || unsupported("table property index out of bounds");
    match site {
        TableSite::Table => Ok(table),
        TableSite::Row(r) => table
            .elements_mut()
            .filter(|n| n.name == "w:tr")
            .nth(r)
            .ok_or_else(absent),
        TableSite::Cell(r, c) => site_node(table, TableSite::Row(r))?
            .elements_mut()
            .filter(|n| n.name == "w:tc")
            .nth(c)
            .ok_or_else(absent),
        TableSite::Column(c) => table
            .child_mut("w:tblGrid")
            .and_then(|g| g.elements_mut().filter(|n| n.name == "w:gridCol").nth(c))
            .ok_or_else(absent),
    }
}
fn child<'a>(
    node: &'a mut Node,
    key: &str,
    create: bool,
    first: bool,
) -> Result<Option<&'a mut Node>> {
    if unique(node, key)?.is_none() && create {
        if first {
            node.children.insert(0, Content::Element(Node::new(key)));
        } else {
            node.push(Node::new(key));
        }
    }
    Ok(node.child_mut(key))
}
fn del_attr(node: &mut Node, key: &str) {
    node.attrs.retain(|(k, _)| k != key);
}

pub(super) fn patch(
    table: &mut Node,
    site: TableSite,
    key: &str,
    source: Option<&ResolvedTable>,
    package: &Package,
    main: &str,
) -> Result<bool> {
    let node = site_node(table, site)?;
    if matches!(site, TableSite::Column(_)) {
        if key != "width" {
            return Err(unsupported("unknown grid column property"));
        }
        if let Some(t) = source {
            let w = t
                .column_widths()
                .and_then(|w| w.first())
                .ok_or_else(|| unsupported("source needs an explicit first column width"))?;
            let w = w.to_string();
            if node.get("w:w") == Some(w.as_str()) {
                return Ok(false);
            }
            node.set("w:w", w);
        } else {
            if node.get("w:w").is_none() {
                return Ok(false);
            }
            del_attr(node, "w:w");
        }
        return Ok(true);
    }
    let (properties, fields) = fields(site);
    let f = fields
        .iter()
        .find(|f| f.key == key)
        .ok_or_else(|| unsupported("unknown property for table site"))?;
    // The codec only needs formatting. Never import the donor's content,
    // relationships, or content controls for a property edit.
    let value = if let Some(source) = source {
        use crate::ooxml::wordprocessing::table::{
            cell_properties, row_properties, table_properties,
        };
        if f.bit != 0 {
            let v = source.look.unwrap_or_default();
            if match f.bit {
                0x20 => v.first_row,
                0x40 => v.last_row,
                0x80 => v.first_column,
                0x100 => v.last_column,
                0x200 => v.horizontal_banding,
                _ => v.vertical_banding,
            }
            .is_none()
            {
                return Err(unsupported("source look flag is not explicit"));
            }
        }
        let first_row = || {
            source
                .rows()
                .first()
                .ok_or_else(|| unsupported("missing resolved source row"))
        };
        let pr = match site {
            TableSite::Table => {
                let style = source
                    .style()
                    .filter(|_| key == "style")
                    .map(|s| crate::ooxml::editor_table_style(package, main, s))
                    .transpose()?;
                table_properties(source, style.as_ref().map(|s| s.as_str()), &[])
            }
            TableSite::Row(_) => row_properties(first_row()?)?,
            TableSite::Cell(..) => cell_properties(
                first_row()?
                    .cells()
                    .first()
                    .ok_or_else(|| unsupported("missing resolved source cell"))?,
                0,
            )?,
            TableSite::Column(_) => unreachable!(),
        };
        let value = f
            .parent
            .map_or(Some(&pr), |name| pr.child(name))
            .and_then(|p| p.child(f.leaf));
        Some(
            value
                .cloned()
                .ok_or_else(|| unsupported("selected property is missing from resolved source"))?,
        )
    } else {
        None
    };
    // A new continuation hides the physical cell's contents in Word. Only
    // allow blank cells to become continuations; an existing continuation
    // can retain its private content unchanged.
    if key == "verticalMerge"
        && value
            .as_ref()
            .is_some_and(|n| n.get("w:val") == Some("continue"))
        && !node
            .child(properties)
            .and_then(|p| p.child("w:vMerge"))
            .is_some_and(|m| m.get("w:val") != Some("restart"))
        && node.elements().filter(|n| n.name != "w:tcPr").any(|n| {
            n.name != "w:p"
                || !n.attrs.is_empty()
                || n.children
                    .iter()
                    .any(|c| !matches!(c,Content::Text(s) if s.trim().is_empty()))
        })
    {
        return Err(unsupported(
            "merge continuation would hide existing content",
        ));
    }
    let Some(pr) = child(node, properties, value.is_some(), true)? else {
        return Ok(false);
    };
    if pr.child("w:tblPrChange").is_some() {
        return Err(unsupported("tracked table properties"));
    }
    let parent = if let Some(p) = f.parent {
        let Some(v) = child(pr, p, value.is_some(), false)? else {
            return Ok(false);
        };
        v
    } else {
        pr
    };
    let created = parent.child(f.leaf).is_none();
    let Some(dest) = child(parent, f.leaf, value.is_some(), false)? else {
        return Ok(false);
    };
    let before = dest.attrs.clone();
    if f.bit != 0 {
        let a = f.attrs[0];
        let bit = value.as_ref().and_then(|n| n.get(a));
        if let Some(old) = dest.get("w:val") {
            let mask =
                u16::from_str_radix(old, 16).map_err(|_| unsupported("invalid table look mask"))?;
            if let Some(bit) = bit {
                let patched = (mask & !f.bit) | if bit == "1" { f.bit } else { 0 };
                if patched != mask {
                    dest.set("w:val", format!("{patched:04X}"));
                }
            } else {
                // A zero bit still specifies false. Preserve the other mask
                // settings explicitly before removing the selected setting.
                let known = TABLE.iter().fold(0, |mask, f| mask | f.bit);
                if mask & !known != 0 {
                    return Err(unsupported(
                        "cannot clear one look flag with unknown mask bits",
                    ));
                }
                for flag in TABLE.iter().filter(|g| g.bit != 0 && g.bit != f.bit) {
                    if dest.get(flag.attrs[0]).is_none() {
                        dest.set(flag.attrs[0], u8::from(mask & flag.bit != 0));
                    }
                }
                del_attr(dest, "w:val");
            }
        }
        if let Some(v) = bit {
            dest.set(a, v)
        } else {
            del_attr(dest, a)
        }
    } else if let Some(v) = value {
        if key == "backgroundColor" && dest.get("w:val").is_none() {
            dest.set("w:val", "clear");
        }
        for &a in f.attrs {
            if let Some(val) = v.get(a) {
                dest.set(a, val)
            } else {
                del_attr(dest, a)
            }
        }
    } else if key == "backgroundColor" {
        if dest.get("w:fill").is_some()
            && dest.children.is_empty()
            && dest.attrs.iter().all(|(a, v)| {
                a.declaration().is_some()
                    || a == "w:fill"
                    || (a == "w:val" && v == "clear")
                    || (a == "w:color" && v == "auto")
            })
        {
            parent.remove(f.leaf);
            return Ok(true);
        } else {
            del_attr(dest, "w:fill");
        }
    } else {
        if dest
            .attrs
            .iter()
            .any(|(k, _)| k.declaration().is_none() && !f.attrs.contains(&k.as_str()))
            || dest
                .children
                .iter()
                .any(|c| !matches!(c, Content::Text(t) if t.trim().is_empty()))
        {
            return Err(unsupported(
                "clearing property would discard unsupported markup",
            ));
        }
        parent.remove(f.leaf);
        return Ok(true);
    }
    Ok(created || dest.attrs != before)
}
