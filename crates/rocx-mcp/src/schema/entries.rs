use rmcp::schemars::{self, JsonSchema};
use rocx::editor::{EntryEdit, SectionStoryKind};
use serde::{Deserialize, Deserializer};

use super::{
    entry_inlines::EntryInlineChange,
    layout::{LayoutChange, LayoutScope},
    notes::{NoteChange, NoteKind},
    numbering::PhysicalNumberingScope,
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
#[serde(rename_all = "kebab-case")]
pub(super) enum LinkableStoryKind {
    Header,
    Footer,
}

impl From<LinkableStoryKind> for SectionStoryKind {
    fn from(kind: LinkableStoryKind) -> Self {
        match kind {
            LinkableStoryKind::Header => Self::Header,
            LinkableStoryKind::Footer => Self::Footer,
        }
    }
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
        /// Select all layout, only direct body paragraphs, or only sections.
        #[serde(default)]
        scope: LayoutScope,
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
    /// Page through effective default headers without reading other header or footer bodies.
    /// Use `image_slot.anchor` to append images or splice blocks in that section's default header.
    /// Each paragraph's `run_targets` lists eligible `anchor` values for text
    /// edits and `format_target.anchor` values for direct run formatting.
    DefaultHeaders {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Page through related header or footer parts eligible to link.
    /// Use an issued owner anchor as the source of an edit-story-reference link.
    LinkableStories {
        story_kind: LinkableStoryKind,
        /// Include parts already used by a section. Defaults to false.
        #[serde(default)]
        include_referenced: bool,
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        /// Opaque next-page cursor for this document, story kind, and include_referenced setting.
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Inspect typed inlines in one direct header/footer paragraph.
    StoryInlines {
        /// Physical owner anchor from the `stories` view (not a section slot anchor).
        anchor: String,
        /// Original zero-based index in the story's `blocks`, including opaque blocks.
        block_index: usize,
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Inspect typed inlines in one direct body table-cell paragraph.
    TableCellInlines {
        /// Table anchor from the `body-tables` view.
        anchor: String,
        /// Zero-based index in the table's `rows` (physical rows, not opaque siblings).
        row_index: usize,
        /// Zero-based index in the row's `cells` (physical cells, not grid columns).
        cell_index: usize,
        /// Original zero-based index in the cell's blocks, including opaque blocks.
        block_index: usize,
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
    /// Read one field at its original inline slot in an inspected body paragraph.
    FieldAt { anchor: String, inline_index: usize },
    /// Inspect supported fields in original main-story order.
    Fields {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Inspect typed direct body inlines, readback handles, and original edit anchors.
    BodyInlines {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Read one native single-series column chart at an original body-inlines anchor.
    ChartSeries { anchor: String },
    /// Resolve a read-only handle issued by an inline view of this open snapshot.
    InlineAt { inline_handle: String },
    /// Inspect physical bookmark markers, comment markers and comment references in direct body paragraphs.
    RangeMarkers {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Inspect main-story content controls and their stored properties.
    Controls {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Inspect eligible original main-story sites for wrapping.
    ControlSites {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Inspect read-only controls in physical headers, footers and ordinary entries.
    StoryControls {
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
        /// Omit for the aggregate catalog, or select one physical owner.
        #[serde(default, deserialize_with = "present")]
        #[schemars(
            with = "PhysicalTableSelector",
            skip_serializing_if = "Option::is_none"
        )]
        scope: Option<PhysicalTableSelector>,
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
    /// Page through typed note/comment inlines with readback handles and eligible edit anchors.
    EntryInlines {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Inspect the relationship-selected styles catalog, in stored order.
    Styles {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Page through native directly stored abstract numbering definitions.
    NumberingAbstracts {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Page through native concrete instances and their modeled overrides.
    NumberingInstances {
        #[serde(default = "default_limit", deserialize_with = "entry_limit")]
        #[schemars(range(min = MIN_LIMIT, max = MAX_LIMIT))]
        limit: usize,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Read a direct body's stored list assignment at a compact paragraph anchor.
    ParagraphNumbering { anchor: String },
    /// Read direct numbering and issue anchors for physical story, entry, or body-cell paragraphs.
    NumberingParagraphs {
        scope: PhysicalNumberingScope,
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
    /// Inspect compact body blocks and their edit anchors without reading headers or footers.
    Body,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum PhysicalTableSelector {
    /// Read only the actual main document part.
    Main,
    /// Read one physical header/footer owner without reading entry catalogs.
    Story {
        /// Physical owner anchor from the `stories` view (not a section slot anchor).
        anchor: String,
    },
    /// Read one physical footnote, endnote, or comment part using native catalog validation.
    Entry {
        /// Physical owner anchor from `entry-runs` or `entry-inlines` (not an individual entry).
        anchor: String,
    },
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
        /// Physical owner anchor from the stories, entry-runs, or default-headers view.
        anchor: String,
        edits: Vec<StoryTextChange>,
    },
    /// Edit inlines in a single physical footnote, endnote, or comment part.
    EditEntryInlines {
        /// Physical owner anchor from the entry-inlines view.
        anchor: String,
        #[schemars(length(min = 1))]
        edits: Vec<EntryInlineChange>,
    },
    /// Create, link or unlink an explicit story reference for a logical section slot.
    EditStoryReference {
        /// Logical slot anchor from the stories view.
        anchor: String,
        /// To link, use a same-kind owner anchor from stories or linkable-stories.
        edit: StoryReferenceChange,
    },
    /// Splice original blocks in an existing header/footer slot.
    EditStoryBlocks {
        /// Slot anchor from stories or `image_slot.anchor` from default-headers.
        anchor: String,
        edit: StoryBlockChange,
    },
    /// Edit native direct run formatting in one physical owner.
    EditRunFormat {
        /// Owner anchor from the format-runs or default-headers view.
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
