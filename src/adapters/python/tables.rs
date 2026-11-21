//! Native conversion into resolved domain values.
use super::*;
pub(super) fn table(o: &Bound<'_, PyAny>) -> PyResult<Option<ResolvedTable>> {
    ResolvedTable::checked(
        list(&o.getattr("rows")?, |r| Ok(Some(tablerow(r)?)))?,
        field!(o, "column_widths", Option<Vec<u32>>),
        optional_enum(o, "layout", tablelayout)?,
        field!(o, "style", Option<String>),
        list(&o.getattr("borders")?, |v| Ok(Some(tableborder(v)?)))?,
        optional_record(o, "look", table_look)?,
        table_alignment(o)?,
    )
    .map_err(|d| rust_error(d.into()))
}
fn table_alignment(o: &Bound<'_, PyAny>) -> PyResult<Option<crate::TableAlignment>> {
    match field!(o, "alignment", Option<String>).as_deref() {
        None => Ok(None),
        Some("left") => Ok(Some(crate::TableAlignment::Left)),
        Some("center") => Ok(Some(crate::TableAlignment::Center)),
        Some("right") => Ok(Some(crate::TableAlignment::Right)),
        Some("start") => Ok(Some(crate::TableAlignment::Start)),
        Some("end") => Ok(Some(crate::TableAlignment::End)),
        _ => Err(rust_error(
            crate::Diagnostic::new(crate::Rule::InvalidContent, "unknown table alignment")
                .at("alignment")
                .into(),
        )),
    }
}
fn table_look(o: &Bound<'_, PyAny>) -> PyResult<crate::TableLook> {
    Ok(crate::TableLook {
        first_row: field!(o, "first_row", Option<bool>),
        last_row: field!(o, "last_row", Option<bool>),
        first_column: field!(o, "first_column", Option<bool>),
        last_column: field!(o, "last_column", Option<bool>),
        horizontal_banding: field!(o, "horizontal_banding", Option<bool>),
        vertical_banding: field!(o, "vertical_banding", Option<bool>),
    })
}
pub(super) fn tablerow(o: &Bound<'_, PyAny>) -> PyResult<ResolvedTableRow> {
    Ok(ResolvedTableRow {
        cells: list(&o.getattr("children")?, |c| Ok(Some(tablecell(c)?)))?,
        properties: crate::TableRowProperties {
            height: optional_record(o, "height", row_height)?,
            repeat_header: field!(o, "repeat_header", Option<bool>),
            cant_split: field!(o, "cant_split", Option<bool>),
            content_control: optional_record(o, "content_control", table_content_control)?,
        },
    })
}
fn table_content_control(o: &Bound<'_, PyAny>) -> PyResult<crate::ContentControlProperties> {
    if kind(o)? != "ContentControlMetadata" {
        return Err(PyTypeError::new_err(
            "row and cell controls require ContentControlMetadata",
        ));
    }
    super::content::content_control_metadata(o)
}
fn row_height(o: &Bound<'_, PyAny>) -> PyResult<crate::RowHeight> {
    let points = field!(o, "value", f64);
    match field!(o, "rule", String).as_str() {
        "exact" => Ok(crate::RowHeight::Exact(points)),
        "at_least" => Ok(crate::RowHeight::AtLeast(points)),
        _ => Err(rust_error(
            crate::Diagnostic::new(crate::Rule::InvalidContent, "unknown row height rule")
                .at("height.rule")
                .into(),
        )),
    }
}
pub(super) fn tablecell(o: &Bound<'_, PyAny>) -> PyResult<ResolvedTableCell> {
    let span = field!(o, "grid_span", Option<u32>).unwrap_or(1);
    let margins = optional_record(o, "margins", cell_margins)?;
    crate::TableCellProperties {
        grid_span: Some(span),
        margins: margins.clone(),
        ..Default::default()
    }
    .validate()
    .map_err(|error| rust_error(error.into()))?;
    Ok(ResolvedTableCell {
        blocks: blocks(&o.getattr("children")?)?,
        grid_span: span,
        vmerge: optional_enum(o, "vmerge", verticalmerge)?,
        borders: list(&o.getattr("borders")?, |v| Ok(Some(cellborder(v)?)))?,
        background_color: field!(o, "background_color", Option<[u8; 3]>),
        vertical_alignment: optional_enum(o, "vertical_alignment", verticalalignment)?,
        margins,
        text_direction: optional_record(o, "text_direction", cell_text_direction)?,
        content_control: optional_record(o, "content_control", table_content_control)?,
    })
}
fn cell_margins(o: &Bound<'_, PyAny>) -> PyResult<crate::CellMargins> {
    Ok(crate::CellMargins {
        top: field!(o, "top", Option<f64>),
        start: field!(o, "start", Option<f64>),
        bottom: field!(o, "bottom", Option<f64>),
        end: field!(o, "end", Option<f64>),
    })
}
fn cell_text_direction(o: &Bound<'_, PyAny>) -> PyResult<crate::CellTextDirection> {
    match o.extract::<String>()?.as_str() {
        "lrTb" => Ok(crate::CellTextDirection::LeftToRight),
        "tbRl" => Ok(crate::CellTextDirection::TopToBottomRightToLeft),
        "btLr" => Ok(crate::CellTextDirection::BottomToTopLeftToRight),
        "lrTbV" => Ok(crate::CellTextDirection::LeftToRightVertical),
        "tbRlV" => Ok(crate::CellTextDirection::TopToBottomRightToLeftVertical),
        "tbLrV" => Ok(crate::CellTextDirection::TopToBottomLeftToRightVertical),
        _ => Err(rust_error(
            crate::Diagnostic::new(crate::Rule::InvalidContent, "unknown cell text direction")
                .at("text_direction")
                .into(),
        )),
    }
}

use crate::{ResolvedTable, ResolvedTableCell, ResolvedTableRow};
