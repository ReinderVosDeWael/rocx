use super::{ImportedDocument, ParagraphAnchor, insertion::BoundaryState, unsupported};
use crate::opc::Package;
use crate::xml::{Content, Node};
use crate::{ResolvedBlock, ResolvedParagraph, Result};
use std::collections::{BTreeMap, BTreeSet};

/// Structural edits only accept paragraphs whose discarded markup is known local.
#[derive(Clone, Debug)]
pub enum BlockEdit {
    Replace(ParagraphAnchor, Box<ResolvedParagraph>),
    Remove(ParagraphAnchor),
    /// Target the first direct paragraph of a section. A paragraph-owned break
    /// is retained as a separate empty boundary paragraph.
    ReplaceSection(ParagraphAnchor, Vec<ResolvedBlock>),
    KeepWithNext(ParagraphAnchor, bool),
}

/// Allow-list *discardable* markup, including attributes and non-element
/// children. In particular paragraph IDs, fields, ranges and relationship
/// references cannot be silently dropped.
pub(super) fn local(node: &Node, parent: &str) -> bool {
    let name = node.name.as_str();
    let attrs: &[&str] = match (parent, name) {
        ("body", "w:p") | ("w:p", "w:r" | "w:pPr") | ("w:r", "w:rPr") => &[],
        ("w:r", "w:t") => &["xml:space"],
        ("w:r", "w:tab") => &[],
        ("w:pPr", "w:jc" | "w:keepNext" | "w:keepLines" | "w:pageBreakBefore")
        | ("w:rPr", "w:b" | "w:i" | "w:u" | "w:color" | "w:sz") => &["w:val"],
        _ => return false,
    };
    if let Some(prop_name) = match name {
        "w:p" => Some("w:pPr"),
        "w:r" => Some("w:rPr"),
        _ => None,
    } && node.elements().skip(1).any(|n| n.name == prop_name)
    {
        return false;
    }
    if matches!(name, "w:pPr" | "w:rPr") {
        let mut names = BTreeSet::new();
        if node.elements().any(|n| !names.insert(n.name.as_str())) {
            return false;
        }
    }
    node.attrs
        .iter()
        .all(|(n, _)| n.declaration().is_some() || attrs.iter().any(|a| n == *a))
        && node.children.iter().all(|c| match c {
            Content::Element(child) => local(child, name),
            Content::Text(text) => name == "w:t" || text.trim().is_empty(),
            Content::Raw(_) => false,
        })
}

#[derive(Default)]
pub(super) struct Plan {
    deletes: BTreeSet<usize>,
    payloads: Vec<(usize, ResolvedBlock, i64)>,
    pub(super) changes: Vec<usize>,
}

