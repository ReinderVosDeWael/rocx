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
fn typed_style_id(styles: &Node, name: &str, kind: &str) -> Result<String> {
    let id = style_id(styles, name)?;
    let style = styles
        .elements()
        .find(|node| node.get("w:styleId") == Some(id.as_str()))
        .unwrap();
    if style.get("w:type").unwrap_or("paragraph") != kind {
        return Err(Diagnostic::new(
            Rule::InvalidContent,
            format!("style {name:?} must be a {kind} style"),
        )
        .into());
    }
    Ok(id)
}
fn validate_inheritance(styles: &Node, roots: &[(String, bool)]) -> Result<()> {
    let parents: std::collections::BTreeMap<_, _> = styles
        .elements()
        .filter(|node| node.name == "w:style")
        .filter_map(|node| {
            Some((
                node.get("w:styleId")?,
                node.child("w:basedOn")
                    .and_then(|parent| parent.get("w:val")),
            ))
        })
        .collect();
    for (index, (root, _)) in roots.iter().enumerate() {
        let mut seen = std::collections::BTreeSet::new();
        let mut current = Some(root.as_str());
        while let Some(id) = current {
            if !seen.insert(id) {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    format!("style inheritance cycle reachable from {root:?}"),
                )
                .at(format!("styles[{index}].base_style"))
                .into());
            }
            current = parents.get(id).copied().flatten();
        }
    }
    Ok(())
}
fn apply_run(style: &mut Node, f: &RunFormat) -> Result<()> {
    let rpr = style.ensure("w:rPr");
    crate::ooxml::wordprocessing::format::merge_properties(
        rpr,
        crate::ooxml::wordprocessing::format::run(f)?,
    );
    if let Some(fonts) = rpr.child_mut("w:rFonts") {
        for (theme, overridden) in [
            ("w:asciiTheme", f.font.is_some()),
            ("w:hAnsiTheme", f.font.is_some()),
            ("w:eastAsiaTheme", f.font_east_asia.is_some()),
            ("w:cstheme", f.font_complex_script.is_some()),
        ] {
            if overridden {
                fonts.attrs.retain(|(name, _)| name != theme);
            }
        }
    }
    Ok(())
}
fn section_format(fmt: &TableSectionFormat) -> Result<Node> {
    let mut n = Node::new("w:tblStylePr");
    let mut run = font(
        &fmt.font,
        fmt.font_size,
        fmt.bold,
        fmt.italic,
        fmt.underline,
        fmt.color,
    );
    run.language = fmt.language.clone();
    run.bold_complex_script = fmt.bold_complex_script;
    run.italic_complex_script = fmt.italic_complex_script;
    run.size_complex_script = fmt.size_complex_script;
    run.rtl = fmt.rtl;
    run.complex_script = fmt.complex_script;
    run.font_east_asia = fmt.font_east_asia.clone();
    run.font_complex_script = fmt.font_complex_script.clone();
    let r = crate::ooxml::wordprocessing::format::run(&run)?;
    if !r.children.is_empty() {
        n.push(r);
    }
    let p = crate::ooxml::wordprocessing::format::paragraph(
        &ParagraphFormat {
            alignment: fmt.alignment,
            spacing_before: fmt.spacing_before,
            spacing_after: fmt.spacing_after,
            line_height: fmt.line_height,
            tab_stops: fmt.tab_stops.clone(),
            bidi: fmt.bidi,
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
    defaults: Option<&crate::DocumentDefaults>,
) -> Result<Node> {
    let style_path = package
        .related(main, "styles")?
        .map(|(_, p)| p)
        .unwrap_or("word/styles.xml".into());
    let mut styles = package.word(&style_path)?;
    if let Some(defaults) = defaults {
        defaults.validate()?;
        if let Some(run) = &defaults.run {
            apply_run(styles.ensure("w:docDefaults").ensure("w:rPrDefault"), run)?;
        }
        if let Some(paragraph) = &defaults.paragraph {
            let target = styles
                .ensure("w:docDefaults")
                .ensure("w:pPrDefault")
                .ensure("w:pPr");
            if let Some(ind) = target.child_mut("w:ind") {
                // Character-unit attributes take precedence over point lengths.
                ind.attrs.retain(|(name, _)| {
                    let replaces_first_line = paragraph.first_line_indent.is_some()
                        && [
                            "w:firstLine",
                            "w:hanging",
                            "w:firstLineChars",
                            "w:hangingChars",
                        ]
                        .iter()
                        .any(|key| name == key);
                    let replaces_left_chars =
                        paragraph.left_indent.is_some() && name == "w:leftChars";
                    let replaces_right_chars =
                        paragraph.right_indent.is_some() && name == "w:rightChars";
                    !(replaces_first_line || replaces_left_chars || replaces_right_chars)
                });
            }
            crate::ooxml::wordprocessing::format::merge_properties(
                target,
                crate::ooxml::wordprocessing::format::paragraph(paragraph, 20.)?,
            );
        }
    }
    // Reserve identities before resolving links so authored references can point
    // forward. Existing paragraph styles still update in place; table duplicates fail.
    let mut allocated = Vec::with_capacity(definitions.len());
    for definition in definitions {
        let (name, typ) = match definition {
            StyleDefinition::Character(s) => (&s.name, "character"),
            StyleDefinition::Numbering(s) => (&s.name, "numbering"),
            StyleDefinition::Paragraph(s) => (&s.name, "paragraph"),
            StyleDefinition::Table(s) => (&s.name, "table"),
        };
        if name.is_empty() {
            return Err(Error::Validation("style name cannot be empty".into()));
        }
        let existing = style_id(&styles, name).ok();
        if matches!(typ, "table" | "numbering") && existing.is_some() {
            return Err(Error::Validation(format!(
                "{typ} style {name:?} already exists"
            )));
        }
        if existing.is_some() {
            typed_style_id(&styles, name, typ)?;
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
            let style = Node::new("w:style")
                .attr("w:type", typ)
                .attr("w:customStyle", "1")
                .attr("w:styleId", &id)
                .with(val("name", name));
            styles.push(style);
        }
        allocated.push((id, existing.is_none()));
    }
    let mut numbering_styles = std::collections::BTreeMap::new();
    for (index, (definition, (id, created))) in definitions.iter().zip(&allocated).enumerate() {
        let (base, kind) = match definition {
            StyleDefinition::Character(s) => (
                if *created {
                    s.base_style.as_ref()
                } else {
                    None
                },
                "character",
            ),
            StyleDefinition::Paragraph(s) => (
                if *created {
                    s.base_style.as_ref()
                } else {
                    None
                },
                "paragraph",
            ),
            StyleDefinition::Table(s) => (s.base_style.as_ref(), "table"),
            StyleDefinition::Numbering(_) => (None, "numbering"),
        };
        let base = base
            .map(|name| typed_style_id(&styles, name, kind))
            .transpose()
            .map_err(|error| error.at(format!("styles[{index}].base_style")))?;
        let next = match definition {
            StyleDefinition::Paragraph(s) => s
                .next_paragraph_style
                .as_ref()
                .map(|n| typed_style_id(&styles, n, "paragraph"))
                .transpose()
                .map_err(|error| error.at(format!("styles[{index}].next_paragraph_style")))?,
            _ => None,
        };
        let linked = match definition {
            StyleDefinition::Paragraph(s) => {
                s.linked_style.as_ref().map(|name| (name, "character"))
            }
            StyleDefinition::Character(s) => {
                s.linked_style.as_ref().map(|name| (name, "paragraph"))
            }
            StyleDefinition::Table(_) | StyleDefinition::Numbering(_) => None,
        }
        .map(|(name, kind)| typed_style_id(&styles, name, kind))
        .transpose()
        .map_err(|error| error.at(format!("styles[{index}].linked_style")))?;
        let style = styles
            .elements_mut()
            .find(|n| n.get("w:styleId") == Some(id.as_str()))
            .unwrap();
        if let Some(base) = base {
            style.push(val("basedOn", base));
        }
        if let Some(linked) = linked {
            style.replace(val("link", linked));
        }
        match definition {
            StyleDefinition::Character(s) => {
                if s.format.style.is_some() {
                    return Err(Diagnostic::new(Rule::InvalidContent,
                        "character style formatting cannot contain a run style reference; use base_style")
                        .at(format!("styles[{index}].format.style")).into());
                }
                apply_run(style, &s.format)?;
            }
            StyleDefinition::Paragraph(s) => {
                if let Some(next) = next {
                    style.replace(val("next", next));
                }
                let mut run = font(&s.font, s.font_size, s.bold, s.italic, s.underline, s.color);
                run.language = s.language.clone();
                run.bold_complex_script = s.bold_complex_script;
                run.italic_complex_script = s.italic_complex_script;
                run.size_complex_script = s.size_complex_script;
                run.rtl = s.rtl;
                run.complex_script = s.complex_script;
                run.font_east_asia = s.font_east_asia.clone();
                run.font_complex_script = s.font_complex_script.clone();
                apply_run(style, &run)?;
                let p = ParagraphFormat {
                    bidi: s.bidi,
                    alignment: s.alignment,
                    spacing_before: s.spacing_before,
                    spacing_after: s.spacing_after,
                    line_spacing: s.line_spacing,
                    line_height: s.line_height,
                    tab_stops: s.tab_stops.clone(),
                    left_indent: s.left_indent,
                    right_indent: s.right_indent,
                    first_line_indent: s.first_line_indent,
                    keep_together: s.keep_together,
                    keep_with_next: s.keep_with_next,
                    page_break_before: s.page_break_before,
                    widow_control: s.widow_control,
                    suppress_line_numbers: s.suppress_line_numbers,
                    contextual_spacing: s.contextual_spacing,
                    mirror_indents: s.mirror_indents,
                    snap_to_grid: s.snap_to_grid,
                    ..ParagraphFormat::default()
                };
                crate::ooxml::wordprocessing::format::merge_properties(
                    style.ensure("w:pPr"),
                    crate::ooxml::wordprocessing::format::paragraph(&p, 1440.)?,
                );
            }
            StyleDefinition::Numbering(s) => {
                if s.numbering_id == 0
                    || !numbering.iter().any(|def| {
                        def.id == s.numbering_id
                            && def.inherit_style.is_none()
                            && !def.levels.is_empty()
                    })
                    || numbering_styles
                        .insert(s.numbering_id, id.as_str())
                        .is_some()
                {
                    return Err(Diagnostic::new(
                        Rule::InvalidContent,
                        "numbering style requires a unique nonzero authored concrete numbering ID with local levels",
                    )
                    .at(format!("styles[{index}].numbering_id"))
                    .into());
                }
                style
                    .ensure("w:pPr")
                    .push(Node::new("w:numPr").with(val("numId", s.numbering_id)));
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
    validate_inheritance(&styles, &allocated)?;
    package.put_word(&style_path, styles.clone())?;
    if !numbering.is_empty() {
        let numbering_path = package
            .related(main, "numbering")?
            .map(|(_, p)| p)
            .unwrap_or("word/numbering.xml".into());
        let mut nums = package.word(&numbering_path)?;
        let imported: std::collections::BTreeSet<u32> = nums
            .elements()
            .filter_map(|n| n.get("w:abstractNumId")?.trim().parse().ok())
            .collect();
        let reserved: std::collections::BTreeSet<_> =
            numbering.iter().filter_map(|def| def.abstract_id).collect();
        let mut written = std::collections::BTreeSet::new();
        let mut automatic = imported.last().copied().unwrap_or(0);
        for def in numbering {
            def.validate()?;
            if nums
                .elements()
                .any(|n| n.get("w:numId") == Some(def.id.to_string().as_str()))
            {
                return Err(Error::Validation("numbering ID already exists".into()));
            }
            // P1 (2016) §17.9.1–2: multiple concrete instances may reference one abstract.
            let absid = if let Some(id) = def.abstract_id {
                if imported.contains(&id) {
                    return Err(Error::Validation(
                        "abstract numbering ID already exists".into(),
                    ));
                }
                id
            } else {
                loop {
                    automatic = automatic.checked_add(1).ok_or_else(|| {
                        Error::Validation("automatic abstract numbering IDs exhausted".into())
                    })?;
                    if !reserved.contains(&automatic) {
                        break automatic;
                    }
                }
            };
            if written.insert(absid) {
                let mut abs = Node::new("w:abstractNum").attr("w:abstractNumId", absid);
                if let Some(name) = &def.inherit_style {
                    let id = typed_style_id(&styles, name, "numbering")
                        .map_err(|error| error.at("inherit_style"))?;
                    if !numbering_styles.values().any(|authored| *authored == id) {
                        return Err(Error::Validation(
                            "inherit_style must target an authored numbering style".into(),
                        ));
                    }
                    // P1 (2016) Annex A.1 CT_AbstractNum: a level-free style reference.
                    abs.push(val("numStyleLink", id));
                }
                for level in &def.levels {
                    abs.push(numbering_level(level, &styles)?);
                }
                nums.push(abs);
            }
            let mut concrete = Node::new("w:num")
                .attr("w:numId", def.id)
                .with(val("abstractNumId", absid));
            let mut overrides = std::collections::BTreeMap::new();
            for entry in &def.start_overrides {
                overrides.insert(
                    entry.level,
                    Node::new("w:lvlOverride")
                        .attr("w:ilvl", entry.level)
                        .with(val("startOverride", entry.start)),
                );
            }
            for entry in &def.level_overrides {
                overrides
                    .entry(entry.level)
                    .or_insert_with(|| Node::new("w:lvlOverride").attr("w:ilvl", entry.level))
                    .push(numbering_level(entry, &styles)?);
            }
            for entry in overrides.into_values() {
                concrete.push(entry);
            }
            nums.push(concrete);
            if let Some(style_id) = numbering_styles.get(&def.id) {
                let abstract_node = nums
                    .elements_mut()
                    .find(|node| {
                        node.name == "w:abstractNum"
                            && node.get("w:abstractNumId") == Some(absid.to_string().as_str())
                    })
                    .expect("authored abstract numbering node");
                if abstract_node.child("w:styleLink").is_some() {
                    return Err(Error::Validation(
                        "an abstract numbering definition can own only one numbering style".into(),
                    ));
                }
                abstract_node.push(val("styleLink", *style_id));
            }
        }
        package.put_word(&numbering_path, nums)?;
    }
    Ok(styles)
}

fn numbering_level(level: &crate::NumberingLevel, styles: &Node) -> Result<Node> {
    let mut node = Node::new("w:lvl")
        .attr("w:ilvl", level.level)
        .with(val("start", level.start))
        .with(val("numFmt", &level.format));
    if let Some(restart) = level.restart {
        node.push(val("lvlRestart", restart));
    }
    // P1 (2016) Annex A.1 CT_Lvl: pStyle precedes isLgl and label text.
    if let Some(name) = &level.paragraph_style {
        node.push(val(
            "pStyle",
            typed_style_id(styles, name, "paragraph")
                .map_err(|error| error.at("paragraph_style"))?,
        ));
    }
    if let Some(enabled) = level.is_legal {
        node.push(val("isLgl", u8::from(enabled)));
    }
    if let Some(suffix) = level.suffix {
        node.push(val("suff", suffix.xml()));
    }
    node.push(val("lvlText", &level.text));
    if let Some(alignment) = level.alignment {
        node.push(val("lvlJc", alignment.xml()));
    }
    // P1 (2016) Annex A.1 CT_Lvl: paragraph properties precede label run properties.
    if let Some(format) = &level.paragraph_format {
        node.push(crate::ooxml::wordprocessing::format::paragraph(
            format, 20.,
        )?);
    }
    if let Some(format) = &level.label_format {
        node.push(crate::ooxml::wordprocessing::format::run(format)?);
    }
    Ok(node)
}

use crate::ooxml::WordParts;
