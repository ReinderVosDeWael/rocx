//! Scalar style fields. Region names and reference values use physical OOXML IDs.
use super::{Node, Result, unique_child, unsupported};
use crate::editor::ALTERNATE;

fn region_name(s: &str) -> bool {
    matches!(
        s,
        "wholeTable"
            | "firstRow"
            | "lastRow"
            | "firstCol"
            | "lastCol"
            | "band1Horz"
            | "band2Horz"
            | "band1Vert"
            | "band2Vert"
            | "nwCell"
            | "neCell"
            | "swCell"
            | "seCell"
    )
}

// Only replace structures represented by the table-region authoring codec.
// In particular, never silently drop change markup, extensions or comments.
pub(super) fn known(n: &Node, parent: &str) -> bool {
    let attrs: &[&str] = match (parent, n.name.as_str()) {
        ("w:style", "w:tblStylePr") if n.get("w:type").is_some_and(region_name) => &["w:type"],
        ("w:style", "w:name" | "w:basedOn" | "w:next" | "w:link") => &["w:val"],
        ("w:style" | "w:tblStylePr", "w:rPr" | "w:pPr" | "w:tcPr") | ("w:pPr", "w:tabs") => &[],
        ("w:rPr", "w:rFonts") => &["w:ascii", "w:hAnsi", "w:eastAsia", "w:cs"],
        ("w:rPr", "w:lang") => &["w:val", "w:eastAsia", "w:bidi"],
        (
            "w:rPr",
            "w:b" | "w:i" | "w:bCs" | "w:iCs" | "w:sz" | "w:szCs" | "w:color" | "w:u" | "w:rtl"
            | "w:cs" | "w:strike" | "w:caps" | "w:smallCaps" | "w:vanish",
        )
        | (
            "w:pPr",
            "w:bidi"
            | "w:jc"
            | "w:keepNext"
            | "w:keepLines"
            | "w:pageBreakBefore"
            | "w:widowControl"
            | "w:contextualSpacing"
            | "w:mirrorIndents"
            | "w:snapToGrid"
            | "w:suppressLineNumbers",
        ) => &["w:val"],
        ("w:pPr", "w:ind") => &["w:left", "w:right", "w:firstLine", "w:hanging"],
        ("w:pPr", "w:spacing") => &["w:before", "w:after", "w:line", "w:lineRule"],
        ("w:tabs", "w:tab") => &["w:val", "w:pos", "w:leader"],
        ("w:tcPr", "w:shd") => &["w:val", "w:color", "w:fill"],
        _ => return false,
    };
    n.attrs
        .iter()
        .all(|(k, _)| k.declaration().is_some() || attrs.contains(&k.as_str()))
        && n.children.iter().all(|c| match c {
            crate::xml::Content::Element(e) => known(e, n.name.as_str()),
            crate::xml::Content::Text(t) => t.trim().is_empty(),
            _ => false,
        })
}

pub(super) fn region(
    style: &mut Node,
    region: &str,
    format: Option<&crate::TableSectionFormat>,
) -> Result<bool> {
    if style.get("w:type") != Some("table")
        || !region_name(region)
        || style.child(ALTERNATE).is_some()
    {
        return Err(unsupported("invalid or ambiguous style region"));
    }
    let mut nodes = style
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match c {
            crate::xml::Content::Element(n)
                if n.name == "w:tblStylePr" && n.get("w:type") == Some(region) =>
            {
                Some((i, n))
            }
            _ => None,
        });
    let first = nodes.next();
    if nodes.next().is_some() || first.is_some_and(|(_, n)| !known(n, "w:style")) {
        return Err(unsupported("duplicate or unsupported table style region"));
    }
    let slot = first.map(|(i, _)| i);
    let replacement = format
        .map(super::styles::section_format)
        .transpose()?
        .map(|n| n.attr("w:type", region));
    match (slot, replacement) {
        (Some(i), Some(n)) => style.children[i] = crate::xml::Content::Element(n),
        (Some(i), None) => {
            style.children.remove(i);
        }
        (None, Some(n)) => style.push(n),
        (None, None) => return Ok(false),
    }
    Ok(true)
}

