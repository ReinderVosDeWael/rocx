//! P1 (2016) §§20.1 and 20.3, CT_GvmlGroupShape, CT_GvmlShape and CT_GvmlTextShape.
use super::placement::placement;
use crate::ooxml::Compiler;
use crate::xml::{Name, Node};
use crate::{Result, ShapeGeometry, ShapeProperties};

const CANVAS: &str = "http://schemas.openxmlformats.org/drawingml/2006/lockedCanvas";

impl Compiler<'_> {
    pub(crate) fn shape(&mut self, shape: &ShapeProperties) -> Result<Node> {
        let (width, height) = shape.dimensions_emu()?;
        let group_id = self.drawing.next()?;
        let shape_id = group_id.next()?;
        self.drawing = if shape.geometry == ShapeGeometry::TextBox {
            shape_id
        } else {
            shape_id.next()?
        };
        let offset = |name| Node::new(name).attr("x", 0).attr("y", 0);
        let extent = |name| Node::new(name).attr("cx", width).attr("cy", height);
        let transform = Node::new("a:xfrm")
            .with(offset("a:off"))
            .with(extent("a:ext"));
        let shape_node = if let Some(geometry) = shape.geometry.xml() {
            geometric_shape(shape, shape_id, geometry, transform)?
        } else {
            Node::new("a:txSp")
                .with(text_body(shape, shape.text.as_deref().unwrap_or(""))?)
                .with(transform)
        };
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
        let drawing = placement(extent("wp:extent"), shape.anchor.as_ref())?
            .with(Node::new("wp:docPr").attr("id", self.drawing).attr(
                "name",
                if shape.geometry == ShapeGeometry::TextBox {
                    "Text box"
                } else {
                    "Shape"
                },
            ))
            .with(
                Node::new("a:graphic")
                    .with(Node::new("a:graphicData").attr("uri", CANVAS).with(canvas)),
            );
        Ok(Node::new("w:r").with(Node::new("w:drawing").with(drawing)))
    }
}

fn geometric_shape(
    shape: &ShapeProperties,
    shape_id: crate::DrawingId,
    geometry: &str,
    transform: Node,
) -> Result<Node> {
    let mut shape_properties = Node::new("a:spPr").with(transform).with(
        Node::new("a:prstGeom")
            .attr("prst", geometry)
            .with(Node::new("a:avLst")),
    );
    if let Some(rgb) = shape.fill {
        shape_properties.push(Node::new("a:solidFill").with(
            Node::new("a:srgbClr").attr("val", crate::ooxml::wordprocessing::format::color(rgb)),
        ));
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
        shape_node.push(
            Node::new("a:txSp")
                .with(text_body(shape, text)?)
                .with(Node::new("a:useSpRect")),
        );
    }
    Ok(shape_node)
}

fn text_body(shape: &ShapeProperties, text: &str) -> Result<Node> {
    crate::xml::validate_text(text).map_err(|error| error.at("shape.text"))?;
    let format = shape.text_format.as_ref().cloned().unwrap_or_default();
    let mut body_properties = Node::new("a:bodyPr");
    if let Some(alignment) = format.vertical_alignment {
        body_properties = body_properties.attr("anchor", alignment.xml());
    }
    let mut run_properties = Node::new("a:rPr");
    if let Some(size) = format.font_size_hundredths()? {
        run_properties = run_properties.attr("sz", size);
    }
    if let Some(bold) = format.bold {
        run_properties = run_properties.attr("b", if bold { "1" } else { "0" });
    }
    if let Some(italic) = format.italic {
        run_properties = run_properties.attr("i", if italic { "1" } else { "0" });
    }
    if let Some(color) = format.color {
        run_properties.push(Node::new("a:solidFill").with(
            Node::new("a:srgbClr").attr("val", crate::ooxml::wordprocessing::format::color(color)),
        ));
    }
    if let Some(font) = &format.font {
        crate::xml::validate_text(font).map_err(|error| error.at("shape.text_format.font"))?;
        run_properties.push(Node::new("a:latin").attr("typeface", font));
    }
    let has_run_properties = format.font_size.is_some()
        || format.bold.is_some()
        || format.color.is_some()
        || format.italic.is_some()
        || format.font.is_some();
    let mut body = Node::new("a:txBody").with(body_properties);
    for line in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        let mut paragraph = Node::new("a:p");
        if let Some(alignment) = format.alignment {
            paragraph.push(Node::new("a:pPr").attr("algn", alignment.xml()));
        }
        if !line.is_empty() {
            let mut run = Node::new("a:r");
            if has_run_properties {
                run.push(run_properties.clone());
            }
            paragraph.push(run.with(Node::new("a:t").text(line)));
        }
        if has_run_properties {
            let mut end_properties = run_properties.clone();
            end_properties.name = Name::from("a:endParaRPr");
            paragraph.push(end_properties);
        }
        body.push(paragraph);
    }
    Ok(body)
}
