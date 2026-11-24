//! Native conversion into resolved domain values.
use super::*;
use crate::ParagraphFormat;
pub(super) fn run_format(o: &Bound<'_, PyAny>) -> PyResult<RunFormat> {
    Ok(RunFormat {
        style: None,
        bold: field!(o, "bold", Option<bool>),
        italic: field!(o, "italic", Option<bool>),
        underline: optional_record(o, "underline", underline)?,
        font: field!(o, "font", Option<String>),
        size: field!(o, "size", Option<f64>),
        color: optional_record(o, "color", color)?,
        superscript: field!(o, "superscript", Option<bool>),
        subscript: field!(o, "subscript", Option<bool>),
        strike: field!(o, "strike", Option<bool>),
        all_caps: field!(o, "all_caps", Option<bool>),
        small_caps: field!(o, "small_caps", Option<bool>),
        snap_to_grid: field!(o, "snap_to_grid", Option<bool>),
        language: optional_record(o, "language", run_language)?,
        bold_complex_script: field!(o, "bold_complex_script", Option<bool>),
        italic_complex_script: field!(o, "italic_complex_script", Option<bool>),
        size_complex_script: field!(o, "size_complex_script", Option<f64>),
        rtl: field!(o, "rtl", Option<bool>),
        complex_script: field!(o, "complex_script", Option<bool>),
        font_east_asia: field!(o, "font_east_asia", Option<String>),
        font_complex_script: field!(o, "font_complex_script", Option<String>),
    })
}
pub(super) fn character_style(o: &Bound<'_, PyAny>) -> PyResult<crate::CharacterStyleDefinition> {
    Ok(crate::CharacterStyleDefinition {
        name: field!(o, "name", String),
        base_style: field!(o, "base_style", Option<String>),
        format: run_format(o)?,
        linked_style: field!(o, "linked_style", Option<String>),
    })
}
fn note_numbering(o: &Bound<'_, PyAny>) -> PyResult<crate::NoteNumbering> {
    let restart = match field!(o, "restart", Option<String>).as_deref() {
        None => None,
        Some("continuous") => Some(crate::NoteNumberRestart::Continuous),
        Some("eachSect") => Some(crate::NoteNumberRestart::EachSection),
        Some("eachPage") => Some(crate::NoteNumberRestart::EachPage),
        _ => return Err(PyValueError::new_err("unknown note numbering restart")),
    };
    let format = match field!(o, "format", Option<String>).as_deref() {
        None => None,
        Some("decimal") => Some(crate::NoteNumberFormat::Decimal),
        Some("lowerRoman") => Some(crate::NoteNumberFormat::LowerRoman),
        Some("upperRoman") => Some(crate::NoteNumberFormat::UpperRoman),
        Some("lowerLetter") => Some(crate::NoteNumberFormat::LowerLetter),
        Some("upperLetter") => Some(crate::NoteNumberFormat::UpperLetter),
        _ => return Err(PyValueError::new_err("unknown note numbering format")),
    };
    Ok(crate::NoteNumbering {
        format,
        start: field!(o, "start", Option<u32>),
        restart,
    })
}

