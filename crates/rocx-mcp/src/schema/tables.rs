use rmcp::schemars::{self, JsonSchema};
use rocx::{
    TableLayout,
    editor::{TableOperation, TableSite},
};
use serde::Deserialize;

use super::{
    blocks::Nullable,
    payloads::{BlockInput, RichTableInput},
};
use crate::diagnostics::ScopedEditError;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(inline)]
pub(crate) struct PhysicalTableChange {
    /// Table anchor from the physical-tables view, not an owner or cell anchor.
    pub(crate) anchor: String,
    pub(crate) edit: PhysicalTableEdit,
}

/// Coordinates are zero-based and refer to the original table. Insertion is
/// before the supplied coordinate; the current count appends.
#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum PhysicalTableEdit {
    Layout {
        /// Direct table layout; null clears it.
        value: Nullable<PhysicalLayout>,
    },
    InsertRow {
        row: usize,
    },
    RemoveRow {
        row: usize,
    },
    InsertCell {
        row: usize,
        cell: usize,
    },
    RemoveCell {
        row: usize,
        cell: usize,
    },
    InsertColumn {
        column: usize,
    },
    RemoveColumn {
        column: usize,
    },
    ReplaceCell {
        row: usize,
        cell: usize,
        /// Replacement blocks; an empty array leaves a required empty paragraph.
        blocks: Vec<BlockInput>,
    },
    ClearProperty {
        site: PhysicalTableSite,
        /// Native field key from the corresponding properties map, or "width" for a grid column.
        field: String,
    },
    SetProperty {
        site: PhysicalTableSite,
        /// Native field key from the corresponding properties map, or "width" for a grid column.
        field: String,
        /// Donor declaration. The core copies only the selected direct property,
        /// so the donor's rows may have no cells.
        source: Box<RichTableInput>,
    },
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PhysicalLayout {
    Fixed,
    Autofit,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum PhysicalTableSite {
    Table {},
    Row { row: usize },
    Cell { row: usize, cell: usize },
    Column { column: usize },
}

#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
pub(crate) enum BodyTableEdit {
    Whole(WholeTableEdit),
    Physical(PhysicalTableEdit),
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum WholeTableEdit {
    /// Replace a direct body table, or a physical row or cell in one.
    ReplaceNode {
        site: ReplaceTableSite,
        /// Match the original grid for a row or cell replacement.
        source: Box<RichTableInput>,
    },
    /// Remove a direct body table without changing the other source coordinates.
    RemoveTable {},
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum ReplaceTableSite {
    Table {},
    Row { row: usize },
    Cell { row: usize, cell: usize },
}

impl BodyTableEdit {
    pub(crate) fn resolve(self) -> Result<TableOperation, ScopedEditError> {
        Ok(match self {
            Self::Physical(edit) => edit.resolve()?,
            Self::Whole(WholeTableEdit::ReplaceNode { site, source }) => {
                TableOperation::ReplaceNode {
                    site: match site {
                        ReplaceTableSite::Table {} => TableSite::Table,
                        ReplaceTableSite::Row { row } => TableSite::Row(row),
                        ReplaceTableSite::Cell { row, cell } => TableSite::Cell(row, cell),
                    },
                    source: Box::new(source.resolve()?),
                }
            }
            Self::Whole(WholeTableEdit::RemoveTable {}) => TableOperation::RemoveTable,
        })
    }
}

impl From<PhysicalTableSite> for TableSite {
    fn from(site: PhysicalTableSite) -> Self {
        match site {
            PhysicalTableSite::Table {} => Self::Table,
            PhysicalTableSite::Row { row } => Self::Row(row),
            PhysicalTableSite::Cell { row, cell } => Self::Cell(row, cell),
            PhysicalTableSite::Column { column } => Self::Column(column),
        }
    }
}

impl PhysicalTableEdit {
    pub(crate) fn resolve(self) -> Result<TableOperation, ScopedEditError> {
        Ok(match self {
            Self::Layout { value } => {
                TableOperation::Layout(value.into_option().map(|layout| match layout {
                    PhysicalLayout::Fixed => TableLayout::Fixed,
                    PhysicalLayout::Autofit => TableLayout::Autofit,
                }))
            }
            Self::InsertRow { row } => TableOperation::InsertRow(row),
            Self::RemoveRow { row } => TableOperation::RemoveRow(row),
            Self::InsertCell { row, cell } => TableOperation::InsertCell { row, cell },
            Self::RemoveCell { row, cell } => TableOperation::RemoveCell { row, cell },
            Self::InsertColumn { column } => TableOperation::InsertColumn(column),
            Self::RemoveColumn { column } => TableOperation::RemoveColumn(column),
            Self::ReplaceCell { row, cell, blocks } => TableOperation::ReplaceCell {
                row,
                cell,
                blocks: blocks
                    .into_iter()
                    .map(BlockInput::resolve)
                    .collect::<Result<_, _>>()?,
            },
            Self::ClearProperty { site, field } => TableOperation::Property {
                site: site.into(),
                field,
                source: None,
            },
            Self::SetProperty {
                site,
                field,
                source,
            } => TableOperation::Property {
                site: site.into(),
                field,
                source: Some(Box::new(source.resolve_donor()?)),
            },
        })
    }
}
