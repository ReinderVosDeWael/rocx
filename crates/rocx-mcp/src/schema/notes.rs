use rmcp::schemars::{self, JsonSchema};
use rocx::editor::NoteReferenceEdit;
use serde::Deserialize;

const MAX_ID: u32 = u32::MAX;

#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(inline)]
pub(crate) enum NoteKind {
    Footnote,
    Endnote,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(crate) enum NoteChange {
    /// Insert a reference after an insertion candidate and create its definition.
    Create {
        /// New unused unsigned note ID.
        #[schemars(range(min = 0, max = MAX_ID))]
        id: u32,
        /// Plain text of the new note.
        text: String,
    },
    /// Insert a reference after an insertion candidate to an unused ordinary definition.
    Attach {
        /// Unsigned numeric value of the definition ID from the entries view.
        #[schemars(range(min = 0, max = MAX_ID))]
        id: u32,
    },
    /// Remove the sole reference without deleting the definition.
    Detach {},
    /// Remove the sole reference and delete its discardable definition.
    Delete {},
    /// Point the reference at an unused ordinary definition, keeping the old definition.
    Retarget {
        /// Unsigned numeric value of the definition ID from the entries view.
        #[schemars(range(min = 0, max = MAX_ID))]
        id: u32,
    },
    /// Change the ID of both the reference and its definition.
    Renumber {
        /// New unused unsigned note ID.
        #[schemars(range(min = 0, max = MAX_ID))]
        id: u32,
    },
}

impl From<NoteChange> for NoteReferenceEdit {
    fn from(edit: NoteChange) -> Self {
        match edit {
            NoteChange::Create { id, text } => Self::Create { id, text },
            NoteChange::Attach { id } => Self::Attach(id),
            NoteChange::Detach {} => Self::Detach,
            NoteChange::Delete {} => Self::Delete,
            NoteChange::Retarget { id } => Self::Retarget(id),
            NoteChange::Renumber { id } => Self::Renumber(id),
        }
    }
}
