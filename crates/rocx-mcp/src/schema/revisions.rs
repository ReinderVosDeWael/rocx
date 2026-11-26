use rmcp::schemars::{self, JsonSchema};
use rocx::TextRevisionAction;
use serde::Deserialize;

use super::entries::present;

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(crate) enum RevisionChange {
    /// Track plain text inserted after a direct body run.
    InsertTextAfter {
        text: String,
        author: String,
        /// UTC timestamp (YYYY-MM-DDTHH:MM:SSZ). Omit when not available.
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        date: Option<String>,
    },
    /// Track replacement of a direct body run as deletion and insertion.
    ReplaceText {
        text: String,
        author: String,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        date: Option<String>,
    },
    /// Track deletion of a direct body run.
    DeleteText {
        author: String,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        date: Option<String>,
    },
    /// Track a move after the second run supplied as paired_anchor.
    MoveText {
        /// Fresh Word move name.
        name: String,
        author: String,
        /// Required UTC timestamp (YYYY-MM-DDTHH:MM:SSZ).
        date: String,
    },
    /// Track direct bold formatting on a supported run.
    SetBold {
        bold: bool,
        author: String,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        date: Option<String>,
    },
    /// Track insertion of a plain paragraph before the target paragraph.
    InsertParagraphBefore {
        text: String,
        author: String,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        date: Option<String>,
    },
    /// Track deletion of the target paragraph, including its mark.
    DeleteParagraph {
        author: String,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        date: Option<String>,
    },
    /// Review an inline wrapper or run property change. Move review requires both wrappers.
    Review { action: Disposition },
    /// Review a paragraph mark and its matching inline wrapper together.
    ReviewParagraph { action: Disposition },
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(inline)]
pub(crate) enum Disposition {
    Accept,
    Reject,
}

impl From<Disposition> for TextRevisionAction {
    fn from(value: Disposition) -> Self {
        match value {
            Disposition::Accept => Self::Accept,
            Disposition::Reject => Self::Reject,
        }
    }
}
