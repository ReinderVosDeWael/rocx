//! Encode the implemented run and paragraph formatting properties.
//!
//! References: ECMA-376 Part 1 (2016), §§17.3.1.26 (`pPr`), 17.3.2.28
//! (`rPr`), 17.3.1.33 (paragraph spacing), and 17.3.2.38 (font size).
//! [Official Part 1](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip).
//! Public units are converted to wire units here; implementation limits and
//! historical compatibility behavior are not necessarily specification rules.
use crate::xml::{Node, val};
use crate::{Color, Error, ParagraphFormat, Points, Result, RunFormat};
pub(crate) fn color(c: [u8; 3]) -> String {
    format!("{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}
fn number(value: f64, factor: f64) -> Result<i64> {
    Ok(Points::new(value)?.scaled(factor)?)
}
pub(crate) fn run(f: &RunFormat) -> Result<Node> {
    let mut n = Node::new("w:rPr");
    if let Some(font) = &f.font {
        n.push(
            Node::new("w:rFonts")
                .attr("w:ascii", font)
                .attr("w:hAnsi", font),
        );
    }
    for (k, v) in [
        ("b", f.bold),
        ("i", f.italic),
        ("caps", f.all_caps),
        ("smallCaps", f.small_caps),
        ("strike", f.strike),
    ] {
        if let Some(v) = v {
            n.push(val(k, if v { "1" } else { "0" }));
        }
    }
    if let Some(c) = f.color {
        let mut value = Node::new("w:color");
        match c {
            Color::Rgb(rgb) => value.set("w:val", color(rgb)),
            Color::Auto => value.set("w:val", "auto"),
            Color::Theme { name, tint, shade } => {
                value.set("w:val", "auto");
                value.set("w:themeColor", name.xml());
                if let Some(tint) = tint {
                    value.set("w:themeTint", format!("{tint:02X}"));
                }
                if let Some(shade) = shade {
                    value.set("w:themeShade", format!("{shade:02X}"));
                }
            }
        }
        n.push(value);
    }
    if let Some(size) = f.size {
        if size < 0. {
            return Err(Error::Validation("font size cannot be negative".into()));
        }
        n.push(val("sz", number(size, 2.)?));
    }
    if let Some(u) = f.underline {
        n.push(val("u", u.xml()));
    }
    if f.subscript == Some(true) {
        n.push(val("vertAlign", "subscript"));
    } else if f.superscript == Some(true) {
        n.push(val("vertAlign", "superscript"));
    }
    Ok(n)
}
pub(crate) fn paragraph(f: &ParagraphFormat, indent_factor: f64) -> Result<Node> {
    let mut n = Node::new("w:pPr");
    if let Some(style) = &f.style {
        n.push(val("pStyle", style));
    }
    if let Some(heading) = f.heading {
        if heading > 9 {
            return Err(Error::Validation("heading must be in 0..=9".into()));
        }
        if heading > 0 {
            n.replace(val("pStyle", format!("Heading{heading}")));
        }
    }
    for (k, v) in [
        ("keepNext", f.keep_with_next),
        ("keepLines", f.keep_together),
        ("pageBreakBefore", f.page_break_before),
        ("widowControl", f.widow_control),
    ] {
        if let Some(v) = v {
            n.push(val(k, if v { "1" } else { "0" }));
        }
    }
    let mut spacing = Node::new("w:spacing");
    for (k, v) in [("before", f.spacing_before), ("after", f.spacing_after)] {
        if let Some(v) = v {
            if v < 0. {
                return Err(Error::Validation(
                    "paragraph spacing cannot be negative".into(),
                ));
            }
            spacing.set(format!("w:{k}"), number(v, 20.)?);
        }
    }
    if let Some(line) = f.line_spacing {
        spacing.set("w:line", number(line, 240.)?);
        spacing.set("w:lineRule", "auto");
    }
    if !spacing.attrs.is_empty() {
        n.push(spacing);
    }
    let mut ind = Node::new("w:ind");
    for (k, v) in [("left", f.left_indent), ("right", f.right_indent)] {
        if let Some(v) = v {
            ind.set(format!("w:{k}"), number(v, indent_factor)?);
        }
    }
    if let Some(first) = f.first_line_indent {
        ind.set(
            if first < 0. {
                "w:hanging"
            } else {
                "w:firstLine"
            },
            number(first.abs(), indent_factor)?,
        );
    }
    if !ind.attrs.is_empty() {
        n.push(ind);
    }
    if let Some(a) = f.alignment {
        n.push(val("jc", a.xml()));
    }
    Ok(n)
}
pub(crate) fn shading(c: [u8; 3]) -> Node {
    Node::new("w:shd")
        .attr("w:val", "clear")
        .attr("w:color", "auto")
        .attr("w:fill", color(c))
}
pub(crate) fn merge_properties(target: &mut Node, source: Node) {
    for item in source.children {
        if let crate::xml::Content::Element(n) = item {
            let name = n.name.clone();
            if let Some(existing) = target.child_mut(&name) {
                if n.children.is_empty() {
                    for (k, v) in n.attrs {
                        existing.set(k, v);
                    }
                } else {
                    target.replace(n);
                }
            } else {
                target.push(n);
            }
        }
    }
}
