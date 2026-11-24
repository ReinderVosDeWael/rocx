use super::{ImportedDocument, insertion::BoundaryState, unsupported};
use crate::xml::{Content, Node};
use crate::{ContentControlListItem, Result};
use std::{collections::BTreeSet, sync::Arc};

/// Physical identity within one opened document, independent of source IDs and tags.
#[derive(Debug, Clone)]
pub struct ControlAnchor {
    pub(super) snapshot: Arc<()>,
    index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlContext {
    Block,
    Inline,
    Row,
    Cell,
}

impl ControlContext {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Inline => "inline",
            Self::Row => "row",
            Self::Cell => "cell",
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedControl {
    pub anchor: ControlAnchor,
    pub context: ControlContext,
    /// Literal `w:id`; neither this value nor `tag` is an address.
    pub id: Option<String>,
    pub tag: Option<String>,
    pub alias: Option<String>,
    pub lock: Option<String>,
    pub bound: bool,
    /// OOXML local type name; absent type means rich text, unknown types are opaque.
    pub kind: &'static str,
    pub choices: Vec<ContentControlListItem>,
    pub selected: Option<String>,
    /// Literal `w:text/@w:multiLine`, absent if not explicitly present.
    pub multi_line: Option<String>,
    path: Vec<usize>,
    safe_site: bool,
}

/// Eligible physical wrapper insertion site. `text` is a label, not an address.
#[derive(Clone, Debug)]
pub struct ControlSite {
    snapshot: Arc<()>,
    pub context: ControlContext,
    pub text: String,
    path: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct ControlEdit {
    pub target: ControlAnchor,
    pub tag: Option<String>,
    pub alias: Option<String>,
    pub text: Option<String>,
}

fn contains(n: &Node, test: fn(&Node) -> bool) -> bool {
    test(n) || n.elements().any(|n| contains(n, test))
}

fn binding(n: &Node) -> bool {
    n.name.local() == "dataBinding"
}

fn at<'a>(mut n: &'a mut Node, path: &[usize]) -> Result<&'a mut Node> {
    for &i in path {
        let Some(Content::Element(c)) = n.children.get_mut(i) else {
            return Err(unsupported("control site not found"));
        };
        n = c;
    }
    Ok(n)
}

fn structural_guard(c: &ImportedControl) -> Result<()> {
    if c.bound {
        return Err(unsupported("custom XML-bound control"));
    }
    if !c.safe_site {
        return Err(unsupported("nested or protected control site"));
    }
    Ok(())
}

fn single_line(s: &str) -> Result<()> {
    if crate::document::invalid_xml_character(s).is_some() || s.contains(['\t', '\r', '\n']) {
        Err(unsupported("expected single-line XML text"))
    } else {
        Ok(())
    }
}

fn leaf_text(leaf: &Node) -> String {
    leaf.children
        .iter()
        .filter_map(|c| match c {
            Content::Text(s) => Some(s.as_str()),
            _ => None,
        })
        .collect()
}

fn validate_choices(items: &[ContentControlListItem]) -> Result<()> {
    let mut values = BTreeSet::new();
    for i in items {
        single_line(&i.display_text)?;
        single_line(&i.value)?;
        if i.display_text.is_empty() || i.value.is_empty() || !values.insert(&i.value) {
            return Err(unsupported(
                "control choices require unique values and nonempty labels",
            ));
        }
    }
    Ok(())
}

fn describe(n: &Node) -> (&'static str, Vec<ContentControlListItem>, Option<String>) {
    let mut kind = None;
    let mut choices = Vec::new();
    let mut selected = None;
    if let Some(props) = n.child("w:sdtPr") {
        for p in props.elements() {
            let ty = match p.name.as_str() {
                "w:tag" | "w:alias" | "w:id" | "w:lock" | "w:rPr" | "w:dataBinding" | "w:label"
                | "w:tabIndex" | "w:placeholder" | "w:showingPlcHdr" | "w:temporary" => continue,
                "w:text" => "text",
                "w:richText" => "richText",
                "w:dropDownList" | "w:comboBox" => {
                    selected = p.get("w:lastValue").map(str::to_owned);
                    for item in p.elements().filter(|v| v.name == "w:listItem") {
                        if let (Some(label), Some(value)) =
                            (item.get("w:displayText"), item.get("w:value"))
                        {
                            choices.push(ContentControlListItem {
                                display_text: label.into(),
                                value: value.into(),
                            });
                        }
                    }
                    if p.name == "w:comboBox" {
                        "comboBox"
                    } else {
                        "dropDownList"
                    }
                }
                "w:date" => "date",
                "w:picture" => "picture",
                _ => "opaque",
            };
            if kind.replace(ty).is_some() {
                kind = Some("opaque");
            }
        }
    }
    (kind.unwrap_or("richText"), choices, selected)
}

// A destructive wrapper/list edit must account for everything it might drop,
// including comments, extension attributes, and local namespace declarations.
fn strict_props(n: &Node) -> Result<()> {
    let fail = || unsupported("opaque or ambiguous control properties");
    let props = n.child("w:sdtPr").ok_or_else(fail)?;
    // The imported tree carries each element's full namespace scope; the
    // promoted payload keeps that scope, including prefix-valued attributes.
    let only = |n: &Node, attrs: &[&str]| {
        n.attrs
            .iter()
            .all(|(a, _)| a.declaration().is_some() || attrs.contains(&a.as_str()))
    };
    let clean = |n: &Node| {
        n.children.iter().all(|c| match c {
            Content::Element(_) => true,
            Content::Text(s) => s.trim().is_empty(),
            _ => false,
        })
    };
    if !only(n, &[])
        || !only(props, &[])
        || !clean(props)
        || !only(n.child("w:sdtContent").ok_or_else(fail)?, &[])
    {
        return Err(fail());
    }
    let mut seen = BTreeSet::new();
    let mut typed = false;
    for p in props.elements() {
        if !seen.insert(p.name.as_str()) {
            return Err(fail());
        }
        match p.name.as_str() {
            "w:lock" | "w:tag" | "w:alias" | "w:id"
                if p.children.is_empty() && only(p, &["w:val"]) && p.get("w:val").is_some() => {}
            "w:text" | "w:richText"
                if !typed
                    && p.children.is_empty()
                    && only(
                        p,
                        if p.name == "w:text" {
                            &["w:multiLine"]
                        } else {
                            &[]
                        },
                    ) =>
            {
                typed = true
            }
            "w:dropDownList" if !typed && only(p, &["w:lastValue"]) && clean(p) => {
                typed = true;
                for i in p.elements() {
                    if i.name != "w:listItem"
                        || !i.children.is_empty()
                        || !only(i, &["w:displayText", "w:value"])
                        || i.get("w:displayText").is_none()
                        || i.get("w:value").is_none()
                    {
                        return Err(fail());
                    }
                }
                validate_choices(&describe(n).1)?;
            }
            _ => return Err(fail()),
        }
    }
    Ok(())
}

pub(super) fn inspect(body: &Node, snapshot: &Arc<()>) -> (Vec<ImportedControl>, Vec<ControlSite>) {
    struct Walk<'a> {
        snapshot: &'a Arc<()>,
        items: Vec<ImportedControl>,
        sites: Vec<ControlSite>,
        path: Vec<usize>,
        boundaries: BoundaryState,
    }
    impl Walk<'_> {
        fn visit(&mut self, n: &Node, parent: &str, safe: bool) {
            let ctx = match (parent, n.name.as_str()) {
                ("w:body" | "w:tc", "w:p") => Some(ControlContext::Block),
                ("w:p", "w:r") => Some(ControlContext::Inline),
                ("w:tbl", "w:tr") => Some(ControlContext::Row),
                ("w:tr", "w:tc") => Some(ControlContext::Cell),
                _ => None,
            };
            if safe
                && self.boundaries.safe()
                && let Some(context) = ctx
                && let Ok(leaf) = follow(n, &content_path(context)[1..])
            {
                self.sites.push(ControlSite {
                    snapshot: self.snapshot.clone(),
                    context,
                    text: leaf_text(leaf),
                    path: self.path.clone(),
                });
            }
            let child_parent = match (parent, n.name.as_str()) {
                ("w:body" | "w:tc", "w:p" | "w:tbl") | ("w:tbl", "w:tr") | ("w:tr", "w:tc") => {
                    n.name.as_str()
                }
                (_, "w:sdt") => {
                    let context = match parent {
                        "w:p" => ControlContext::Inline,
                        "w:body" | "w:tc" => ControlContext::Block,
                        "w:tbl" => ControlContext::Row,
                        "w:tr" => ControlContext::Cell,
                        _ => return,
                    };
                    let p = super::read_content_control(n);
                    let (kind, choices, selected) = describe(n);
                    self.items.push(ImportedControl {
                        anchor: ControlAnchor {
                            snapshot: self.snapshot.clone(),
                            index: self.items.len(),
                        },
                        id: n
                            .child("w:sdtPr")
                            .and_then(|p| p.child("w:id"))
                            .and_then(|n| n.get("w:val"))
                            .map(str::to_owned),
                        tag: p.tag,
                        alias: p.alias,
                        lock: p.lock,
                        bound: n
                            .elements()
                            .filter(|n| n.name == "w:sdtPr")
                            .any(|n| contains(n, binding)),
                        path: self.path.clone(),
                        context,
                        kind,
                        multi_line: n
                            .child("w:sdtPr")
                            .and_then(|p| p.child("w:text"))
                            .and_then(|t| t.get("w:multiLine"))
                            .map(str::to_owned),
                        choices,
                        selected,
                        safe_site: safe && self.boundaries.safe(),
                    });
                    for (i, child) in n.children.iter().enumerate() {
                        let Content::Element(c) = child else {
                            continue;
                        };
                        if c.name == "w:sdtContent" {
                            self.path.push(i);
                            self.children(c, parent, false);
                            self.path.pop();
                        }
                    }
                    return;
                }
                _ => {
                    self.boundaries.advance(n);
                    return;
                }
            };
            let safe = safe
                && !n
                    .elements()
                    .filter(|c| {
                        matches!(c.name.as_str(), "w:trPr" | "w:tcPr" | "w:pPr" | "w:tblPr")
                    })
                    .any(|p| contains(p, unsafe_property));
            self.children(n, child_parent, safe);
        }
        fn children(&mut self, n: &Node, parent: &str, safe: bool) {
            for (i, child) in n.children.iter().enumerate() {
                let Content::Element(c) = child else {
                    continue;
                };
                self.path.push(i);
                self.visit(c, parent, safe);
                self.path.pop();
            }
        }
    }
    let mut walk = Walk {
        snapshot,
        items: Vec::new(),
        sites: Vec::new(),
        path: Vec::new(),
        boundaries: Default::default(),
    };
    walk.children(body, "w:body", true);
    (walk.items, walk.sites)
}

