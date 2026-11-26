use rmcp::schemars::{self, JsonSchema};
use rocx::editor::EntryEdit;
use serde::{Deserialize, Deserializer};

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
