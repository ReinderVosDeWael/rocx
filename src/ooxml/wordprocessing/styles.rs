//! Serialize style definitions and numbering resources.
//!
//! References: ECMA-376 Part 1 (2016), §§17.7 (styles), 17.9 (numbering),
//! 11.3.12 (Style Definitions Part), and 11.3.11 (Numbering Definitions Part).
//! [Official Part 1](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip).
use crate::opc::Package;
use crate::xml::{Node, val};
use crate::{
    Color, Diagnostic, Error, NumberingDefinition, ParagraphFormat, Result, Rule, RunFormat,
    StyleDefinition, TableSectionFormat, Underline,
};
fn font(
    name: &Option<String>,
    size: Option<f64>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    color: Option<[u8; 3]>,
) -> RunFormat {
    RunFormat {
        font: name.clone(),
        size,
        bold,
        italic,
        underline: underline.map(Underline::from),
        color: color.map(Color::Rgb),
        ..RunFormat::default()
    }
}
pub(crate) fn style_id(styles: &Node, name: &str) -> Result<String> {
    styles
        .elements()
        .find(|n| {
            n.name == "w:style"
                && (n.get("w:styleId") == Some(name)
                    || n.child("w:name")
                        .and_then(|n| n.get("w:val"))
                        .is_some_and(|s| s.eq_ignore_ascii_case(name)))
        })
        .and_then(|n| n.get("w:styleId"))
        .map(str::to_owned)
        .ok_or_else(|| {
            Error::Diagnostic(Diagnostic::new(
                Rule::UnknownStyle,
                format!("unknown style {name:?}"),
            ))
        })
}
fn apply_run(style: &mut Node, f: &RunFormat) -> Result<()> {
    let rpr = style.ensure("w:rPr");
    crate::ooxml::wordprocessing::format::merge_properties(
        rpr,
        crate::ooxml::wordprocessing::format::run(f)?,
    );
    if f.font.is_some()
        && let Some(fonts) = rpr.child_mut("w:rFonts")
    {
        fonts
            .attrs
            .retain(|(k, _)| k != "w:asciiTheme" && k != "w:hAnsiTheme");
    }
    Ok(())
}
fn section_format(fmt: &TableSectionFormat) -> Result<Node> {
    let mut n = Node::new("w:tblStylePr");
    let r = crate::ooxml::wordprocessing::format::run(&font(
        &fmt.font,
        fmt.font_size,
        fmt.bold,
        fmt.italic,
        fmt.underline,
        fmt.color,
    ))?;
    if !r.children.is_empty() {
        n.push(r);
    }
    let p = crate::ooxml::wordprocessing::format::paragraph(
        &ParagraphFormat {
            alignment: fmt.alignment,
            spacing_before: fmt.spacing_before,
            spacing_after: fmt.spacing_after,
            ..ParagraphFormat::default()
        },
        20.,
    )?;
    if !p.children.is_empty() {
        n.push(p);
    }
    if let Some(c) = fmt.background {
        n.push(Node::new("w:tcPr").with(crate::ooxml::wordprocessing::format::shading(c)));
    }
    Ok(n)
}
pub(crate) fn apply(
    package: &mut Package,
    main: &str,
    definitions: &[StyleDefinition],
    numbering: &[NumberingDefinition],
) -> Result<Node> {
    let style_path = package
        .related(main, "styles")?
        .map(|(_, p)| p)
        .unwrap_or("word/styles.xml".into());
    let mut styles = package.word(&style_path)?;
    for definition in definitions {
        let (name, typ, base) = match definition {
            StyleDefinition::Paragraph(s) => (&s.name, "paragraph", &s.base_style),
            StyleDefinition::Table(s) => (&s.name, "table", &s.base_style),
        };
        if name.is_empty() {
            return Err(Error::Validation("style name cannot be empty".into()));
        }
        let existing = style_id(&styles, name).ok();
        if typ == "table" && existing.is_some() {
            return Err(Error::Validation(format!(
                "table style {name:?} already exists"
            )));
        }
        let id = existing.clone().unwrap_or_else(|| name.replace(' ', ""));
        if existing.is_none() {
            if styles
                .elements()
                .any(|n| n.get("w:styleId") == Some(id.as_str()))
            {
                return Err(Error::Validation(
                    "style IDs collide after removing spaces".into(),
                ));
            }
            let mut style = Node::new("w:style")
                .attr("w:type", typ)
                .attr("w:customStyle", "1")
                .attr("w:styleId", &id)
                .with(val("name", name));
            if let Some(base) = base {
                style.push(val("basedOn", style_id(&styles, base)?));
            }
            styles.push(style);
        }
        let next = match definition {
            StyleDefinition::Paragraph(s) => s
                .next_paragraph_style
                .as_ref()
                .map(|n| style_id(&styles, n))
                .transpose()?,
            _ => None,
        };
        let style = styles
            .elements_mut()
            .find(|n| n.get("w:styleId") == Some(id.as_str()))
            .unwrap();
        match definition {
            StyleDefinition::Paragraph(s) => {
                if let Some(next) = next {
                    style.replace(val("next", next));
                }
                apply_run(
                    style,
                    &font(&s.font, s.font_size, s.bold, s.italic, s.underline, s.color),
                )?;
                let p = ParagraphFormat {
                    alignment: s.alignment,
                    spacing_before: s.spacing_before,
                    spacing_after: s.spacing_after,
                    line_spacing: s.line_spacing,
                    left_indent: s.left_indent,
                    right_indent: s.right_indent,
                    first_line_indent: s.first_line_indent,
                    keep_together: s.keep_together,
                    keep_with_next: s.keep_with_next,
                    page_break_before: s.page_break_before,
                    widow_control: s.widow_control,
                    ..ParagraphFormat::default()
                };
                crate::ooxml::wordprocessing::format::merge_properties(
                    style.ensure("w:pPr"),
                    crate::ooxml::wordprocessing::format::paragraph(&p, 1440.)?,
                );
            }
            StyleDefinition::Table(s) => {
                if let Some(fmt) = &s.whole_table {
                    let n = section_format(fmt)?;
                    for c in n.children {
                        if let crate::xml::Content::Element(n) = c {
                            style.push(n);
                        }
                    }
                }
                for (kind, fmt) in [
                    ("firstRow", &s.first_row),
                    ("lastRow", &s.last_row),
                    ("firstCol", &s.first_column),
                    ("lastCol", &s.last_column),
                    ("band1Horz", &s.banding_1_row),
                    ("band2Horz", &s.banding_2_row),
                    ("band1Vert", &s.banding_1_column),
                    ("band2Vert", &s.banding_2_column),
                    ("nwCell", &s.top_left_cell),
                    ("neCell", &s.top_right_cell),
                    ("swCell", &s.bottom_left_cell),
                    ("seCell", &s.bottom_right_cell),
                ] {
                    if let Some(fmt) = fmt {
                        style.push(section_format(fmt)?.attr("w:type", kind));
                    }
                }
            }
        }
    }
    package.put_word(&style_path, styles.clone())?;
    if !numbering.is_empty() {
        let numbering_path = package
            .related(main, "numbering")?
            .map(|(_, p)| p)
            .unwrap_or("word/numbering.xml".into());
        let mut nums = package.word(&numbering_path)?;
        for def in numbering {
            if nums
                .elements()
                .any(|n| n.get("w:numId") == Some(def.id.to_string().as_str()))
            {
                return Err(Error::Validation("numbering ID already exists".into()));
            }
            let absid = nums
                .elements()
                .filter_map(|n| n.get("w:abstractNumId"))
                .filter_map(|s| s.parse::<u32>().ok())
                .max()
                .unwrap_or(0)
                + 1;
            let mut abs = Node::new("w:abstractNum").attr("w:abstractNumId", absid);
            for l in &def.levels {
                if l.level > 8 {
                    return Err(Error::Validation("numbering level must be in 0..=8".into()));
                }
                abs.push(
                    Node::new("w:lvl")
                        .attr("w:ilvl", l.level)
                        .with(val("start", l.start))
                        .with(val("numFmt", &l.format))
                        .with(val("lvlText", &l.text)),
                );
            }
            nums.push(abs);
            nums.push(
                Node::new("w:num")
                    .attr("w:numId", def.id)
                    .with(val("abstractNumId", absid)),
            );
        }
        package.put_word(&numbering_path, nums)?;
    }
    Ok(styles)
}

use crate::ooxml::WordParts;