fn unsafe_property(n: &Node) -> bool {
    n.name.local().ends_with("Change")
        || matches!(
            n.name.local(),
            "ins"
                | "del"
                | "cellIns"
                | "cellDel"
                | "cellMerge"
                | "hMerge"
                | "vMerge"
                | "sectPr"
                | "AlternateContent"
                | "checkbox"
                | "repeatingSection"
                | "repeatingSectionItem"
        )
}

// Require one ordinary text leaf. Never flatten a multi-run control, discard
// table structure, or guess the meaning of a specialized control type.
fn content_path(ctx: ControlContext) -> Vec<&'static str> {
    let mut path = Vec::new();
    match ctx {
        ControlContext::Row => path.extend(["w:tr", "w:tc", "w:p"]),
        ControlContext::Cell => path.extend(["w:tc", "w:p"]),
        ControlContext::Block => path.push("w:p"),
        ControlContext::Inline => (),
    }
    path.extend(["w:r", "w:t"]);
    path
}

fn text_path(control: &Node, ctx: ControlContext) -> Result<Vec<&'static str>> {
    let mut path = vec!["w:sdtContent"];
    path.extend(content_path(ctx));
    follow(control, &path)?;
    Ok(path)
}

fn follow<'a>(mut n: &'a Node, path: &[&str]) -> Result<&'a Node> {
    let fail = || unsupported("unsupported content control structure");
    for &next in path {
        let property = match n.name.as_str() {
            "w:sdt" => Some("w:sdtPr"),
            "w:tr" => Some("w:trPr"),
            "w:tc" => Some("w:tcPr"),
            "w:p" => Some("w:pPr"),
            "w:r" => Some("w:rPr"),
            _ => None,
        };
        let mut seen_property = false;
        let mut child = None;
        for c in &n.children {
            match c {
                Content::Element(c)
                    if Some(c.name.as_str()) == property && !seen_property && child.is_none() =>
                {
                    if contains(c, unsafe_property) {
                        return Err(fail());
                    }
                    seen_property = true;
                }
                Content::Element(c) if c.name == next && child.is_none() => child = Some(c),
                Content::Text(s) if s.trim().is_empty() => (),
                _ => return Err(fail()),
            }
        }
        n = child.ok_or_else(fail)?;
    }
    if n.children.iter().any(|c| !matches!(c, Content::Text(_))) {
        return Err(fail());
    }
    Ok(n)
}

