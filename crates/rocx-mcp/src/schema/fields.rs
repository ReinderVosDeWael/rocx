use rmcp::schemars::{self, JsonSchema};
use rocx::editor::{FieldAction, FieldPatch};
use serde::Deserialize;

use super::{blocks::Nullable, payloads::InlineInput};
use crate::diagnostics::ScopedEditError;

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(crate) enum FieldChange {
    /// Set the instruction without changing the cached display.
    Instruction { value: String },
    Dirty {
        /// Required; null removes the direct dirty flag.
        value: Nullable<bool>,
    },
    Locked {
        /// Required; null removes the direct lock flag.
        value: Nullable<bool>,
    },
    /// Replace the cached display without changing the instruction or flags.
    Cache { children: Vec<InlineInput> },
    /// Replace the entire original field. The inline must have kind "field".
    Replace { inline: Box<InlineInput> },
    /// Remove the entire original field.
    Remove {},
}

impl FieldChange {
    pub(crate) fn resolve(self) -> Result<FieldAction, ScopedEditError> {
        Ok(match self {
            Self::Instruction { value } => FieldAction::Patch(FieldPatch::Instruction(value)),
            Self::Dirty { value } => FieldAction::Patch(FieldPatch::Dirty(value.into_option())),
            Self::Locked { value } => FieldAction::Patch(FieldPatch::Locked(value.into_option())),
            Self::Cache { children } => FieldAction::Cache(
                children
                    .into_iter()
                    .map(InlineInput::resolve)
                    .collect::<Result<_, _>>()?,
            ),
            Self::Replace { inline } => FieldAction::Replace(Box::new(inline.resolve()?)),
            Self::Remove {} => FieldAction::Remove,
        })
    }
}
