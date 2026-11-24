//! Direct layout fields. Measurements are wire-exact integers, not effective
//! formatting. Changes are made only on the immutable source's direct owners.
use super::{
    ALTERNATE, BoundaryState, EditPreview, ImportedDocument, ParagraphAnchor, SectionAnchor,
    header, missing_body, unsupported,
};
use crate::Result;
use crate::ooxml::WordParts;
use crate::opc::PackagePlan;
use crate::xml::{Content, Node};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutTab {
    pub position: i32,
    pub alignment: String,
    pub leader: Option<String>,
}

/// A single declarative page border. Size is in eighth-points; space in points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutBorder {
    pub size: Option<i32>,
    pub space: Option<i32>,
    pub color: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutValue {
    Integer(i32),
    Token(String),
    Boolean(bool),
    Tabs(Vec<LayoutTab>),
    Border(LayoutBorder),
}

#[derive(Clone, Copy)]
enum Kind {
    Num(i32, i32),
    Token(&'static str),
    Bool,
    Style,
    Heading,
    Tabs,
    Edge(&'static str),
    Group,
}
use Kind::*;
const SIGNED: Kind = Num(i32::MIN, i32::MAX);
const UNSIGNED: Kind = Num(0, i32::MAX);
const POSITIVE: Kind = Num(1, i32::MAX);
const TAB_ALIGN: &str = "left center right decimal bar clear start end";
const TAB_LEADER: &str = "none dot hyphen underscore heavy middleDot";
type Spec = (bool, &'static str, &'static str, Kind);
macro_rules! fields {
    ($($f:ident: $section:expr, $node:literal, $attr:literal, $kind:expr;)+) => {
        #[derive(Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord)]
        pub enum LayoutField { $($f,)+ }
        impl LayoutField {
            pub const ALL: &'static [Self] = &[$(Self::$f,)+];
            fn variant(self) -> &'static str { match self { $(Self::$f => stringify!($f),)+ } }
            fn spec(self) -> Spec { match self { $(Self::$f => ($section, concat!("w:",$node), concat!("w:",$attr),$kind),)+ } }
        }
    }
}
fields! {
    StyleId: false,"pStyle","val",Style;
    Heading: false,"pStyle","val",Heading;
    KeepWithNext: false,"keepNext","val",Bool;
    KeepTogether: false,"keepLines","val",Bool;
    PageBreakBefore: false,"pageBreakBefore","val",Bool;
    WidowControl: false,"widowControl","val",Bool;
    SuppressLineNumbers: false,"suppressLineNumbers","val",Bool;
    ParagraphBidi: false,"bidi","val",Bool;
    SnapToGrid: false,"snapToGrid","val",Bool;
    ContextualSpacing: false,"contextualSpacing","val",Bool;
    MirrorIndents: false,"mirrorIndents","val",Bool;
    Alignment: false,"jc","val",Token("left start center right end both distribute mediumKashida highKashida lowKashida thaiDistribute");
    OutlineLevel: false,"outlineLvl","val",Num(0,9);
    SpaceBefore: false,"spacing","before",UNSIGNED;
    SpaceAfter: false,"spacing","after",UNSIGNED;
    LineHeight: false,"spacing","line",POSITIVE;
    LineRule: false,"spacing","lineRule",Token("auto exact atLeast");
    LeftIndent: false,"ind","left",SIGNED;
    RightIndent: false,"ind","right",SIGNED;
    FirstLineIndent: false,"ind","firstLine",SIGNED;
    TabStops: false,"tabs","",Tabs;
    PageWidth: true,"pgSz","w",POSITIVE;
    PageHeight: true,"pgSz","h",POSITIVE;
    Orientation: true,"pgSz","orient",Token("portrait landscape");
    MarginTop: true,"pgMar","top",SIGNED;
    MarginBottom: true,"pgMar","bottom",SIGNED;
    MarginLeft: true,"pgMar","left",UNSIGNED;
    MarginRight: true,"pgMar","right",UNSIGNED;
    MarginHeader: true,"pgMar","header",UNSIGNED;
    MarginFooter: true,"pgMar","footer",UNSIGNED;
    MarginGutter: true,"pgMar","gutter",UNSIGNED;
    ColumnCount: true,"cols","num",Num(1,45);
    ColumnSpace: true,"cols","space",UNSIGNED;
    ColumnSeparator: true,"cols","sep",Bool;
    GridMode: true,"docGrid","type",Token("default lines linesAndChars snapToChars");
    GridPitch: true,"docGrid","linePitch",POSITIVE;
    GridCharSpace: true,"docGrid","charSpace",SIGNED;
    PageStart: true,"pgNumType","start",UNSIGNED;
    PageFormat: true,"pgNumType","fmt",Token("decimal upperRoman lowerRoman upperLetter lowerLetter ordinal cardinalText ordinalText decimalZero none");
    LineStart: true,"lnNumType","start",Num(0,32767);
    LineCountBy: true,"lnNumType","countBy",Num(1,100);
    LineDistance: true,"lnNumType","distance",UNSIGNED;
    LineRestart: true,"lnNumType","restart",Token("newPage newSection continuous");
    VerticalAlign: true,"vAlign","val",Token("top center bottom both");
    SectionType: true,"type","val",Token("nextPage nextColumn continuous evenPage oddPage");
    TitlePage: true,"titlePg","val",Bool;
    Bidi: true,"bidi","val",Bool;
    RtlGutter: true,"rtlGutter","val",Bool;
    BorderOffset: true,"pgBorders","offsetFrom",Token("page text");
    BorderDisplay: true,"pgBorders","display",Token("allPages firstPage notFirstPage");
    BorderZOrder: true,"pgBorders","zOrder",Token("front back");
    BorderTop: true,"pgBorders","top",Edge("w:top");
    BorderLeft: true,"pgBorders","left",Edge("w:left");
    BorderBottom: true,"pgBorders","bottom",Edge("w:bottom");
    BorderRight: true,"pgBorders","right",Edge("w:right");
    Margins: true,"pgMar","",Group;
    LineNumbering: true,"lnNumType","",Group;
}

impl LayoutField {
    pub fn is_section(self) -> bool {
        self.spec().0
    }
    pub fn name(self) -> String {
        let variant = self.variant();
        format!("{}{}", variant[..1].to_ascii_lowercase(), &variant[1..])
    }
    pub fn python_name(self) -> String {
        let mut out = String::new();
        for c in self.variant().chars() {
            if c.is_ascii_uppercase() && !out.is_empty() {
                out.push('_')
            }
            out.push(c.to_ascii_lowercase());
        }
        out
    }
}

#[derive(Clone, Debug)]
pub enum LayoutEdit {
    Paragraph(ParagraphAnchor, LayoutField, Option<LayoutValue>),
    Section(SectionAnchor, LayoutField, Option<LayoutValue>),
}

fn contains(choices: &str, token: &str) -> bool {
    choices.split_ascii_whitespace().any(|v| v == token)
}
fn simple(node: &Node, attrs: &[&str]) -> bool {
    node.attrs
        .iter()
        .all(|(k, _)| k.declaration().is_some() || attrs.iter().any(|a| k == *a))
        && node
            .children
            .iter()
            .all(|c| matches!(c,Content::Text(t) if t.trim().is_empty()))
}
fn tab_list(node: &Node) -> Option<Vec<LayoutTab>> {
    let mut seen = BTreeSet::new();
    node.elements()
        .map(|t| {
            if t.name != "w:tab" {
                return None;
            }
            let position = t.get("w:pos")?.parse().ok()?;
            let alignment = t.get("w:val")?;
            let leader = t.get("w:leader");
            (seen.insert(position)
                && contains(TAB_ALIGN, alignment)
                && leader.is_none_or(|l| contains(TAB_LEADER, l)))
            .then(|| LayoutTab {
                position,
                alignment: alignment.into(),
                leader: leader.map(str::to_owned),
            })
        })
        .collect()
}
fn value(field: LayoutField, n: &Node) -> Option<LayoutValue> {
    if n.child(ALTERNATE).is_some() {
        return None;
    }
    let (_, _, attr, kind) = field.spec();
    let raw = n.get(attr);
    if field == LayoutField::FirstLineIndent
        && let Some(v) = n.get("w:hanging")
    {
        return if raw.is_some() {
            None
        } else {
            v.parse::<i32>()
                .ok()
                .filter(|&v| v >= 0)?
                .checked_neg()
                .map(LayoutValue::Integer)
        };
    }
    Some(match kind {
        Num(min, max) => LayoutValue::Integer(
            raw?.parse::<i32>()
                .ok()
                .filter(|v| (min..=max).contains(v))?,
        ),
        Token(c) => LayoutValue::Token(raw.filter(|v| contains(c, v))?.into()),
        Style => LayoutValue::Token(raw?.into()),
        Heading => {
            let suffix = raw?.strip_prefix("Heading")?;
            if suffix.len() != 1 {
                return None;
            }
            LayoutValue::Integer(suffix.parse::<i32>().ok().filter(|v| (1..=9).contains(v))?)
        }
        Bool => LayoutValue::Boolean(match raw {
            None if attr == "w:val" => true,
            Some("true" | "1" | "on") => true,
            Some("false" | "0" | "off") => false,
            _ => return None,
        }),
        Tabs => LayoutValue::Tabs(tab_list(n)?),
        Group => LayoutValue::Boolean(true),
        Edge(edge) => {
            let mut children = n.elements().filter(|c| c.name == edge);
            let e = children.next()?;
            if children.next().is_some() || e.get("w:val") != Some("single") {
                return None;
            }
            let number = |key: &str| match e.get(key) {
                None => Some(None),
                Some(v) => v.parse::<i32>().ok().map(Some),
            };
            let b = LayoutBorder {
                size: number("w:sz")?,
                space: number("w:space")?,
                color: e.get("w:color").map(str::to_owned),
            };
            if !valid_border(&b) {
                return None;
            }
            LayoutValue::Border(b)
        }
    })
}
fn valid_border(b: &LayoutBorder) -> bool {
    b.size.is_none_or(|v| (2..=96).contains(&v))
        && b.space.is_none_or(|v| (0..=31).contains(&v))
        && b.color.as_ref().is_none_or(|v| {
            v == "auto" || (v.len() == 6 && v.bytes().all(|c| c.is_ascii_hexdigit()))
        })
}
const MARGINS: [(&str, i32); 7] = [
    ("w:top", 1440),
    ("w:bottom", 1440),
    ("w:left", 1800),
    ("w:right", 1800),
    ("w:header", 720),
    ("w:footer", 720),
    ("w:gutter", 0),
];
pub(super) fn inspect(props: Option<&Node>, section: bool) -> BTreeMap<LayoutField, LayoutValue> {
    let Some(props) = props.filter(|p| p.child(ALTERNATE).is_none()) else {
        return BTreeMap::new();
    };
    LayoutField::ALL
        .iter()
        .filter(|f| f.is_section() == section)
        .filter_map(|&f| {
            let mut matches = props.elements().filter(|n| n.name == f.spec().1);
            let n = matches.next()?;
            matches
                .next()
                .is_none()
                .then(|| value(f, n))?
                .map(|v| (f, v))
        })
        .collect()
}

fn apply(props: &mut Node, f: LayoutField, new: Option<&LayoutValue>) -> Result<bool> {
    let (_, child, attr, kind) = f.spec();
    if props.elements().any(|n| {
        matches!(
            n.name.as_str(),
            ALTERNATE | "w:sectPrChange" | "w:pPrChange"
        )
    }) || props.elements().filter(|n| n.name == child).count() > 1
    {
        return Err(unsupported("ambiguous or revised layout property"));
    }
    let current = props.child(child);
    let old = current.and_then(|n| value(f, n));
    if new.is_none() && current.is_none() {
        return Ok(false);
    }
    if matches!(kind, Heading) && new.is_none() && old.is_none() {
        return Ok(false);
    }
    if let Some(n) = current {
        if n.child(ALTERNATE).is_some() {
            return Err(unsupported("ambiguous layout alternatives"));
        }
        if child == "w:cols"
            && (n.elements().next().is_some()
                || n.get("w:equalWidth")
                    .is_some_and(|v| !matches!(v, "true" | "on" | "1")))
        {
            return Err(unsupported("individual or unequal column widths"));
        }
        let competing: &[&str] = match f {
            LayoutField::SpaceBefore => &["w:beforeLines", "w:beforeAutospacing"],
            LayoutField::SpaceAfter => &["w:afterLines", "w:afterAutospacing"],
            LayoutField::FirstLineIndent => &["w:firstLineChars", "w:hangingChars"],
            LayoutField::LeftIndent => &["w:leftChars", "w:start", "w:startChars"],
            LayoutField::RightIndent => &["w:rightChars", "w:end", "w:endChars"],
            _ => &[],
        };
        if competing.iter().any(|a| n.get(a).is_some()) {
            return Err(unsupported("competing layout units or settings"));
        }
        if child == "w:pgMar" && MARGINS.iter().any(|(k, _)| n.get(k).is_none()) {
            return Err(unsupported("incomplete margin record"));
        }
    }
    if let Some(new) = new
        && !matches!(kind, Group)
        && old.as_ref() == Some(new)
    {
        return Ok(false);
    }
    if matches!(kind, Group) {
        if new.is_some() {
            return Err(unsupported(
                "use group fields only to clear a complete layout group",
            ));
        }
        let attrs: &[&str] = if child == "w:pgMar" {
            &MARGINS.map(|(k, _)| k)
        } else {
            &["w:start", "w:countBy", "w:distance", "w:restart"]
        };
        if !simple(current.unwrap(), attrs) {
            return Err(unsupported("cannot clear opaque layout group"));
        }
        props.remove(child);
        return Ok(true);
    }
    if let Edge(edge) = kind {
        if let Some(n) = current {
            if n.elements().filter(|e| e.name == edge).count() > 1 {
                return Err(unsupported("duplicate page border edge"));
            }
            if let Some(e) = n.child(edge)
                && (!simple(e, &["w:val", "w:sz", "w:space", "w:color"])
                    || e.get("w:val") != Some("single"))
            {
                return Err(unsupported("unmodeled or opaque page border edge"));
            }
        }
        if new.is_none() && current.is_none_or(|n| n.child(edge).is_none()) {
            return Ok(false);
        }
        let borders = props.ensure(child);
        if let Some(v) = new {
            let LayoutValue::Border(b) = v else {
                return Err(unsupported("expected single page border"));
            };
            if !valid_border(b) {
                return Err(unsupported("invalid page border"));
            }
            let mut n = Node::new(edge).attr("w:val", "single");
            if let Some(sz) = b.size {
                n.set("w:sz", sz)
            }
            if let Some(space) = b.space {
                n.set("w:space", space)
            }
            if let Some(rgb) = &b.color {
                n.set("w:color", rgb)
            }
            borders.replace(n);
        } else {
            borders.remove(edge);
            if simple(borders, &[]) {
                props.remove(child)
            }
        }
        return Ok(true);
    }
    if matches!(kind, Tabs) {
        if let Some(n) = current
            && (n.attrs.iter().any(|(k, _)| k.declaration().is_none())
                || n.children.iter().any(|c| match c {
                    Content::Element(t) => {
                        t.name != "w:tab" || !simple(t, &["w:pos", "w:val", "w:leader"])
                    }
                    Content::Text(s) => !s.trim().is_empty(),
                    Content::Raw(_) => true,
                })
                || tab_list(n).is_none())
        {
            return Err(unsupported("unsafe tab list replacement"));
        }
        let mut tabs = Node::new(child);
        if let Some(v) = new {
            let LayoutValue::Tabs(stops) = v else {
                return Err(unsupported("expected tab stops"));
            };
            let mut seen = BTreeSet::new();
            if stops.is_empty() {
                return Err(unsupported("clear tab stops with null"));
            }
            for stop in stops {
                if !seen.insert(stop.position)
                    || !contains(TAB_ALIGN, &stop.alignment)
                    || stop
                        .leader
                        .as_deref()
                        .is_some_and(|l| !contains(TAB_LEADER, l))
                {
                    return Err(unsupported("invalid or duplicate tab stop"));
                }
                let mut t = Node::new("w:tab")
                    .attr("w:pos", stop.position)
                    .attr("w:val", &stop.alignment);
                if let Some(l) = &stop.leader {
                    t.set("w:leader", l)
                }
                tabs.push(t);
            }
            props.replace(tabs)
        } else {
            props.remove(child)
        }
        return Ok(true);
    }
    if new.is_none() && attr == "w:val" {
        if !simple(current.unwrap(), &[attr]) {
            return Err(unsupported("cannot clear opaque layout property"));
        }
        props.remove(child);
        return Ok(true);
    }
    if new.is_none() && child == "w:pgMar" {
        return Err(unsupported(
            "individual margins are required; set a margin instead",
        ));
    }
    if child == "w:pgMar" && current.is_none() {
        let mut n = Node::new(child);
        for (k, v) in MARGINS {
            n.set(k, v)
        }
        props.push(n)
    }
    let n = props.ensure(child);
    if f == LayoutField::FirstLineIndent && n.get(attr).is_some() && n.get("w:hanging").is_some() {
        return Err(unsupported("conflicting first-line and hanging indents"));
    }
    if let Some(v) = new {
        let mut key = attr;
        let lex = match (kind, v) {
            (Num(min, max), LayoutValue::Integer(i)) if (min..=max).contains(i) => {
                if f == LayoutField::FirstLineIndent {
                    n.attrs.retain(|(k, _)| k != "w:hanging" && k != attr);
                    if *i < 0 {
                        key = "w:hanging"
                    }
                    i.checked_abs()
                        .ok_or_else(|| unsupported("indent out of range"))?
                        .to_string()
                } else {
                    i.to_string()
                }
            }
            (Token(tokens), LayoutValue::Token(s)) if contains(tokens, s) => s.clone(),
            (Style, LayoutValue::Token(s)) => s.clone(),
            (Heading, LayoutValue::Integer(i)) if (1..=9).contains(i) => format!("Heading{i}"),
            (Bool, LayoutValue::Boolean(b)) => u8::from(*b).to_string(),
            _ => return Err(unsupported("invalid layout value")),
        };
        n.set(key, lex);
    } else {
        let before = n.attrs.len();
        n.attrs
            .retain(|(k, _)| k != attr && !(f == LayoutField::FirstLineIndent && k == "w:hanging"));
        if n.attrs.len() == before {
            return Ok(false);
        }
        if simple(n, &[]) {
            props.remove(child)
        }
    }
    Ok(true)
}

impl ImportedDocument {
    /// Preview direct layout fields against their original physical owners.
    pub fn preview_layout(&self, edits: &[LayoutEdit]) -> Result<EditPreview> {
        let mut package = self.package.clone();
        let mut root = package.word(&self.main)?;
        if root.elements().filter(|n| n.name == "w:body").count() != 1 {
            return Err(unsupported("ambiguous document body"));
        }
        let mut paragraphs = BTreeMap::new();
        let mut sections = BTreeMap::new();
        let mut style_ids = None;
        for edit in edits {
            let (map, idx, field, value) = match edit {
                LayoutEdit::Paragraph(a, f, v) if !f.is_section() => {
                    self.validate_snapshot(&a.snapshot)?;
                    (&mut paragraphs, a.body_index, *f, v)
                }
                LayoutEdit::Section(a, f, v) if f.is_section() => {
                    self.validate_snapshot(&a.snapshot)?;
                    (&mut sections, a.index, *f, v)
                }
                _ => return Err(unsupported("layout property does not belong to target")),
            };
            let fields = map.entry(idx).or_insert_with(BTreeMap::new);
            if fields.keys().any(|f: &LayoutField| {
                f.spec().1 == field.spec().1
                    && (matches!(f.spec().3, Group | Heading)
                        || matches!(field.spec().3, Group | Heading))
            }) || fields.insert(field, value.as_ref()).is_some()
            {
                return Err(unsupported("duplicate layout field"));
            }
            let style_id = match (field, value) {
                (LayoutField::StyleId, Some(LayoutValue::Token(id))) => Some(id.clone()),
                (LayoutField::Heading, Some(LayoutValue::Integer(i))) if (1..=9).contains(i) => {
                    Some(format!("Heading{i}"))
                }
                _ => None,
            };
            if let Some(id) = style_id {
                if style_ids.is_none() {
                    let styles = self
                        .styles()?
                        .ok_or_else(|| unsupported("styles part missing"))?;
                    let mut ids = BTreeMap::new();
                    for s in styles.styles {
                        if let Some(id) = s.id {
                            ids.entry(id)
                                .and_modify(|unique| *unique = false)
                                .or_insert(s.kind == super::StyleKind::Paragraph);
                        }
                    }
                    style_ids = Some(ids);
                }
                if style_ids.as_ref().and_then(|ids| ids.get(&id)) != Some(&true) {
                    return Err(unsupported("unknown or ambiguous paragraph style ID"));
                }
            }
        }
        let mut owned = BTreeMap::new();
        let mut terminal = None;
        if !sections.is_empty() {
            for (i, (slot, _, _)) in header::validated_section_slots(&root, &package, &self.main)?
                .into_iter()
                .enumerate()
            {
                if let Some(fields) = sections.remove(&i) {
                    match slot {
                        header::Slot::Paragraph(pos) => {
                            owned.insert(pos, fields);
                        }
                        header::Slot::Terminal => terminal = Some(fields),
                    }
                }
            }
        }
        let body = root.child_mut("w:body").ok_or_else(missing_body)?;
        let mut bounds = BoundaryState::default();
        let mut changed = false;
        let mut index = 0;
        let mut after_terminal = false;
        for (slot, c) in body.children.iter_mut().enumerate() {
            let Content::Element(n) = c else { continue };
            if after_terminal {
                return Err(unsupported("body section properties are not terminal"));
            }
            if n.name == "w:sectPr" {
                after_terminal = true;
                continue;
            }
            let before = bounds.safe();
            bounds.advance(n);
            let p = paragraphs.remove(&index);
            let s = owned.remove(&slot);
            index += 1;
            if p.is_none() && s.is_none() {
                continue;
            }
            if !before
                || !bounds.safe()
                || n.name != "w:p"
                || n.child(ALTERNATE).is_some()
                || n.elements().filter(|n| n.name == "w:pPr").count() > 1
                || n.child("w:pPr").is_some_and(|p| {
                    p.child(ALTERNATE).is_some() || p.child("w:pPrChange").is_some()
                })
            {
                return Err(unsupported("protected or ambiguous layout owner"));
            }
            super::revisions::guard_paragraph_mark(n)?;
            if let Some(fields) = p {
                if n.child("w:pPr").is_none() && fields.values().any(Option::is_some) {
                    n.children.insert(0, Content::Element(Node::new("w:pPr")))
                }
                if let Some(props) = n.child_mut("w:pPr") {
                    for (f, v) in fields {
                        changed |= apply(props, f, v)?
                    }
                }
            }
            if let Some(fields) = s {
                let owner = n
                    .child_mut("w:pPr")
                    .and_then(|p| p.child_mut("w:sectPr"))
                    .ok_or_else(|| unsupported("section owner missing"))?;
                for (f, v) in fields {
                    changed |= apply(owner, f, v)?
                }
            }
        }
        if let Some(fields) = terminal {
            if !bounds.safe() {
                return Err(unsupported("protected terminal section"));
            }
            if body.child("w:sectPr").is_some() || fields.values().any(Option::is_some) {
                let owner = body.ensure("w:sectPr");
                for (f, v) in fields {
                    changed |= apply(owner, f, v)?
                }
            }
        }
        if !sections.is_empty() || !paragraphs.is_empty() || !owned.is_empty() {
            return Err(unsupported("layout target not found"));
        }
        if changed {
            package.put_word(&self.main, root)?
        }
        Ok(EditPreview {
            output: PackagePlan::new(package)?,
            block_changes: vec![],
            inline_changes: vec![],
            changes: vec![],
            insertions: vec![],
            paragraph_insertions: vec![],
            header_changes: vec![],
            run_text_changes: vec![],
            run_removals: vec![],
            cell_alignment_changes: vec![],
        })
    }
}
