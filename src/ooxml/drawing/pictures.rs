//! ECMA-376 Part 1 (2016): P1 §§20.2 and 20.4, inline and floating pictures.
use super::placement::placement;
use crate::ooxml::Compiler;
use crate::xml::Node;
use crate::{ResolvedImage, Result};
impl Compiler<'_> {
    pub(crate) fn image(
        &mut self,
        image: &ResolvedImage,
        owner: &crate::opc::PartName,
    ) -> Result<Node> {
        let path = self.media.register(&mut self.package, image)?;
        let id = self.package.link(owner.zip_name(), "image", &path)?;
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
        let drawing = placement(extent, image.anchor.as_deref())?
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
