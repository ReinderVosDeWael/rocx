//! ECMA-376 P1 §§20.4 and 21.2: an inline drawing and its literal-data chart.
use super::placement::placement;
use crate::ooxml::{Compiler, PartContext};
use crate::xml::{Name, Node};
use crate::{ColumnChartProperties, Result};

const C: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const CHART_TYPE: &str = "application/vnd.openxmlformats-officedocument.drawingml.chart+xml";
const CATEGORY_AXIS: u32 = 101;
const VALUE_AXIS: u32 = 102;

impl Compiler<'_> {
    pub(crate) fn column_chart(
        &mut self,
        chart: &ColumnChartProperties,
        context: &PartContext,
    ) -> Result<Node> {
        let (width, height) = chart.dimensions_emu()?;
        let path = self.package.unique("word/charts/chart", "xml");
        self.package.put(&path, chart_space(chart))?;
        self.package.content_type(&path, CHART_TYPE)?;
        let id = self
            .package
            .link(context.source.zip_name(), "chart", &path)?;
        self.drawing = self.drawing.next()?;
        let extent = Node::new("wp:extent").attr("cx", width).attr("cy", height);
        let drawing = placement(extent, None)?
            .with(
                Node::new("wp:docPr")
                    .attr("id", self.drawing)
                    .attr("name", "Column chart"),
            )
            .with(
                Node::new("a:graphic").with(
                    Node::new("a:graphicData")
                        .attr("uri", C)
                        .with(c("chart").attr("r:id", id)),
                ),
            );
        Ok(Node::new("w:r").with(Node::new("w:drawing").with(drawing)))
    }
}

fn c(local: &str) -> Node {
    Node::new(Name::expanded(Some(C), local, Some("c")))
}

fn val(local: &str, value: impl ToString) -> Node {
    c(local).attr("val", value)
}

fn point(index: usize, value: impl Into<String>) -> Node {
    c("pt").attr("idx", index).with(c("v").text(value))
}

fn axis(kind: &str, id: u32, side: &str, crosses: u32) -> Node {
    c(kind)
        .with(val("axId", id))
        .with(c("scaling").with(val("orientation", "minMax")))
        .with(val("axPos", side))
        .with(val("crossAx", crosses))
        .with(val("crosses", "autoZero"))
}

fn chart_space(chart: &ColumnChartProperties) -> Node {
    let count = chart.categories.len();
    let mut labels = c("strLit").with(val("ptCount", count));
    let mut values = c("numLit")
        .with(c("formatCode").text("General"))
        .with(val("ptCount", count));
    for (index, (label, value)) in chart.categories.iter().zip(&chart.values).enumerate() {
        labels.push(point(index, label));
        values.push(point(index, value.to_string()));
    }
    let series = c("ser")
        .with(val("idx", 0))
        .with(val("order", 0))
        .with(c("cat").with(labels))
        .with(c("val").with(values));
    let plot = c("plotArea")
        .with(
            c("barChart")
                .with(val("barDir", "col"))
                .with(val("grouping", "clustered"))
                .with(series)
                .with(val("axId", CATEGORY_AXIS))
                .with(val("axId", VALUE_AXIS)),
        )
        .with(axis("catAx", CATEGORY_AXIS, "b", VALUE_AXIS))
        .with(axis("valAx", VALUE_AXIS, "l", CATEGORY_AXIS).with(val("crossBetween", "between")));
    c("chartSpace").with(c("chart").with(plot))
}
