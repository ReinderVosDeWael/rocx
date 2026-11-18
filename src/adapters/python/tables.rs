//! Native conversion into resolved domain values.
use super::*;
pub(super) fn table(o: &Bound<'_, PyAny>) -> PyResult<Option<ResolvedTable>> {
    ResolvedTable::checked(
        list(&o.getattr("rows")?, |r| Ok(Some(tablerow(r)?)))?,
        field!(o, "column_widths", Option<Vec<u32>>),
        optional_enum(o, "layout", tablelayout)?,
        field!(o, "style", Option<String>),
        list(&o.getattr("borders")?, |v| Ok(Some(tableborder(v)?)))?,
    )
    .map_err(|d| rust_error(d.into()))
}
pub(super) fn tablerow(o: &Bound<'_, PyAny>) -> PyResult<ResolvedTableRow> {
    Ok(ResolvedTableRow {
        cells: list(&o.getattr("children")?, |c| Ok(Some(tablecell(c)?)))?,
    })
}
pub(super) fn tablecell(o: &Bound<'_, PyAny>) -> PyResult<ResolvedTableCell> {
    let span = field!(o, "grid_span", Option<u32>).unwrap_or(1);
    if span == 0 {
        return Err(PyValueError::new_err("grid_span must be positive"));
    }
    Ok(ResolvedTableCell {
        blocks: blocks(&o.getattr("children")?)?,
        grid_span: span,
        vmerge: optional_enum(o, "vmerge", verticalmerge)?,
        borders: list(&o.getattr("borders")?, |v| Ok(Some(cellborder(v)?)))?,
        background_color: field!(o, "background_color", Option<[u8; 3]>),
        vertical_alignment: optional_enum(o, "vertical_alignment", verticalalignment)?,
    })
}

use crate::{ResolvedTable, ResolvedTableCell, ResolvedTableRow};
