use super::{DrawingProperty as P, disposable, drawing, unique};
use crate::Result;
use crate::editor::unsupported;
use crate::xml::{Content, Node};

fn ambiguous() -> crate::Error {
    unsupported("drawing property is absent, ambiguous or opaque")
}

fn path<'a>(mut node: &'a Node, names: &[&str]) -> Result<&'a Node> {
    for tag in names {
        node = unique(node, tag).ok_or_else(ambiguous)?;
    }
    Ok(node)
}
fn path_mut<'a>(mut node: &'a mut Node, names: &[&str]) -> Result<&'a mut Node> {
    for tag in names {
        unique(node, tag).ok_or_else(ambiguous)?;
        node = node.child_mut(tag).unwrap();
    }
    Ok(node)
}
fn attrs(node: &mut Node, from: &Node, names: &[&str]) {
    for &key in names {
        match from.get(key) {
            Some(v) => node.set(key, v),
            None => node.attrs.retain(|(name, _)| name != key),
        }
    }
}
/// Splice one modeled optional element. An extension on the element being
/// replaced must never disappear as a side effect of a property patch.
fn child(dst: &mut Node, src: &Node, tag: &str, before: &[&str]) -> Result<()> {
    if dst.child(super::super::ALTERNATE).is_some()
        || dst.elements().filter(|n| n.name == tag).count() > 1
        || dst.child(tag).is_some_and(|n| !disposable(n))
    {
        return Err(ambiguous());
    }
    let old = dst
        .children
        .iter()
        .position(|c| matches!(c,Content::Element(n) if n.name==tag));
    match (old, src.child(tag)) {
        (Some(i), Some(n)) => dst.children[i] = Content::Element(n.clone()),
        (Some(i), None) => {
            dst.children.remove(i);
        }
        (None, Some(n)) => {
            let slot = dst
                .children
                .iter()
                .position(|c| matches!(c,Content::Element(n) if before.contains(&n.name.as_str())))
                .unwrap_or(dst.children.len());
            dst.children.insert(slot, Content::Element(n.clone()));
        }
        _ => (),
    }
    Ok(())
}
fn body(place: &Node) -> Result<&Node> {
    path(place, &["a:graphic", "a:graphicData"])?
        .elements()
        .next()
        .ok_or_else(ambiguous)
}
fn body_mut(place: &mut Node) -> Result<&mut Node> {
    path_mut(place, &["a:graphic", "a:graphicData"])?
        .elements_mut()
        .next()
        .ok_or_else(ambiguous)
}

