use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;

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
    /// Set an explicit reference to an inspected same-kind physical owner.
    Link { anchor: String },
    /// Remove an explicit reference, resuming inheritance without deleting the part.
    Remove {},
}