pub(super) fn patch(
    style: &mut Node,
    region: Option<&str>,
    field: &str,
    value: Option<&str>,
) -> Result<bool> {
    let kind = style.get("w:type");
    if style.child(ALTERNATE).is_some()
        || region.is_some_and(|s| kind != Some("table") || !region_name(s))
    {
        return Err(unsupported("invalid or ambiguous style region"));
    }
    let (group, tag, valid) = match field {
        "basedOn" | "next" | "link"
            if region.is_none() && matches!(kind, Some("paragraph" | "character" | "table")) =>
        {
            if field == "next" && kind != Some("paragraph")
                || field == "link" && kind == Some("table")
            {
                return Err(unsupported("invalid reference for style kind"));
            }
            (
                None,
                format!("w:{field}"),
                value.is_none_or(|s| {
                    !s.trim().is_empty() && crate::document::invalid_xml_character(s).is_none()
                }),
            )
        }
        "b" | "i" | "bCs" | "iCs" | "rtl" | "cs" | "strike" | "caps" | "smallCaps" | "vanish"
            if matches!(kind, Some("paragraph" | "character" | "table")) =>
        {
            (
                Some("w:rPr"),
                format!("w:{field}"),
                value.is_none_or(|s| matches!(s, "0" | "1" | "true" | "false" | "on" | "off")),
            )
        }
        "keepNext" | "keepLines" | "pageBreakBefore" | "widowControl" | "bidi"
        | "contextualSpacing" | "mirrorIndents" | "snapToGrid"
            if matches!(kind, Some("paragraph" | "table")) =>
        {
            (
                Some("w:pPr"),
                format!("w:{field}"),
                value.is_none_or(|s| matches!(s, "0" | "1" | "true" | "false" | "on" | "off")),
            )
        }
        "jc" if matches!(kind, Some("paragraph" | "table")) => (
            Some("w:pPr"),
            "w:jc".into(),
            value.is_none_or(|s| crate::editor::read_alignment(s).is_some()),
        ),
        _ => return Err(unsupported("unsupported style field or style kind")),
    };
    if !valid {
        return Err(unsupported("invalid style field value"));
    }
    let container = if let Some(region) = region {
        let matches = style
            .elements()
            .filter(|n| n.name == "w:tblStylePr" && n.get("w:type") == Some(region));
        let count = matches.count();
        if count > 1 {
            return Err(unsupported("duplicate table style region"));
        }
        if count == 0 {
            if value.is_none() {
                return Ok(false);
            }
            style.push(Node::new("w:tblStylePr").attr("w:type", region));
        }
        style
            .elements_mut()
            .find(|n| n.name == "w:tblStylePr" && n.get("w:type") == Some(region))
            .unwrap()
    } else {
        style
    };
    if container.child(ALTERNATE).is_some() {
        return Err(unsupported("ambiguous conditional style properties"));
    }
    let parent = if let Some(group) = group {
        if unique_child(container, group)?.is_none() && value.is_none() {
            return Ok(false);
        }
        container.ensure(group)
    } else {
        container
    };
    if parent.child(ALTERNATE).is_some() {
        return Err(unsupported("ambiguous style field"));
    }
    let old = unique_child(parent, &tag)?;
    if let Some(s) = value {
        if old.and_then(|n| n.get("w:val")) == Some(s) {
            return Ok(false);
        }
        parent.ensure(&tag).set("w:val", s);
    } else if let Some(n) = old {
        if !n.children.is_empty()
            || n.attrs
                .iter()
                .any(|(a, _)| a.declaration().is_none() && a != "w:val")
        {
            return Err(unsupported("cannot discard annotated style field"));
        }
        parent.remove(&tag);
    } else {
        return Ok(false);
    }
    Ok(true)
}
