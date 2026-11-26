use rocx::editor::{BodyBlock, EntryKind, ImportedInline, ImportedInlineNode, ImportedRun};
use serde_json::{Value, json};

use super::Snapshot;
use crate::schema::notes::{NoteChange, NoteKind};

impl Snapshot {
    pub fn note_reference_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        if limit == 0 {
            return Err("note-reference page limit must be positive".into());
        }
        let mut sites = self
            .document
            .body()
            .iter()
            .enumerate()
            .filter_map(|(body_index, block)| match block {
                BodyBlock::Paragraph(paragraph) => Some((body_index, &paragraph.inline_structure)),
                _ => None,
            })
            .flat_map(|(body_index, inlines)| {
                inlines
                    .iter()
                    .enumerate()
                    .filter_map(move |(inline_index, inline)| {
                        Some((body_index, inline_index, candidate(inline)?))
                    })
            })
            .skip(start);
        let page: Vec<_> = sites
            .by_ref()
            .take(limit)
            .map(|(body_index, inline_index, site)| {
                let mut row = json!({
                    "kind": match site {
                        Site::Insertion => "insertion-candidate",
                        Site::Reference { .. } => "reference",
                    },
                    "body_index": body_index,
                    "inline_index": inline_index,
                    "anchor": format!("note-reference:{body_index}:{inline_index}"),
                });
                if let Site::Reference { kind, id } = site {
                    row["note_kind"] = json!(kind.as_str());
                    row["id"] = json!(id);
                }
                row
            })
            .collect();
        let next = sites.next().map(|_| start + page.len());
        Ok((page, next))
    }

    // Session checks that this local anchor was issued by the note-references view.
    pub fn edit_note_reference(
        &self,
        local: &str,
        kind: NoteKind,
        edit: NoteChange,
    ) -> Result<Vec<u8>, String> {
        let (body_index, inline_index) = local
            .strip_prefix("note-reference:")
            .and_then(|address| address.split_once(':'))
            .and_then(|(body, inline)| Some((index(body)?, index(inline)?)))
            .ok_or("unknown local note-reference anchor")?;
        let Some(BodyBlock::Paragraph(paragraph)) = self.document.body().get(body_index) else {
            return Err("unknown local note-reference anchor".into());
        };
        let Some(inline @ ImportedInlineNode::Run(run)) =
            paragraph.inline_structure.get(inline_index)
        else {
            return Err("unknown local note-reference anchor".into());
        };
        candidate(inline).ok_or("unknown local note-reference anchor")?;

        let (kind, conventional_owner) = match kind {
            NoteKind::Footnote => (EntryKind::Footnote, "word/footnotes.xml"),
            NoteKind::Endnote => (EntryKind::Endnote, "word/endnotes.xml"),
        };
        // Include reserved entries: any entry identifies the selected owner.
        // The public API cannot reveal a relocated owner with zero entries.
        // The conventional path lets the native editor handle first-part creation;
        // it will still reject an unrelated part occupying that path.
        let owner = match self.document.entry_stories() {
            Ok(entries) => entries
                .iter()
                .find(|entry| entry.kind == kind)
                .map_or(conventional_owner, |entry| entry.part_name.as_str()),
            // Creation of a first part has its own native guard path. An
            // unrelated invalid entry catalog must not preempt that path.
            Err(_) if matches!(edit, NoteChange::Create { .. }) => conventional_owner,
            Err(error) => return Err(error.to_string()),
        };
        self.document
            .edit_note_reference(owner, &run.anchor, edit.into())
            .map_err(|e| e.to_string())
    }
}

fn index(digits: &str) -> Option<usize> {
    if digits.len() > 1 && digits.starts_with('0') || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

enum Site<'a> {
    Insertion,
    Reference { kind: rocx::NoteKind, id: &'a str },
}

fn candidate(inline: &ImportedInlineNode) -> Option<Site<'_>> {
    let ImportedInlineNode::Run(ImportedRun { items, .. }) = inline else {
        return None;
    };
    // Use the public projection to find candidate shapes, not to authorize
    // edits. Native validation still checks the actual XML and surrounding ranges.
    match items.as_slice() {
        [ImportedInline::NoteReference { kind, id }] => Some(Site::Reference { kind: *kind, id }),
        [ImportedInline::Text(_) | ImportedInline::Tab | ImportedInline::Break(_)] => {
            Some(Site::Insertion)
        }
        // An unrecognized break type is opaque in the projection, but
        // inserting after a direct break does not alter that break.
        [ImportedInline::Opaque(name)] if name == "w:br" => Some(Site::Insertion),
        _ => None,
    }
}
