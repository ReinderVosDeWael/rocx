//! Direct paragraph object occurrences. Matching always uses expanded names.
use crate::xml::{Content, Node};
pub(super) mod chart;
mod patch;
pub(super) use patch::patch;

const M: &str = "http://schemas.openxmlformats.org/officeDocument/2006/math";
const CHART: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const CANVAS: &str = "http://schemas.openxmlformats.org/drawingml/2006/lockedCanvas";
const PIC: &str = "http://schemas.openxmlformats.org/drawingml/2006/picture";

#[derive(Clone, Copy, Debug)]
pub enum DrawingProperty {
    Size,
    Placement,
    Crop,
    Transform,
    ShapeFill,
    ShapeOutline,
    ShapeText,
    Accessibility,
    ChartSeries,
}
impl DrawingProperty {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "size" => Self::Size,
            "placement" => Self::Placement,
            "crop" => Self::Crop,
            "transform" => Self::Transform,
            "shapeFill" => Self::ShapeFill,
            "shapeOutline" => Self::ShapeOutline,
            "shapeText" => Self::ShapeText,
            "accessibility" => Self::Accessibility,
            "chartSeries" => Self::ChartSeries,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct DrawingOccurrence {
    pub kind: &'static str,
    pub placement: &'static str,
    pub description: Option<String>,
    pub title: Option<String>,
    pub name: Option<String>,
    pub geometry: Option<String>,
    pub shape_text: Option<String>,
    pub crop: Option<[i32; 4]>,
    pub rotation: Option<i32>,
    pub flip_horizontal: Option<bool>,
    pub flip_vertical: Option<bool>,
    pub relationship_id: Option<String>,
    pub width_emu: Option<i64>,
    pub height_emu: Option<i64>,
}

fn descend<'a>(n: &'a Node, path: &[&str]) -> Option<&'a Node> {
    path.iter().try_fold(n, |n, tag| unique(n, tag))
}

fn plain_shape_text(body: &Node) -> Option<String> {
    let mut lines = Vec::new();
    for p in body.elements().filter(|n| n.name == "a:p") {
        let mut line = String::new();
        for r in p.elements() {
            match r.name.as_str() {
                "a:pPr" | "a:endParaRPr" => (),
                "a:r" => {
                    let text = unique(r, "a:t")?;
                    for c in &text.children {
                        match c {
                            Content::Text(s) => line.push_str(s),
                            _ => return None,
                        }
                    }
                }
                _ => return None,
            }
        }
        lines.push(line);
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn only(node: &Node) -> Option<&Node> {
    let mut children = node.elements();
    let child = children.next()?;
    children.next().is_none().then_some(child)
}

fn unique<'a>(node: &'a Node, tag: &str) -> Option<&'a Node> {
    if node.child(super::ALTERNATE).is_some() {
        return None;
    }
    let mut children = node.elements().filter(|n| n.name == tag);
    let child = children.next()?;
    children.next().is_none().then_some(child)
}

pub(super) fn image_asset(run: &mut Node, id: &str) -> crate::Result<bool> {
    let info = drawing(run)
        .filter(|d| d.kind == "image" && d.relationship_id.is_some())
        .ok_or_else(|| super::unsupported("expected a single embedded picture"))?;
    let place = run
        .child_mut("w:drawing")
        .unwrap()
        .elements_mut()
        .next()
        .unwrap();
    let mut blip = place;
    for tag in [
        "a:graphic",
        "a:graphicData",
        "pic:pic",
        "pic:blipFill",
        "a:blip",
    ] {
        blip = blip.child_mut(tag).unwrap();
    }
    if blip.get("r:link").is_some() {
        return Err(super::unsupported("picture also links external media"));
    }
    if info.relationship_id.as_deref() == Some(id) {
        return Ok(false);
    }
    blip.set("r:embed", id);
    Ok(true)
}

pub(super) fn chart_link(run: &mut Node, id: &str) -> crate::Result<bool> {
    let d = drawing(run)
        .filter(|d| d.kind == "chart")
        .ok_or_else(|| super::unsupported("expected chart occurrence"))?;
    if d.relationship_id.as_deref() == Some(id) {
        return Ok(false);
    }
    let place = run
        .child_mut("w:drawing")
        .unwrap()
        .elements_mut()
        .next()
        .unwrap();
    place
        .child_mut("a:graphic")
        .unwrap()
        .child_mut("a:graphicData")
        .unwrap()
        .elements_mut()
        .next()
        .unwrap()
        .set("r:id", id);
    Ok(true)
}

