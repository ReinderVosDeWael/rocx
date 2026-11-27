use std::collections::BTreeMap;

use rocx::editor::{ImportedInline, ImportedInlineNode, ImportedRun, InlineAnchor, InlineEdit};
use serde_json::{Value, json};

use super::{index_of, is_candidate};
use crate::editor::{
    Snapshot,
    body_inlines::{InlineHandles, project},
};
use crate::{diagnostics::ScopedEditError, schema::entry_inlines::EntryInlineChange};

const OWNER: &str = "entry-inline-owner:";
const INLINE: &str = "entry-inline:";

fn run_candidate(run: &ImportedRun) -> Option<&InlineAnchor> {
    (!run.items.iter().any(|item| match item {
        ImportedInline::NoteReference { .. } => true,
        ImportedInline::Opaque(name) => matches!(
            name.as_str(),
            "w:annotationRef"
                | "w:commentReference"
                | "w:endnoteRef"
                | "w:footnoteRef"
                | "w:endnoteReference"
                | "w:footnoteReference"
        ),
        _ => false,
    }))
    .then_some(&run.anchor)
}

fn direct_candidate(node: &ImportedInlineNode) -> Option<&InlineAnchor> {
    match node {
        ImportedInlineNode::Run(run) => run_candidate(run),
        ImportedInlineNode::Hyperlink(link) => Some(&link.anchor),
        _ => None,
    }
}

fn hyperlink_child_candidate(node: &ImportedInlineNode) -> Option<&InlineAnchor> {
    match node {
        ImportedInlineNode::Run(run) => run_candidate(run),
        _ => None,
    }
}

impl Snapshot {
    pub fn entry_inline_page(
        &self,
        start: usize,
        limit: usize,
        handles: &mut InlineHandles<'_>,
    ) -> rocx::Result<(Vec<Value>, Option<usize>)> {
        let entries = self.document.entry_stories()?;
        let mut owners = BTreeMap::new();
        for (i, entry) in entries.iter().enumerate() {
            owners.entry(entry.part_name.as_str()).or_insert(i);
        }
        let mut nodes = entries
            .iter()
            .enumerate()
            .flat_map(|(index, entry)| {
                (0..entry.blocks().len()).flat_map(move |block| {
                    entry
                        .inline_structure(block)
                        .unwrap_or_default()
                        .iter()
                        .enumerate()
                        .map(move |(inline, node)| (index, entry, block, inline, node))
                })
            })
            .skip(start);
        let rows: Vec<_> = nodes
            .by_ref()
            .take(limit)
            .map(|(i, entry, block, inline, node)| {
                let mut row = project(node, &format!("inline:entry:{i}:{block}:{inline}"), handles);
                row["entry_kind"] = json!(entry.kind.as_str());
                row["entry_index"] = json!(i);
                row["id"] = json!(entry.id);
                if let ImportedInlineNode::Revision(revision) = node {
                    row["revision_id"] = json!(revision.id);
                }
                row["note_type"] = json!(entry.note_type);
                row["block_index"] = json!(block);
                row["inline_index"] = json!(inline);
                let owner = owners[entry.part_name.as_str()];
                row["owner"] =
                    json!({"part_name": entry.part_name, "anchor": format!("{OWNER}{owner}")});
                if is_candidate(entry.kind, entry.note_type.as_deref(), &entry.id) {
                    let local = format!("{INLINE}{i}:{block}:{inline}");
                    if direct_candidate(node).is_some() {
                        row["anchor"] = json!(local);
                    }
                    if let (ImportedInlineNode::Hyperlink(link), Some(children)) =
                        (node, row["children"].as_array_mut())
                    {
                        for (j, (node, value)) in link.children.iter().zip(children).enumerate() {
                            if hyperlink_child_candidate(node).is_some() {
                                value["anchor"] = json!(format!("{local}:{j}"));
                            }
                        }
                    }
                }
                row
            })
            .collect();
        let next = nodes.next().map(|_| start + rows.len());
        Ok((rows, next))
    }

    pub fn edit_entry_inlines(
        &self,
        owner: &str,
        edits: Vec<EntryInlineChange>,
    ) -> Result<Vec<u8>, ScopedEditError> {
        let entries = self.document.entry_stories()?;
        let owner = owner
            .strip_prefix(OWNER)
            .and_then(index_of)
            .and_then(|i| entries.get(i))
            .ok_or_else(|| {
                ScopedEditError::WrongAnchorType("expected a physical entry inline owner".into())
            })?;
        let target = |local: &str| {
            let mut coordinates = local.strip_prefix(INLINE)?.split(':');
            let entry = entries.get(index_of(coordinates.next()?)?)?;
            let block = entry.inline_structure(index_of(coordinates.next()?)?)?;
            let node = block.get(index_of(coordinates.next()?)?)?;
            match coordinates.next() {
                None => direct_candidate(node),
                Some(child) => {
                    let ImportedInlineNode::Hyperlink(link) = node else {
                        return None;
                    };
                    if coordinates.next().is_some() {
                        return None;
                    }
                    hyperlink_child_candidate(link.children.get(index_of(child)?)?)
                }
            }
        };
        let edits = edits
            .into_iter()
            .map(|change| {
                Ok(InlineEdit {
                    target: target(&change.anchor)
                        .ok_or_else(|| {
                            ScopedEditError::WrongAnchorType(
                                "expected an entry inline anchor".into(),
                            )
                        })?
                        .clone(),
                    action: change.edit.resolve()?,
                })
            })
            .collect::<Result<Vec<_>, ScopedEditError>>()?;
        // Native entry-inline edits accept child paths and shape/chart runs
        // only in format-only batches.
        self.document
            .edit_entry_inlines(&owner.part_name, &edits)
            .map_err(Into::into)
    }
}