impl ImportedDocument {
    pub fn control_sites(&self) -> &[ControlSite] {
        &self.control_sites
    }

    /// Wrap a single existing payload, retaining its XML and formatting.
    pub fn preview_wrap_control(
        &self,
        site: &ControlSite,
        tag: &str,
    ) -> Result<super::EditPreview> {
        self.validate_snapshot(&site.snapshot)?;
        single_line(tag)?;
        self.stage_body(|body| {
            let n = at(body, &site.path)?;
            let wrapper = Node::new("w:sdt").with(
                Node::new("w:sdtPr")
                    .with(Node::new("w:tag").attr("w:val", tag))
                    .with(Node::new("w:richText")),
            );
            let payload = std::mem::replace(n, wrapper);
            n.push(Node::new("w:sdtContent").with(payload));
            Ok(true)
        })
    }

    /// Remove just an understood wrapper; promote its original payload.
    pub fn preview_unwrap_control(&self, anchor: &ControlAnchor) -> Result<super::EditPreview> {
        self.validate_snapshot(&anchor.snapshot)?;
        let c = &self.controls[anchor.index];
        structural_guard(c)?;
        if !matches!(c.lock.as_deref(), None | Some("unlocked" | "contentLocked")) {
            return Err(unsupported("control is locked"));
        }
        self.stage_body(|body| {
            let n = at(body, &c.path)?;
            text_path(n, c.context)?;
            strict_props(n)?;
            let content = n.child_mut("w:sdtContent").unwrap();
            let slot = content
                .children
                .iter()
                .position(|c| matches!(c, Content::Element(_)))
                .unwrap();
            let Content::Element(payload) = content.children.remove(slot) else {
                unreachable!()
            };
            *n = payload;
            Ok(true)
        })
    }

