use rmcp::schemars::{self, JsonSchema};
use rocx::{
    ImageRun,
    editor::{DrawingProperty, InlineAction},
};
use serde::Deserialize;

use super::{
    blocks::Nullable,
    payloads::{InlineInput, resolve_component},
    run_format::RunFormatAction,
};

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(crate) enum BodyInlineChange {
    /// Insert a declaration immediately after an original inline.
    InsertAfter { inline: Box<InlineInput> },
    /// Replace an eligible original inline with a declaration.
    Replace { inline: Box<InlineInput> },
    /// Remove an eligible hyperlink and its displayed runs.
    RemoveHyperlink {},
    /// Set the hyperlink tooltip; null clears it.
    HyperlinkTooltip { value: Nullable<String> },
    /// Set the hyperlink history flag; null clears it.
    HyperlinkHistory { value: Nullable<bool> },
    /// Apply direct formatting to an eligible direct body run.
    Format { action: RunFormatAction },
    /// Remove an eligible drawing or fraction.
    RemoveObject {},
    /// Set the drawing description; null removes the stored attribute.
    DrawingDescription { value: Nullable<String> },
    /// Redirect this picture to different encoded image bytes, preserving its occurrence properties.
    ImageAsset { bytes: Vec<u8> },
    /// Copy one property group from a resolved declaration of the same drawing kind.
    DrawingProperty {
        property: DrawingGroup,
        inline: Box<InlineInput>,
    },
}

impl BodyInlineChange {
    pub(crate) fn resolve(self) -> Result<InlineAction, String> {
        Ok(match self {
            Self::InsertAfter { inline } => InlineAction::InsertAfter(inline.resolve()?),
            Self::Replace { inline } => InlineAction::Replace(inline.resolve()?),
            Self::RemoveHyperlink {} => InlineAction::RemoveHyperlink,
            Self::HyperlinkTooltip { value } => InlineAction::HyperlinkTooltip(value.into_option()),
            Self::HyperlinkHistory { value } => InlineAction::HyperlinkHistory(value.into_option()),
            Self::Format { action } => action.into(),
            Self::RemoveObject {} => InlineAction::RemoveObject,
            Self::DrawingDescription { value } => {
                InlineAction::SetDrawingDescription(value.into_option())
            }
            Self::ImageAsset { bytes } => InlineAction::SetImageAsset(
                resolve_component(ImageRun::new(bytes))?
                    .image()
                    .ok_or("expected resolved image")?
                    .clone(),
            ),
            Self::DrawingProperty { property, inline } => {
                InlineAction::PatchDrawing(property.into(), inline.resolve()?)
            }
        })
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DrawingGroup {
    Size,
    Placement,
    Crop,
    Transform,
    ShapeFill,
    ShapeGeometry,
    ShapeOutline,
    ShapeText,
    Accessibility,
    ChartSeries,
}

impl From<DrawingGroup> for DrawingProperty {
    fn from(value: DrawingGroup) -> Self {
        match value {
            DrawingGroup::Size => Self::Size,
            DrawingGroup::Placement => Self::Placement,
            DrawingGroup::Crop => Self::Crop,
            DrawingGroup::Transform => Self::Transform,
            DrawingGroup::ShapeFill => Self::ShapeFill,
            DrawingGroup::ShapeGeometry => Self::ShapeGeometry,
            DrawingGroup::ShapeOutline => Self::ShapeOutline,
            DrawingGroup::ShapeText => Self::ShapeText,
            DrawingGroup::Accessibility => Self::Accessibility,
            DrawingGroup::ChartSeries => Self::ChartSeries,
        }
    }
}
