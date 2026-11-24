//! Move existing XML nodes without re-encoding their drawings or relationships.
use super::*;
use crate::editor::{EditBatch, EditPreview};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub enum WrapperEdit {
    /// An inclusive range of siblings; `last: None` selects only `first`.
    Wrap {
        first: ControlSite,
        last: Option<ControlSite>,
        tag: String,
    },
    Unwrap(ControlAnchor),
    Define(ControlAnchor, crate::ContentControlProperties),
    Choice {
        target: ControlAnchor,
        selected: String,
        items: Option<Vec<ContentControlListItem>>,
    },
    ReplaceInlines(ControlAnchor, Vec<crate::ResolvedInline>),
    ReplaceBlocks(ControlAnchor, Vec<crate::ResolvedBlock>),
}

fn protected(n: &Node) -> bool {
    unsafe_property(n)
        || binding(n)
        || n.name.local().ends_with("RangeStart")
        || n.name.local().ends_with("RangeEnd")
        || matches!(
            n.name.as_str(),
            "w:bookmarkStart"
                | "w:bookmarkEnd"
                | "w:permStart"
                | "w:permEnd"
                | "w:proofErr"
                | "w:fldSimple"
                | "w:fldChar"
                | "w:instrText"
                | "w:sdt"
                | "w:customXml"
                | "w:txbxContent"
                | "w:moveFrom"
                | "w:moveTo"
        )
}

fn rich_node(n: &Node, label: &mut String) -> bool {
    let allowed: &[&str] = match n.name.as_str() {
        "w:p" => &["w:pPr", "w:r", "w:hyperlink"],
        "w:hyperlink" => &["w:r"],
        "w:r" => &["w:rPr", "w:t", "w:tab", "w:br", "w:drawing"],
        "w:t" => {
            if n.children.iter().any(|v| !matches!(v, Content::Text(_))) {
                return false;
            }
            label.push_str(&leaf_text(n));
            return true;
        }
        "w:tab" | "w:br" => {
            label.push(if n.name == "w:tab" { '\t' } else { '\n' });
            return n.children.is_empty();
        }
        "w:tbl" => &["w:tblPr", "w:tblGrid", "w:tr"],
        "w:tr" => &["w:tblPrEx", "w:trPr", "w:tc"],
        "w:tc" => &["w:tcPr", "w:p", "w:tbl"],
        // These subtrees move intact, within their original relationship owner.
        "w:pPr" | "w:rPr" | "w:tblPr" | "w:tblPrEx" | "w:tblGrid" | "w:trPr" | "w:tcPr"
        | "w:drawing" => return !contains(n, protected),
        _ => return false,
    };
    n.children.iter().all(|v| match v {
        Content::Element(c) => allowed.contains(&c.name.as_str()) && rich_node(c, label),
        Content::Text(s) => s.trim().is_empty(),
        _ => false,
    })
}

pub(super) fn rich_label(n: &Node, ctx: ControlContext) -> Option<String> {
    if !match ctx {
        ControlContext::Block => matches!(n.name.as_str(), "w:p" | "w:tbl"),
        ControlContext::Inline => n.name == "w:r",
        ControlContext::Row => n.name == "w:tr",
        ControlContext::Cell => n.name == "w:tc",
    } {
        return None;
    }
    let mut label = String::new();
    rich_node(n, &mut label).then_some(label)
}

pub(super) fn validate_payload(n: &Node, ctx: ControlContext) -> Result<()> {
    let fail = || unsupported("unsupported content control structure");
    let mut elements = n.elements();
    if !matches!((elements.next(),elements.next(),elements.next()),(Some(p),Some(c),None) if p.name=="w:sdtPr" && c.name=="w:sdtContent")
        || n.children.iter().any(|v| {
            !matches!(v, Content::Element(_) | Content::Text(_))
                || matches!(v,Content::Text(s) if !s.trim().is_empty())
        })
    {
        return Err(fail());
    }
    let content = n.child("w:sdtContent").unwrap();
    if content.elements().next().is_none()
        || !content.children.iter().all(|v| match v {
            Content::Element(c) => rich_label(c, ctx).is_some(),
            Content::Text(s) => s.trim().is_empty(),
            _ => false,
        })
    {
        return Err(fail());
    }
    Ok(())
}

