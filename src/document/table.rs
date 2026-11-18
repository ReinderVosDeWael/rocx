//! Resolved table grid values.
use super::*;
/// Cloneable table properties, independent of owned child producers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableProperties {
    pub column_widths: Option<Vec<u32>>,
    pub layout: Option<TableLayout>,
    pub style: Option<String>,
    pub borders: Vec<TableBorder>,
}
/// Cloneable cell properties; child content is resolved separately.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableCellProperties {
    pub grid_span: Option<u32>,
    pub vmerge: Option<VerticalMerge>,
    pub borders: Vec<CellBorder>,
    pub background_color: Option<[u8; 3]>,
    pub vertical_alignment: Option<VerticalAlignment>,
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
impl ResolvedTable {
    pub fn column_widths(&self) -> Option<&[u32]> {
        self.column_widths.as_deref()
    }
    pub fn layout(&self) -> Option<TableLayout> {
        self.layout
    }
    pub fn style(&self) -> Option<&str> {
        self.style.as_deref()
    }
    pub fn borders(&self) -> &[TableBorder] {
        &self.borders
    }
    pub fn columns(&self) -> usize {
        self.columns
    }
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
    pub fn grid_span(&self) -> u32 {
        self.grid_span
    }
    pub fn vmerge(&self) -> Option<VerticalMerge> {
        self.vmerge
    }
    pub fn borders(&self) -> &[CellBorder] {
        &self.borders
    }
    pub fn background_color(&self) -> Option<[u8; 3]> {
        self.background_color
    }
    pub fn vertical_alignment(&self) -> Option<VerticalAlignment> {
        self.vertical_alignment
    }
    pub fn blocks(&self) -> &[ResolvedBlock] {
        &self.blocks
    }
}

impl ResolvedTable {
    /// Normalize the resolved grid before either frontend compiles it (P1 §17.4).
    pub(crate) fn checked(
        rows: Vec<ResolvedTableRow>,
        column_widths: Option<Vec<u32>>,
        layout: Option<TableLayout>,
        style: Option<String>,
        borders: Vec<TableBorder>,
    ) -> Result<Option<Self>, Diagnostic> {
        if rows.is_empty() {
            return Ok(None);
        }
        let columns = rows
            .iter()
            .map(|row| row.cells.iter().map(|c| c.grid_span as usize).sum())
            .max()
            .unwrap_or(0);
        if columns > 16384 {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "table exceeds application column limit (16384)",
            ));
        }
        if rows.iter().flat_map(|r| &r.cells).any(|c| c.grid_span == 0) {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "grid_span must be positive",
            ));
        }
        if layout != Some(TableLayout::Autofit)
            && let Some(widths) = &column_widths
        {
            if widths.len() != columns {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    format!(
                        "column_widths length ({}) must match number of columns ({columns})",
                        widths.len()
                    ),
                )
                .at("column_widths"));
            }
            if widths.contains(&0) {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "column_widths must be positive",
                )
                .at("column_widths"));
            }
        }
        Ok(Some(Self {
            rows,
            column_widths,
            layout,
            style,
            borders,
            columns,
        }))
    }
}
