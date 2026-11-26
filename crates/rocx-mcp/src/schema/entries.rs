use rmcp::schemars::{self, JsonSchema};
use rocx::editor::EntryEdit;
use serde::{Deserialize, Deserializer};

use super::{
    layout::LayoutChange,
    notes::{NoteChange, NoteKind},
    resources::ResourceQuery,
    revisions::RevisionChange,
    run_format::RunFormatChange,
    stories::{StoryBlockChange, StoryReferenceChange, StoryTextChange},
    tables::PhysicalTableChange,
};

const MIN_LIMIT: usize = 1;
const MAX_LIMIT: usize = 100;

fn default_limit() -> usize {
    25
}

// An omitted selector or cursor is valid; an explicit null is not.
pub(super) fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

fn entry_limit<'de, D: Deserializer<'de>>(deserializer: D) -> Result<usize, D::Error> {
    let limit = usize::deserialize(deserializer)?;
    if !(MIN_LIMIT..=MAX_LIMIT).contains(&limit) {
        return Err(serde::de::Error::custom(format_args!(
            "entry page limit must be between {MIN_LIMIT} and {MAX_LIMIT}"
        )));
    }
    Ok(limit)
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(super) enum View {
    /// Page through ordinary footnote, endnote, and comment definitions.
    Entries {
        /// Maximum number of entries to return on this page.
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        /// Opaque next-page cursor issued for this document's entries view. Omit for the first page.
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Page through direct body note references and insertion candidates.
    NoteReferences {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Page through revision proposal and review targets in the direct body.
    Revisions {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Page through direct paragraph and section layout properties.
    Layout {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Page through all logical header/footer slots and their physical owners.
    Stories {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Page through native section boundaries and direct section properties.
    Sections {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Inspect supported fields in original main-story order.
    Fields {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Inspect direct body inlines and original node anchors.
    BodyInlines {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Inspect direct body tables for ordinary mixed geometry edits.
    BodyTables {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Inspect direct run formatting in physical owners.
    FormatRuns {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Inspect top-level and nested physical tables and their owners.
    PhysicalTables {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Page through direct runs in note and comment entries, by physical owner.
    EntryRuns {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Read direct defaults and document properties without evaluating style inheritance.
    Resources {
        /// Look up a single custom property by name.
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        custom_property: Option<String>,
    },
    /// Inspect one resource via its independent native accessor.
    ResourceValue { query: ResourceQuery },
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(super) enum Operation {
    /// Apply one edit to an ordinary note or comment definition in the original snapshot.
    EditEntry {
        /// Entry definition anchor from the entries view, not a note reference anchor.
        anchor: String,
        edit: EntryChange,
    },
    /// Create, attach, detach, delete, retarget or renumber a direct body note reference.
    EditNoteReference {
        /// Anchor from the note-references view.
        anchor: String,
        note_kind: NoteKind,
        edit: NoteChange,
    },
    /// Propose or review one tracked edit.
    EditRevision {
        /// Direct paragraph/run anchor from compact inspection for proposals,
        /// or pending revision anchor from the revisions view for review.
        anchor: String,
        /// Move destination or other wrapper of a paired move review.
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        paired_anchor: Option<String>,
        edit: RevisionChange,
    },
    /// Apply one native layout transaction against the original paragraph and section owners.
    EditLayout { edits: Vec<LayoutChange> },
    /// Change run text in one physical header, footer, note or comment part.
    EditStoryText {
        /// Physical owner anchor from the stories or entry-runs view.
        anchor: String,
        edits: Vec<StoryTextChange>,
    },
    /// Create, link or unlink an explicit story reference for a logical section slot.
    EditStoryReference {
        /// Logical slot anchor from the stories view.
        anchor: String,
        edit: StoryReferenceChange,
    },
    /// Splice original blocks in an existing header/footer slot.
    EditStoryBlocks {
        /// Logical slot anchor from the stories view.
        anchor: String,
        edit: StoryBlockChange,
    },
    /// Edit native direct run formatting in one physical owner.
    EditRunFormat {
        /// Owner anchor from the format-runs view.
        anchor: String,
        #[schemars(length(min = 1))]
        edits: Vec<RunFormatChange>,
    },
    /// Edit tables in a single physical part.
    EditPartTables {
        /// Owner anchor from the physical-tables view.
        anchor: String,
        #[schemars(length(min = 1))]
        edits: Vec<PhysicalTableChange>,
    },
    /// Preview the package produced by an empty native EditBatch, without mutating the source.
    NoOp {},
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(super) enum EntryChange {
    /// Delete a discardable, unreferenced ordinary footnote or endnote.
    DeleteUnusedNote {},
    /// Insert a plain paragraph at a block gap within the entry.
    InsertParagraph {
        /// Zero-based original block index to insert before, or the block count to append.
        index: usize,
        text: String,
    },
    /// Replace an eligible entry paragraph with plain text.
    ReplaceParagraph {
        /// Zero-based original block index, counting non-paragraph blocks as well.
        index: usize,
        text: String,
    },
    /// Remove an eligible entry paragraph.
    RemoveParagraph {
        /// Zero-based original block index, counting non-paragraph blocks as well.
        index: usize,
    },
    /// Set the author of a base comment.
    CommentAuthor { value: String },
    /// Set or clear the initials of a base comment.
    CommentInitials {
        /// Required. Pass null to clear.
        value: NullableString,
    },
    /// Set or clear the stored date of a base comment.
    CommentDate {
        /// Required. Pass a date-time string to set or null to clear.
        value: NullableString,
    },
}

// A non-Option field makes omission an error while the inner Option accepts null.
#[derive(Deserialize, JsonSchema)]
#[schemars(inline)]
pub(super) struct NullableString(Option<String>);

impl From<EntryChange> for EntryEdit {
    fn from(edit: EntryChange) -> Self {
        match edit {
            EntryChange::DeleteUnusedNote {} => Self::DeleteUnusedNote,
            EntryChange::InsertParagraph { index, text } => Self::InsertParagraph { index, text },
            EntryChange::ReplaceParagraph { index, text } => Self::ReplaceParagraph { index, text },
            EntryChange::RemoveParagraph { index } => Self::RemoveParagraph(index),
            EntryChange::CommentAuthor { value } => Self::CommentAuthor(value),
            EntryChange::CommentInitials { value } => Self::CommentInitials(value.0),
            EntryChange::CommentDate { value } => Self::CommentDate(value.0),
        }
    }
}
