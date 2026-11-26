use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;

use crate::schema::{HeaderMode, payloads::BlockInput};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(inline)]
pub(crate) struct StoryTextChange {
    /// Direct run candidate from this physical story.
    pub(crate) anchor: String,
    pub(crate) text: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(crate) enum StoryReferenceChange {
    /// Create a new physical part in a missing or inherited slot.
    Create {
        /// Plain paragraphs in source order. An empty list creates an empty paragraph.
        paragraphs: Vec<String>,
    },
    /// Create a new physical part using declared content blocks.
    CreateRich { blocks: Vec<BlockInput> },
    /// Set an explicit reference to an inspected same-kind physical owner.
    Link { anchor: String },
    /// Remove an explicit reference, resuming inheritance without deleting the part.
    Remove {},
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(inline)]
pub(crate) struct StoryBlockChange {
    pub(crate) mode: HeaderMode,
    /// Original zero-based block gap, counting opaque blocks as well.
    pub(crate) index: usize,
    /// Number of consecutive original blocks to remove at this gap.
    pub(crate) remove: usize,
    /// Blocks inserted at the original gap in supplied order.
    pub(crate) blocks: Vec<BlockInput>,
}
