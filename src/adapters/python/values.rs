//! Native conversion into resolved domain values.
use super::*;
pub(super) fn alignment(o: &Bound<'_, PyAny>) -> PyResult<Alignment> {
    if let Ok(v) = o.extract::<i64>() {
        return match v {
            0 => Ok(Alignment::Left),
            1 => Ok(Alignment::Center),
            2 => Ok(Alignment::Right),
            3 => Ok(Alignment::Justify),
            4 => Ok(Alignment::Distribute),
            5 => Ok(Alignment::JustifyMedium),
            7 => Ok(Alignment::JustifyHigh),
            8 => Ok(Alignment::JustifyLow),
            9 => Ok(Alignment::ThaiJustify),
            _ => Err(PyValueError::new_err("Invalid paragraph alignment")),
        };
    }
    match o.extract::<String>()?.as_str() {
        "left" => Ok(Alignment::Left),
        "center" => Ok(Alignment::Center),
        "right" => Ok(Alignment::Right),
        "both" | "justify" => Ok(Alignment::Justify),
        _ => Err(PyValueError::new_err("Invalid paragraph alignment")),
    }
}

pub(super) fn breaktype(o: &Bound<'_, PyAny>) -> PyResult<BreakType> {
    match o.extract::<String>()?.as_str() {
        "line" => Ok(BreakType::Line),
        "textWrapping" => Ok(BreakType::TextWrapping),
        "page" => Ok(BreakType::Page),
        "column" => Ok(BreakType::Column),
        _ => Err(PyValueError::new_err("Invalid BreakType value")),
    }
}
pub(super) fn orientation(o: &Bound<'_, PyAny>) -> PyResult<Orientation> {
    match o.extract::<String>()?.as_str() {
        "portrait" => Ok(Orientation::Portrait),
        "landscape" => Ok(Orientation::Landscape),
        _ => Err(PyValueError::new_err("Invalid Orientation value")),
    }
}
pub(super) fn headerfootertype(o: &Bound<'_, PyAny>) -> PyResult<HeaderFooterType> {
    match o.extract::<String>()?.as_str() {
        "default" => Ok(HeaderFooterType::Default),
        "first" => Ok(HeaderFooterType::First),
        "even" => Ok(HeaderFooterType::Even),
        _ => Err(PyValueError::new_err("Invalid HeaderFooterType value")),
    }
}
pub(super) fn verticalalignment(o: &Bound<'_, PyAny>) -> PyResult<VerticalAlignment> {
    match o.extract::<String>()?.as_str() {
        "top" => Ok(VerticalAlignment::Top),
        "center" => Ok(VerticalAlignment::Center),
        "bottom" => Ok(VerticalAlignment::Bottom),
        "both" => Ok(VerticalAlignment::Both),
        _ => Err(PyValueError::new_err("Invalid VerticalAlignment value")),
    }
}
pub(super) fn sectiontype(o: &Bound<'_, PyAny>) -> PyResult<SectionType> {
    match o.extract::<String>()?.as_str() {
        "nextPage" => Ok(SectionType::NextPage),
        "nextColumn" => Ok(SectionType::NextColumn),
        "continuous" => Ok(SectionType::Continuous),
        "evenPage" => Ok(SectionType::EvenPage),
        "oddPage" => Ok(SectionType::OddPage),
        _ => Err(PyValueError::new_err("Invalid SectionType value")),
    }
}
pub(super) fn verticalmerge(o: &Bound<'_, PyAny>) -> PyResult<VerticalMerge> {
    match o.extract::<String>()?.as_str() {
        "restart" => Ok(VerticalMerge::Restart),
        "continue" => Ok(VerticalMerge::Continue),
        _ => Err(PyValueError::new_err("Invalid VerticalMerge value")),
    }
}
pub(super) fn tablelayout(o: &Bound<'_, PyAny>) -> PyResult<TableLayout> {
    match o.extract::<String>()?.as_str() {
        "fixed" => Ok(TableLayout::Fixed),
        "autofit" => Ok(TableLayout::Autofit),
        _ => Err(PyValueError::new_err("Invalid TableLayout value")),
    }
}
pub(super) fn borderside(o: &Bound<'_, PyAny>) -> PyResult<BorderSide> {
    match o.extract::<String>()?.as_str() {
        "top" => Ok(BorderSide::Top),
        "left" => Ok(BorderSide::Left),
        "bottom" => Ok(BorderSide::Bottom),
        "right" => Ok(BorderSide::Right),
        "insideH" => Ok(BorderSide::InsideH),
        "insideV" => Ok(BorderSide::InsideV),
        _ => Err(PyValueError::new_err("Invalid BorderSide value")),
    }
}
pub(super) fn cellborderside(o: &Bound<'_, PyAny>) -> PyResult<CellBorderSide> {
    match o.extract::<String>()?.as_str() {
        "top" => Ok(CellBorderSide::Top),
        "bottom" => Ok(CellBorderSide::Bottom),
        "start" => Ok(CellBorderSide::Start),
        "end" => Ok(CellBorderSide::End),
        _ => Err(PyValueError::new_err("Invalid CellBorderSide value")),
    }
}
pub(super) fn borderline(o: &Bound<'_, PyAny>) -> PyResult<BorderLine> {
    match o.extract::<String>()?.as_str() {
        "single" => Ok(BorderLine::Single),
        "dashed" => Ok(BorderLine::Dashed),
        _ => Err(PyValueError::new_err("Invalid BorderLine value")),
    }
}
pub(super) fn pagesize(o: &Bound<'_, PyAny>) -> PyResult<PageSize> {
    let d = o;
    Ok(PageSize {
        width: dict_field!(d, "width", i64),
        height: dict_field!(d, "height", i64),
    })
}
pub(super) fn pagemargins(o: &Bound<'_, PyAny>) -> PyResult<PageMargins> {
    let d = o;
    Ok(PageMargins {
        top: dict_field!(d, "top", i64),
        bottom: dict_field!(d, "bottom", i64),
        left: dict_field!(d, "left", i64),
        right: dict_field!(d, "right", i64),
    })
}
pub(super) fn pagenumbering(o: &Bound<'_, PyAny>) -> PyResult<PageNumbering> {
    let d = o;
    Ok(PageNumbering {
        start: dict_field!(d, "start", u32),
        format: dict_field!(d, "format", String),
    })
}
pub(super) fn columns(o: &Bound<'_, PyAny>) -> PyResult<Columns> {
    let d = o;
    Ok(Columns {
        count: dict_field!(d, "count", u32),
        space: dict_field!(d, "space", u32),
        separator: dict_field!(d, "separator", bool),
    })
}
pub(super) fn imagetransformation(o: &Bound<'_, PyAny>) -> PyResult<ImageTransformation> {
    let d = o;
    Ok(ImageTransformation {
        width: dict_field!(d, "width", f64),
        height: dict_field!(d, "height", f64),
    })
}
pub(super) fn alttext(o: &Bound<'_, PyAny>) -> PyResult<AltText> {
    let d = o;
    Ok(AltText {
        title: dict_field!(d, "title", String),
        description: dict_field!(d, "description", String),
        name: dict_field!(d, "name", String),
    })
}
pub(super) fn paragraphstyledefinition(o: &Bound<'_, PyAny>) -> PyResult<ParagraphStyleDefinition> {
    Ok(ParagraphStyleDefinition {
        name: field!(o, "name", String),
        base_style: field!(o, "base_style", Option<String>),
        next_paragraph_style: field!(o, "next_paragraph_style", Option<String>),
        bold: field!(o, "bold", Option<bool>),
        italic: field!(o, "italic", Option<bool>),
        underline: field!(o, "underline", Option<bool>),
        font: field!(o, "font", Option<String>),
        font_size: field!(o, "font_size", Option<f64>),
        color: field!(o, "color", Option<[u8; 3]>),
        alignment: optional_enum(o, "alignment", alignment)?,
        spacing_before: field!(o, "spacing_before", Option<f64>),
        spacing_after: field!(o, "spacing_after", Option<f64>),
        line_spacing: field!(o, "line_spacing", Option<f64>),
        left_indent: field!(o, "left_indent", Option<f64>),
        right_indent: field!(o, "right_indent", Option<f64>),
        first_line_indent: field!(o, "first_line_indent", Option<f64>),
        keep_together: field!(o, "keep_together", Option<bool>),
        keep_with_next: field!(o, "keep_with_next", Option<bool>),
        page_break_before: field!(o, "page_break_before", Option<bool>),
        widow_control: field!(o, "widow_control", Option<bool>),
    })
}
pub(super) fn tablesectionformat(o: &Bound<'_, PyAny>) -> PyResult<TableSectionFormat> {
    Ok(TableSectionFormat {
        bold: field!(o, "bold", Option<bool>),
        italic: field!(o, "italic", Option<bool>),
        underline: field!(o, "underline", Option<bool>),
        font: field!(o, "font", Option<String>),
        font_size: field!(o, "font_size", Option<f64>),
        color: field!(o, "color", Option<[u8; 3]>),
        background: field!(o, "background", Option<[u8; 3]>),
        alignment: optional_enum(o, "alignment", alignment)?,
        spacing_before: field!(o, "spacing_before", Option<f64>),
        spacing_after: field!(o, "spacing_after", Option<f64>),
    })
}
pub(super) fn tableborder(o: &Bound<'_, PyAny>) -> PyResult<TableBorder> {
    Ok(TableBorder {
        side: borderside(&o.getattr("side")?)?,
        sz: field!(o, "sz", u32),
        color: field!(o, "color", [u8; 3]),
        val: borderline(&o.getattr("val")?)?,
    })
}
pub(super) fn cellborder(o: &Bound<'_, PyAny>) -> PyResult<CellBorder> {
    Ok(CellBorder {
        side: cellborderside(&o.getattr("side")?)?,
        sz: field!(o, "sz", u32),
        color: field!(o, "color", [u8; 3]),
        val: borderline(&o.getattr("val")?)?,
    })
}
pub(super) fn tablestyledefinition(o: &Bound<'_, PyAny>) -> PyResult<TableStyleDefinition> {
    Ok(TableStyleDefinition {
        name: field!(o, "name", String),
        base_style: field!(o, "base_style", Option<String>),
        whole_table: optional_record(o, "whole_table", tablesectionformat)?,
        first_row: optional_record(o, "first_row", tablesectionformat)?,
        last_row: optional_record(o, "last_row", tablesectionformat)?,
        first_column: optional_record(o, "first_column", tablesectionformat)?,
        last_column: optional_record(o, "last_column", tablesectionformat)?,
        banding_1_row: optional_record(o, "banding_1_row", tablesectionformat)?,
        banding_2_row: optional_record(o, "banding_2_row", tablesectionformat)?,
        banding_1_column: optional_record(o, "banding_1_column", tablesectionformat)?,
        banding_2_column: optional_record(o, "banding_2_column", tablesectionformat)?,
        top_left_cell: optional_record(o, "top_left_cell", tablesectionformat)?,
        top_right_cell: optional_record(o, "top_right_cell", tablesectionformat)?,
        bottom_left_cell: optional_record(o, "bottom_left_cell", tablesectionformat)?,
        bottom_right_cell: optional_record(o, "bottom_right_cell", tablesectionformat)?,
    })
}
pub(super) fn sectionproperties(o: &Bound<'_, PyAny>) -> PyResult<SectionProperties> {
    Ok(SectionProperties {
        page_size: optional_record(o, "page_size", pagesize)?,
        page_orientation: optional_enum(o, "page_orientation", orientation)?,
        page_margins: optional_record(o, "page_margins", pagemargins)?,
        page_numbering: optional_record(o, "page_numbering", pagenumbering)?,
        columns: optional_record(o, "columns", columns)?,
        vertical_align: optional_enum(o, "vertical_align", verticalalignment)?,
        title_page: field!(o, "title_page", Option<bool>),
        r#type: optional_enum(o, "type", sectiontype)?,
    })
}

