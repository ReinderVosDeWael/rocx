use std::collections::BTreeMap;

use rocx::editor::{ImportedInline, ImportedInlineNode, InlineAnchor, InlineEdit};
use serde_json::{Value, json};

use super::{index_of, is_candidate};
use crate::editor::{
    Snapshot,
    body_inlines::{InlineHandles, project},
};
use crate::{diagnostics::ScopedEditError, schema::entry_inlines::EntryInlineChange};

const OWNER: &str = "entry-inline-owner:";
const INLINE: &str = "entry-inline:";

fn candidate(node: &ImportedInlineNode) -> Option<&InlineAnchor> {
    match node {
        ImportedInlineNode::Run(run)
            if run.drawing.as_ref().is_none_or(|d| d.kind == "image")
                && !run.items.iter().any(|item| match item {
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
                }) =>
        {
            Some(&run.anchor)
        }
        ImportedInlineNode::Hyperlink(link) => Some(&link.anchor),
        _ => None,
    }
}

impl Snapshot {
    pub fn entry_inline_page(
        &self,
        start: usize,
        limit: usize,
        handles: &mut InlineHandles<'_>,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        let entries = self.document.entry_stories().map_err(|e| e.to_string())?;
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
                row["note_type"] = json!(entry.note_type);
                row["block_index"] = json!(block);
                row["inline_index"] = json!(inline);
                let owner = owners[entry.part_name.as_str()];
                row["owner"] =
                    json!({"part_name": entry.part_name, "anchor": format!("{OWNER}{owner}")});
                if is_candidate(entry.kind, entry.note_type.as_deref(), &entry.id)
                    && candidate(node).is_some()
                {
                    row["anchor"] = json!(format!("{INLINE}{i}:{block}:{inline}"));
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
            let (Some(i), Some(block), Some(inline), None) = (
                coordinates.next(),
                coordinates.next(),
                coordinates.next(),
                coordinates.next(),
            ) else {
                return None;
            };
            entries
                .get(index_of(i)?)?
                .inline_structure(index_of(block)?)?
                .get(index_of(inline)?)
                .and_then(candidate)
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
        self.document
            .edit_entry_inlines(&owner.part_name, &edits)
            .map_err(Into::into)
    }
}