#[derive(Default)]
pub(crate) struct WrapperPlan {
    pub(crate) images: Option<crate::ooxml::HeaderImageResources>,
    /// Body splices are merged with block insertions/removals by original slot.
    pub(crate) splices: BTreeMap<usize, Vec<Content>>,
    pub(crate) ids: BTreeSet<u32>,
    pub(crate) changed: bool,
}

impl ImportedDocument {
    pub(super) fn preview_wrapper(&self, edit: WrapperEdit) -> Result<EditPreview> {
        self.preview_edits(EditBatch {
            wrappers: &[edit],
            ..Default::default()
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn stage_wrappers(
        &self,
        mut package: crate::opc::Package,
        body: &mut Node,
        edits: &[WrapperEdit],
        controls: &[ControlEdit],
        occupied: &mut BTreeSet<usize>,
        blocks: &[crate::editor::BlockEdit],
    ) -> Result<(crate::opc::Package, WrapperPlan)> {
        let mut plan = WrapperPlan::default();
        if edits.is_empty() {
            return Ok((package, plan));
        }
        let slot_to_index: BTreeMap<_, _> = body
            .children
            .iter()
            .enumerate()
            .filter(|(_, v)| matches!(v,Content::Element(n) if n.name!="w:sectPr"))
            .enumerate()
            .map(|(i, (slot, _))| (slot, i))
            .collect();
        let mut busy = occupied.clone();
        for edit in controls {
            self.validate_control(&edit.target)?;
            busy.insert(slot_to_index[&self.controls[edit.target.index].path[0]]);
        }
        for b in blocks {
            if let crate::editor::BlockEdit::KeepWithNext(a, _)
            | crate::editor::BlockEdit::ParagraphFlag(a, ..) = b
            {
                busy.insert(a.body_index);
            }
        }
        for edit in edits {
            let (path, end, ctx) = match edit {
                WrapperEdit::Wrap { first, last, tag } => {
                    self.validate_snapshot(&first.snapshot)?;
                    let last = last.as_ref().unwrap_or(first);
                    self.validate_snapshot(&last.snapshot)?;
                    single_line(tag)?;
                    let (start, parent) = first.path.split_last().unwrap();
                    let (end, other) = last.path.split_last().unwrap();
                    if parent != other || end < start {
                        return Err(unsupported("wrapper range needs ordered sibling sites"));
                    }
                    let ctx = match at(body, parent)?.name.as_str() {
                        "w:p" => ControlContext::Inline,
                        "w:body" | "w:tc" => ControlContext::Block,
                        "w:tbl" => ControlContext::Row,
                        "w:tr" => ControlContext::Cell,
                        _ => return Err(unsupported("invalid wrapper parent")),
                    };
                    (&first.path, *end, ctx)
                }
                WrapperEdit::Unwrap(a)
                | WrapperEdit::Define(a, _)
                | WrapperEdit::Choice { target: a, .. }
                | WrapperEdit::ReplaceInlines(a, _)
                | WrapperEdit::ReplaceBlocks(a, _) => {
                    self.validate_control(a)?;
                    let c = &self.controls[a.index];
                    structural_guard(c)?;
                    (&c.path, *c.path.last().unwrap(), c.context)
                }
            };
            let (start, parent) = path.split_last().unwrap();
            let stop = if parent.is_empty() { end } else { path[0] };
            for (_, index) in slot_to_index.range(path[0]..=stop) {
                if !busy.insert(*index) {
                    return Err(unsupported("conflicting edits share a control body block"));
                }
                occupied.insert(*index);
            }
            let container = at(body, parent)?;
            let splice = match edit {
                WrapperEdit::Wrap { tag, .. } => {
                    let children = &container.children[*start..=end];
                    if children.iter().any(|v| match v {
                        Content::Element(n) => rich_label(n, ctx).is_none(),
                        Content::Text(s) => !s.trim().is_empty(),
                        _ => true,
                    }) {
                        return Err(unsupported("wrapper range crosses unsupported content"));
                    }
                    let mut cached = Node::new("w:sdtContent");
                    cached.children = children.to_vec();
                    let props = Node::new("w:sdtPr")
                        .with(Node::new("w:tag").attr("w:val", tag))
                        .with(Node::new("w:richText"));
                    vec![Content::Element(
                        Node::new("w:sdt").with(props).with(cached),
                    )]
                }
                WrapperEdit::Unwrap(a) => {
                    let c = &self.controls[a.index];
                    if !matches!(c.lock.as_deref(), None | Some("unlocked" | "contentLocked")) {
                        return Err(unsupported("control is locked"));
                    }
                    let n = at(container, &[*start])?;
                    validate_payload(n, ctx)?;
                    strict_props(n)?;
                    n.child("w:sdtContent").unwrap().children.clone()
                }
                WrapperEdit::Define(a, props) => {
                    self.define(at(container, &[*start])?, &self.controls[a.index], props)?;
                    if let Some(id) = props.id {
                        plan.ids.insert(id);
                    }
                    plan.changed = true;
                    continue;
                }
                WrapperEdit::Choice {
                    target,
                    selected,
                    items,
                } => {
                    plan.changed |= Self::choose(
                        at(container, &[*start])?,
                        &self.controls[target.index],
                        selected,
                        items.as_deref(),
                    )?;
                    continue;
                }
                WrapperEdit::ReplaceBlocks(a, _) | WrapperEdit::ReplaceInlines(a, _) => {
                    let c = &self.controls[a.index];
                    if !matches!(c.lock.as_deref(), None | Some("unlocked" | "sdtLocked")) {
                        return Err(unsupported("control content is locked"));
                    }
                    if c.kind != "richText" {
                        return Err(unsupported("rich-text control required"));
                    }
                    let n = at(container, &[*start])?;
                    validate_payload(n, ctx)?;
                    strict_props(n)?;
                    let cached = n.child_mut("w:sdtContent").unwrap();
                    if cached.attrs.iter().any(|(a, _)| a.declaration().is_none())
                        || cached.elements().any(|e| {
                            !crate::editor::blocks::local(
                                e,
                                if ctx == ControlContext::Inline {
                                    "w:p"
                                } else {
                                    "body"
                                },
                            )
                        })
                    {
                        return Err(unsupported(
                            "control payload contains references or opaque markup",
                        ));
                    }
                    let nodes = match edit {
                        WrapperEdit::ReplaceInlines(_, runs)
                            if ctx == ControlContext::Inline && !runs.is_empty() =>
                        {
                            crate::ResolvedInlineContentControl::new(
                                runs.clone(),
                                Default::default(),
                            )?;
                            let (staged, groups) = crate::ooxml::encode_inlines(
                                package,
                                &self.main,
                                &runs.iter().collect::<Vec<_>>(),
                                Some(&mut plan.images),
                            )?;
                            package = staged;
                            groups.into_iter().flatten().collect()
                        }
                        WrapperEdit::ReplaceBlocks(_, blocks)
                            if ctx == ControlContext::Block && !blocks.is_empty() =>
                        {
                            let (staged, nodes) = crate::ooxml::encode_blocks(
                                package,
                                &self.main,
                                blocks,
                                None,
                                "editor.control",
                                Some(&mut plan.images),
                            )?;
                            package = staged;
                            nodes
                        }
                        _ => {
                            return Err(unsupported(
                                "expected nonempty payload in matching wrapper context",
                            ));
                        }
                    };
                    cached.children = nodes.into_iter().map(Content::Element).collect();
                    plan.changed = true;
                    continue;
                }
            };
            if parent.is_empty() {
                plan.splices.insert(*start, splice);
                for slot in start + 1..=end {
                    plan.splices.insert(slot, Vec::new());
                }
            } else {
                container.children.splice(*start..=end, splice);
            }
            plan.changed = true;
        }
        Ok((package, plan))
    }
}