pub(in crate::editor) fn patch(run: &mut Node, from: &Node, property: P) -> Result<bool> {
    let kind = drawing(run).ok_or_else(ambiguous)?.kind;
    if drawing(from).is_none_or(|src| src.kind != kind) {
        return Err(unsupported("drawing patch requires the same object kind"));
    }
    let dest = run
        .child_mut("w:drawing")
        .unwrap()
        .elements_mut()
        .next()
        .unwrap();
    let source = from.child("w:drawing").unwrap().elements().next().unwrap();
    let before = dest.bytes();
    match property {
        P::Size => {
            let extent = path(source, &["wp:extent"])?;
            let mut paths: Vec<&[&str]> = vec![];
            match kind {
                "image" => paths.push(&["pic:spPr", "a:xfrm", "a:ext"]),
                "shape" => {
                    paths.extend([
                        &["a:grpSpPr", "a:xfrm", "a:ext"][..],
                        &["a:grpSpPr", "a:xfrm", "a:chExt"][..],
                    ]);
                    let canvas = body(dest)?;
                    if canvas.child("a:sp").is_some() {
                        paths.push(&["a:sp", "a:spPr", "a:xfrm", "a:ext"]);
                    } else {
                        paths.push(&["a:txSp", "a:xfrm", "a:ext"]);
                    }
                }
                _ => (),
            }
            attrs(path_mut(dest, &["wp:extent"])?, extent, &["cx", "cy"]);
            for names in paths {
                attrs(path_mut(body_mut(dest)?, names)?, extent, &["cx", "cy"]);
            }
        }
        P::Accessibility => {
            attrs(
                path_mut(dest, &["wp:docPr"])?,
                path(source, &["wp:docPr"])?,
                &["name", "title", "descr"],
            );
        }
        P::Placement if kind != "chart" => {
            let common = ["wp:extent", "wp:docPr", "wp:cNvGraphicFramePr", "a:graphic"];
            let mut layout = dest.clone();
            layout
                .children
                .retain(|c| !matches!(c,Content::Element(n) if common.contains(&n.name.as_str())));
            if !disposable(&layout) {
                return Err(ambiguous());
            }
            let mut replacement = source.clone();
            replacement
                .children
                .retain(|c| !matches!(c,Content::Element(n) if common.contains(&n.name.as_str())));
            // CT_Anchor orders extent before the wrap element, unlike the
            // remaining common children which follow it.
            let at = replacement
                .children
                .iter()
                .position(
                    |c| matches!(c,Content::Element(n) if n.name.as_str().starts_with("wp:wrap")),
                )
                .unwrap_or(replacement.children.len());
            replacement
                .children
                .insert(at, Content::Element(path(dest, &["wp:extent"])?.clone()));
            for tag in &common[1..] {
                if dest.elements().filter(|n| n.name == *tag).count() > 1 {
                    return Err(ambiguous());
                }
                if let Some(n) = dest.child(tag) {
                    replacement.push(n.clone());
                }
            }
            // Keep namespace declarations: opaque descendants can use prefix-valued attributes.
            for (name, v) in &dest.attrs {
                if name.declaration().is_some() {
                    replacement.set(name.clone(), v);
                }
            }
            *dest = replacement;
        }
        P::Crop if kind == "image" => {
            child(
                path_mut(body_mut(dest)?, &["pic:blipFill"])?,
                path(body(source)?, &["pic:blipFill"])?,
                "a:srcRect",
                &["a:stretch", "a:tile"],
            )?;
        }
        P::Transform if kind == "image" => {
            let names = ["pic:spPr", "a:xfrm"];
            attrs(
                path_mut(body_mut(dest)?, &names)?,
                path(body(source)?, &names)?,
                &["rot", "flipH", "flipV"],
            );
        }
        P::ShapeGeometry if kind == "shape" => {
            let names = ["a:sp", "a:spPr", "a:prstGeom"];
            let geom = path_mut(body_mut(dest)?, &names)?;
            if !matches!(geom.get("prst"), Some("rect" | "ellipse"))
                || !disposable(geom)
                || geom
                    .elements()
                    .any(|n| n.name != "a:avLst" || n.elements().next().is_some())
            {
                return Err(ambiguous());
            }
            attrs(geom, path(body(source)?, &names)?, &["prst"]);
        }
        P::ShapeFill | P::ShapeOutline if kind == "shape" => {
            let names = ["a:sp", "a:spPr"];
            let dst = path_mut(body_mut(dest)?, &names)?;
            let src = path(body(source)?, &names)?;
            if matches!(property, P::ShapeFill) {
                if [
                    "a:noFill",
                    "a:gradFill",
                    "a:blipFill",
                    "a:pattFill",
                    "a:grpFill",
                ]
                .iter()
                .any(|k| dst.child(k).is_some())
                {
                    return Err(ambiguous());
                }
                child(
                    dst,
                    src,
                    "a:solidFill",
                    &[
                        "a:ln",
                        "a:effectLst",
                        "a:effectDag",
                        "a:scene3d",
                        "a:sp3d",
                        "a:extLst",
                    ],
                )?;
            } else {
                child(
                    dst,
                    src,
                    "a:ln",
                    &[
                        "a:effectLst",
                        "a:effectDag",
                        "a:scene3d",
                        "a:sp3d",
                        "a:extLst",
                    ],
                )?;
            }
        }
        P::ShapeText if kind == "shape" => {
            let dst = body_mut(dest)?;
            let src = body(source)?;
            if dst.child("a:sp").is_some() {
                child(
                    path_mut(dst, &["a:sp"])?,
                    path(src, &["a:sp"])?,
                    "a:txSp",
                    &["a:style", "a:extLst"],
                )?;
            } else {
                child(
                    path_mut(dst, &["a:txSp"])?,
                    path(src, &["a:txSp"])?,
                    "a:txBody",
                    &["a:xfrm", "a:useSpRect", "a:extLst"],
                )?;
            }
        }
        _ => return Err(unsupported("property does not apply to this drawing kind")),
    }
    Ok(dest.bytes() != before)
}
