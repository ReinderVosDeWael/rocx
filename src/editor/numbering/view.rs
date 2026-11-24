//! Direct, typed numbering readback; absence never invents a Word default.
use super::*;
use crate::{
    LineHeight, NumberingAlignment, NumberingSuffix, ParagraphFormat, RunFormat, TabAlignment,
    TabLeader, TabStop,
};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ImportedNumberingLevel {
    pub level: u8,
    pub start: Option<u32>,
    pub format: Option<String>,
    pub text: Option<String>,
    pub restart: Option<u32>,
    pub is_legal: Option<bool>,
    pub suffix: Option<NumberingSuffix>,
    pub alignment: Option<NumberingAlignment>,
    pub paragraph_style: Option<String>,
    pub label_format: Option<RunFormat>,
    pub paragraph_format: Option<ParagraphFormat>,
    /// True when some physical content cannot be represented by this view.
    pub has_opaque_content: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ImportedNumberingAbstract {
    pub abstract_id: u32,
    /// Exact stored ID of the owning numbering style (`w:styleLink`).
    pub style_link: Option<String>,
    /// Exact stored ID of a numbering style supplying levels (`w:numStyleLink`).
    pub num_style_link: Option<String>,
    pub levels: Vec<ImportedNumberingLevel>,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ImportedConcreteNumbering {
    pub num_id: u32,
    pub abstract_id: u32,
    /// Exact numbering-style IDs that directly name this concrete instance.
    pub style_ids: Vec<String>,
    pub start_overrides: Vec<NumberingStartOverride>,
    pub level_overrides: Vec<ImportedNumberingLevel>,
}

#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct ImportedNumberingCatalog {
    pub abstracts: Vec<ImportedNumberingAbstract>,
    pub instances: Vec<ImportedConcreteNumbering>,
}

pub(super) fn optional_value(n: &Node, tag: &str) -> Result<Option<String>> {
    unique(n, tag)?.map(|v| scalar(v, true)).transpose()
}

pub(super) fn style_ids(styles: &Node) -> Result<BTreeMap<u32, Vec<String>>> {
    let mut ids = BTreeMap::<_, Vec<_>>::new();
    let mut seen = BTreeSet::new();
    for style in styles.elements().filter(|n| n.name == "w:style") {
        let numbering = style.get("w:type") == Some("numbering");
        if !numbering && style.get("w:styleId").is_none() {
            continue;
        }
        let id = style
            .get("w:styleId")
            .filter(|id| !id.is_empty())
            .ok_or_else(|| unsupported("missing numbering style ID"))?;
        if !seen.insert(id) {
            return Err(unsupported("duplicate style ID in numbering catalog"));
        }
        if !numbering {
            continue;
        }
        let Some(props) = unique(style, "w:pPr")? else {
            continue;
        };
        let Some(num) = unique(props, "w:numPr")? else {
            continue;
        };
        if let Some(n) = unique(num, "w:numId")? {
            ids.entry(value(n)?).or_default().push(id.to_owned());
        }
    }
    Ok(ids)
}

fn toggle(n: &Node) -> Option<bool> {
    match n.get("w:val") {
        None | Some("true" | "1" | "on") => Some(true),
        Some("false" | "0" | "off") => Some(false),
        _ => None,
    }
}

fn integer(value: Option<&str>) -> Option<i32> {
    value?.trim_matches([' ', '\r', '\n', '\t']).parse().ok()
}

fn paragraph_properties(n: &Node) -> (ParagraphFormat, bool) {
    let mut f = ParagraphFormat::default();
    let mut opaque = !discardable(n);
    let mut seen = BTreeSet::new();
    for c in n.elements() {
        if !seen.insert(c.name.as_str()) {
            opaque = true;
            continue;
        }
        let switch = match c.name.as_str() {
            "w:bidi" => Some(&mut f.bidi),
            "w:keepNext" => Some(&mut f.keep_with_next),
            "w:keepLines" => Some(&mut f.keep_together),
            "w:pageBreakBefore" => Some(&mut f.page_break_before),
            "w:widowControl" => Some(&mut f.widow_control),
            "w:suppressLineNumbers" => Some(&mut f.suppress_line_numbers),
            "w:contextualSpacing" => Some(&mut f.contextual_spacing),
            "w:mirrorIndents" => Some(&mut f.mirror_indents),
            "w:snapToGrid" => Some(&mut f.snap_to_grid),
            _ => None,
        };
        if let Some(slot) = switch {
            *slot = toggle(c);
            opaque |= slot.is_none();
            continue;
        }
        let points = |attr| integer(c.get(attr)).map(|v| f64::from(v) / 20.);
        match c.name.as_str() {
            "w:jc" => {
                f.alignment = c.get("w:val").and_then(super::super::read_alignment);
                opaque |= f.alignment.is_none();
            }
            "w:spacing" => {
                f.spacing_before = points("w:before");
                f.spacing_after = points("w:after");
                opaque |= c.get("w:before").is_some() && f.spacing_before.is_none()
                    || c.get("w:after").is_some() && f.spacing_after.is_none();
                if let Some(line) = integer(c.get("w:line")) {
                    f.line_height = match c.get("w:lineRule").unwrap_or("auto") {
                        "auto" => Some(LineHeight::Multiple(f64::from(line) / 240.)),
                        "exact" => Some(LineHeight::Exact(f64::from(line) / 20.)),
                        "atLeast" => Some(LineHeight::AtLeast(f64::from(line) / 20.)),
                        _ => None,
                    };
                    opaque |= f.line_height.is_none();
                } else {
                    opaque |= c.get("w:line").is_some() || c.get("w:lineRule").is_some();
                }
            }
            "w:ind" => {
                f.left_indent = points("w:left");
                f.right_indent = points("w:right");
                let first = points("w:firstLine");
                let hanging = points("w:hanging");
                if c.get("w:firstLine").is_some() && c.get("w:hanging").is_some() {
                    opaque = true;
                } else {
                    f.first_line_indent = first.or_else(|| hanging.map(|v| -v));
                }
                opaque |= c.get("w:left").is_some() && f.left_indent.is_none()
                    || c.get("w:right").is_some() && f.right_indent.is_none()
                    || c.get("w:firstLine").is_some() && first.is_none()
                    || c.get("w:hanging").is_some() && hanging.is_none();
            }
            "w:tabs" => {
                let mut positions = BTreeSet::new();
                for tab in c.elements().filter(|t| t.name == "w:tab") {
                    if let Some((position, alignment, leader)) = tab_stop(tab) {
                        if positions.insert(position) {
                            f.tab_stops.push(TabStop {
                                position: f64::from(position) / 20.,
                                alignment,
                                leader,
                            });
                            continue;
                        }
                    }
                    opaque = true;
                }
            }
            _ => opaque = true,
        }
    }
    (f, opaque)
}

fn tab_stop(n: &Node) -> Option<(i32, TabAlignment, Option<TabLeader>)> {
    let pos = integer(n.get("w:pos"))?;
    let alignment = [
        TabAlignment::Left,
        TabAlignment::Center,
        TabAlignment::Right,
        TabAlignment::Decimal,
        TabAlignment::Bar,
        TabAlignment::Clear,
        TabAlignment::Start,
        TabAlignment::End,
    ]
    .into_iter()
    .find(|a| Some(a.xml()) == n.get("w:val"))?;
    let leader = if let Some(raw) = n.get("w:leader") {
        Some(
            [
                TabLeader::None,
                TabLeader::Dot,
                TabLeader::Hyphen,
                TabLeader::Underscore,
                TabLeader::Heavy,
                TabLeader::MiddleDot,
            ]
            .into_iter()
            .find(|t| t.xml() == raw)?,
        )
    } else {
        None
    };
    Some((pos, alignment, leader))
}

pub(super) fn read_level(n: &Node) -> Result<ImportedNumberingLevel> {
    let level = id(n, "w:ilvl")?;
    if level > 8 {
        return Err(unsupported("invalid numbering level index"));
    }
    let mut opaque = !discardable(n);
    let mut read = |tag: &str| match optional_value(n, tag) {
        Ok(v) => v,
        Err(_) => {
            opaque = true;
            None
        }
    };
    let text = read("w:lvlText");
    let format = read("w:numFmt");
    let paragraph_style = read("w:pStyle");
    let raw_suffix = read("w:suff");
    let raw_alignment = read("w:lvlJc");
    let raw_start = read("w:start");
    let raw_restart = read("w:lvlRestart");
    let start = raw_start.as_deref().and_then(unsigned_decimal);
    let restart = raw_restart.as_deref().and_then(unsigned_decimal);
    let suffix = [
        NumberingSuffix::Tab,
        NumberingSuffix::Space,
        NumberingSuffix::Nothing,
    ]
    .into_iter()
    .find(|v| Some(v.xml()) == raw_suffix.as_deref());
    let alignment = [
        NumberingAlignment::Left,
        NumberingAlignment::Center,
        NumberingAlignment::Right,
        NumberingAlignment::Start,
        NumberingAlignment::End,
    ]
    .into_iter()
    .find(|v| Some(v.xml()) == raw_alignment.as_deref());
    opaque |= raw_start.is_some() && start.is_none()
        || raw_restart.is_some() && restart.is_none()
        || raw_suffix.is_some() && suffix.is_none()
        || raw_alignment.is_some() && alignment.is_none();
    opaque |= format
        .as_deref()
        .is_some_and(|v| !super::super::resource_fields::is_numbering_format(v));
    let is_legal = match unique(n, "w:isLgl") {
        Ok(None) => None,
        Ok(Some(v)) => {
            let value = toggle(v);
            opaque |= value.is_none();
            value
        }
        Err(_) => {
            opaque = true;
            None
        }
    };
    let label_format = match unique(n, "w:rPr") {
        Ok(Some(_)) => {
            let (f, hidden, unmodeled) = super::super::run_format::inspect(n);
            opaque |= unmodeled || hidden.is_some() || f.style.is_some();
            // The declarative theme color cannot store an independent fallback.
            opaque |= n
                .child("w:rPr")
                .and_then(|p| p.child("w:color"))
                .is_some_and(|c| c.get("w:themeColor").is_some() && c.get("w:val") != Some("auto"));
            Some(f)
        }
        Ok(None) => None,
        Err(_) => {
            opaque = true;
            None
        }
    };
    let paragraph_format = match unique(n, "w:pPr") {
        Ok(Some(p)) => {
            let (f, unmodeled) = paragraph_properties(p);
            opaque |= unmodeled;
            Some(f)
        }
        Ok(None) => None,
        Err(_) => {
            opaque = true;
            None
        }
    };
    Ok(ImportedNumberingLevel {
        level: level as u8,
        start,
        format,
        text,
        restart,
        is_legal,
        suffix,
        alignment,
        paragraph_style,
        label_format,
        paragraph_format,
        has_opaque_content: opaque,
    })
}

impl ImportedDocument {
    /// Inspect directly stored levels, overrides and style links in physical
    /// order. This is not a computation of inherited labels or formatting.
    pub fn numbering_catalog(&self) -> Result<ImportedNumberingCatalog> {
        let Some((_, root)) = Part::Numbers.load(&self.package, &self.main)? else {
            return Ok(ImportedNumberingCatalog::default());
        };
        let owners = self
            .style_part()?
            .map(|(_, root)| style_ids(&root))
            .transpose()?
            .unwrap_or_default();
        let mut seen = BTreeSet::new();
        let abstracts = root
            .elements()
            .filter(|n| n.name == "w:abstractNum")
            .map(|n| {
                let abstract_id = id(n, "w:abstractNumId")?;
                if !seen.insert(abstract_id) {
                    return Err(unsupported("duplicate abstract numbering identity"));
                }
                let mut indices = BTreeSet::new();
                let levels = n
                    .elements()
                    .filter(|n| n.name == "w:lvl")
                    .map(|n| {
                        let level = read_level(n)?;
                        if !indices.insert(level.level) {
                            return Err(unsupported("duplicate abstract numbering level"));
                        }
                        Ok(level)
                    })
                    .collect::<Result<_>>()?;
                Ok(ImportedNumberingAbstract {
                    abstract_id,
                    levels,
                    style_link: optional_value(n, "w:styleLink")?,
                    num_style_link: optional_value(n, "w:numStyleLink")?,
                })
            })
            .collect::<Result<_>>()?;
        let instances = instances(&root)?
            .into_iter()
            .zip(root.elements().filter(|n| n.name == "w:num"))
            .map(|(i, n)| {
                let level_overrides = n
                    .elements()
                    .filter(|o| o.name == "w:lvlOverride")
                    .filter_map(|o| o.child("w:lvl"))
                    .map(read_level)
                    .collect::<Result<_>>()?;
                Ok(ImportedConcreteNumbering {
                    num_id: i.num_id,
                    abstract_id: i.abstract_id,
                    start_overrides: i.start_overrides,
                    style_ids: owners.get(&i.num_id).cloned().unwrap_or_default(),
                    level_overrides,
                })
            })
            .collect::<Result<_>>()?;
        Ok(ImportedNumberingCatalog {
            abstracts,
            instances,
        })
    }
}
