//! Wire shapes for rich operations; all target and preservation checks stay in the core.
use super::*;
use rocx::{
    ContentControlListItem, ContentControlLock, ContentControlProperties, ContentControlText,
    ResolvedInline, RunFormat,
};

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RichFormat {
    style: Option<String>,
    font: Option<String>,
    size: Option<f64>,
    bold: Option<bool>,
    italic: Option<bool>,
    strike: Option<bool>,
    all_caps: Option<bool>,
    small_caps: Option<bool>,
}

impl From<RichFormat> for RunFormat {
    fn from(f: RichFormat) -> Self {
        Self {
            style: f.style,
            font: f.font,
            size: f.size,
            bold: f.bold,
            italic: f.italic,
            strike: f.strike,
            all_caps: f.all_caps,
            small_caps: f.small_caps,
            ..Default::default()
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum RichInline {
    Format {
        #[serde(default)]
        set: RichFormat,
        #[serde(default)]
        clear: Vec<String>,
    },
    DrawingDescription {
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<String>,
    },
    DrawingProperty {
        property: String,
        inline: Inline,
    },
    ImageAsset {
        bytes: Vec<u8>,
    },
    RemoveObject {},
    HyperlinkTooltip {
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<String>,
    },
    HyperlinkHistory {
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<bool>,
    },
    RemoveHyperlink {},
}

impl RichInline {
    pub(super) fn resolve(self) -> Result<InlineAction, String> {
        Ok(match self {
            Self::Format { set, clear } => InlineAction::PatchFormat {
                set: Box::new(set.into()),
                clear: clear
                    .iter()
                    .map(|s| {
                        RunFormatField::parse(s).ok_or_else(|| format!("unknown run field {s}"))
                    })
                    .collect::<Result<_, _>>()?,
            },
            Self::DrawingDescription { value } => InlineAction::SetDrawingDescription(value),
            Self::DrawingProperty { property, inline } => InlineAction::PatchDrawing(
                DrawingProperty::parse(&property).ok_or("unknown drawing property")?,
                inline.resolve()?,
            ),
            Self::ImageAsset { bytes } => InlineAction::SetImageAsset(
                resolve(ImageRun::new(bytes))?
                    .image()
                    .ok_or("expected image")?
                    .clone(),
            ),
            Self::RemoveObject {} => InlineAction::RemoveObject,
            Self::HyperlinkTooltip { value } => InlineAction::HyperlinkTooltip(value),
            Self::HyperlinkHistory { value } => InlineAction::HyperlinkHistory(value),
            Self::RemoveHyperlink {} => InlineAction::RemoveHyperlink,
        })
    }
}

pub(super) fn resolve_inlines(children: Vec<Inline>) -> Result<Vec<ResolvedInline>, String> {
    children.into_iter().map(Inline::resolve).collect()
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum FieldOperation {
    Instruction {
        value: String,
    },
    Dirty {
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<bool>,
    },
    Locked {
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<bool>,
    },
    Cache {
        children: Vec<Inline>,
    },
    Replace {
        inline: Inline,
    },
    Remove {},
}

impl FieldOperation {
    pub(super) fn resolve(self) -> Result<FieldAction, String> {
        Ok(match self {
            Self::Instruction { value } => FieldAction::Patch(FieldPatch::Instruction(value)),
            Self::Dirty { value } => FieldAction::Patch(FieldPatch::Dirty(value)),
            Self::Locked { value } => FieldAction::Patch(FieldPatch::Locked(value)),
            Self::Cache { children } => FieldAction::Cache(resolve_inlines(children)?),
            Self::Replace { inline } => FieldAction::Replace(Box::new(inline.resolve()?)),
            Self::Remove {} => FieldAction::Remove,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Choice {
    display_text: String,
    value: String,
}

fn choices(items: Vec<Choice>) -> Vec<ContentControlListItem> {
    items
        .into_iter()
        .map(|c| ContentControlListItem {
            display_text: c.display_text,
            value: c.value,
        })
        .collect()
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Lock {
    Unlocked,
    Control,
    Content,
    Both,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum WrapperOperation {
    Wrap {
        first: usize,
        last: Option<usize>,
        tag: String,
    },
    Unwrap {
        target: usize,
    },
    Define {
        target: usize,
        text: ControlDefinition,
        id: Option<u32>,
        tag: Option<String>,
        alias: Option<String>,
        lock: Option<Lock>,
    },
    Choice {
        target: usize,
        selected: String,
        items: Option<Vec<Choice>>,
    },
    ReplaceInlines {
        target: usize,
        inlines: Vec<Inline>,
    },
    ReplaceBlocks {
        target: usize,
        blocks: Vec<Block>,
    },
}

impl WrapperOperation {
    pub(super) fn resolve(self, doc: &ImportedDocument) -> Result<WrapperEdit, String> {
        let target = |i: usize| {
            doc.controls()
                .get(i)
                .map(|c| c.anchor.clone())
                .ok_or("unknown control target")
        };
        let site = |i: usize| {
            doc.control_sites()
                .get(i)
                .cloned()
                .ok_or("unknown control site")
        };
        Ok(match self {
            Self::Wrap { first, last, tag } => WrapperEdit::Wrap {
                first: site(first)?,
                last: last.map(site).transpose()?,
                tag,
            },
            Self::Unwrap { target: i } => WrapperEdit::Unwrap(target(i)?),
            Self::Define {
                target: i,
                text,
                id,
                tag,
                alias,
                lock,
            } => WrapperEdit::Define(
                target(i)?,
                ContentControlProperties {
                    text: match text {
                        ControlDefinition::Rich {} => ContentControlText::Rich,
                        ControlDefinition::Plain { multi_line } => {
                            ContentControlText::Plain { multi_line }
                        }
                        ControlDefinition::DropDown { items, selected } => {
                            ContentControlText::DropDown {
                                items: choices(items),
                                selected,
                            }
                        }
                    },
                    id,
                    tag,
                    alias,
                    lock: lock.map(|l| match l {
                        Lock::Unlocked => ContentControlLock::Unlocked,
                        Lock::Control => ContentControlLock::ControlLocked,
                        Lock::Content => ContentControlLock::ContentLocked,
                        Lock::Both => ContentControlLock::ControlAndContentLocked,
                    }),
                },
            ),
            Self::Choice {
                target: i,
                selected,
                items,
            } => WrapperEdit::Choice {
                target: target(i)?,
                selected,
                items: items.map(choices),
            },
            Self::ReplaceInlines { target: i, inlines } => {
                WrapperEdit::ReplaceInlines(target(i)?, resolve_inlines(inlines)?)
            }
            Self::ReplaceBlocks { target: i, blocks } => {
                WrapperEdit::ReplaceBlocks(target(i)?, resolve_blocks(blocks)?)
            }
        })
    }
}
