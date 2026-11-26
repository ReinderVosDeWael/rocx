//! Strict authoring payloads shared by edit requests.

use rmcp::schemars::{self, JsonSchema};
use rocx::{
    Component, Paragraph, ResolvedBlock, ResolvedParagraph, Table, TableCell, TableProperties,
    TableRow,
};
use serde::Deserialize;

mod drawing;
mod format;
mod inline;
mod table;
#[cfg(test)]
mod tests;

use format::ParagraphFormatInput;
pub(crate) use format::RunFormatInput;
pub(crate) use inline::InlineInput;
pub(crate) use table::RichTableInput;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParagraphInput {
    /// Inline children in document order. An empty array creates an empty paragraph.
    inlines: Vec<InlineInput>,
    format: Option<ParagraphFormatInput>,
}

impl ParagraphInput {
    pub(crate) fn resolve(self) -> Result<ResolvedParagraph, String> {
        let children = self
            .inlines
            .into_iter()
            .map(|inline| inline.resolve().map(Into::into))
            .collect::<Result<_, _>>()?;
        let mut paragraph = Paragraph::from_children(children);
        if let Some(format) = self.format {
            paragraph = paragraph.with_format(rocx::ParagraphFormat::from(format));
        }
        resolve_component(paragraph)
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum BlockInput {
    Paragraph {
        paragraph: ParagraphInput,
    },
    Table {
        /// Nonempty rectangular array of plain-text cells.
        #[schemars(length(min = 1))]
        rows: Vec<Vec<String>>,
        /// Grid widths in twips, one per column.
        column_widths: Option<Vec<u32>>,
    },
    RichTable {
        table: Box<RichTableInput>,
    },
}

impl BlockInput {
    pub(crate) fn resolve(self) -> Result<ResolvedBlock, String> {
        match self {
            Self::RichTable { table } => table.resolve().map(ResolvedBlock::Table),
            Self::Paragraph { paragraph } => paragraph.resolve().map(ResolvedBlock::Paragraph),
            Self::Table {
                rows,
                column_widths,
            } => {
                let columns = rows.first().map_or(0, Vec::len);
                if columns == 0 || rows.iter().any(|row| row.len() != columns) {
                    return Err("rows must form a nonempty rectangular table".into());
                }
                let rows = rows
                    .into_iter()
                    .map(|row| {
                        let cells = row
                            .into_iter()
                            .map(|text| TableCell::new(vec![Paragraph::new(text).into()]).into())
                            .collect();
                        TableRow::new(cells).into()
                    })
                    .collect();
                resolve_component(Table::new(rows).with_properties(TableProperties {
                    column_widths,
                    ..TableProperties::default()
                }))
                .map(ResolvedBlock::Table)
            }
        }
    }
}

pub(crate) fn resolve_component<C: Component>(component: C) -> Result<C::Resolved, String> {
    futures_executor::block_on(component.resolve())
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "declaration produced no content".into())
}
