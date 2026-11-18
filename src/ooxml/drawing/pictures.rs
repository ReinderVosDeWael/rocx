//! ECMA-376 Part 1 (2016): P1 §§20.2 and 20.4, inline pictures.
use crate::ooxml::Compiler;
use crate::xml::Node;
use crate::{ResolvedImage, Result};
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
            .with(
                Node::new("pic:blipFill")
                    .with(Node::new("a:blip").attr("r:embed", id))
                    .with(Node::new("a:stretch").with(Node::new("a:fillRect"))),
            )
            .with(
                Node::new("pic:spPr")
                    .with(
                        Node::new("a:xfrm")
                            .with(Node::new("a:off").attr("x", 0).attr("y", 0))
                            .with(
                                Node::new("a:ext")
                                    .attr("cx", image.width)
                                    .attr("cy", image.height),
                            ),
                    )
                    .with(
                        Node::new("a:prstGeom")
                            .attr("prst", "rect")
                            .with(Node::new("a:avLst")),
                    ),
            );
        let inline = Node::new("wp:inline")
            .with(
                Node::new("wp:extent")
                    .attr("cx", image.width)
                    .attr("cy", image.height),
            )
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
        Ok(Node::new("w:r").with(Node::new("w:drawing").with(inline)))
    }
}
