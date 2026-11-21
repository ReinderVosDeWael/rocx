//! P1 (2016) §§20.1 and 20.3, CT_GvmlGroupShape and CT_GvmlShape.
use crate::ooxml::Compiler;
use crate::xml::{Name, Node};
use crate::{Result, ShapeProperties};

const CANVAS: &str = "http://schemas.openxmlformats.org/drawingml/2006/lockedCanvas";

impl Compiler<'_> {
    pub(crate) fn shape(&mut self, shape: &ShapeProperties) -> Result<Node> {
        let (width, height) = shape.dimensions_emu()?;
        let group_id = self.drawing.next()?;
        let shape_id = group_id.next()?;
        self.drawing = shape_id.next()?;
        let offset = |name| Node::new(name).attr("x", 0).attr("y", 0);
        let extent = |name| Node::new(name).attr("cx", width).attr("cy", height);
        let mut shape_properties = Node::new("a:spPr")
            .with(
                Node::new("a:xfrm")
                    .with(offset("a:off"))
                    .with(extent("a:ext")),
            )
            .with(
                Node::new("a:prstGeom")
                    .attr("prst", shape.geometry.xml())
                    .with(Node::new("a:avLst")),
            );
        if let Some(rgb) = shape.fill {
            shape_properties.push(
                Node::new("a:solidFill").with(
                    Node::new("a:srgbClr")
                        .attr("val", crate::ooxml::wordprocessing::format::color(rgb)),
                ),
            );
        }
        if let Some(outline) = &shape.outline {
            shape_properties.push(Node::new("a:ln").attr("w", outline.width_emu()?).with(
                Node::new("a:solidFill").with(Node::new("a:srgbClr").attr(
                    "val",
                    crate::ooxml::wordprocessing::format::color(outline.color),
                )),
            ));
        }
        let mut shape_node = Node::new("a:sp")
            .with(
                Node::new("a:nvSpPr")
                    .with(
                        Node::new("a:cNvPr")
                            .attr("id", shape_id)
                            .attr("name", "Shape"),
                    )
                    .with(Node::new("a:cNvSpPr")),
            )
            .with(shape_properties);
        if let Some(text) = &shape.text {
            crate::xml::validate_text(text).map_err(|error| error.at("shape.text"))?;
            let mut body = Node::new("a:txBody").with(Node::new("a:bodyPr"));
            for line in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
                let mut paragraph = Node::new("a:p");
                if !line.is_empty() {
                    paragraph.push(Node::new("a:r").with(Node::new("a:t").text(line)));
                }
                body.push(paragraph);
            }
            shape_node.push(
                Node::new("a:txSp")
                    .with(body)
                    .with(Node::new("a:useSpRect")),
            );
        }
        let canvas = Node::new(Name::expanded(Some(CANVAS), "lockedCanvas", Some("lc")))
            .with(
                Node::new("a:nvGrpSpPr")
                    .with(
                        Node::new("a:cNvPr")
                            .attr("id", group_id)
                            .attr("name", "Shape canvas"),
                    )
                    .with(Node::new("a:cNvGrpSpPr")),
            )
            .with(
                Node::new("a:grpSpPr").with(
                    Node::new("a:xfrm")
                        .with(offset("a:off"))
                        .with(extent("a:ext"))
                        .with(offset("a:chOff"))
                        .with(extent("a:chExt")),
                ),
            )
            .with(shape_node);
        let inline = Node::new("wp:inline")
            .with(extent("wp:extent"))
            .with(
                Node::new("wp:docPr")
                    .attr("id", self.drawing)
                    .attr("name", "Shape"),
            )
            .with(
                Node::new("a:graphic")
                    .with(Node::new("a:graphicData").attr("uri", CANVAS).with(canvas)),
            );
        Ok(Node::new("w:r").with(Node::new("w:drawing").with(inline)))
    }
}
