//! Shared Wordprocessing Drawing placement (P1 (2016) §20.4, Annex A.4 CT_Anchor).
use crate::xml::Node;
use crate::{ImageAnchor, ImageWrap, Result};

pub(super) fn placement(extent: Node, anchor: Option<&ImageAnchor>) -> Result<Node> {
    let Some(anchor) = anchor else {
        return Ok(Node::new("wp:inline").with(extent));
    };
    let [x, y] = anchor.offsets_emu()?;
    let position = |name, reference, offset: i32, alignment: Option<&str>| {
        Node::new(name)
            .attr("relativeFrom", reference)
            .with(match alignment {
                Some(alignment) => Node::new("wp:align").text(alignment),
                None => Node::new("wp:posOffset").text(offset.to_string()),
            })
    };
    let wrap_side = || {
        anchor
            .wrap_side
            .map_or("bothSides", crate::ImageWrapSide::xml)
    };
    let mut wrap = match anchor.wrap {
        ImageWrap::None => Node::new("wp:wrapNone"),
        ImageWrap::Square => Node::new("wp:wrapSquare").attr("wrapText", wrap_side()),
        ImageWrap::TopAndBottom => Node::new("wp:wrapTopAndBottom"),
        ImageWrap::Tight | ImageWrap::Through => {
            let mut polygon = Node::new("wp:wrapPolygon").attr("edited", 1);
            for (index, &(x, y)) in anchor.wrap_polygon.iter().enumerate() {
                let tag = if index == 0 { "wp:start" } else { "wp:lineTo" };
                polygon.push(Node::new(tag).attr("x", x).attr("y", y));
            }
            Node::new(if anchor.wrap == ImageWrap::Tight {
                "wp:wrapTight"
            } else {
                "wp:wrapThrough"
            })
            .attr("wrapText", wrap_side())
            .with(polygon)
        }
    };
    for (name, distance) in ["distT", "distB", "distL", "distR"]
        .into_iter()
        .zip(anchor.wrap_distances_emu()?)
    {
        if let Some(distance) = distance {
            wrap.set(name, distance);
        }
    }
    Ok(Node::new("wp:anchor")
        .attr("simplePos", 0)
        .attr("relativeHeight", anchor.relative_height)
        .attr("behindDoc", u8::from(anchor.behind_text))
        .attr("locked", u8::from(anchor.locked.unwrap_or(false)))
        .attr(
            "layoutInCell",
            u8::from(anchor.layout_in_cell.unwrap_or(true)),
        )
        .attr(
            "allowOverlap",
            u8::from(anchor.allow_overlap.unwrap_or(true)),
        )
        .with(Node::new("wp:simplePos").attr("x", 0).attr("y", 0))
        .with(position(
            "wp:positionH",
            anchor.horizontal_reference.xml(),
            x,
            anchor
                .horizontal_alignment
                .map(crate::ImageAnchorHorizontalAlignment::xml),
        ))
        .with(position(
            "wp:positionV",
            anchor.vertical_reference.xml(),
            y,
            anchor
                .vertical_alignment
                .map(crate::ImageAnchorVerticalAlignment::xml),
        ))
        .with(extent)
        .with(wrap))
}
