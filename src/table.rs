//! Table declarations, resolution, and resolved grid content.
//!
//! References: ECMA-376 Part 1 (2016), §17.4 (tables), §17.4.17
//! (`gridSpan`), and §17.4.84 (`vMerge`).
//! [Official Part 1](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip).
use crate::*;
use futures_util::future::{BoxFuture, try_join_all};
#[derive(Debug, Default)]
pub struct Table {
    pub rows: Value<Vec<Value<TableRow>>>,
    pub column_widths: Option<Vec<u32>>,
    pub layout: Option<TableLayout>,
    pub style: Option<String>,
    pub borders: Vec<TableBorder>,
    pub condition: Condition,
}
#[derive(Debug, Default)]
pub struct TableRow {
    pub children: Value<Vec<Value<TableCell>>>,
    pub condition: Condition,
}
#[derive(Debug, Default)]
pub struct TableCell {
    pub children: Value<BlockChildren>,
    pub grid_span: Option<u32>,
    pub vmerge: Option<VerticalMerge>,
    pub borders: Vec<CellBorder>,
    pub background_color: Option<[u8; 3]>,
    pub vertical_alignment: Option<VerticalAlignment>,
    pub condition: Condition,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedBlock {
    Paragraph(ResolvedParagraph),
    Table(ResolvedTable),
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTable {
    pub(crate) rows: Vec<ResolvedTableRow>,
    pub(crate) column_widths: Option<Vec<u32>>,
    pub(crate) layout: Option<TableLayout>,
    pub(crate) style: Option<String>,
    pub(crate) borders: Vec<TableBorder>,
    pub(crate) columns: usize,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTableRow {
    pub(crate) cells: Vec<ResolvedTableCell>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTableCell {
    pub(crate) blocks: Vec<ResolvedBlock>,
    pub(crate) grid_span: u32,
    pub(crate) vmerge: Option<VerticalMerge>,
    pub(crate) borders: Vec<CellBorder>,
    pub(crate) background_color: Option<[u8; 3]>,
    pub(crate) vertical_alignment: Option<VerticalAlignment>,
}
impl Table {
    pub fn new(rows: Vec<Value<TableRow>>) -> Self {
        Self {
            rows: rows.into(),
            ..Self::default()
        }
    }
    pub fn when(mut self, c: impl Into<Condition>) -> Self {
        self.condition = c.into();
        self
    }
    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedTable>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let rows = self
            .rows
            .resolve()
            .await
            .map_err(|e| e.at(format!("{path}.rows")))?;
        let rows: Vec<_> = try_join_all(rows.into_iter().enumerate().map(|(i, row)| {
            let p = format!("{path}.rows[{i}]");
            async move {
                row.resolve()
                    .await
                    .map_err(|e| e.at(&p))?
                    .resolve_at(p)
                    .await
            }
        }))
        .await?
        .into_iter()
        .flatten()
        .collect();
        if rows.is_empty() {
            return Ok(None);
        }
        let columns = rows
            .iter()
            .map(|r| r.cells.iter().map(|c| c.grid_span as usize).sum())
            .max()
            .unwrap_or(0);
        if columns > 16384 {
            return Err(Error::Validation("table has too many columns".into()).at(&path));
        }
        if self.layout != Some(TableLayout::Autofit)
            && let Some(widths) = &self.column_widths
            && (widths.len() != columns || widths.contains(&0))
        {
            return Err(Error::Validation(if widths.len() != columns {
                format!(
                    "column_widths length ({}) must match number of columns ({columns})",
                    widths.len()
                )
            } else {
                "column_widths must be positive".into()
            })
            .at(format!("{path}.column_widths")));
        }
        Ok(Some(ResolvedTable {
            rows,
            column_widths: self.column_widths,
            layout: self.layout,
            style: self.style,
            borders: self.borders,
            columns,
        }))
    }
}
impl TableRow {
    pub fn new(children: Vec<Value<TableCell>>) -> Self {
        Self {
            children: children.into(),
            ..Self::default()
        }
    }
    pub fn when(mut self, c: impl Into<Condition>) -> Self {
        self.condition = c.into();
        self
    }
    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedTableRow>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let children = self
            .children
            .resolve()
            .await
            .map_err(|e| e.at(format!("{path}.children")))?;
        let cells = try_join_all(children.into_iter().enumerate().map(|(i, cell)| {
            let p = format!("{path}.children[{i}]");
            async move {
                cell.resolve()
                    .await
                    .map_err(|e| e.at(&p))?
                    .resolve_at(p)
                    .await
            }
        }))
        .await?
        .into_iter()
        .flatten()
        .collect();
        Ok(Some(ResolvedTableRow { cells }))
    }
}
impl TableCell {
    pub fn new(children: BlockChildren) -> Self {
        Self {
            children: children.into(),
            ..Self::default()
        }
    }
    pub fn when(mut self, c: impl Into<Condition>) -> Self {
        self.condition = c.into();
        self
    }
    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedTableCell>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let span = self.grid_span.unwrap_or(1);
        if span == 0 {
            return Err(Error::Validation("grid_span must be positive".into())
                .at(format!("{path}.grid_span")));
        }
        let children = self
            .children
            .resolve()
            .await
            .map_err(|e| e.at(format!("{path}.children")))?;
        let blocks = resolve_blocks(children, format!("{path}.children")).await?;
        Ok(Some(ResolvedTableCell {
            blocks,
            grid_span: span,
            vmerge: self.vmerge,
            borders: self.borders,
            background_color: self.background_color,
            vertical_alignment: self.vertical_alignment,
        }))
    }
}
impl From<Table> for Block {
    fn from(t: Table) -> Self {
        Self::Table(t)
    }
}
impl From<Table> for Value<Block> {
    fn from(t: Table) -> Self {
        Self::Ready(Block::Table(t))
    }
}
pub(crate) fn resolve_blocks(
    children: BlockChildren,
    path: String,
) -> BoxFuture<'static, Result<Vec<ResolvedBlock>>> {
    Box::pin(async move {
        Ok(try_join_all(children.into_iter().enumerate().map(|(i, b)| {
            let path = format!("{path}[{i}]");
            async move {
                match b.resolve().await.map_err(|e| e.at(&path))? {
                    Block::Paragraph(p) => {
                        Ok::<_, Error>(p.resolve_at(path).await?.map(ResolvedBlock::Paragraph))
                    }
                    Block::Table(t) => Ok(t.resolve_at(path).await?.map(ResolvedBlock::Table)),
                }
            }
        }))
        .await?
        .into_iter()
        .flatten()
        .collect())
    })
}
impl ResolvedTable {
    pub fn rows(&self) -> &[ResolvedTableRow] {
        &self.rows
    }
}
impl ResolvedTableRow {
    pub fn cells(&self) -> &[ResolvedTableCell] {
        &self.cells
    }
}
impl ResolvedTableCell {
    pub fn blocks(&self) -> &[ResolvedBlock] {
        &self.blocks
    }
}