impl Plan {
    pub(super) fn build(
        doc: &ImportedDocument,
        root: &mut Node,
        edits: &[BlockEdit],
        other_targets: &BTreeSet<usize>,
        inserted_after: &BTreeSet<usize>,
    ) -> Result<Self> {
        let mut plan = Self::default();
        if edits.is_empty() {
            return Ok(plan);
        }
        let sections = if edits
            .iter()
            .any(|e| matches!(e, BlockEdit::ReplaceSection(..)))
        {
            super::header::rewrite_section_ranges(root, &doc.package, &doc.main)?
        } else {
            Vec::new()
        };
        if root.elements().filter(|n| n.name == "w:body").count() != 1 {
            return Err(unsupported("ambiguous document body"));
        }
        let body = root.child("w:body").ok_or_else(super::missing_body)?;
        let mut terminal = false;
        for n in body.elements() {
            if terminal {
                return Err(unsupported("body section properties are not terminal"));
            }
            terminal = n.name == "w:sectPr";
        }
        let mut assigned = BTreeMap::new();
        let mut section_payloads = BTreeMap::new();
        for edit in edits {
            let target = match edit {
                BlockEdit::Replace(a, _)
                | BlockEdit::Remove(a)
                | BlockEdit::ReplaceSection(a, _)
                | BlockEdit::KeepWithNext(a, _) => a,
            };
            doc.validate_snapshot(&target.snapshot)?;
            let start = target.body_index;
            let end = if let BlockEdit::ReplaceSection(_, payload) = edit {
                // The slot after the preceding break is inside this section.
                if start
                    .checked_sub(1)
                    .is_some_and(|i| inserted_after.contains(&i))
                {
                    return Err(unsupported("insertion conflicts with section replacement"));
                }
                let (range, owner) = sections
                    .iter()
                    .find(|(s, _)| s.start == start && !s.is_empty())
                    .ok_or_else(|| unsupported("section target must be its first paragraph"))?;
                let mut checked = Vec::new();
                for block in payload {
                    let width = match block {
                        ResolvedBlock::Paragraph(_) => 0,
                        ResolvedBlock::Table(table) => {
                            super::insertion::table_width(table, Ok(*owner))?
                        }
                        _ => {
                            return Err(unsupported(
                                "section payload must contain resolved paragraphs or tables",
                            ));
                        }
                    };
                    checked.push((block.clone(), width));
                }
                section_payloads.insert(start, checked);
                range.end
            } else {
                start + 1
            };
            for index in start..end {
                if (other_targets.contains(&index) && !matches!(edit, BlockEdit::KeepWithNext(..)))
                    || assigned.insert(index, (start, edit)).is_some()
                {
                    return Err(unsupported("conflicting edits target the same body block"));
                }
            }
        }
        let mut boundaries = BoundaryState::default();
        let body = root.child_mut("w:body").ok_or_else(super::missing_body)?;
        let mut index = 0;
        for (slot, content) in body.children.iter_mut().enumerate() {
            let Content::Element(node) = content else {
                if assigned.get(&index).is_some_and(|(start, edit)| {
                    *start < index && matches!(edit, BlockEdit::ReplaceSection(..))
                }) && !matches!(content, Content::Text(t) if t.trim().is_empty())
                {
                    return Err(unsupported("opaque content inside section range"));
                }
                continue;
            };
            if node.name == "w:sectPr" {
                continue;
            }
            let before = boundaries.safe();
            boundaries.advance(node);
            if let Some((start, edit)) = assigned.remove(&index) {
                if !before || !boundaries.safe() || node.name != "w:p" {
                    return Err(unsupported(
                        "target must be a direct paragraph outside protected ranges",
                    ));
                }
                match edit {
                    BlockEdit::KeepWithNext(_, after) => {
                        if super::patch_property(
                            node,
                            "w:keepNext",
                            if *after { "1" } else { "0" },
                        )?
                        .is_some()
                        {
                            plan.changes.push(index);
                        }
                    }
                    _ => {
                        let boundary = node
                            .child("w:pPr")
                            .is_some_and(|p| p.child("w:sectPr").is_some());
                        if boundary && matches!(edit, BlockEdit::ReplaceSection(..)) {
                            // Keep the original ancestor chain (namespace bindings and
                            // inherited attributes) around the original sectPr.
                            if node
                                .elements()
                                .any(|n| n.name != "w:pPr" && !local(n, "w:p"))
                                || node.attrs.iter().any(|(n, _)| n.declaration().is_none())
                                || node
                                    .child("w:pPr")
                                    .unwrap()
                                    .elements()
                                    .any(|p| p.name != "w:sectPr" && !local(p, "w:pPr"))
                                || node.children.iter().any(|c| {
                                    matches!(c, Content::Raw(_))
                                        || matches!(c, Content::Text(t) if !t.trim().is_empty())
                                })
                            {
                                return Err(unsupported(
                                    "section break contains unsupported content",
                                ));
                            }
                            node.children
                                .retain(|c| !matches!(c, Content::Element(n) if n.name != "w:pPr"));
                        } else if local(node, "body") {
                            plan.deletes.insert(slot);
                        } else {
                            return Err(unsupported(
                                "paragraph contains references or unsupported markup",
                            ));
                        }
                        if index == start {
                            plan.changes.push(index);
                            match edit {
                                BlockEdit::Replace(_, p) => plan.payloads.push((
                                    slot,
                                    ResolvedBlock::Paragraph(p.as_ref().clone()),
                                    0,
                                )),
                                BlockEdit::ReplaceSection(..) => {
                                    plan.payloads.extend(
                                        section_payloads
                                            .remove(&start)
                                            .unwrap()
                                            .into_iter()
                                            .map(|(b, w)| (slot, b, w)),
                                    );
                                }
                                _ => (),
                            }
                        }
                    }
                }
            }
            index += 1;
        }
        if !assigned.is_empty() {
            return Err(unsupported("body block not found"));
        }
        Ok(plan)
    }

    pub(super) fn stage(
        &mut self,
        mut package: Package,
        main: &str,
        body: &mut Node,
        mut pending: Vec<(usize, Node)>,
    ) -> Result<Package> {
        let mut slots = Vec::new();
        let mut paragraphs = Vec::new();
        let mut tables = Vec::new();
        for (slot, block, width) in self.payloads.drain(..) {
            slots.push((slot, matches!(&block, ResolvedBlock::Table(_))));
            match block {
                ResolvedBlock::Paragraph(p) => paragraphs.push(p),
                ResolvedBlock::Table(t) => tables.push((t, width)),
                _ => unreachable!("validated section payload"),
            }
        }
        let mut ps = if !paragraphs.is_empty() {
            let (staged, nodes) =
                crate::ooxml::encode_editor_paragraphs(package, main, paragraphs)?;
            package = staged;
            nodes
        } else {
            Vec::new()
        }
        .into_iter();
        let mut ts = if !tables.is_empty() {
            let (staged, nodes) = crate::ooxml::encode_editor_tables(package, main, tables)?;
            package = staged;
            nodes
        } else {
            Vec::new()
        }
        .into_iter();
        pending.extend(
            slots.into_iter().map(|(slot, is_table)| {
                (slot, if is_table { ts.next() } else { ps.next() }.unwrap())
            }),
        );
        if !pending.is_empty() || !self.deletes.is_empty() {
            pending.sort_by_key(|(slot, _)| *slot);
            let mut pending = pending.into_iter().peekable();
            for (index, c) in std::mem::take(&mut body.children).into_iter().enumerate() {
                while pending.peek().is_some_and(|(slot, _)| *slot == index) {
                    body.children
                        .push(Content::Element(pending.next().unwrap().1));
                }
                if !self.deletes.contains(&index) {
                    body.children.push(c);
                }
            }
            body.children
                .extend(pending.map(|(_, n)| Content::Element(n)));
        }
        Ok(package)
    }
}