    /// Replace the entire property definition, including clearing omitted
    /// metadata. The selected choice must describe the existing cached text.
    pub fn preview_replace_control(
        &self,
        anchor: &ControlAnchor,
        properties: &crate::ContentControlProperties,
    ) -> Result<super::EditPreview> {
        use crate::ContentControlText;
        self.validate_snapshot(&anchor.snapshot)?;
        let c = &self.controls[anchor.index];
        structural_guard(c)?;
        if !matches!(c.lock.as_deref(), None | Some("unlocked")) {
            return Err(unsupported("control is locked"));
        }
        if matches!(c.context, ControlContext::Row | ControlContext::Cell)
            && properties.text != ContentControlText::Rich
        {
            return Err(unsupported("row and cell controls require rich text"));
        }
        properties.validate()?;
        for s in [&properties.tag, &properties.alias].into_iter().flatten() {
            single_line(s)?;
        }
        if let Some(id) = properties.id {
            let mut count = 0usize;
            self.package.visit_source_xml(|n| {
                if n.name == "w:sdtPr" {
                    count += n
                        .elements()
                        .filter(|v| {
                            v.name == "w:id"
                                && v.get("w:val").and_then(|s| s.trim().parse::<u32>().ok())
                                    == Some(id)
                        })
                        .count();
                }
            })?;
            if count > usize::from(c.id.as_ref().and_then(|s| s.trim().parse().ok()) == Some(id)) {
                return Err(unsupported("duplicate content control ID"));
            }
        }
        self.stage_body(|body| {
            let n = at(body, &c.path)?;
            let path = text_path(n, c.context)?;
            strict_props(n)?;
            if matches!(properties.text, ContentControlText::Plain { .. }) {
                single_line(&leaf_text(follow(n, &path)?))?;
            }
            if let ContentControlText::DropDown { items, selected } = &properties.text {
                validate_choices(items)?;
                let leaf = follow(n, &path)?;
                let cached = leaf_text(leaf);
                if !items
                    .iter()
                    .any(|i| i.value == *selected && i.display_text == cached)
                {
                    return Err(unsupported("selected choice must match cached text"));
                }
            }
            *n.child_mut("w:sdtPr").unwrap() =
                crate::ooxml::wordprocessing::control_props(properties)?;
            Ok(true)
        })
    }

