use rmcp::schemars::{self, JsonSchema};
use rocx::editor::{InlineAction, RunFormatField, RunToggle};
use serde::Deserialize;

use super::payloads::RunFormatInput;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunFormatChange {
    /// An issued run anchor from the format-runs view.
    pub(crate) anchor: String,
    pub(crate) edit: RunFormatAction,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum RunFormatAction {
    SetBold {
        value: bool,
    },
    SetToggle {
        toggle: Toggle,
        /// Set a direct value, or pass null to clear it.
        value: ToggleValue,
    },
    PatchFormat {
        /// Set only supplied direct fields. Font sizes are in points.
        set: Option<Box<RunFormatInput>>,
        /// Remove only these direct fields, restoring inheritance.
        #[serde(default)]
        clear: Vec<Field>,
    },
}

#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
pub(crate) enum ToggleValue {
    Set(bool),
    Clear(()),
}

macro_rules! choices {
    ($name:ident => $native:ident, $case:literal, $($variant:ident),+ $(,)?) => {
        #[derive(Deserialize, JsonSchema)]
        #[serde(rename_all = $case)]
        pub(crate) enum $name {
            $($variant),+
        }

        impl From<$name> for $native {
            fn from(value: $name) -> Self {
                match value {
                    $($name::$variant => Self::$variant),+
                }
            }
        }
    };
}

choices!(Toggle => RunToggle, "lowercase", Bold, Italic, Strike, Hidden);
choices!(
    Field => RunFormatField, "camelCase",
    Style, Font, FontEastAsia, FontComplexScript, Bold, BoldComplexScript, Italic,
    ItalicComplexScript, AllCaps, SmallCaps, Strike, SnapToGrid, Hidden, Color,
    Size, SizeComplexScript, Underline, VerticalAlignment, Rtl, ComplexScript,
    LanguageLatin, LanguageEastAsia, LanguageBidi,
);

impl From<RunFormatAction> for InlineAction {
    fn from(value: RunFormatAction) -> Self {
        match value {
            RunFormatAction::SetBold { value } => Self::SetBold(value),
            RunFormatAction::SetToggle { toggle, value } => Self::SetToggle(
                toggle.into(),
                match value {
                    ToggleValue::Set(value) => Some(value),
                    ToggleValue::Clear(()) => None,
                },
            ),
            RunFormatAction::PatchFormat { set, clear } => Self::PatchFormat {
                set: Box::new(set.map(|value| (*value).into()).unwrap_or_default()),
                clear: clear.into_iter().map(Into::into).collect(),
            },
        }
    }
}