pub(super) fn underline(o: &Bound<'_, PyAny>) -> PyResult<Underline> {
    if let Ok(value) = o.extract::<bool>() {
        return Ok(value.into());
    }
    match o.extract::<String>()?.as_str() {
        "none" => Ok(Underline::None),
        "single" => Ok(Underline::Single),
        "double" => Ok(Underline::Double),
        "dotted" => Ok(Underline::Dotted),
        "dash" => Ok(Underline::Dash),
        "wave" => Ok(Underline::Wave),
        "words" => Ok(Underline::Words),
        _ => Err(PyValueError::new_err("invalid underline style")),
    }
}
pub(super) fn color(o: &Bound<'_, PyAny>) -> PyResult<Color> {
    if let Ok(rgb) = o.extract::<[u8; 3]>() {
        return Ok(Color::Rgb(rgb));
    }
    if o.extract::<String>().is_ok_and(|s| s == "auto") {
        return Ok(Color::Auto);
    }
    let theme = o.call_method1("get", ("theme",))?.extract::<String>()?;
    let name = match theme.as_str() {
        "dark1" => ThemeColor::Dark1,
        "light1" => ThemeColor::Light1,
        "dark2" => ThemeColor::Dark2,
        "light2" => ThemeColor::Light2,
        "accent1" => ThemeColor::Accent1,
        "accent2" => ThemeColor::Accent2,
        "accent3" => ThemeColor::Accent3,
        "accent4" => ThemeColor::Accent4,
        "accent5" => ThemeColor::Accent5,
        "accent6" => ThemeColor::Accent6,
        "hyperlink" => ThemeColor::Hyperlink,
        "followedHyperlink" => ThemeColor::FollowedHyperlink,
        "background1" => ThemeColor::Background1,
        "text1" => ThemeColor::Text1,
        "background2" => ThemeColor::Background2,
        "text2" => ThemeColor::Text2,
        _ => return Err(PyValueError::new_err("invalid theme color")),
    };
    Ok(Color::Theme {
        name,
        tint: dict_field!(o, "tint", u8),
        shade: dict_field!(o, "shade", u8),
    })
}

use crate::{
    Alignment, AltText, BorderLine, BorderSide, BreakType, CellBorder, CellBorderSide, Color,
    Columns, HeaderFooterType, ImageTransformation, Orientation, PageMargins, PageNumbering,
    PageSize, ParagraphStyleDefinition, SectionProperties, SectionType, TableBorder, TableLayout,
    TableSectionFormat, TableStyleDefinition, ThemeColor, Underline, VerticalAlignment,
    VerticalMerge,
};

pub(super) fn numbering_reference(o: &Bound<'_, PyAny>) -> PyResult<crate::NumberingReference> {
    Ok(crate::NumberingReference {
        id: field!(o, "id", u32),
        level: field!(o, "level", u8),
    })
}
pub(super) fn numbering_definition(o: &Bound<'_, PyAny>) -> PyResult<crate::NumberingDefinition> {
    Ok(crate::NumberingDefinition {
        id: field!(o, "id", u32),
        levels: list(&o.getattr("levels")?, |level| {
            Ok(Some(crate::NumberingLevel {
                level: field!(level, "level", u8),
                start: field!(level, "start", u32),
                format: field!(level, "format", String),
                text: field!(level, "text", String),
            }))
        })?,
    })
}
