//! Relationship-selected literal data, forked per occurrence on write.
use super::{CHART, unique};
use crate::editor::{
    ImportedDocument, ImportedInlineNode, InlineAnchor, InlineLocation, unsupported,
};
use crate::{
    ColumnChartProperties, Result,
    opc::Package,
    xml::{Content, Node},
};

const MIME: &str = "application/vnd.openxmlformats-officedocument.drawingml.chart+xml";
fn bad() -> crate::Error {
    unsupported("expected a workbook-free, single-series column chart with indexed literal data")
}
fn tag(local: &str) -> String {
    format!("{{{CHART}}}{local}")
}
fn path<'a>(mut n: &'a Node, names: &[&str]) -> Result<&'a Node> {
    for name in names {
        n = unique(n, &tag(name)).ok_or_else(bad)?;
    }
    Ok(n)
}
fn path_mut<'a>(mut n: &'a mut Node, names: &[&str]) -> Result<&'a mut Node> {
    for name in names {
        n = n.child_mut(&tag(name)).ok_or_else(bad)?;
    }
    Ok(n)
}
const LABELS: &[&str] = &["chart", "plotArea", "barChart", "ser", "cat", "strLit"];
const VALUES: &[&str] = &["chart", "plotArea", "barChart", "ser", "val", "numLit"];

fn points(root: &Node, names: &[&str]) -> Result<Vec<String>> {
    let literal = path(root, names)?;
    if path(root, &names[..names.len() - 1])?.elements().count() != 1 {
        return Err(bad());
    }
    let count = path(literal, &["ptCount"])?
        .get("val")
        .and_then(|s| s.parse::<usize>().ok())
        .ok_or_else(bad)?;
    let mut result = Vec::new();
    for p in literal.elements().filter(|n| n.name == tag("pt")) {
        if p.get("idx").and_then(|v| v.parse().ok()) != Some(result.len()) {
            return Err(bad());
        }
        let leaf = path(p, &["v"])?;
        let mut text = String::new();
        for c in &leaf.children {
            if let Content::Text(s) = c {
                text.push_str(s);
            } else {
                return Err(bad());
            }
        }
        result.push(text);
    }
    if result.is_empty() || result.len() != count {
        return Err(bad());
    }
    Ok(result)
}

fn load(package: &Package, main: &str, id: &str) -> Result<(String, Node)> {
    if package
        .relationships(main, "chart")?
        .iter()
        .filter(|(rid, _)| rid == id)
        .count()
        != 1
    {
        return Err(bad());
    }
    let (_, part) = package.related(main, &format!("#{id}"))?.ok_or_else(bad)?;
    if package.content_type_for(&part)?.as_deref() != Some(MIME) {
        return Err(bad());
    }
    let root = package.xml(&part)?;
    let plot = path(&root, &["chart", "plotArea"])?;
    if root.name != tag("chartSpace")
        || root.child(&tag("externalData")).is_some()
        || plot
            .elements()
            .filter(|n| n.name.namespace() == Some(CHART) && n.name.local().ends_with("Chart"))
            .count()
            != 1
        || path(plot, &["barChart", "barDir"])?.get("val") != Some("col")
    {
        return Err(bad());
    }
    Ok((part, root))
}
fn series(root: &Node) -> Result<(Vec<String>, Vec<f64>)> {
    let labels = points(root, LABELS)?;
    let values = points(root, VALUES)?
        .into_iter()
        .map(|s| {
            s.trim()
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(bad)
        })
        .collect::<Result<Vec<_>>>()?;
    if labels.len() != values.len() {
        return Err(bad());
    }
    Ok((labels, values))
}
impl ImportedDocument {
    /// Read explicit category/value pairs from one direct body chart occurrence.
    pub fn chart_series(&self, anchor: &InlineAnchor) -> Result<(Vec<String>, Vec<f64>)> {
        let id = self.chart_id(anchor)?;
        series(&load(&self.package, &self.main, id)?.1)
    }
    pub(in crate::editor) fn chart_id(&self, anchor: &InlineAnchor) -> Result<&str> {
        let node = self.inline_at(anchor)?;
        if !matches!(anchor.location, InlineLocation::Body(_)) || anchor.path.len() != 1 {
            return Err(bad());
        }
        let ImportedInlineNode::Run(r) = node else {
            return Err(bad());
        };
        r.drawing
            .as_ref()
            .filter(|d| d.kind == "chart")
            .and_then(|d| d.relationship_id.as_deref())
            .ok_or_else(bad)
    }
}
pub(in crate::editor) fn fork(
    package: &mut Package,
    main: &str,
    id: &str,
    chart: &ColumnChartProperties,
) -> Result<String> {
    chart.validate()?;
    let (part, mut root) = load(package, main, id)?;
    let (labels, values) = series(&root)?;
    if labels.len() != chart.categories.len() {
        return Err(unsupported(
            "chartSeries patch requires the original point count; replace the chart to change it",
        ));
    }
    if labels == chart.categories && values == chart.values {
        return Ok(id.to_owned());
    }
    for (names, items) in [
        (LABELS, chart.categories.clone()),
        (VALUES, chart.values.iter().map(f64::to_string).collect()),
    ] {
        let literal = path_mut(&mut root, names)?;
        for (point, value) in literal
            .elements_mut()
            .filter(|n| n.name == tag("pt"))
            .zip(items)
        {
            path_mut(point, &["v"])?.children = vec![Content::Text(value)];
        }
    }
    let prefix = part.rsplit_once('/').map_or_else(
        || "rocx-chart".into(),
        |(dir, _)| format!("{dir}/rocx-chart"),
    );
    let out = package.unique(&prefix, "xml");
    package.copy_sibling_relationships(&part, &out)?;
    package.put(&out, root)?;
    package.content_type(&out, MIME)?;
    package.link(main, "chart", &out)
}