pub(super) fn drawing(run: &Node) -> Option<DrawingOccurrence> {
    if run.name != "w:r" || run.elements().filter(|n| n.name != "w:rPr").count() != 1 {
        return None;
    }
    let place = only(unique(run, "w:drawing")?)?;
    let placement = match place.name.as_str() {
        "wp:inline" => "inline",
        "wp:anchor" => "floating",
        _ => return None,
    };
    let props = unique(place, "wp:docPr")?;
    let extent = unique(place, "wp:extent")?;
    let data = unique(unique(place, "a:graphic")?, "a:graphicData")?;
    let object = only(data)?;
    let uri = data.get("uri")?;
    if object.name.namespace() != Some(uri) {
        return None;
    }
    let (kind, id) = match (uri, object.name.local()) {
        (PIC, "pic") => (
            "image",
            unique(unique(object, "pic:blipFill")?, "a:blip")?.get("r:embed"),
        ),
        (CHART, "chart") => ("chart", object.get("r:id")),
        (CANVAS, "lockedCanvas") => ("shape", None),
        _ => return None,
    };
    let tx = descend(object, &["a:sp", "a:txSp", "a:txBody"])
        .or_else(|| descend(object, &["a:txSp", "a:txBody"]));
    let geometry = descend(object, &["a:sp", "a:spPr", "a:prstGeom"])
        .and_then(|n| n.get("prst"))
        .or_else(|| (kind == "shape" && object.child("a:txSp").is_some()).then_some("textBox"));
    let xfrm = descend(object, &["pic:spPr", "a:xfrm"]);
    let flip = |name| match xfrm?.get(name)? {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    };
    let crop = descend(object, &["pic:blipFill", "a:srcRect"]).and_then(|r| {
        let edge = |k| r.get(k).unwrap_or("0").parse::<i32>().ok();
        Some([edge("l")?, edge("t")?, edge("r")?, edge("b")?])
    });
    Some(DrawingOccurrence {
        title: props.get("title").map(str::to_owned),
        name: props.get("name").map(str::to_owned),
        shape_text: tx.and_then(plain_shape_text),
        geometry: geometry.map(str::to_owned),
        crop,
        rotation: xfrm.and_then(|n| n.get("rot")?.parse().ok()),
        flip_horizontal: flip("flipH"),
        flip_vertical: flip("flipV"),
        kind,
        placement,
        relationship_id: id.map(str::to_owned),
        description: props.get("descr").map(str::to_owned),
        width_emu: extent.get("cx").and_then(|s| s.parse().ok()),
        height_emu: extent.get("cy").and_then(|s| s.parse().ok()),
    })
}

pub(super) fn description(run: &mut Node, value: Option<&str>) -> crate::Result<bool> {
    drawing(run).ok_or_else(|| super::unsupported("expected a single drawing occurrence"))?;
    if let Some(text) = value {
        crate::xml::validate_text(text)?;
    }
    let props = run
        .child_mut("w:drawing")
        .unwrap()
        .elements_mut()
        .next()
        .unwrap()
        .child_mut("wp:docPr")
        .unwrap();
    if props.get("descr") == value {
        return Ok(false);
    }
    props.attrs.retain(|(name, _)| name != "descr");
    if let Some(text) = value {
        props.set("descr", text);
    }
    Ok(true)
}