    /// Select by literal value, optionally replacing the ordered list at the same time.
    pub fn preview_control_choice(
        &self,
        anchor: &ControlAnchor,
        selected: &str,
        items: Option<&[ContentControlListItem]>,
    ) -> Result<super::EditPreview> {
        self.validate_snapshot(&anchor.snapshot)?;
        let c = &self.controls[anchor.index];
        structural_guard(c)?;
        if !matches!(c.lock.as_deref(), None | Some("unlocked"))
            && !(c.lock.as_deref() == Some("sdtLocked") && items.is_none())
        {
            return Err(unsupported("control is locked"));
        }
        if c.kind != "dropDownList"
            || !matches!(c.context, ControlContext::Block | ControlContext::Inline)
        {
            return Err(unsupported("expected block or inline drop-down control"));
        }
        self.stage_body(|body| {
            let n = at(body, &c.path)?;
            let path = text_path(n, c.context)?;
            strict_props(n)?;
            let choices = items.unwrap_or(&c.choices);
            validate_choices(choices)?;
            let display = &choices
                .iter()
                .find(|i| i.value == selected)
                .ok_or_else(|| unsupported("unknown control choice value"))?
                .display_text;
            let list = n
                .child_mut("w:sdtPr")
                .unwrap()
                .child_mut("w:dropDownList")
                .unwrap();
            let mut changed = list.get("w:lastValue") != Some(selected);
            list.set("w:lastValue", selected);
            if items.is_some() && choices != c.choices {
                list.children.clear();
                for item in choices {
                    list.push(
                        Node::new("w:listItem")
                            .attr("w:displayText", &item.display_text)
                            .attr("w:value", &item.value),
                    );
                }
                changed = true;
            }
            let mut leaf = n;
            for name in path {
                leaf = leaf.child_mut(name).unwrap();
            }
            if leaf_text(leaf) != *display {
                leaf.children = vec![Content::Text(display.clone())];
                if display.starts_with(' ') || display.ends_with(' ') {
                    leaf.set("xml:space", "preserve");
                }
                changed = true;
            }
            Ok(changed)
        })
    }

