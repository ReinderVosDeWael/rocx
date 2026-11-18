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
        if let Some(m) = &p.page_margins {
            for (k, v) in [
                ("top", m.top),
                ("bottom", m.bottom),
                ("left", m.left),
                ("right", m.right),
            ] {
                if let Some(v) = v {
                    previous
                        .ensure("w:pgMar")
                        .set(format!("w:{k}"), crate::Emu::new(v).twips().get());
                }
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
