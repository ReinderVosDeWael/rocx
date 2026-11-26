use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;

use super::{blocks::Nullable, payloads::InlineInput, run_format::RunFormatAction};

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
}