    /// Main-body controls in physical document order, including controls in tables.
    pub fn controls(&self) -> &[ImportedControl] {
        &self.controls
    }

    pub(super) fn apply_controls(&self, body: &mut Node, edits: &[ControlEdit]) -> Result<bool> {
        let mut seen = BTreeSet::new();
        let mut changed = false;
        for edit in edits {
            let control = &self.controls[edit.target.index];
            if !seen.insert(edit.target.index) {
                return Err(unsupported("duplicate control target"));
            }
            if control.bound {
                return Err(unsupported("custom XML-bound control"));
            }
            match control.lock.as_deref() {
                None | Some("unlocked") => (),
                Some("sdtLocked") if edit.tag.is_none() && edit.alias.is_none() => (),
                Some("contentLocked") if edit.text.is_none() => (),
                _ => return Err(unsupported("control is locked")),
            }
            if !control.safe_site {
                return Err(unsupported(
                    "control is nested or inside a protected range or structure",
                ));
            }
            if edit.tag.is_none() && edit.alias.is_none() && edit.text.is_none() {
                return Err(unsupported("empty control edit"));
            }
            let mut n = &mut *body;
            for &i in &control.path {
                let Some(Content::Element(child)) = n.children.get_mut(i) else {
                    return Err(unsupported("control target not found"));
                };
                n = child;
            }
            let path = text_path(n, control.context)?;
            let props = n
                .child("w:sdtPr")
                .ok_or_else(|| unsupported("missing control properties"))?;
            let mut keys = BTreeSet::new();
            let mut typed = false;
            for p in props.elements() {
                if !keys.insert(p.name.as_str()) {
                    return Err(unsupported("ambiguous control properties"));
                }
                match p.name.as_str() {
                    "w:alias" | "w:tag" | "w:id" | "w:lock"
                        if p.get("w:val").is_some() && p.elements().next().is_none() => {}
                    "w:text" | "w:richText" if !typed => typed = true,
                    "w:rPr" | "w:label" | "w:tabIndex" => (),
                    // Extensions are retained; their semantics are unknown, so
                    // do not change the displayed value of such a control.
                    _ if p.name.namespace() != Some(crate::xml::W) && edit.text.is_none() => (),
                    _ => return Err(unsupported("unsupported content control properties")),
                }
            }
            for value in [&edit.tag, &edit.alias, &edit.text].into_iter().flatten() {
                if crate::document::invalid_xml_character(value).is_some()
                    || value.contains(['\t', '\r', '\n'])
                {
                    return Err(unsupported("control edit requires single-line XML text"));
                }
            }
            let props = n.child_mut("w:sdtPr").unwrap();
            for (name, value) in [("w:alias", &edit.alias), ("w:tag", &edit.tag)] {
                if let Some(value) = value
                    && props.child(name).and_then(|p| p.get("w:val")) != Some(value)
                {
                    if let Some(property) = props.child_mut(name) {
                        property.set("w:val", value);
                    } else {
                        // CT_SdtPr sequence starts rPr, alias, tag, id, lock.
                        let slot = props.children.iter().position(|child| matches!(child,
                            Content::Element(p) if p.name.namespace() == Some(crate::xml::W)
                                && p.name != "w:rPr" && !(name == "w:tag" && p.name == "w:alias")
                        )).unwrap_or(props.children.len());
                        props
                            .children
                            .insert(slot, Content::Element(Node::new(name).attr("w:val", value)));
                    }
                    changed = true;
                }
            }
            if let Some(text) = &edit.text {
                for next in path {
                    n = n.child_mut(next).unwrap();
                }
                let before: String = n
                    .children
                    .iter()
                    .filter_map(|c| {
                        if let Content::Text(s) = c {
                            Some(s.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();
                if &before != text {
                    n.children = vec![Content::Text(text.clone())];
                    if text.starts_with(' ') || text.ends_with(' ') {
                        n.set("xml:space", "preserve");
                    }
                    changed = true;
                }
            }
        }
        Ok(changed)
    }
}
