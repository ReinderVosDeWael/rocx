//! Narrow native lookups for the workspace MCP transport.

use super::{ImportedDocument, SectionStoryKind};
use crate::{NoteKind, Result};

/// Resolve the main document's selected note part, independent of its entries.
/// This only locates the target; the native edit validates the owner and edit.
pub fn note_owner(document: &ImportedDocument, kind: NoteKind) -> Result<Option<String>> {
    let relation = match kind {
        NoteKind::Footnote => "footnotes",
        NoteKind::Endnote => "endnotes",
    };
    Ok(document
        .package
        .related(&document.main, relation)?
        .map(|(_, part)| part))
}

/// List native link candidates for the selected physical story kind.
pub fn linkable_story_parts(
    document: &ImportedDocument,
    kind: SectionStoryKind,
) -> Result<Vec<String>> {
    document.linkable_story_parts(kind)
}
