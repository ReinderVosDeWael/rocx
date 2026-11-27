use rmcp::schemars::{self, JsonSchema};
use rocx::editor::InlineAction;
use serde::Deserialize;

use super::{
    blocks::Nullable,
    body_inlines::{BodyInlineChange, DrawingGroup},
    payloads::InlineInput,
    run_format::RunFormatAction,
};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EntryInlineChange {
    /// Original anchor from entry-inlines. Hyperlink child runs and chart or
    /// shape runs are usable only in a batch consisting entirely of format edits.
    pub(crate) anchor: String,
    pub(crate) edit: EntryInlineAction,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum EntryInlineAction {
    /// Insert a supported declaration immediately after an eligible entry inline.
    InsertAfter { inline: Box<InlineInput> },
    /// Replace an eligible entry inline with a supported declaration.
    Replace { inline: Box<InlineInput> },
    /// Apply direct formatting to an eligible entry run, including a direct
    /// hyperlink child when all edits in the batch are format edits.
    Format { action: RunFormatAction },
    /// Remove an eligible picture, retaining other occurrences.
    RemoveObject {},
    /// Set the picture description; null clears it.
    DrawingDescription { value: Nullable<String> },
    /// Redirect only this picture to different image bytes.
    ImageAsset { bytes: Vec<u8> },
    /// Copy one property group from an image declaration.
    DrawingProperty {
        property: PictureGroup,
        inline: Box<InlineInput>,
    },
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PictureGroup {
    Size,
    Placement,
    Crop,
    Transform,
    Accessibility,
}

impl EntryInlineAction {
    pub(crate) fn resolve(self) -> Result<InlineAction, String> {
        let change = match self {
            Self::InsertAfter { inline } => BodyInlineChange::InsertAfter { inline },
            Self::Replace { inline } => BodyInlineChange::Replace { inline },
            Self::Format { action } => BodyInlineChange::Format { action },
            Self::RemoveObject {} => BodyInlineChange::RemoveObject {},
            Self::DrawingDescription { value } => BodyInlineChange::DrawingDescription { value },
            Self::ImageAsset { bytes } => BodyInlineChange::ImageAsset { bytes },
            Self::DrawingProperty { property, inline } => {
                if !matches!(*inline, InlineInput::Image { .. }) {
                    return Err("entry picture patches require an image declaration".into());
                }
                BodyInlineChange::DrawingProperty {
                    property: match property {
                        PictureGroup::Size => DrawingGroup::Size,
                        PictureGroup::Placement => DrawingGroup::Placement,
                        PictureGroup::Crop => DrawingGroup::Crop,
                        PictureGroup::Transform => DrawingGroup::Transform,
                        PictureGroup::Accessibility => DrawingGroup::Accessibility,
                    },
                    inline,
                }
            }
        };
        change.resolve()
    }
}
