use rocx::editor::{EntryEdit, EntryKind, ImportedEntry, ImportedStoryBlock};
use serde_json::{Value, json};

use super::{Snapshot, projected_run_text};

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
    pub fn edit_entry(&self, local: &str, edit: EntryEdit) -> Result<Vec<u8>, String> {
        let index = entry_index(local).ok_or("unknown local entry anchor")?;
        let entries = self.document.entry_stories().map_err(|e| e.to_string())?;
        let entry = entries
            .get(index)
            .filter(|entry| is_candidate(entry.kind, entry.note_type.as_deref(), &entry.id))
            .ok_or("unknown local entry anchor")?;
        self.document
            .edit_entry(&entry.part_name, &entry.id, edit)
            .map_err(|e| e.to_string())
    }
}

fn entry_index(local: &str) -> Option<usize> {
    let digits = local.strip_prefix("entry:")?;
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
