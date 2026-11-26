use rmcp::schemars::{self, JsonSchema, Schema, SchemaGenerator, json_schema};
use rocx::editor::{
    AbstractStart, NumberingEdit, ParagraphAnchor, ResourceEdit, StoryParagraphAnchor,
};
use rocx::{
    NumberingAlignment, NumberingDefinition, NumberingLevel, NumberingReference,
    NumberingStartOverride, NumberingSuffix,
};
use serde::Deserialize;

use super::{
    blocks::Nullable,
    payloads::{ParagraphFormatInput, RunFormatInput},
};

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(crate) enum NumberingChange {
    AddAbstract {
        abstract_id: u32,
        levels: Vec<Level>,
    },
    ReplaceAbstract {
        abstract_id: u32,
        levels: Vec<Level>,
    },
    RemoveAbstract {
        abstract_id: u32,
    },
    /// Set or clear a shared abstract level's start.
    AbstractStart {
        abstract_id: u32,
        level: LevelIndex,
        value: Nullable<u32>,
    },
    /// Replace or remove one shared abstract level; null removes it.
    AbstractLevel {
        abstract_id: u32,
        level: LevelIndex,
        value: Nullable<Box<Level>>,
    },
    AddInstance {
        num_id: u32,
        abstract_id: u32,
    },
    RelinkInstance {
        num_id: u32,
        abstract_id: u32,
    },
    ReplaceInstance {
        definition: Instance,
    },
    RemoveInstance {
        num_id: u32,
    },
    /// Set or clear one concrete start override.
    InstanceStart {
        num_id: u32,
        level: LevelIndex,
        value: Nullable<u32>,
    },
    /// Set or clear one concrete level override, independent of the start override.
    InstanceLevel {
        num_id: u32,
        level: LevelIndex,
        value: Nullable<Box<Level>>,
    },
    /// Set or clear a direct body paragraph's stored list assignment.
    Paragraph {
        anchor: String,
        value: Nullable<ListReference>,
    },
    /// Set or clear a physical paragraph from the numbering-paragraphs view.
    PhysicalParagraph {
        anchor: String,
        value: Nullable<ListReference>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PhysicalNumberingScope {
    Cells,
    Stories,
    Entries,
}

impl NumberingChange {
    pub(crate) fn resolve(
        self,
        paragraph: impl FnOnce(&str) -> Result<ParagraphAnchor, String>,
        physical: impl FnOnce(&str) -> Result<StoryParagraphAnchor, String>,
    ) -> Result<ResourceEdit, String> {
        let levels = |levels: Vec<Level>| levels.into_iter().map(Into::into).collect();
        let level_value =
            |value: Nullable<Box<Level>>| value.into_option().map(|v| Box::new((*v).into()));
        let edit = match self {
            Self::AddAbstract {
                abstract_id,
                levels: input,
            } => NumberingEdit::AddAbstract {
                abstract_id,
                levels: levels(input),
            },
            Self::ReplaceAbstract {
                abstract_id,
                levels: input,
            } => NumberingEdit::ReplaceAbstract {
                abstract_id,
                levels: levels(input),
            },
            Self::RemoveAbstract { abstract_id } => NumberingEdit::RemoveAbstract { abstract_id },
            Self::AbstractStart {
                abstract_id,
                level,
                value,
            } => {
                return Ok(ResourceEdit::AbstractStart(AbstractStart {
                    abstract_id,
                    level: level.0,
                    start: value.into_option(),
                }));
            }
            Self::AbstractLevel {
                abstract_id,
                level,
                value,
            } => NumberingEdit::AbstractLevel {
                abstract_id,
                level: level.0,
                value: level_value(value),
            },
            Self::AddInstance {
                num_id,
                abstract_id,
            } => NumberingEdit::AddInstance {
                num_id,
                abstract_id,
            },
            Self::RelinkInstance {
                num_id,
                abstract_id,
            } => NumberingEdit::RelinkInstance {
                num_id,
                abstract_id,
            },
            Self::ReplaceInstance { definition } => {
                NumberingEdit::ReplaceInstance(Box::new(definition.into()))
            }
            Self::RemoveInstance { num_id } => NumberingEdit::RemoveInstance { num_id },
            Self::InstanceStart {
                num_id,
                level,
                value,
            } => NumberingEdit::InstanceStart {
                num_id,
                level: level.0,
                start: value.into_option(),
            },
            Self::InstanceLevel {
                num_id,
                level,
                value,
            } => NumberingEdit::InstanceLevel {
                num_id,
                level: level.0,
                value: level_value(value),
            },
            Self::Paragraph { anchor, value } => NumberingEdit::Paragraph {
                target: paragraph(&anchor)?,
                value: value.into_option().map(Into::into),
            },
            Self::PhysicalParagraph { anchor, value } => NumberingEdit::StoryParagraph {
                target: physical(&anchor)?,
                value: value.into_option().map(Into::into),
            },
        };
        Ok(ResourceEdit::Numbering(edit))
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(try_from = "u8")]
#[schemars(inline, schema_with = "level_index_schema")]
pub(crate) struct LevelIndex(u8);

fn level_index_schema(_: &mut SchemaGenerator) -> Schema {
    json_schema!({"type": "integer", "minimum": 0, "maximum": 8})
}

impl TryFrom<u8> for LevelIndex {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= 8 {
            Ok(Self(value))
        } else {
            Err("numbering level must be in 0..=8")
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListReference {
    id: u32,
    level: LevelIndex,
}

impl From<ListReference> for NumberingReference {
    fn from(value: ListReference) -> Self {
        Self {
            id: value.id,
            level: value.level.0,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Level {
    level: LevelIndex,
    start: u32,
    /// Native OOXML number format, e.g. decimal, lowerLetter, or bullet.
    format: String,
    text: String,
    is_legal: Option<bool>,
    restart: Option<u32>,
    suffix: Option<Suffix>,
    alignment: Option<Alignment>,
    paragraph_style: Option<String>,
    label_format: Option<Box<RunFormatInput>>,
    paragraph_format: Option<Box<ParagraphFormatInput>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum Suffix {
    Tab,
    Space,
    Nothing,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum Alignment {
    Left,
    Center,
    Right,
    Start,
    End,
}

impl From<Level> for NumberingLevel {
    fn from(value: Level) -> Self {
        Self {
            level: value.level.0,
            start: value.start,
            format: value.format,
            text: value.text,
            is_legal: value.is_legal,
            restart: value.restart,
            paragraph_style: value.paragraph_style,
            suffix: value.suffix.map(|s| match s {
                Suffix::Tab => NumberingSuffix::Tab,
                Suffix::Space => NumberingSuffix::Space,
                Suffix::Nothing => NumberingSuffix::Nothing,
            }),
            alignment: value.alignment.map(|a| match a {
                Alignment::Left => NumberingAlignment::Left,
                Alignment::Center => NumberingAlignment::Center,
                Alignment::Right => NumberingAlignment::Right,
                Alignment::Start => NumberingAlignment::Start,
                Alignment::End => NumberingAlignment::End,
            }),
            label_format: value.label_format.map(|f| (*f).into()),
            paragraph_format: value.paragraph_format.map(|f| (*f).into()),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Instance {
    id: u32,
    abstract_id: u32,
    #[serde(default)]
    start_overrides: Vec<Start>,
    #[serde(default)]
    level_overrides: Vec<Level>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Start {
    level: LevelIndex,
    start: u32,
}

impl From<Instance> for NumberingDefinition {
    fn from(value: Instance) -> Self {
        Self {
            id: value.id,
            abstract_id: Some(value.abstract_id),
            start_overrides: value
                .start_overrides
                .into_iter()
                .map(|s| NumberingStartOverride {
                    level: s.level.0,
                    start: s.start,
                })
                .collect(),
            level_overrides: value.level_overrides.into_iter().map(Into::into).collect(),
            ..Default::default()
        }
    }
}
