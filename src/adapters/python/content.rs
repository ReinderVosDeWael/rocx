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
                    revision: None,
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
        text: crate::inline_text(&runs),
        runs,
        comment: comment(o)?,
        numbering: optional_record(o, "numbering", numbering_reference)?,
        format: ParagraphFormat {
            heading: heading.map(|n| n as u8),
            style: field!(o, "style", Option<String>),
            ..paragraph_format(o)?
        },
    })
}
pub(super) fn textrun(o: &Bound<'_, PyAny>) -> PyResult<ResolvedInline> {
    let mut format = run_format(o)?;
    format.style = field!(o, "style", Option<String>);
    Ok(ResolvedInline::Text {
        revision: optional_record(o, "revision", |item| {
            let move_name = field!(item, "move_name", Option<String>);
            let name = || {
                crate::BookmarkName::new(
                    move_name
                        .as_deref()
                        .ok_or_else(|| PyValueError::new_err("text move requires move_name"))?,
                )
                .map_err(|error| rust_error(error.at("move_name").into()))
            };
            let kind = match field!(item, "kind", String).as_str() {
                "insertion" => crate::RevisionKind::Insertion,
                "deletion" => crate::RevisionKind::Deletion,
                "move_source" => crate::RevisionKind::MoveSource(name()?),
                "move_destination" => crate::RevisionKind::MoveDestination(name()?),
                _ => return Err(PyValueError::new_err("unknown text revision kind")),
            };
            if kind.move_name().is_none() && move_name.is_some() {
                return Err(PyValueError::new_err("move_name requires a text move"));
            }
            Ok(crate::TextRevision {
                kind,
                author: field!(item, "author", String),
                date: field!(item, "date", Option<String>),
            })
        })?,
        text: field!(o, "text", String),
        comment: comment(o)?,
        format,
    })
}
pub(super) fn inline(o: &Bound<'_, PyAny>) -> PyResult<Option<ResolvedInline>> {
    let value = match kind(o)?.as_str() {
        "Field" => {
            let instruction = o.getattr("instruction")?;
            if kind(&instruction)? != "FieldInstruction" {
                return Err(PyTypeError::new_err("expected FieldInstruction"));
            }
            let instruction = match field!(&instruction, "kind", String).as_str() {
                "page" => crate::FieldInstruction::Page,
                "num_pages" => crate::FieldInstruction::NumPages,
                "table_of_contents" => crate::FieldInstruction::TableOfContents,
                "reference" => crate::FieldInstruction::Reference(
                    crate::BookmarkName::new(field!(&instruction, "name", String))
                        .map_err(|e| rust_error(e.at("instruction.name").into()))?,
                ),
                "raw" => crate::FieldInstruction::raw(field!(&instruction, "code", String))
                    .map_err(|e| rust_error(e.into()))?,
                _ => {
                    return Err(rust_error(
                        crate::Diagnostic::new(
                            crate::Rule::InvalidContent,
                            "unknown field instruction kind",
                        )
                        .at("instruction.kind")
                        .into(),
                    ));
                }
            };
            let form = match field!(o, "form", String).as_str() {
                "simple" => crate::FieldForm::Simple,
                "complex" => crate::FieldForm::Complex,
                _ => {
                    return Err(rust_error(
                        crate::Diagnostic::new(crate::Rule::InvalidContent, "unknown field form")
                            .at("form")
                            .into(),
                    ));
                }
            };
            let children = list(&o.getattr("children")?, inline)?;
            ResolvedInline::Field(
                crate::ResolvedField::new(
                    instruction,
                    children,
                    crate::FieldProperties {
                        form,
                        dirty: field!(o, "dirty", Option<bool>),
                        locked: field!(o, "locked", Option<bool>),
                    },
                )
                .map_err(|e| rust_error(e.into()))?,
            )
        }
        "NoteReference" => ResolvedInline::NoteReference(note_target(o)?),
        "InlineContentControl" => {
            let mut children = Vec::new();
            for (index, child) in o.getattr("children")?.try_iter()?.enumerate() {
                let child = child?;
                if !matches!(
                    kind(&child)?.as_str(),
                    "TextRun" | "Tab" | "Break" | "ImageRun"
                ) {
                    return Err(rust_error(
                        crate::Diagnostic::new(
                            crate::Rule::InvalidContent,
                            "inline content control children must be text, tabs, breaks or images",
                        )
                        .at(format!("children[{index}]"))
                        .into(),
                    ));
                }
                if let Some(child) = inline(&child)? {
                    children.push(child);
                }
            }
            ResolvedInline::ContentControl(
                crate::ResolvedInlineContentControl::new(children, content_control_properties(o)?)
                    .map_err(|error| rust_error(error.into()))?,
            )
        }
        "Hyperlink" => {
            let target = o.getattr("target")?;
            let target = if let Ok(value) = target.extract::<String>() {
                crate::HyperlinkTarget::external(value).map_err(|e| rust_error(e.into()))?
            } else if kind(&target)? == "BookmarkTarget" {
                crate::HyperlinkTarget::bookmark(
                    crate::BookmarkName::new(field!(&target, "name", String))
                        .map_err(|e| rust_error(e.at("target.name").into()))?,
                )
            } else {
                return Err(PyTypeError::new_err(
                    "expected an external URI or BookmarkTarget",
                ));
            };
            let mut children = Vec::new();
            for (index, child) in o.getattr("children")?.try_iter()?.enumerate() {
                let child = child?;
                // Check the parent model before native recursion; an unsupported
                // nested container must not cause unbounded native descent.
                if matches!(
                    kind(&child)?.as_str(),
                    "Hyperlink" | "Field" | "RangeStart" | "RangeEnd" | "InlineContentControl"
                ) {
                    return Err(rust_error(
                        crate::Diagnostic::new(
                            crate::Rule::InvalidContent,
                            "hyperlink children must be leaf runs (no fields, nested links or range markers)",
                        )
                        .at(format!("children[{index}]"))
                        .into(),
                    ));
                }
                if let Some(child) = inline(&child)? {
                    children.push(child);
                }
            }
            let properties = crate::HyperlinkProperties {
                tooltip: field!(o, "tooltip", Option<String>),
                history: field!(o, "history", Option<bool>),
            };
            ResolvedInline::Hyperlink(
                crate::ResolvedHyperlink::new(target, children, properties)
                    .map_err(|e| rust_error(e.into()))?,
            )
        }
        "RangeStart" => {
            let id = crate::RangeId::new(field!(o, "id", String))
                .map_err(|e| rust_error(e.at("id").into()))?;
            let kind = match field!(o, "kind", String).as_str() {
                "bookmark" => crate::RangeKind::Bookmark(
                    crate::BookmarkName::new(field!(o, "name", String))
                        .map_err(|e| rust_error(e.at("name").into()))?,
                ),
                "comment" => crate::RangeKind::Comment(ResolvedComment {
                    text: field!(o, "text", String),
                    author: field!(o, "author", Option<String>),
                }),
                "structured_comment" => {
                    crate::RangeKind::StructuredComment(structured_comment(&o.getattr("content")?)?)
                }
                _ => {
                    return Err(rust_error(
                        crate::Diagnostic::new(crate::Rule::InvalidContent, "unknown range kind")
                            .at("kind")
                            .into(),
                    ));
                }
            };
            ResolvedInline::RangeStart { id, kind }
        }
        "RangeEnd" => ResolvedInline::RangeEnd(
            crate::RangeId::new(field!(o, "id", String))
                .map_err(|e| rust_error(e.at("id").into()))?,
        ),
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
        "InlineFraction" => ResolvedInline::Fraction {
            numerator: field!(o, "numerator", String),
            denominator: field!(o, "denominator", String),
        },
        "ColumnChartRun" => ResolvedInline::ColumnChart(crate::ColumnChartProperties {
            width: field!(o, "width", f64),
            height: field!(o, "height", f64),
            categories: field!(o, "categories", Vec<String>),
            values: field!(o, "values", Vec<f64>),
        }),
        "ShapeRun" => ResolvedInline::Shape(crate::ShapeProperties {
            anchor: optional_record(o, "anchor", image_anchor)?,
            text_format: optional_record(o, "text_format", |format| {
                Ok(crate::ShapeTextFormat {
                    italic: field!(format, "italic", Option<bool>),
                    font: field!(format, "font", Option<String>),
                    font_size: field!(format, "font_size", Option<f64>),
                    bold: field!(format, "bold", Option<bool>),
                    color: field!(format, "color", Option<[u8; 3]>),
                    alignment: match field!(format, "alignment", Option<String>).as_deref() {
                        None => None,
                        Some("left") => Some(crate::ShapeTextAlignment::Left),
                        Some("center") => Some(crate::ShapeTextAlignment::Center),
                        Some("right") => Some(crate::ShapeTextAlignment::Right),
                        _ => return Err(PyValueError::new_err("unknown shape text alignment")),
                    },
                    vertical_alignment: match field!(format, "vertical_alignment", Option<String>)
                        .as_deref()
                    {
                        None => None,
                        Some("top") => Some(crate::ShapeTextVerticalAlignment::Top),
                        Some("center") => Some(crate::ShapeTextVerticalAlignment::Center),
                        Some("bottom") => Some(crate::ShapeTextVerticalAlignment::Bottom),
                        _ => {
                            return Err(PyValueError::new_err(
                                "unknown shape text vertical alignment",
                            ));
                        }
                    },
                })
            })?,
            outline: optional_record(o, "outline", |outline| {
                Ok(crate::ShapeOutline {
                    width: field!(outline, "width", f64),
                    color: field!(outline, "color", [u8; 3]),
                })
            })?,
            width: field!(o, "width", f64),
            height: field!(o, "height", f64),
            fill: field!(o, "fill", Option<[u8; 3]>),
            text: field!(o, "text", Option<String>),
            geometry: match field!(o, "geometry", String).as_str() {
                "rectangle" => crate::ShapeGeometry::Rectangle,
                "ellipse" => crate::ShapeGeometry::Ellipse,
                "text_box" => crate::ShapeGeometry::TextBox,
                _ => return Err(PyValueError::new_err("unknown DrawingML shape geometry")),
            },
        }),
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
pub(super) fn content_control_properties(
    o: &Bound<'_, PyAny>,
) -> PyResult<crate::ContentControlProperties> {
    let multi_line = field!(o, "multi_line", Option<bool>);
    let selected = field!(o, "selected", Option<String>);
    let items = list(&o.getattr("items")?, |item| {
        Ok(Some(crate::ContentControlListItem {
            display_text: field!(item, "display_text", String),
            value: field!(item, "value", String),
        }))
    })?;
    let text_type = field!(o, "text_type", String);
    if text_type != "dropdown" && (!items.is_empty() || selected.is_some()) {
        return Err(PyValueError::new_err(
            "items and selected require a drop-down control",
        ));
    }
    let text = match text_type.as_str() {
        "rich" if multi_line.is_none() => crate::ContentControlText::Rich,
        "rich" => {
            return Err(PyValueError::new_err(
                "multi_line requires a plain-text control",
            ));
        }
        "plain" => crate::ContentControlText::Plain { multi_line },
        "dropdown" if multi_line.is_some() => {
            return Err(PyValueError::new_err(
                "multi_line requires a plain-text control",
            ));
        }
        "dropdown" => crate::ContentControlText::DropDown {
            items,
            selected: selected
                .ok_or_else(|| PyValueError::new_err("drop-down control requires selected"))?,
        },
        _ => return Err(PyValueError::new_err("unknown content control text type")),
    };
    Ok(crate::ContentControlProperties {
        text,
        ..content_control_metadata(o)?
    })
}
pub(super) fn content_control_metadata(
    o: &Bound<'_, PyAny>,
) -> PyResult<crate::ContentControlProperties> {
    let lock = match field!(o, "lock", Option<String>).as_deref() {
        None => None,
        Some("unlocked") => Some(crate::ContentControlLock::Unlocked),
        Some("sdtLocked") => Some(crate::ContentControlLock::ControlLocked),
        Some("contentLocked") => Some(crate::ContentControlLock::ContentLocked),
        Some("sdtContentLocked") => Some(crate::ContentControlLock::ControlAndContentLocked),
        _ => return Err(PyValueError::new_err("unknown content control lock")),
    };
    Ok(crate::ContentControlProperties {
        alias: field!(o, "alias", Option<String>),
        tag: field!(o, "tag", Option<String>),
        id: field!(o, "id", Option<u32>),
        lock,
        ..Default::default()
    })
}
pub(super) fn block(o: &Bound<'_, PyAny>) -> PyResult<Option<ResolvedBlock>> {
    match kind(o)?.as_str() {
        "BlockContentControl" => {
            let mut children = Vec::new();
            for (index, child) in o.getattr("children")?.try_iter()?.enumerate() {
                let child = child?;
                if kind(&child)? == "BlockContentControl" {
                    return Err(rust_error(
                        crate::Diagnostic::new(
                            crate::Rule::InvalidContent,
                            "nested block content controls are unsupported",
                        )
                        .at(format!("children[{index}]"))
                        .into(),
                    ));
                }
                if let Some(child) = block(&child)? {
                    children.push(child);
                }
            }
            Ok(Some(ResolvedBlock::ContentControl(
                crate::ResolvedBlockContentControl::new(children, content_control_properties(o)?)
                    .map_err(|error| rust_error(error.into()))?,
            )))
        }
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
