use super::*;
use rocx::{
    CharacterStyleDefinition, Hyperlink, HyperlinkTarget, ParagraphStyleDefinition, ResolvedBlock,
    ResolvedInline, ResolvedTable, RunFormat, StyleDefinition, TableStyleDefinition, TextRun,
};

pub(super) fn table(rows: Vec<Vec<String>>) -> Result<ResolvedTable, String> {
    let width = rows.first().map_or(0, Vec::len);
    if width == 0 || rows.iter().any(|r| r.len() != width) {
        return Err("rows must form a nonempty rectangular table".into());
    }
    let rows = rows
        .into_iter()
        .map(|row| {
            TableRow::new(
                row.into_iter()
                    .map(|s| TableCell::new(vec![Paragraph::new(s).into()]).into())
                    .collect::<Vec<_>>(),
            )
            .into()
        })
        .collect::<Vec<_>>();
    resolve(Table::new(rows))
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum Block {
    Paragraph { text: String },
    Table { rows: Vec<Vec<String>> },
}
pub(super) fn resolve_blocks(blocks: Vec<Block>) -> Result<Vec<ResolvedBlock>, String> {
    blocks
        .into_iter()
        .map(|b| {
            Ok(match b {
                Block::Paragraph { text } => {
                    ResolvedBlock::Paragraph(resolve(Paragraph::new(text))?)
                }
                Block::Table { rows } => ResolvedBlock::Table(table(rows)?),
            })
        })
        .collect()
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum Inline {
    Text {
        text: String,
        bold: Option<bool>,
        italic: Option<bool>,
        style: Option<String>,
        size: Option<f64>,
    },
    Hyperlink {
        url: String,
        text: String,
    },
    Image {
        bytes: Vec<u8>,
    },
    BookmarkLink {
        name: String,
        text: String,
    },
    Fraction {
        numerator: String,
        denominator: String,
    },
    ColumnChart {
        width: f64,
        height: f64,
        categories: Vec<String>,
        values: Vec<f64>,
    },
    Field {
        instruction: Instruction,
        #[serde(default)]
        children: Vec<Inline>,
        #[serde(default)]
        complex: bool,
        dirty: Option<bool>,
        locked: Option<bool>,
    },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum Instruction {
    Page {},
    NumPages {},
    Reference { name: String },
    Raw { text: String },
    TableOfContents {},
}

impl Inline {
    pub(super) fn resolve(self) -> Result<ResolvedInline, String> {
        match self {
            Self::Text {
                text,
                bold,
                italic,
                style,
                size,
            } => resolve(TextRun::new(text).with_format(RunFormat {
                bold,
                italic,
                style,
                size,
                ..Default::default()
            })),
            Self::Image { bytes } => resolve(ImageRun::new(bytes)),
            Self::Hyperlink { url, text } => resolve(Hyperlink::new(
                HyperlinkTarget::external(url).map_err(|e| e.to_string())?,
                vec![TextRun::new(text).into()],
            )),
            Self::BookmarkLink { name, text } => resolve(Hyperlink::new(
                HyperlinkTarget::bookmark(
                    rocx::BookmarkName::new(name).map_err(|e| e.to_string())?,
                ),
                vec![TextRun::new(text).into()],
            )),
            Self::Fraction {
                numerator,
                denominator,
            } => resolve(rocx::InlineFraction::new(numerator, denominator)),
            Self::ColumnChart {
                width,
                height,
                categories,
                values,
            } => resolve(rocx::ColumnChartRun::new(rocx::ColumnChartProperties {
                width,
                height,
                categories,
                values,
            })),
            Self::Field {
                instruction,
                children,
                complex,
                dirty,
                locked,
            } => {
                use rocx::FieldInstruction as F;
                let instruction = match instruction {
                    Instruction::Page {} => F::Page,
                    Instruction::NumPages {} => F::NumPages,
                    Instruction::TableOfContents {} => F::TableOfContents,
                    Instruction::Reference { name } => {
                        F::Reference(rocx::BookmarkName::new(name).map_err(|e| e.to_string())?)
                    }
                    Instruction::Raw { text } => F::raw(text).map_err(|e| e.to_string())?,
                };
                Ok(ResolvedInline::Field(
                    rocx::ResolvedField::new(
                        instruction,
                        super::rich::resolve_inlines(children)?,
                        rocx::FieldProperties {
                            form: if complex {
                                rocx::FieldForm::Complex
                            } else {
                                rocx::FieldForm::Simple
                            },
                            dirty,
                            locked,
                        },
                    )
                    .map_err(|e| e.to_string())?,
                ))
            }
        }
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum Geometry {
    Layout {
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<String>,
    },
    InsertRow {
        index: usize,
    },
    RemoveRow {
        index: usize,
    },
    InsertColumn {
        index: usize,
    },
    RemoveColumn {
        index: usize,
    },
    InsertCell {
        row: usize,
        cell: usize,
    },
    RemoveCell {
        row: usize,
        cell: usize,
    },
    ReplaceCell {
        row: usize,
        cell: usize,
        blocks: Vec<Block>,
    },
}
impl Geometry {
    pub(super) fn resolve(self) -> Result<TableOperation, String> {
        Ok(match self {
            Self::Layout { value } => TableOperation::Layout(match value.as_deref() {
                None => None,
                Some("fixed") => Some(rocx::TableLayout::Fixed),
                Some("autofit") => Some(rocx::TableLayout::Autofit),
                _ => return Err("unknown table layout".into()),
            }),
            Self::InsertRow { index } => TableOperation::InsertRow(index),
            Self::RemoveRow { index } => TableOperation::RemoveRow(index),
            Self::InsertColumn { index } => TableOperation::InsertColumn(index),
            Self::RemoveColumn { index } => TableOperation::RemoveColumn(index),
            Self::InsertCell { row, cell } => TableOperation::InsertCell { row, cell },
            Self::RemoveCell { row, cell } => TableOperation::RemoveCell { row, cell },
            Self::ReplaceCell { row, cell, blocks } => TableOperation::ReplaceCell {
                row,
                cell,
                blocks: resolve_blocks(blocks)?,
            },
        })
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum Style {
    Numbering {
        name: String,
        numbering_id: u32,
    },
    Paragraph {
        name: String,
        base_style: Option<String>,
        bold: Option<bool>,
        italic: Option<bool>,
    },
    Character {
        name: String,
        base_style: Option<String>,
        bold: Option<bool>,
        italic: Option<bool>,
    },
    Table {
        name: String,
        base_style: Option<String>,
    },
}
impl From<Style> for StyleDefinition {
    fn from(value: Style) -> Self {
        match value {
            Style::Numbering { name, numbering_id } => {
                rocx::NumberingStyleDefinition { name, numbering_id }.into()
            }
            Style::Paragraph {
                name,
                base_style,
                bold,
                italic,
            } => ParagraphStyleDefinition {
                name,
                base_style,
                bold,
                italic,
                ..Default::default()
            }
            .into(),
            Style::Character {
                name,
                base_style,
                bold,
                italic,
            } => CharacterStyleDefinition {
                name,
                base_style,
                format: RunFormat {
                    bold,
                    italic,
                    ..Default::default()
                },
                ..Default::default()
            }
            .into(),
            Style::Table { name, base_style } => TableStyleDefinition {
                name,
                base_style,
                ..Default::default()
            }
            .into(),
        }
    }
}
