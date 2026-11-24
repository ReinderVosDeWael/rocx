//! Strict JSON payloads for direct defaults, concrete lists and table regions.
use super::*;
use serde::Serialize;
mod level_format;
use level_format::{LabelFormat, NumberingAlignment, NumberingSuffix, ParagraphFormat};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Tab {
    position_twips: i32,
    alignment: String,
    leader: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum DefaultScalar {
    Flag(bool),
    Integer(i64),
    Text(String),
    Tabs(Vec<Tab>),
}
impl From<DefaultScalar> for DefaultValue {
    fn from(value: DefaultScalar) -> Self {
        match value {
            DefaultScalar::Flag(v) => Self::Bool(v),
            DefaultScalar::Integer(v) => Self::Integer(v),
            DefaultScalar::Text(v) => Self::Text(v),
            DefaultScalar::Tabs(tabs) => Self::Tabs(
                tabs.into_iter()
                    .map(|t| DefaultTab {
                        position_twips: t.position_twips,
                        alignment: t.alignment,
                        leader: t.leader,
                    })
                    .collect(),
            ),
        }
    }
}
impl From<DefaultValue> for DefaultScalar {
    fn from(value: DefaultValue) -> Self {
        match value {
            DefaultValue::Bool(v) => Self::Flag(v),
            DefaultValue::Integer(v) => Self::Integer(v),
            DefaultValue::Text(v) => Self::Text(v),
            DefaultValue::Tabs(tabs) => Self::Tabs(
                tabs.into_iter()
                    .map(|t| Tab {
                        position_twips: t.position_twips,
                        alignment: t.alignment,
                        leader: t.leader,
                    })
                    .collect(),
            ),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Level {
    level: u8,
    start: u32,
    format: String,
    text: String,
    is_legal: Option<bool>,
    restart: Option<u32>,
    suffix: Option<NumberingSuffix>,
    alignment: Option<NumberingAlignment>,
    paragraph_style: Option<String>,
    label_format: Option<LabelFormat>,
    paragraph_format: Option<ParagraphFormat>,
}
impl From<Level> for rocx::NumberingLevel {
    fn from(l: Level) -> Self {
        Self {
            level: l.level,
            start: l.start,
            format: l.format,
            text: l.text,
            is_legal: l.is_legal,
            restart: l.restart,
            suffix: l.suffix.map(Into::into),
            alignment: l.alignment.map(Into::into),
            paragraph_style: l.paragraph_style,
            label_format: l.label_format.map(Into::into),
            paragraph_format: l.paragraph_format.map(Into::into),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListRef {
    pub id: u32,
    pub level: u8,
}

impl From<ListRef> for rocx::NumberingReference {
    fn from(value: ListRef) -> Self {
        Self {
            id: value.id,
            level: value.level,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum NumberingSite {
    Story {
        part: String,
        block_index: usize,
        entry_id: Option<String>,
    },
    Cell {
        target: usize,
        row_index: usize,
        cell_index: usize,
        block_index: usize,
    },
}

impl NumberingSite {
    pub(super) fn resolve(&self, doc: &ImportedDocument) -> Result<StoryParagraphAnchor, String> {
        match self {
            Self::Story {
                part,
                block_index,
                entry_id,
            } => doc.story_paragraph_anchor(part, *block_index, entry_id.as_deref()),
            Self::Cell {
                target,
                row_index,
                cell_index,
                block_index,
            } => {
                let Some(BodyBlock::Table(table)) = doc.body().get(*target) else {
                    return Err("numbering cell target is not a direct body table".into());
                };
                doc.cell_paragraph_anchor(&table.anchor, *row_index, *cell_index, *block_index)
            }
        }
        .map_err(|error| error.to_string())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Concrete {
    id: u32,
    abstract_id: u32,
    #[serde(default)]
    start_overrides: Vec<Start>,
    #[serde(default)]
    level_overrides: Vec<Level>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Start {
    level: u8,
    start: u32,
}

impl From<Concrete> for rocx::NumberingDefinition {
    fn from(value: Concrete) -> Self {
        Self {
            id: value.id,
            abstract_id: Some(value.abstract_id),
            start_overrides: value
                .start_overrides
                .into_iter()
                .map(|o| rocx::NumberingStartOverride {
                    level: o.level,
                    start: o.start,
                })
                .collect(),
            level_overrides: value.level_overrides.into_iter().map(Into::into).collect(),
            ..Default::default()
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Region {
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    font: Option<String>,
    font_size: Option<f64>,
    color: Option<[u8; 3]>,
    background: Option<[u8; 3]>,
    alignment: Option<String>,
    spacing_before: Option<f64>,
    spacing_after: Option<f64>,
    bidi: Option<bool>,
}
impl Region {
    pub(super) fn resolve(self) -> Result<Box<rocx::TableSectionFormat>, String> {
        Ok(Box::new(rocx::TableSectionFormat {
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
            font: self.font,
            font_size: self.font_size,
            color: self.color,
            background: self.background,
            alignment: self.alignment.as_deref().map(alignment).transpose()?,
            spacing_before: self.spacing_before,
            spacing_after: self.spacing_after,
            bidi: self.bidi,
            ..Default::default()
        }))
    }
}
