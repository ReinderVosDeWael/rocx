//! P1 §17.6, section properties and usable page width.
use crate::xml::{Node, val};
use crate::{Error, Orientation, Result, SectionProperties};
pub(crate) fn section_properties(
    mut previous: Node,
    props: &Option<SectionProperties>,
) -> Result<Node> {
    if let Some(p) = props {
        p.validate()?;
        let size = previous.ensure("w:pgSz");
        let mut w = size
            .get("w:w")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(12240);
        let mut h = size
            .get("w:h")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(15840);
        if let Some(size) = &p.page_size {
            if let Some(v) = size.width {
                w = crate::Emu::new(v).twips().get();
            }
            if let Some(v) = size.height {
                h = crate::Emu::new(v).twips().get();
            }
        }
        if let Some(o) = p.page_orientation {
            if (o == Orientation::Landscape && w < h) || (o == Orientation::Portrait && w > h) {
                std::mem::swap(&mut w, &mut h);
            }
            size.set("w:orient", o.xml());
        }
        if w <= 0 || h <= 0 {
            return Err(Error::Validation("page size must be positive".into()));
        }
        size.set("w:w", w);
        size.set("w:h", h);
        // P1 (2016) §17.6.11: header and footer distances are independent of body margins.
        if let Some(m) = &p.page_margins {
            let mut patched = false;
            for (k, v) in [
                ("top", m.top),
                ("bottom", m.bottom),
                ("left", m.left),
                ("right", m.right),
                ("header", m.header),
                ("footer", m.footer),
            ] {
                if let Some(v) = v {
                    patched = true;
                    previous
                        .ensure("w:pgMar")
                        .set(format!("w:{k}"), crate::Emu::new(v).twips().get());
                }
            }
            if patched {
                // CT_PageMar requires all seven attributes even when only one
                // margin is changed. Match the built-in template's defaults.
                let margins = previous.ensure("w:pgMar");
                for (key, default) in [
                    ("w:top", 1440),
                    ("w:right", 1800),
                    ("w:bottom", 1440),
                    ("w:left", 1800),
                    ("w:header", 720),
                    ("w:footer", 720),
                    ("w:gutter", 0),
                ] {
                    if margins.get(key).is_none() {
                        margins.set(key, default);
                    }
                }
            }
        }
        // P1 (2016) §17.6.10 and Annex A.1 CT_PageBorders.
        if let Some(borders) = &p.page_borders {
            for (name, request) in [
                (
                    "w:offsetFrom",
                    borders.offset_from.map(crate::PageBorderOffset::xml),
                ),
                (
                    "w:display",
                    borders.display.map(crate::PageBorderDisplay::xml),
                ),
                (
                    "w:zOrder",
                    borders.z_order.map(crate::PageBorderZOrder::xml),
                ),
            ] {
                if let Some(value) = request {
                    previous.ensure("w:pgBorders").set(name, value);
                }
            }
            for (side, border) in [
                ("w:top", &borders.top),
                ("w:left", &borders.left),
                ("w:bottom", &borders.bottom),
                ("w:right", &borders.right),
            ] {
                let Some(border) = border else { continue };
                let mut node = Node::new(side).attr("w:val", border.style.xml());
                if let Some(size) = border.size {
                    node.set("w:sz", size);
                }
                if let Some(space) = border.space {
                    node.set("w:space", space);
                }
                if let Some(rgb) = border.color {
                    node.set("w:color", super::format::color(rgb));
                }
                previous.ensure("w:pgBorders").replace(node);
            }
        }
        // P1 (2016) §17.6.8 and Annex A.1 CT_LineNumber.
        if p.line_numbering.as_ref().is_some_and(|line| line.clear) {
            previous.remove("w:lnNumType");
        } else if let Some(line) = &p.line_numbering {
            let node = previous.ensure("w:lnNumType");
            for (key, value) in [
                ("w:countBy", line.count_by),
                ("w:start", line.start),
                ("w:distance", line.distance),
            ] {
                if let Some(value) = value {
                    node.set(key, value);
                }
            }
            if let Some(restart) = line.restart {
                node.set("w:restart", restart.xml());
            }
        }
        if let Some(n) = &p.page_numbering {
            let node = previous.ensure("w:pgNumType");
            if let Some(start) = n.start {
                node.set("w:start", start);
            }
            if let Some(format) = &n.format {
                node.set("w:fmt", format);
            }
        }
        // P1 (2016) §17.6.5 and Annex A.1 CT_DocGrid.
        if let Some(grid) = &p.document_grid {
            let node = previous.ensure("w:docGrid");
            if let Some(mode) = grid.mode {
                node.set("w:type", mode.xml());
            }
            if let Some(pitch) = grid.line_pitch {
                node.set("w:linePitch", pitch);
            }
            if let Some(delta) = grid.character_pitch_delta {
                node.set("w:charSpace", delta);
            }
        }
        if let Some(c) = &p.columns {
            let node = previous.ensure("w:cols");
            if let Some(count) = c.count {
                node.set("w:num", count);
            }
            if let Some(space) = c.space {
                node.set("w:space", space);
            }
            if let Some(separator) = c.separator {
                node.set("w:sep", u8::from(separator));
            }
        }
        if let Some(v) = p.vertical_align {
            previous.replace(val("vAlign", v.xml()));
        }
        if let Some(v) = p.title_page {
            previous.remove("w:titlePg");
            if v {
                previous.push(Node::new("w:titlePg"));
            }
        }
        if let Some(t) = p.r#type {
            previous.replace(val("type", t.xml()));
        }
        if let Some(bidi) = p.bidi {
            previous.replace(val("bidi", u8::from(bidi)));
        }
        if let Some(rtl_gutter) = p.rtl_gutter {
            previous.replace(val("rtlGutter", u8::from(rtl_gutter)));
        }
    }
    Ok(previous)
}
pub(crate) fn usable_width(props: &Node) -> i64 {
    let size = props
        .child("w:pgSz")
        .and_then(|n| n.get("w:w"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(12240);
    let margin = props.child("w:pgMar");
    let l = margin
        .and_then(|n| n.get("w:left"))
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(1800);
    let r = margin
        .and_then(|n| n.get("w:right"))
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(1800);
    (size - l - r).max(1)
}
