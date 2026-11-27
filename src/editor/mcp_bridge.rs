//! Narrow native lookups for the workspace MCP transport.

use super::{ImportedDocument, SectionStoryKind};
use crate::{NoteKind, Result};

/// Count logical story sections without inspecting unrelated layout properties.
pub fn story_section_count(document: &ImportedDocument) -> Result<usize> {
    document.story_section_count()
}

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
    document.linkable_story_parts(kind, false)
}

/// List native link candidates, including parts already used by a section.
pub fn all_linkable_story_parts(
    document: &ImportedDocument,
    kind: SectionStoryKind,
) -> Result<Vec<String>> {
    document.linkable_story_parts(kind, true)
}