/// Recognize only the simple two-text operand grammar. Annotations, complex
/// math, tracked changes, or extension properties keep the entire math opaque.
pub(super) fn fraction(node: &Node) -> Option<(String, String)> {
    fn plain(n: &Node, tag: &str) -> bool {
        n.name.namespace() == Some(M)
            && n.name.local() == tag
            && n.attrs
                .iter()
                .all(|(k, _)| k.declaration().is_some() || tag == "t" && k == "xml:space")
            && n.children.iter().all(|c| match c {
                Content::Element(_) => tag != "t",
                Content::Text(t) => tag == "t" || t.trim().is_empty(),
                Content::Raw(_) => false,
            })
    }
    fn operand(n: &Node, tag: &str) -> Option<String> {
        let r = only(n)?;
        let t = only(r)?;
        (plain(n, tag) && plain(r, "r") && plain(t, "t")).then(|| {
            t.children
                .iter()
                .filter_map(|c| match c {
                    Content::Text(t) => Some(t.as_str()),
                    _ => None,
                })
                .collect()
        })
    }
    if !plain(node, "oMath") {
        return None;
    }
    let f = only(node)?;
    if !plain(f, "f") {
        return None;
    }
    let mut terms = f.elements();
    let result = (
        operand(terms.next()?, "num")?,
        operand(terms.next()?, "den")?,
    );
    terms.next().is_none().then_some(result)
}

/// Destruction is limited to the vocabulary used by the supported drawing
/// codecs. Relationship targets themselves are never deleted or overwritten.
pub(super) fn disposable(n: &Node) -> bool {
    let attrs: &[&str] = match n.name.as_str() {
        "w:drawing"
        | "a:graphic"
        | "pic:pic"
        | "pic:nvPicPr"
        | "pic:cNvPicPr"
        | "pic:blipFill"
        | "a:stretch"
        | "a:fillRect"
        | "pic:spPr"
        | "a:avLst"
        | "wp:cNvGraphicFramePr"
        | "a:nvGrpSpPr"
        | "a:cNvGrpSpPr"
        | "a:grpSpPr"
        | "a:txSp"
        | "a:sp"
        | "a:nvSpPr"
        | "a:cNvSpPr"
        | "a:spPr"
        | "a:solidFill"
        | "a:txBody"
        | "a:p"
        | "a:r"
        | "a:t"
        | "a:useSpRect" => &[],
        "wp:inline" => &["distT", "distB", "distL", "distR"],
        "wp:anchor" => &[
            "simplePos",
            "relativeHeight",
            "behindDoc",
            "locked",
            "layoutInCell",
            "allowOverlap",
            "distT",
            "distB",
            "distL",
            "distR",
        ],
        "wp:wrapSquare"
        | "wp:wrapThrough"
        | "wp:wrapTight"
        | "wp:wrapTopAndBottom"
        | "wp:wrapNone" => &["wrapText", "distT", "distB", "distL", "distR"],
        "wp:wrapPolygon" => &["edited"],
        "wp:positionH" | "wp:positionV" => &["relativeFrom"],
        "wp:align" | "wp:posOffset" => &[],
        "wp:docPr" | "pic:cNvPr" | "a:cNvPr" => &["id", "name", "descr", "title"],
        "a:graphicFrameLocks" => &["noChangeAspect"],
        "a:graphicData" => &["uri"],
        "a:blip" => &["r:embed"],
        "a:srcRect" => &["l", "r", "t", "b"],
        "a:xfrm" => &["rot", "flipH", "flipV"],
        "a:off" | "a:chOff" | "wp:simplePos" | "wp:start" | "wp:lineTo" => &["x", "y"],
        "a:ext" | "a:chExt" | "wp:extent" => &["cx", "cy"],
        "a:prstGeom" => &["prst"],
        "a:srgbClr" => &["val"],
        "a:ln" => &["w"],
        "a:bodyPr" => &["anchor"],
        "a:rPr" | "a:endParaRPr" => &["sz", "b", "i"],
        "a:pPr" => &["algn"],
        "a:latin" => &["typeface"],
        _ if n.name.namespace() == Some(CANVAS) && n.name.local() == "lockedCanvas" => &[],
        _ if n.name.namespace() == Some(CHART) && n.name.local() == "chart" => &["r:id"],
        _ => return false,
    };
    n.attrs
        .iter()
        .all(|(k, _)| k.declaration().is_some() || attrs.contains(&k.as_str()))
        && n.children.iter().all(|c| match c {
            Content::Element(child) => disposable(child),
            Content::Text(t) => {
                t.trim().is_empty()
                    || matches!(n.name.as_str(), "a:t" | "wp:align" | "wp:posOffset")
            }
            Content::Raw(_) => false,
        })
}
