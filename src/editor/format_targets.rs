//! Resolve physical paths from the same child projections used for inspection.
//! Never rewrite a hyperlink, table, or story to apply a run-property patch.
use super::{
    BoundaryState, ImportedDocument, InlineAnchor, InlineEdit, InlineLocation, PackagePlan,
    run_format::{self, Slots},
    text, unsupported,
};
use crate::{Result, ooxml::WordParts, xml::Node};
use std::collections::BTreeMap;

#[derive(Default)]
struct Targets {
    children: BTreeMap<usize, Targets>,
    slots: Slots,
}

fn reviewed(n: &Node) -> bool {
    n.name.local().ends_with("Change")
        || matches!(
            n.name.as_str(),
            "w:ins"
                | "w:del"
                | "w:moveFrom"
                | "w:moveTo"
                | "w:cellIns"
                | "w:cellDel"
                | "w:cellMerge"
                | "w:sdt"
                | "w:lock"
        )
        || n.name == super::ALTERNATE
        || n.elements().any(reviewed)
}

/// Property-bearing ancestors can carry review markers even when the run
/// itself has no revision wrapper (notably deleted rows and paragraph marks).
fn check_ancestor(n: &Node, props: &[&str]) -> Result<()> {
    for &tag in props {
        let mut items = n.elements().filter(|c| c.name == tag);
        if items.next().is_some_and(reviewed) || items.next().is_some() {
            return Err(unsupported("reviewed or ambiguous run context"));
        }
    }
    Ok(())
}

struct Cursor<'a> {
    n: &'a Node,
    path: Vec<usize>,
}
impl Cursor<'_> {
    fn step(&mut self, index: usize, skip: &[&str], tag: &str) -> Result<()> {
        let (slot, child) = self
            .n
            .elements()
            .enumerate()
            .filter(|(_, n)| !skip.contains(&n.name.as_str()))
            .nth(index)
            .filter(|(_, n)| n.name == tag)
            .ok_or_else(|| {
                unsupported("unsupported run path (expected ordinary run or hyperlink child)")
            })?;
        self.path.push(slot);
        self.n = child;
        Ok(())
    }
}

fn path(story: &Node, a: &InlineAnchor) -> Result<Vec<usize>> {
    let mut c = Cursor {
        n: story,
        path: Vec::new(),
    };
    match a.location {
        InlineLocation::Body(i) => c.step(i, &["w:sectPr"], "w:p")?,
        InlineLocation::TableCell {
            body_index,
            row_index,
            cell_index,
            block_index,
        } => {
            c.step(body_index, &["w:sectPr"], "w:tbl")?;
            check_ancestor(c.n, &["w:tblPr", "w:tblGrid"])?;
            c.step(row_index, &["w:tblPr", "w:tblGrid"], "w:tr")?;
            check_ancestor(c.n, &["w:trPr", "w:tblPrEx"])?;
            c.step(cell_index, &["w:trPr"], "w:tc")?;
            check_ancestor(c.n, &["w:tcPr"])?;
            c.step(block_index, &["w:tcPr"], "w:p")?;
        }
        InlineLocation::Entry { block_index, .. } | InlineLocation::Story { block_index, .. } => {
            c.step(block_index, &[], "w:p")?
        }
    }
    check_ancestor(c.n, &["w:pPr"])?;
    match a.path.as_slice() {
        [i] => c.step(*i, &["w:pPr"], "w:r")?,
        [i, j] => {
            c.step(*i, &["w:pPr"], "w:hyperlink")?;
            c.step(*j, &[], "w:r")?;
        }
        _ => {
            return Err(unsupported(
                "expected ordinary run or direct hyperlink child",
            ));
        }
    }
    check_ancestor(c.n, &["w:rPr"])?;
    Ok(c.path)
}

fn apply(n: &mut Node, mut targets: Targets, boundaries: &mut BoundaryState) -> Result<bool> {
    if !targets.slots.is_empty() {
        if !boundaries.safe() {
            return Err(unsupported("run inside field or active range"));
        }
        text::check_format_target(n)?;
        let changed = run_format::apply(n, targets.slots)?;
        boundaries.advance(n);
        return Ok(changed);
    }
    let mut changed = false;
    for (index, child) in n.elements_mut().enumerate() {
        if let Some(target) = targets.children.remove(&index) {
            changed |= apply(child, target, boundaries)?;
        } else {
            boundaries.advance(child);
        }
    }
    Ok(changed)
}

impl ImportedDocument {
    /// Format runs in one physical owner. `None` selects the main body
    /// (including directly inspected table-cell paragraphs). Named section
    /// parts are shared across aliases. Named entry parts exclude reserved notes.
    pub fn edit_run_format(
        &self,
        owner_part: Option<&str>,
        edits: &[InlineEdit],
    ) -> Result<Vec<u8>> {
        if edits.is_empty() {
            return Err(unsupported("expected a run formatting edit"));
        }
        let owner = owner_part.unwrap_or(&self.main);
        let mut groups = BTreeMap::<Option<usize>, Vec<&InlineEdit>>::new();
        let mut counts = None;
        for edit in edits {
            self.validate_snapshot(&edit.target.snapshot)?;
            let entry = match &edit.target.location {
                InlineLocation::Body(_) | InlineLocation::TableCell { .. }
                    if owner == self.main =>
                {
                    None
                }
                InlineLocation::Story { part, .. } if part.as_ref() == owner => None,
                InlineLocation::Entry { index, .. } => Some(
                    self.editable_entry(owner, *index, &mut counts)?
                        .element_index,
                ),
                _ => return Err(unsupported("run formatting owner mismatch")),
            };
            groups.entry(entry).or_default().push(edit);
        }
        let mut package = self.package.clone();
        if package.relationships(&self.main, "settings")?.len() > 1 {
            return Err(unsupported("ambiguous document protection settings"));
        }
        if let Some((_, p)) = package.related(&self.main, "settings")? {
            let settings = package.word(&p)?;
            fn protected(n: &Node) -> bool {
                n.name == "w:documentProtection"
                    && !matches!(n.get("w:enforcement"), Some("false" | "0" | "off"))
                    || n.elements().any(protected)
            }
            if settings.name != "w:settings" || protected(&settings) {
                return Err(unsupported("protected document formatting"));
            }
        }
        let mut root = package.word(owner)?;
        let mut changed = false;
        for (entry, group) in groups {
            let story = match entry {
                Some(i) => root
                    .elements_mut()
                    .nth(i)
                    .ok_or_else(|| unsupported("missing story entry"))?,
                None if owner == self.main => root
                    .child_mut("w:body")
                    .ok_or_else(|| unsupported("missing body"))?,
                None => &mut root,
            };
            let mut targets = Targets::default();
            for edit in group {
                let mut target = &mut targets;
                for i in path(story, &edit.target)? {
                    target = target.children.entry(i).or_default();
                }
                text::compile_format(&mut target.slots, &edit.action, &package, &self.main)?;
            }
            changed |= apply(story, targets, &mut BoundaryState::default())?;
        }
        if changed {
            package.put_word(owner, root)?;
        }
        PackagePlan::new(package)?.to_bytes()
    }
}
