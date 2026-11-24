use super::*;
use rocx::{
    CharacterStyleDefinition, Hyperlink, HyperlinkTarget, ParagraphStyleDefinition, ResolvedBlock,
    ResolvedInline, ResolvedTable, RunFormat, StyleDefinition, TableStyleDefinition, TextRun,
};

pub(super) fn table(rows: Vec<Vec<String>>) -> Result<ResolvedTable, String> {
    text_table(rows, None)
}

fn text_table(
    rows: Vec<Vec<String>>,
    column_widths: Option<Vec<u32>>,
) -> Result<ResolvedTable, String> {
    let width = rows.first().map_or(0, Vec::len);
    if width == 0 || rows.iter().any(|r| r.len() != width) {
        return Err("rows must form a nonempty rectangular table".into());
    }
    TableSource {
        rows,
        column_widths,
        ..Default::default()
    }
    .resolve()
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum Block {
    Paragraph {
        text: String,
    },
    RichParagraph {
        inlines: Vec<Inline>,
    },
    Table {
        rows: Vec<Vec<String>>,
        column_widths: Option<Vec<u32>>,
    },
}
pub(super) fn resolve_blocks(blocks: Vec<Block>) -> Result<Vec<ResolvedBlock>, String> {
    blocks
        .into_iter()
        .map(|b| {
            Ok(match b {
                Block::Paragraph { text } => {
                    ResolvedBlock::Paragraph(resolve(Paragraph::new(text))?)
                }
                Block::RichParagraph { inlines } => {
                    ResolvedBlock::Paragraph(resolve(Paragraph::from_children(
                        inlines
                            .into_iter()
                            .map(|i| i.resolve().map(Into::into))
                            .collect::<Result<Vec<_>, _>>()?,
                    ))?)
                }
                Block::Table {
                    rows,
                    column_widths,
                } => ResolvedBlock::Table(text_table(rows, column_widths)?),
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
    },
    Hyperlink {
        url: String,
        text: String,
    },
    Image {
        bytes: Vec<u8>,
    },
}
impl Inline {
    pub(super) fn resolve(self) -> Result<ResolvedInline, String> {
        match self {
            Self::Text { text, bold, italic } => {
                resolve(TextRun::new(text).with_format(RunFormat {
                    bold,
                    italic,
                    ..Default::default()
                }))
            }
            Self::Image { bytes } => resolve(ImageRun::new(bytes)),
            Self::Hyperlink { url, text } => resolve(Hyperlink::new(
                HyperlinkTarget::external(url).map_err(|e| e.to_string())?,
                vec![TextRun::new(text).into()],
            )),
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
    RemoveTable {},
    ReplaceNode {
        row: Option<usize>,
        cell: Option<usize>,
        source: Box<TableSource>,
    },
    PatchProperty {
        row: Option<usize>,
        cell: Option<usize>,
        column: Option<usize>,
        field: String,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        source: Option<Box<TableSource>>,
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
            Self::RemoveTable {} => TableOperation::RemoveTable,
            Self::ReplaceNode { row, cell, source } => TableOperation::ReplaceNode {
                site: table_site(row, cell, None)?,
                source: Box::new(source.resolve()?),
            },
            Self::PatchProperty {
                row,
                cell,
                column,
                field,
                source,
            } => TableOperation::Property {
                site: table_site(row, cell, column)?,
                field,
                source: source
                    .map(|source| source.resolve())
                    .transpose()?
                    .map(Box::new),
            },
        })
    }
}

fn table_site(
    row: Option<usize>,
    cell: Option<usize>,
    column: Option<usize>,
) -> Result<TableSite, String> {
    match (row, cell, column) {
        (None, None, None) => Ok(TableSite::Table),
        (Some(r), None, None) => Ok(TableSite::Row(r)),
        (Some(r), Some(c), None) => Ok(TableSite::Cell(r, c)),
        (None, None, Some(c)) => Ok(TableSite::Column(c)),
        _ => Err("invalid table property coordinates".into()),
    }
}

// Donors are real declarations; CLI code never patches OOXML or bypasses core validation.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TableSource {
    rows: Vec<Vec<String>>,
    column_widths: Option<Vec<u32>>,
    layout: Option<String>,
    style: Option<String>,
    alignment: Option<String>,
    look: Option<Look>,
    #[serde(default)]
    borders: Vec<Border>,
    first_row: Option<RowSource>,
    first_cell: Option<CellSource>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Look {
    first_row: Option<bool>,
    last_row: Option<bool>,
    first_column: Option<bool>,
    last_column: Option<bool>,
    horizontal_banding: Option<bool>,
    vertical_banding: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Border {
    side: String,
    size: u32,
    color: [u8; 3],
    line: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RowSource {
    height: Option<Height>,
    repeat_header: Option<bool>,
    cant_split: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Height {
    points: f64,
    rule: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CellSource {
    grid_span: Option<u32>,
    vertical_merge: Option<String>,
    background_color: Option<[u8; 3]>,
    vertical_alignment: Option<String>,
    text_direction: Option<String>,
    margins: Option<Margins>,
    #[serde(default)]
    borders: Vec<Border>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Margins {
    top: Option<f64>,
    start: Option<f64>,
    bottom: Option<f64>,
    end: Option<f64>,
}

macro_rules! token {
    ($value:expr, $name:literal, $($wire:literal => $rust:path),+ $(,)?) => {
        match $value {
            $($wire => Ok($rust),)+
            _ => Err(concat!("unknown ", $name).to_owned()),
        }
    };
}

impl Border {
    fn line(&self) -> Result<rocx::BorderLine, String> {
        use rocx::BorderLine::*;
        token!(self.line.as_deref().unwrap_or("single"), "border line",
            "single" => Single, "dashed" => Dashed)
    }
    fn table(self) -> Result<rocx::TableBorder, String> {
        use rocx::BorderSide::*;
        Ok(rocx::TableBorder {
            side: token!(self.side.as_str(), "table border side",
                "top" => Top, "left" => Left, "bottom" => Bottom, "right" => Right,
                "insideH" => InsideH, "insideV" => InsideV)?,
            val: self.line()?,
            sz: self.size,
            color: self.color,
        })
    }
    fn cell(self) -> Result<rocx::CellBorder, String> {
        use rocx::CellBorderSide::*;
        Ok(rocx::CellBorder {
            side: token!(self.side.as_str(), "cell border side",
                "top" => Top, "bottom" => Bottom, "start" => Start, "end" => End)?,
            val: self.line()?,
            sz: self.size,
            color: self.color,
        })
    }
}

fn optional_token<T>(
    value: Option<String>,
    parse: impl FnOnce(&str) -> Result<T, String>,
) -> Result<Option<T>, String> {
    value.as_deref().map(parse).transpose()
}

impl TableSource {
    fn resolve(self) -> Result<ResolvedTable, String> {
        use rocx::{CellTextDirection, RowHeight, TableAlignment, TableLayout, VerticalMerge};
        if self.rows.is_empty() || self.rows.iter().any(Vec::is_empty) {
            return Err("source needs nonempty table rows and cells".into());
        }
        let properties = rocx::TableProperties {
            column_widths: self.column_widths,
            layout: optional_token(self.layout, |v| {
                token!(v, "table layout",
                "fixed" => TableLayout::Fixed, "autofit" => TableLayout::Autofit)
            })?,
            style: self.style,
            alignment: optional_token(self.alignment, |v| {
                token!(v, "table alignment",
                "left" => TableAlignment::Left, "center" => TableAlignment::Center,
                "right" => TableAlignment::Right, "start" => TableAlignment::Start,
                "end" => TableAlignment::End)
            })?,
            look: self.look.map(|v| rocx::TableLook {
                first_row: v.first_row,
                last_row: v.last_row,
                first_column: v.first_column,
                last_column: v.last_column,
                horizontal_banding: v.horizontal_banding,
                vertical_banding: v.vertical_banding,
            }),
            borders: self
                .borders
                .into_iter()
                .map(Border::table)
                .collect::<Result<_, _>>()?,
        };
        let mut row_props = if let Some(v) = self.first_row {
            rocx::TableRowProperties {
                height: v
                    .height
                    .map(|h| match h.rule.as_str() {
                        "exact" => Ok(RowHeight::Exact(h.points)),
                        "atLeast" => Ok(RowHeight::AtLeast(h.points)),
                        _ => Err("unknown row height rule"),
                    })
                    .transpose()?,
                repeat_header: v.repeat_header,
                cant_split: v.cant_split,
                ..Default::default()
            }
        } else {
            Default::default()
        };
        let mut cell_props = if let Some(v) = self.first_cell {
            use CellTextDirection::*;
            rocx::TableCellProperties {
                grid_span: v.grid_span,
                vmerge: optional_token(v.vertical_merge, |x| {
                    token!(x, "vertical merge",
                    "restart" => VerticalMerge::Restart, "continue" => VerticalMerge::Continue)
                })?,
                background_color: v.background_color,
                vertical_alignment: optional_token(v.vertical_alignment, |x| {
                    token!(x, "vertical alignment",
                    "top" => VerticalAlignment::Top, "center" => VerticalAlignment::Center,
                    "bottom" => VerticalAlignment::Bottom, "both" => VerticalAlignment::Both)
                })?,
                text_direction: optional_token(v.text_direction, |x| {
                    token!(x, "cell text direction",
                    "lrTb" => LeftToRight, "tbRl" => TopToBottomRightToLeft,
                    "btLr" => BottomToTopLeftToRight, "lrTbV" => LeftToRightVertical,
                    "tbRlV" => TopToBottomRightToLeftVertical,
                    "tbLrV" => TopToBottomLeftToRightVertical)
                })?,
                margins: v.margins.map(|m| rocx::CellMargins {
                    top: m.top,
                    start: m.start,
                    bottom: m.bottom,
                    end: m.end,
                }),
                borders: v
                    .borders
                    .into_iter()
                    .map(Border::cell)
                    .collect::<Result<_, _>>()?,
                ..Default::default()
            }
        } else {
            Default::default()
        };
        let rows = self
            .rows
            .into_iter()
            .map(|row| {
                let cells = row
                    .into_iter()
                    .map(|s| {
                        TableCell::new(vec![Paragraph::new(s).into()])
                            .with_properties(std::mem::take(&mut cell_props))
                            .into()
                    })
                    .collect();
                TableRow::new(cells)
                    .with_properties(std::mem::take(&mut row_props))
                    .into()
            })
            .collect();
        resolve(Table::new(rows).with_properties(properties))
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
