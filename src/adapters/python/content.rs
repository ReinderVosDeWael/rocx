//! Native conversion into resolved domain values.
use super::*;
pub(super) fn comment(o: &Bound<'_, PyAny>) -> PyResult<Option<ResolvedComment>> {
    let author = field!(o, "comment_author", Option<String>);
    Ok(field!(o, "comment_text", Option<String>).map(|text| ResolvedComment { text, author }))
}
pub(super) fn paragraph(o: &Bound<'_, PyAny>) -> PyResult<ResolvedParagraph> {
    let text = o.getattr("text")?;
    let children = o.getattr("children")?;
    let runs = match (text.is_none(), children.is_none()) {
        (false, true) => {
            let text = text.extract::<String>()?;
            if text.is_empty() {
                Vec::new()
            } else {
                vec![ResolvedInline::Text {
                    text,
                    format: RunFormat::default(),
                    comment: None,
                }]
            }
        }
        (true, false) => list(&children, inline)?,
        _ => {
            return Err(PyValueError::new_err(
                "Paragraph requires exactly one of text or children",
            ));
        }
    };
    let heading = field!(o, "heading", Option<i64>);
    if let Some(level) = heading
        && !(0..=9).contains(&level)
    {
        return Err(rust_error(
            crate::Diagnostic::new(crate::Rule::InvalidContent, "heading must be in 0..=9")
                .at("heading")
                .into(),
        ));
    }
    Ok(ResolvedParagraph {
        text: runs.iter().map(ResolvedInline::text).collect(),
        runs,
        comment: comment(o)?,
        numbering: optional_record(o, "numbering", numbering_reference)?,
        format: ParagraphFormat {
            heading: heading.map(|n| n as u8),
            style: field!(o, "style", Option<String>),
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
        },
    })
}
pub(super) fn textrun(o: &Bound<'_, PyAny>) -> PyResult<ResolvedInline> {
    Ok(ResolvedInline::Text {
        text: field!(o, "text", String),
        comment: comment(o)?,
        format: RunFormat {
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
        },
    })
}
pub(super) fn inline(o: &Bound<'_, PyAny>) -> PyResult<Option<ResolvedInline>> {
    let value = match kind(o)?.as_str() {
        "TextRun" => textrun(o)?,
        "Tab" => ResolvedInline::Tab {
            format: RunFormat::default(),
            comment: None,
        },
        "Break" => ResolvedInline::Break {
            kind: breaktype(&o.getattr("type")?)?,
            format: RunFormat::default(),
            comment: None,
        },
        "ImageRun" => {
            let bytes = field!(o, "data", Vec<u8>);
            let name = field!(o, "_filename", Option<String>);
            let image = crate::adapters::inspect_image(
                bytes,
                name,
                optional_record(o, "transformation", imagetransformation)?,
                optional_record(o, "alt_text", alttext)?,
            )
            .map_err(rust_error)?;
            ResolvedInline::Image {
                image,
                format: RunFormat::default(),
                comment: None,
            }
        }
        other => {
            return Err(PyTypeError::new_err(format!(
                "Unsupported inline component {other}"
            )));
        }
    };
    Ok(Some(value))
}
pub(super) fn block(o: &Bound<'_, PyAny>) -> PyResult<Option<ResolvedBlock>> {
    match kind(o)?.as_str() {
        "Paragraph" => Ok(Some(ResolvedBlock::Paragraph(paragraph(o)?))),
        "Table" => Ok(table(o)?.map(ResolvedBlock::Table)),
        other => Err(PyTypeError::new_err(format!(
            "Unsupported block component {other}"
        ))),
    }
}
pub(super) fn blocks(o: &Bound<'_, PyAny>) -> PyResult<Vec<ResolvedBlock>> {
    list(o, block)
}

use crate::{
    ParagraphFormat, ResolvedBlock, ResolvedComment, ResolvedInline, ResolvedParagraph, RunFormat,
};
