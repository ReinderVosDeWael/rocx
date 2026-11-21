//! ECMA-376 Part 1 (2016): P1 §§20.2 and 20.4, inline pictures.
use crate::ooxml::Compiler;
use crate::xml::Node;
use crate::{ImageWrap, ResolvedImage, Result};
impl Compiler<'_> {
    pub(crate) fn image(
        &mut self,
        image: &ResolvedImage,
        context: &crate::ooxml::PartContext,
    ) -> Result<Node> {
        let path = self.media.register(&mut self.package, image)?;
        let id = self
            .package
            .link(context.source.zip_name(), "image", &path)?;
        self.drawing = self.drawing.next()?;
        let drawing = self.drawing;
        let alt = image.alt_text.as_ref();
        let name = alt.and_then(|a| a.name.as_deref()).unwrap_or(&image.name);
        let docpr = Node::new("wp:docPr")
            .attr("id", drawing)
            .attr("name", name)
            .attr(
                "descr",
                alt.and_then(|a| a.description.as_deref()).unwrap_or(""),
            )
            .attr("title", alt.and_then(|a| a.title.as_deref()).unwrap_or(""));
        let mut fill = Node::new("pic:blipFill").with(Node::new("a:blip").attr("r:embed", id));
        // P1 (2016) §§20.1–20.2 and Annex A.4.1, percent insets and angle units.
        if let Some([left, top, right, bottom]) = image.crop {
            fill.push(
                Node::new("a:srcRect")
                    .attr("l", left)
                    .attr("t", top)
                    .attr("r", right)
                    .attr("b", bottom),
            );
        }
        fill.push(Node::new("a:stretch").with(Node::new("a:fillRect")));
        let mut transform = Node::new("a:xfrm")
            .with(Node::new("a:off").attr("x", 0).attr("y", 0))
            .with(
                Node::new("a:ext")
                    .attr("cx", image.width)
                    .attr("cy", image.height),
            );
        if let Some(rotation) = image.rotation {
            transform.set("rot", rotation);
        }
        for (name, flip) in [
            ("flipH", image.flip_horizontal),
            ("flipV", image.flip_vertical),
        ] {
            if let Some(flip) = flip {
                transform.set(name, u8::from(flip));
            }
        }
        let pic = Node::new("pic:pic")
            .with(
                Node::new("pic:nvPicPr")
                    .with(
                        Node::new("pic:cNvPr")
                            .attr("id", 0)
                            .attr("name", &image.name),
                    )
                    .with(Node::new("pic:cNvPicPr")),
            )
            .with(fill)
            .with(
                Node::new("pic:spPr").with(transform).with(
                    Node::new("a:prstGeom")
                        .attr("prst", "rect")
                        .with(Node::new("a:avLst")),
                ),
            );
        let extent = Node::new("wp:extent")
            .attr("cx", image.width)
            .attr("cy", image.height);
        // P1 (2016) §20.4 and Annex A.4, CT_Anchor.
        let placement = if let Some(anchor) = &image.anchor {
            let [x, y] = anchor.offsets_emu()?;
            let position = |name, reference, offset: i32, alignment: Option<&str>| {
                Node::new(name)
                    .attr("relativeFrom", reference)
                    .with(match alignment {
                        Some(alignment) => Node::new("wp:align").text(alignment),
                        None => Node::new("wp:posOffset").text(offset.to_string()),
                    })
            };
            let mut wrap = match anchor.wrap {
                ImageWrap::None => Node::new("wp:wrapNone"),
                ImageWrap::Square => Node::new("wp:wrapSquare").attr(
                    "wrapText",
                    anchor
                        .wrap_side
                        .map_or("bothSides", crate::ImageWrapSide::xml),
                ),
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
                    .attr(
                        "wrapText",
                        anchor
                            .wrap_side
                            .map_or("bothSides", crate::ImageWrapSide::xml),
                    )
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
            Node::new("wp:anchor")
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
                .with(wrap)
        } else {
            Node::new("wp:inline").with(extent)
        };
        let drawing = placement
            .with(docpr)
            .with(
                Node::new("wp:cNvGraphicFramePr")
                    .with(Node::new("a:graphicFrameLocks").attr("noChangeAspect", 1)),
            )
            .with(
                Node::new("a:graphic").with(
                    Node::new("a:graphicData")
                        .attr(
                            "uri",
                            "http://schemas.openxmlformats.org/drawingml/2006/picture",
                        )
                        .with(pic),
                ),
            );
        Ok(Node::new("w:r").with(Node::new("w:drawing").with(drawing)))
    }
}
