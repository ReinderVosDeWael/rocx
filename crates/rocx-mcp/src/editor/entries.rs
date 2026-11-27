use std::collections::BTreeMap;

use rocx::editor::{
    EntryEdit, EntryKind, ImportedEntry, ImportedInline, ImportedInlineNode, ImportedStoryBlock,
    RunTextEdit,
};
use serde_json::{Value, json};

use super::{Snapshot, block_readback::inline_items, projected_run_text};
use crate::{diagnostics::ScopedEditError, schema::stories::StoryTextChange};

mod inlines;

impl Snapshot {
    pub fn entry_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        if limit == 0 {
            return Err("entry page limit must be positive".into());
        }
        let entries = self.document.entry_stories().map_err(|e| e.to_string())?;
        let start = start.min(entries.len());
        let end = start.saturating_add(limit).min(entries.len());
        let page = entries[start..end]
            .iter()
            .enumerate()
            .map(|(offset, entry)| entry_value(start + offset, entry))
            .collect();
        Ok((page, (end < entries.len()).then_some(end)))
    }

    // Session must resolve only anchors issued for the caller's document.
    pub fn edit_entry(&self, local: &str, edit: EntryEdit) -> Result<Vec<u8>, ScopedEditError> {
        let index = entry_index(local).ok_or("unknown local entry anchor")?;
        let entries = self.document.entry_stories()?;
        let entry = entries
            .get(index)
            .filter(|entry| is_candidate(entry.kind, entry.note_type.as_deref(), &entry.id))
            .ok_or("unknown local entry anchor")?;
        self.document
            .edit_entry(&entry.part_name, &entry.id, edit)
            .map_err(ScopedEditError::from)
    }

    pub fn entry_run_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        let entries = self.document.entry_stories().map_err(|e| e.to_string())?;
        let mut owners = BTreeMap::new();
        for (index, entry) in entries.iter().enumerate() {
            owners.entry(entry.part_name.as_str()).or_insert(index);
        }
        let end = start.saturating_add(limit).min(entries.len());
        let rows = (start..end)
            .map(|index| {
                let entry = &entries[index];
                let owner = owners[entry.part_name.as_str()];
                let mut runs = Vec::new();
                for block in 0..entry.blocks().len() {
                    for (inline, node) in entry
                        .inline_structure(block)
                        .unwrap_or_default()
                        .iter()
                        .enumerate()
                    {
                        if let ImportedInlineNode::Run(run) = node {
                            let (text, opaque) = projected_run_text(&run.items);
                            let mut result = json!({
                                "block_index": block, "inline_index": inline, "text": text,
                                "has_opaque_content": opaque,
                            });
                            if is_candidate(entry.kind, entry.note_type.as_deref(), &entry.id)
                                && matches!(run.items.as_slice(), [ImportedInline::Text(_)])
                            {
                                result["anchor"] =
                                    json!(format!("entry-run:{index}:{block}:{inline}"));
                            }
                            runs.push(result);
                        }
                    }
                }
                json!({
                    "kind": entry.kind.as_str(), "id": entry.id, "note_type": entry.note_type,
                    "owner": {
                        "part_name": entry.part_name,
                        "anchor": format!("entry-run-owner:{owner}"),
                    },
                    "runs": runs,
                })
            })
            .collect();
        Ok((rows, (end < entries.len()).then_some(end)))
    }

    pub fn edit_entry_story_text(
        &self,
        owner: &str,
        edits: Vec<StoryTextChange>,
    ) -> Result<Vec<u8>, ScopedEditError> {
        let entries = self.document.entry_stories()?;
        let owner = owner
            .strip_prefix("entry-run-owner:")
            .and_then(index_of)
            .and_then(|index| entries.get(index))
            .ok_or_else(|| {
                ScopedEditError::WrongAnchorType("expected a physical entry owner".into())
            })?;
        let mut native = Vec::with_capacity(edits.len());
        for edit in edits {
            let wrong =
                || ScopedEditError::WrongAnchorType("expected a direct entry run anchor".into());
            let mut coords = edit
                .anchor
                .strip_prefix("entry-run:")
                .ok_or_else(wrong)?
                .split(':');
            let (Some(entry), Some(block), Some(inline), None) =
                (coords.next(), coords.next(), coords.next(), coords.next())
            else {
                return Err(wrong());
            };
            let entry = index_of(entry)
                .and_then(|index| entries.get(index))
                .ok_or("unknown entry run anchor")?;
            let Some(ImportedInlineNode::Run(run)) = index_of(block)
                .and_then(|index| entry.inline_structure(index))
                .and_then(|nodes| nodes.get(index_of(inline)?))
            else {
                return Err("unknown entry run anchor".into());
            };
            // Let the native call reject a run in a different physical owner,
            // as well as protected, duplicate and aliased entry targets.
            native.push(RunTextEdit {
                target: run.anchor.clone(),
                text: edit.text,
            });
        }
        self.document
            .edit_story_text(&owner.part_name, &native)
            .map_err(Into::into)
    }
}

fn entry_index(local: &str) -> Option<usize> {
    index_of(local.strip_prefix("entry:")?)
}

fn index_of(digits: &str) -> Option<usize> {
    if !digits.bytes().all(|b| b.is_ascii_digit()) || digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    digits.parse().ok()
}

fn is_candidate(kind: EntryKind, note_type: Option<&str>, id: &str) -> bool {
    if kind == EntryKind::Comment {
        return true;
    }
    // An untyped/normal note with a negative decimal value is reserved too.
    // Leave malformed IDs and aliases to native validation; candidates
    // identify source rows, not a guarantee of editability.
    let negative = id
        .trim_matches([' ', '\t', '\r', '\n'])
        .strip_prefix('-')
        .is_some_and(|digits| {
            digits.bytes().all(|b| b.is_ascii_digit()) && !digits.trim_start_matches('0').is_empty()
        });
    matches!(note_type, None | Some("normal")) && !negative
}

fn entry_value(index: usize, entry: &ImportedEntry) -> Value {
    let blocks: Vec<_> = entry
        .blocks()
        .iter()
        .enumerate()
        .map(|(index, block)| match block {
            ImportedStoryBlock::Paragraph(items) => {
                let (text, opaque) = projected_run_text(items);
                json!({
                    "index": index, "type": "paragraph",
                    "text": text, "has_opaque_content": opaque,
                    "inline_items": inline_items(items),
                })
            }
            ImportedStoryBlock::Opaque(name) => {
                json!({"index": index, "type": "opaque", "name": name})
            }
            _ => json!({"index": index, "type": "opaque", "name": "unknown"}),
        })
        .collect();
    let mut value = json!({
        "kind": entry.kind.as_str(), "part_name": entry.part_name, "id": entry.id,
        "note_type": entry.note_type, "author": entry.author,
        "date": entry.date, "initials": entry.initials, "blocks": blocks,
    });
    if is_candidate(entry.kind, entry.note_type.as_deref(), &entry.id) {
        value["anchor"] = json!(format!("entry:{index}"));
    }
    value
}

#[cfg(test)]
mod tests;
