use rmcp::schemars::{self, JsonSchema};
use rocx::{
    ContentControlListItem, ContentControlLock, ContentControlProperties, ContentControlText,
};
use serde::Deserialize;

use super::{
    entries::present,
    payloads::{BlockInput, InlineInput},
};

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(crate) enum ControlChange {
    /// Wrap one site or an inclusive range of eligible sibling sites.
    Wrap {
        first: String,
        #[serde(default, deserialize_with = "present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        last: Option<String>,
        tag: String,
    },
    /// Change only the supplied tag, alias or single-line text.
    Patch {
        anchor: String,
        tag: Option<String>,
        alias: Option<String>,
        text: Option<String>,
    },
    /// Remove the wrapper, retaining its original payload.
    Unwrap { anchor: String },
    /// Replace all modeled properties; omitted metadata is cleared.
    Define {
        anchor: String,
        text: ControlDefinition,
        #[schemars(range(min = 0, max = 4_294_967_295u32))]
        id: Option<u32>,
        tag: Option<String>,
        alias: Option<String>,
        lock: Option<Lock>,
    },
    /// Select a literal drop-down value, optionally replacing the list.
    Choice {
        anchor: String,
        selected: String,
        items: Option<Vec<Choice>>,
    },
    ReplaceInlines {
        anchor: String,
        inlines: Vec<InlineInput>,
    },
    ReplaceBlocks {
        anchor: String,
        blocks: Vec<BlockInput>,
    },
}

impl ControlChange {
    pub(crate) fn scope(
        &mut self,
        mut resolve: impl FnMut(&mut String, bool) -> Result<(), String>,
    ) -> Result<(), String> {
        match self {
            Self::Wrap { first, last, .. } => {
                resolve(first, true)?;
                if let Some(last) = last {
                    resolve(last, true)?;
                }
                Ok(())
            }
            Self::Patch { anchor, .. }
            | Self::Unwrap { anchor }
            | Self::Define { anchor, .. }
            | Self::Choice { anchor, .. }
            | Self::ReplaceInlines { anchor, .. }
            | Self::ReplaceBlocks { anchor, .. } => resolve(anchor, false),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Choice {
    display_text: String,
    value: String,
}

impl From<Choice> for ContentControlListItem {
    fn from(choice: Choice) -> Self {
        Self {
            display_text: choice.display_text,
            value: choice.value,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum ControlDefinition {
    Rich {},
    Plain {
        multi_line: Option<bool>,
    },
    DropDown {
        items: Vec<Choice>,
        selected: String,
    },
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Lock {
    Unlocked,
    Control,
    Content,
    Both,
}

impl ControlDefinition {
    pub(crate) fn properties(
        self,
        id: Option<u32>,
        tag: Option<String>,
        alias: Option<String>,
        lock: Option<Lock>,
    ) -> ContentControlProperties {
        ContentControlProperties {
            text: match self {
                Self::Rich {} => ContentControlText::Rich,
                Self::Plain { multi_line } => ContentControlText::Plain { multi_line },
                Self::DropDown { items, selected } => ContentControlText::DropDown {
                    items: items.into_iter().map(Into::into).collect(),
                    selected,
                },
            },
            id,
            tag,
            alias,
            lock: lock.map(|lock| match lock {
                Lock::Unlocked => ContentControlLock::Unlocked,
                Lock::Control => ContentControlLock::ControlLocked,
                Lock::Content => ContentControlLock::ContentLocked,
                Lock::Both => ContentControlLock::ControlAndContentLocked,
            }),
        }
    }
}
