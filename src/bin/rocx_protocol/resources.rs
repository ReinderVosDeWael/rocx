//! Strict JSON payloads for direct defaults, concrete lists and table regions.
use super::*;
use serde::Serialize;

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
            ..Default::default()
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListRef {
    pub id: u32,
    pub level: u8,
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