pub(super) fn document_defaults(o: &Bound<'_, PyAny>) -> PyResult<crate::DocumentDefaults> {
    Ok(crate::DocumentDefaults {
        run: optional_record(o, "run", run_format)?,
        paragraph: optional_record(o, "paragraph", paragraph_format)?,
        footnote_numbering: optional_record(o, "footnote_numbering", note_numbering)?,
        endnote_numbering: optional_record(o, "endnote_numbering", note_numbering)?,
        mirror_margins: field!(o, "mirror_margins", Option<bool>),
        gutter_at_top: field!(o, "gutter_at_top", Option<bool>),
        track_revisions: field!(o, "track_revisions", Option<bool>),
        footnote_position: match field!(o, "footnote_position", Option<String>).as_deref() {
            None => None,
            Some("pageBottom") => Some(crate::FootnotePosition::PageBottom),
            Some("beneathText") => Some(crate::FootnotePosition::BeneathText),
            _ => return Err(PyValueError::new_err("unknown footnote position")),
        },
        endnote_position: match field!(o, "endnote_position", Option<String>).as_deref() {
            None => None,
            Some("sectEnd") => Some(crate::EndnotePosition::SectionEnd),
            Some("docEnd") => Some(crate::EndnotePosition::DocumentEnd),
            _ => return Err(PyValueError::new_err("unknown endnote position")),
        },
    })
}
pub(super) fn paragraph_format(o: &Bound<'_, PyAny>) -> PyResult<ParagraphFormat> {
    Ok(ParagraphFormat {
        bidi: field!(o, "bidi", Option<bool>),
        alignment: optional_enum(o, "alignment", alignment)?,
        spacing_before: field!(o, "spacing_before", Option<f64>),
        spacing_after: field!(o, "spacing_after", Option<f64>),
        line_spacing: field!(o, "line_spacing", Option<f64>),
        line_height: optional_record(o, "line_height", line_height)?,
        tab_stops: list(&o.getattr("tab_stops")?, tab_stop)?,
        left_indent: field!(o, "left_indent", Option<f64>),
        right_indent: field!(o, "right_indent", Option<f64>),
        first_line_indent: field!(o, "first_line_indent", Option<f64>),
        keep_together: field!(o, "keep_together", Option<bool>),
        keep_with_next: field!(o, "keep_with_next", Option<bool>),
        page_break_before: field!(o, "page_break_before", Option<bool>),
        widow_control: field!(o, "widow_control", Option<bool>),
        suppress_line_numbers: field!(o, "suppress_line_numbers", Option<bool>),
        contextual_spacing: field!(o, "contextual_spacing", Option<bool>),
        mirror_indents: field!(o, "mirror_indents", Option<bool>),
        snap_to_grid: field!(o, "snap_to_grid", Option<bool>),
        ..Default::default()
    })
}
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
        "distribute" => Ok(Alignment::Distribute),
        "justifyMedium" => Ok(Alignment::JustifyMedium),
        "justifyHigh" => Ok(Alignment::JustifyHigh),
        "justifyLow" => Ok(Alignment::JustifyLow),
        "thaiJustify" => Ok(Alignment::ThaiJustify),
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
        header: dict_field!(d, "header", i64),
        footer: dict_field!(d, "footer", i64),
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
pub(super) fn image_anchor(a: &Bound<'_, PyAny>) -> PyResult<crate::ImageAnchor> {
    let reference = |value: &str| match value {
        "page" => Ok(crate::ImageAnchorReference::Page),
        "margin" => Ok(crate::ImageAnchorReference::Margin),
        "column" => Ok(crate::ImageAnchorReference::Column),
        "character" => Ok(crate::ImageAnchorReference::Character),
        "leftMargin" => Ok(crate::ImageAnchorReference::LeftMargin),
        "rightMargin" => Ok(crate::ImageAnchorReference::RightMargin),
        "paragraph" => Ok(crate::ImageAnchorReference::Paragraph),
        "line" => Ok(crate::ImageAnchorReference::Line),
        "topMargin" => Ok(crate::ImageAnchorReference::TopMargin),
        "bottomMargin" => Ok(crate::ImageAnchorReference::BottomMargin),
        "insideMargin" => Ok(crate::ImageAnchorReference::InsideMargin),
        "outsideMargin" => Ok(crate::ImageAnchorReference::OutsideMargin),
        _ => Err(PyValueError::new_err("unknown picture anchor reference")),
    };
    Ok(crate::ImageAnchor {
        horizontal_reference: reference(&field!(a, "horizontal_reference", String))?,
        vertical_reference: reference(&field!(a, "vertical_reference", String))?,
        horizontal_alignment: match field!(a, "horizontal_alignment", Option<String>).as_deref() {
            None => None,
            Some("left") => Some(crate::ImageAnchorHorizontalAlignment::Left),
            Some("center") => Some(crate::ImageAnchorHorizontalAlignment::Center),
            Some("right") => Some(crate::ImageAnchorHorizontalAlignment::Right),
            Some("inside") => Some(crate::ImageAnchorHorizontalAlignment::Inside),
            Some("outside") => Some(crate::ImageAnchorHorizontalAlignment::Outside),
            _ => {
                return Err(PyValueError::new_err(
                    "unknown picture anchor horizontal alignment",
                ));
            }
        },
        vertical_alignment: match field!(a, "vertical_alignment", Option<String>).as_deref() {
            None => None,
            Some("top") => Some(crate::ImageAnchorVerticalAlignment::Top),
            Some("center") => Some(crate::ImageAnchorVerticalAlignment::Center),
            Some("bottom") => Some(crate::ImageAnchorVerticalAlignment::Bottom),
            Some("inside") => Some(crate::ImageAnchorVerticalAlignment::Inside),
            Some("outside") => Some(crate::ImageAnchorVerticalAlignment::Outside),
            _ => {
                return Err(PyValueError::new_err(
                    "unknown picture anchor vertical alignment",
                ));
            }
        },
        x: field!(a, "x", f64),
        y: field!(a, "y", f64),
        wrap: match field!(a, "wrap", String).as_str() {
            "none" => crate::ImageWrap::None,
            "square" => crate::ImageWrap::Square,
            "topAndBottom" => crate::ImageWrap::TopAndBottom,
            "tight" => crate::ImageWrap::Tight,
            "through" => crate::ImageWrap::Through,
            _ => return Err(PyValueError::new_err("unknown picture anchor wrap")),
        },
        wrap_side: match field!(a, "wrap_side", Option<String>).as_deref() {
            None => None,
            Some("bothSides") => Some(crate::ImageWrapSide::BothSides),
            Some("left") => Some(crate::ImageWrapSide::Left),
            Some("right") => Some(crate::ImageWrapSide::Right),
            Some("largest") => Some(crate::ImageWrapSide::Largest),
            _ => return Err(PyValueError::new_err("unknown picture wrap side")),
        },
        wrap_polygon: field!(a, "wrap_polygon", Vec<(i32, i32)>),
        distance_top: field!(a, "distance_top", Option<f64>),
        distance_bottom: field!(a, "distance_bottom", Option<f64>),
        distance_left: field!(a, "distance_left", Option<f64>),
        distance_right: field!(a, "distance_right", Option<f64>),
        relative_height: field!(a, "relative_height", u32),
        behind_text: field!(a, "behind_text", bool),
        locked: field!(a, "locked", Option<bool>),
        layout_in_cell: field!(a, "layout_in_cell", Option<bool>),
        allow_overlap: field!(a, "allow_overlap", Option<bool>),
    })
}
pub(super) fn imagetransformation(o: &Bound<'_, PyAny>) -> PyResult<ImageTransformation> {
    let d = o;
    Ok(ImageTransformation {
        anchor: dict_field!(d, "anchor", Bound<'_, PyAny>)
            .as_ref()
            .map(image_anchor)
            .transpose()?,
        rotation: dict_field!(d, "rotation", f64),
        flip_horizontal: dict_field!(d, "flip_horizontal", bool),
        flip_vertical: dict_field!(d, "flip_vertical", bool),
        crop: dict_field!(d, "crop", Bound<'_, PyAny>)
            .map(|crop| {
                Ok::<_, PyErr>(crate::ImageCrop {
                    left: field!(crop, "left", f64),
                    top: field!(crop, "top", f64),
                    right: field!(crop, "right", f64),
                    bottom: field!(crop, "bottom", f64),
                })
            })
            .transpose()?,
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
        bidi: field!(o, "bidi", Option<bool>),
        name: field!(o, "name", String),
        base_style: field!(o, "base_style", Option<String>),
        next_paragraph_style: field!(o, "next_paragraph_style", Option<String>),
        linked_style: field!(o, "linked_style", Option<String>),
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
        line_height: optional_record(o, "line_height", line_height)?,
        tab_stops: list(&o.getattr("tab_stops")?, tab_stop)?,
        left_indent: field!(o, "left_indent", Option<f64>),
        right_indent: field!(o, "right_indent", Option<f64>),
        first_line_indent: field!(o, "first_line_indent", Option<f64>),
        keep_together: field!(o, "keep_together", Option<bool>),
        keep_with_next: field!(o, "keep_with_next", Option<bool>),
        page_break_before: field!(o, "page_break_before", Option<bool>),
        widow_control: field!(o, "widow_control", Option<bool>),
        suppress_line_numbers: field!(o, "suppress_line_numbers", Option<bool>),
        contextual_spacing: field!(o, "contextual_spacing", Option<bool>),
        mirror_indents: field!(o, "mirror_indents", Option<bool>),
        snap_to_grid: field!(o, "snap_to_grid", Option<bool>),
        language: optional_record(o, "language", run_language)?,
        bold_complex_script: field!(o, "bold_complex_script", Option<bool>),
        italic_complex_script: field!(o, "italic_complex_script", Option<bool>),
        size_complex_script: field!(o, "size_complex_script", Option<f64>),
        rtl: field!(o, "rtl", Option<bool>),
        complex_script: field!(o, "complex_script", Option<bool>),
        font_east_asia: field!(o, "font_east_asia", Option<String>),
        font_complex_script: field!(o, "font_complex_script", Option<String>),
    })
}
pub(super) fn run_language(o: &Bound<'_, PyAny>) -> PyResult<crate::RunLanguage> {
    Ok(crate::RunLanguage {
        latin: field!(o, "latin", Option<String>),
        east_asia: field!(o, "east_asia", Option<String>),
        bidi: field!(o, "bidi", Option<String>),
    })
}
pub(super) fn line_height(o: &Bound<'_, PyAny>) -> PyResult<crate::LineHeight> {
    let value = field!(o, "value", f64);
    match field!(o, "rule", String).as_str() {
        "multiple" => Ok(crate::LineHeight::Multiple(value)),
        "exact" => Ok(crate::LineHeight::Exact(value)),
        "at_least" => Ok(crate::LineHeight::AtLeast(value)),
        _ => Err(rust_error(
            crate::Diagnostic::new(crate::Rule::InvalidContent, "unknown line_height rule")
                .at("line_height.rule")
                .into(),
        )),
    }
}
pub(super) fn tab_stop(o: &Bound<'_, PyAny>) -> PyResult<Option<crate::TabStop>> {
    let invalid = |name: &str| {
        rust_error(
            crate::Diagnostic::new(
                crate::Rule::InvalidContent,
                format!("unknown tab stop {name}"),
            )
            .at(format!("tab_stops.{name}"))
            .into(),
        )
    };
    let alignment = match field!(o, "alignment", String).as_str() {
        "left" => crate::TabAlignment::Left,
        "center" => crate::TabAlignment::Center,
        "right" => crate::TabAlignment::Right,
        "decimal" => crate::TabAlignment::Decimal,
        "bar" => crate::TabAlignment::Bar,
        "clear" => crate::TabAlignment::Clear,
        "start" => crate::TabAlignment::Start,
        "end" => crate::TabAlignment::End,
        _ => return Err(invalid("alignment")),
    };
    let leader = match field!(o, "leader", Option<String>).as_deref() {
        None => None,
        Some("none") => Some(crate::TabLeader::None),
        Some("dot") => Some(crate::TabLeader::Dot),
        Some("hyphen") => Some(crate::TabLeader::Hyphen),
        Some("underscore") => Some(crate::TabLeader::Underscore),
        Some("heavy") => Some(crate::TabLeader::Heavy),
        Some("middle_dot") => Some(crate::TabLeader::MiddleDot),
        _ => return Err(invalid("leader")),
    };
    Ok(Some(crate::TabStop {
        position: field!(o, "position", f64),
        alignment,
        leader,
    }))
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
        line_height: optional_record(o, "line_height", line_height)?,
        tab_stops: list(&o.getattr("tab_stops")?, tab_stop)?,
        bidi: field!(o, "bidi", Option<bool>),
        language: optional_record(o, "language", run_language)?,
        bold_complex_script: field!(o, "bold_complex_script", Option<bool>),
        italic_complex_script: field!(o, "italic_complex_script", Option<bool>),
        size_complex_script: field!(o, "size_complex_script", Option<f64>),
        rtl: field!(o, "rtl", Option<bool>),
        complex_script: field!(o, "complex_script", Option<bool>),
        font_east_asia: field!(o, "font_east_asia", Option<String>),
        font_complex_script: field!(o, "font_complex_script", Option<String>),
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
fn page_border(o: &Bound<'_, PyAny>) -> PyResult<crate::PageBorder> {
    Ok(crate::PageBorder {
        style: borderline(&o.getattr("style")?)?,
        size: field!(o, "size", Option<u32>),
        space: field!(o, "space", Option<u32>),
        color: field!(o, "color", Option<[u8; 3]>),
    })
}
pub(super) fn sectionproperties(o: &Bound<'_, PyAny>) -> PyResult<SectionProperties> {
    Ok(SectionProperties {
        page_size: optional_record(o, "page_size", pagesize)?,
        page_orientation: optional_enum(o, "page_orientation", orientation)?,
        page_margins: optional_record(o, "page_margins", pagemargins)?,
        page_borders: optional_record(o, "page_borders", |borders| {
            Ok(crate::PageBorders {
                offset_from: match field!(borders, "offset_from", Option<String>).as_deref() {
                    None => None,
                    Some("page") => Some(crate::PageBorderOffset::Page),
                    Some("text") => Some(crate::PageBorderOffset::Text),
                    _ => return Err(PyValueError::new_err("unknown page border offset_from")),
                },
                display: match field!(borders, "display", Option<String>).as_deref() {
                    None => None,
                    Some("allPages") => Some(crate::PageBorderDisplay::AllPages),
                    Some("firstPage") => Some(crate::PageBorderDisplay::FirstPage),
                    Some("notFirstPage") => Some(crate::PageBorderDisplay::NotFirstPage),
                    _ => return Err(PyValueError::new_err("unknown page border display")),
                },
                z_order: match field!(borders, "z_order", Option<String>).as_deref() {
                    None => None,
                    Some("front") => Some(crate::PageBorderZOrder::Front),
                    Some("back") => Some(crate::PageBorderZOrder::Back),
                    _ => return Err(PyValueError::new_err("unknown page border z_order")),
                },
                top: optional_record(borders, "top", page_border)?,
                left: optional_record(borders, "left", page_border)?,
                bottom: optional_record(borders, "bottom", page_border)?,
                right: optional_record(borders, "right", page_border)?,
            })
        })?,
        page_numbering: optional_record(o, "page_numbering", pagenumbering)?,
        line_numbering: optional_record(o, "line_numbering", |line| {
            let restart = match field!(line, "restart", Option<String>).as_deref() {
                None => None,
                Some("newPage") => Some(crate::LineNumberRestart::NewPage),
                Some("newSection") => Some(crate::LineNumberRestart::NewSection),
                Some("continuous") => Some(crate::LineNumberRestart::Continuous),
                _ => return Err(PyValueError::new_err("unknown line numbering restart")),
            };
            Ok(crate::LineNumbering {
                clear: field!(line, "clear", bool),
                count_by: field!(line, "count_by", Option<u32>),
                start: field!(line, "start", Option<u32>),
                distance: field!(line, "distance", Option<u32>),
                restart,
            })
        })?,
        document_grid: optional_record(o, "document_grid", |grid| {
            let mode = match field!(grid, "mode", Option<String>).as_deref() {
                None => None,
                Some("default") => Some(crate::DocumentGridMode::Default),
                Some("lines") => Some(crate::DocumentGridMode::Lines),
                Some("linesAndChars") => Some(crate::DocumentGridMode::LinesAndChars),
                Some("snapToChars") => Some(crate::DocumentGridMode::SnapToChars),
                _ => return Err(PyValueError::new_err("unknown document grid mode")),
            };
            Ok(crate::DocumentGrid {
                mode,
                line_pitch: field!(grid, "line_pitch", Option<u32>),
                character_pitch_delta: field!(grid, "character_pitch_delta", Option<i32>),
            })
        })?,
        columns: optional_record(o, "columns", columns)?,
        vertical_align: optional_enum(o, "vertical_align", verticalalignment)?,
        title_page: field!(o, "title_page", Option<bool>),
        r#type: optional_enum(o, "type", sectiontype)?,
        bidi: field!(o, "bidi", Option<bool>),
        rtl_gutter: field!(o, "rtl_gutter", Option<bool>),
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
    PageSize, ParagraphStyleDefinition, RunFormat, SectionProperties, SectionType, TableBorder,
    TableLayout, TableSectionFormat, TableStyleDefinition, ThemeColor, Underline,
    VerticalAlignment, VerticalMerge,
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
        abstract_id: field!(o, "abstract_id", Option<u32>),
        inherit_style: field!(o, "inherit_style", Option<String>),
        levels: list(&o.getattr("levels")?, numbering_level)?,
        start_overrides: list(&o.getattr("start_overrides")?, |entry| {
            Ok(Some(crate::NumberingStartOverride {
                level: field!(entry, "level", u8),
                start: field!(entry, "start", u32),
            }))
        })?,
        level_overrides: list(&o.getattr("level_overrides")?, numbering_level)?,
    })
}
fn numbering_level(o: &Bound<'_, PyAny>) -> PyResult<Option<crate::NumberingLevel>> {
    let alignment = match field!(o, "alignment", Option<String>).as_deref() {
        None => None,
        Some("left") => Some(crate::NumberingAlignment::Left),
        Some("center") => Some(crate::NumberingAlignment::Center),
        Some("right") => Some(crate::NumberingAlignment::Right),
        Some("start") => Some(crate::NumberingAlignment::Start),
        Some("end") => Some(crate::NumberingAlignment::End),
        _ => {
            return Err(rust_error(
                crate::Diagnostic::new(crate::Rule::InvalidContent, "unknown numbering alignment")
                    .at("alignment")
                    .into(),
            ));
        }
    };
    let suffix = match field!(o, "suffix", Option<String>).as_deref() {
        None => None,
        Some("tab") => Some(crate::NumberingSuffix::Tab),
        Some("space") => Some(crate::NumberingSuffix::Space),
        Some("nothing") => Some(crate::NumberingSuffix::Nothing),
        _ => {
            return Err(rust_error(
                crate::Diagnostic::new(crate::Rule::InvalidContent, "unknown numbering suffix")
                    .at("suffix")
                    .into(),
            ));
        }
    };
    Ok(Some(crate::NumberingLevel {
        level: field!(o, "level", u8),
        start: field!(o, "start", u32),
        format: field!(o, "format", String),
        text: field!(o, "text", String),
        is_legal: field!(o, "is_legal", Option<bool>),
        suffix,
        restart: field!(o, "restart", Option<u32>),
        label_format: optional_record(o, "label_format", run_format)?,
        paragraph_format: optional_record(o, "paragraph_format", paragraph_format)?,
        paragraph_style: field!(o, "paragraph_style", Option<String>),
        alignment,
    }))
}
