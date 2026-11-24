use super::{ImportedDocument, insertion::BoundaryState, unsupported};
use crate::Result;
use crate::xml::{Content, Node};
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
    path: Vec<usize>,
    safe_site: bool,
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

pub(super) fn inspect(body: &Node, snapshot: &Arc<()>) -> Vec<ImportedControl> {
    struct Walk<'a> {
        snapshot: &'a Arc<()>,
        items: Vec<ImportedControl>,
        path: Vec<usize>,
        boundaries: BoundaryState,
    }
    impl Walk<'_> {
        fn visit(&mut self, n: &Node, parent: &str, safe: bool) {
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
        path: Vec::new(),
        boundaries: Default::default(),
    };
    walk.children(body, "w:body", true);
    walk.items
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
fn text_path(control: &Node, ctx: ControlContext) -> Result<Vec<&'static str>> {
    let fail = || unsupported("unsupported content control structure");
    let mut path = vec!["w:sdtContent"];
    match ctx {
        ControlContext::Row => path.extend(["w:tr", "w:tc", "w:p"]),
        ControlContext::Cell => path.extend(["w:tc", "w:p"]),
        ControlContext::Block => path.push("w:p"),
        ControlContext::Inline => (),
    }
    path.extend(["w:r", "w:t"]);
    let mut n = control;
    for &next in &path {
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
    Ok(path)
}

impl ImportedDocument {
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
