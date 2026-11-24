//! Stage edits on the original package; all target and conflict checks live in Rust.
use crate::{
    boundary::{self as b, Failure, Paths, Record},
    host::Wire,
    inspection::ImportedSource,
    program::Hooks,
    text_layout::{alignment, alignment_view},
};
use js_sys::{Array, Uint8Array};
use rocx::{
    ResolvedImage, ResolvedParagraph, ResolvedTable, Result,
    editor::{
        EditPreview, HeaderAnchor, HeaderEditMode, HeaderImageChange, HeaderImageEdit,
        HeaderImagePreview, ParagraphAlignmentEdit, ParagraphAnchor, ParagraphInsertion,
        TableAnchor, TableInsertion,
    },
};
use wasm_bindgen::JsValue;

pub(crate) const HOOKS: Hooks = Hooks {
    operations: &[
        ("importedPreview", (preview, Paths::Package)),
        (
            "importedPreviewHeaderImage",
            (preview_header, Paths::Package),
        ),
        ("editPreviewChanges", (changes, Paths::Package)),
        ("editPreviewBlockChanges", (block_changes, Paths::Package)),
        (
            "editPreviewCellAlignmentChanges",
            (cell_alignment_changes, Paths::Package),
        ),
        (
            "editPreviewRunTextChanges",
            (run_text_changes, Paths::Package),
        ),
        ("editPreviewRunRemovals", (run_removals, Paths::Package)),
        ("editPreviewInsertions", (insertions, Paths::Package)),
        (
            "editPreviewParagraphInsertions",
            (paragraph_insertions, Paths::Package),
        ),
        ("editPreviewHeaderChanges", (header_changes, Paths::Package)),
        ("editPreviewToDocx", (edit_docx, Paths::Package)),
        ("headerImagePreviewChange", (header_change, Paths::Package)),
        ("headerImagePreviewToDocx", (header_docx, Paths::Package)),
    ],
    ..Hooks::EMPTY
};

fn borrow_target<T: 'static>(call: &Record, path: &str) -> Result<std::rc::Rc<T>> {
    b::borrow(&call.get("target")?, path)
}

fn argument(call: &Record, path: &str) -> Result<Wire> {
    b::list(&call.get("args")?, "args")?
        .into_iter()
        .next()
        .ok_or_else(|| Failure::invalid(path, "missing argument"))
}

fn call(input: &Wire) -> Result<Record> {
    Record::new(input, "", &["target", "args"])
}

fn edits<T>(batch: &Record, field: &str, decode: fn(&Wire, &str) -> Result<T>) -> Result<Vec<T>> {
    let value = batch.get(field)?;
    if value.js().is_undefined() {
        return Ok(Vec::new());
    }
    let path = batch.path(field);
    b::list(&value, &path)?
        .iter()
        .enumerate()
        .map(|(index, wire)| decode(wire, &format!("{path}[{index}]")))
        .collect()
}

fn alignment_edit(input: &Wire, path: &str) -> Result<ParagraphAlignmentEdit> {
    let edit = Record::new(input, path, &["target", "alignment"])?;
    Ok(ParagraphAlignmentEdit {
        target: (*borrow_target::<ParagraphAnchor>(&edit, &edit.path("target"))?).clone(),
        alignment: alignment(&edit.get("alignment")?, &edit.path("alignment"))?,
    })
}

fn table_insertion(input: &Wire, path: &str) -> Result<TableInsertion> {
    let edit = Record::new(input, path, &["target", "table"])?;
    Ok(TableInsertion {
        target: (*borrow_target::<ParagraphAnchor>(&edit, &edit.path("target"))?).clone(),
        table: (*b::borrow::<ResolvedTable>(&edit.get("table")?, &edit.path("table"))?).clone(),
    })
}

fn block_edit(input: &Wire, path: &str) -> Result<rocx::editor::BlockEdit> {
    use rocx::editor::BlockEdit;
    let kind = b::string(
        &b::member(&input.js(), "kind", path)?,
        &b::child_path(path, "kind"),
    )?;
    let fields: &[&str] = match kind.as_str() {
        "replace" => &["kind", "target", "paragraph"],
        "remove" => &["kind", "target"],
        "replaceSection" => &["kind", "target", "blocks"],
        "keepWithNext" => &["kind", "target", "value"],
        _ => return Err(Failure::invalid(path, "unknown block edit")),
    };
    let edit = Record::new(input, path, fields)?;
    let target = (*borrow_target::<ParagraphAnchor>(&edit, &edit.path("target"))?).clone();
    let paragraph = |wire: &Wire, p: &str| -> Result<_> {
        Ok((*b::borrow::<ResolvedParagraph>(wire, p)?).clone())
    };
    Ok(match kind.as_str() {
        "replace" => BlockEdit::Replace(
            target,
            Box::new(paragraph(&edit.get("paragraph")?, &edit.path("paragraph"))?),
        ),
        "remove" => BlockEdit::Remove(target),
        "replaceSection" => {
            let p = edit.path("blocks");
            BlockEdit::ReplaceSection(
                target,
                b::list(&edit.get("blocks")?, &p)?
                    .iter()
                    .enumerate()
                    .map(|(i, w)| {
                        let path = format!("{p}[{i}]");
                        paragraph(w, &path)
                            .map(rocx::ResolvedBlock::Paragraph)
                            .or_else(|_| {
                                b::borrow::<ResolvedTable>(w, &path)
                                    .map(|t| rocx::ResolvedBlock::Table((*t).clone()))
                            })
                    })
                    .collect::<Result<_>>()?,
            )
        }
        _ => BlockEdit::KeepWithNext(
            target,
            edit.get("value")?
                .js()
                .as_bool()
                .ok_or_else(|| Failure::invalid(&edit.path("value"), "expected boolean"))?,
        ),
    })
}

fn paragraph_insertion(input: &Wire, path: &str) -> Result<ParagraphInsertion> {
    let edit = Record::new(input, path, &["target", "paragraph"])?;
    Ok(ParagraphInsertion {
        target: (*borrow_target::<ParagraphAnchor>(&edit, &edit.path("target"))?).clone(),
        paragraph: (*b::borrow::<ResolvedParagraph>(
            &edit.get("paragraph")?,
            &edit.path("paragraph"),
        )?)
        .clone(),
    })
}

fn cell_alignment_edit(input: &Wire, path: &str) -> Result<rocx::editor::CellAlignmentEdit> {
    let edit = Record::new(
        input,
        path,
        &["target", "rowIndex", "cellIndex", "alignment"],
    )?;
    let index = |name: &str| -> Result<usize> {
        usize::try_from(b::u64_number(&edit.get(name)?, &edit.path(name))?)
            .map_err(|_| Failure::invalid(&edit.path(name), "index is too large"))
    };
    let alignment = match b::string(&edit.get("alignment")?, &edit.path("alignment"))?.as_str() {
        "top" => rocx::VerticalAlignment::Top,
        "center" => rocx::VerticalAlignment::Center,
        "bottom" => rocx::VerticalAlignment::Bottom,
        "both" => rocx::VerticalAlignment::Both,
        _ => {
            return Err(Failure::invalid(
                &edit.path("alignment"),
                "unknown vertical alignment",
            ));
        }
    };
    Ok(rocx::editor::CellAlignmentEdit {
        target: (*borrow_target::<TableAnchor>(&edit, &edit.path("target"))?).clone(),
        row_index: index("rowIndex")?,
        cell_index: index("cellIndex")?,
        alignment,
    })
}

fn header_edit(input: &Wire, path: &str) -> Result<HeaderImageEdit> {
    let edit = Record::new(input, path, &["target", "mode", "image"])?;
    let mode_path = edit.path("mode");
    let mode = match b::string(&edit.get("mode")?, &mode_path)?.as_str() {
        "shared" => HeaderEditMode::Shared,
        "sectionLocal" => HeaderEditMode::SectionLocal,
        _ => {
            return Err(Failure::invalid(
                &mode_path,
                "unknown HeaderEditMode variant",
            ));
        }
    };
    Ok(HeaderImageEdit {
        target: (*borrow_target::<HeaderAnchor>(&edit, &edit.path("target"))?).clone(),
        mode,
        image: (*b::borrow::<ResolvedImage>(&edit.get("image")?, &edit.path("image"))?).clone(),
    })
}

fn preview(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let batch = Record::new(
        &argument(&call, "batch")?,
        "batch",
        &[
            "alignments",
            "tableInsertions",
            "headerImages",
            "runText",
            "runRemovals",
            "paragraphInsertions",
            "cellAlignments",
            "blockEdits",
        ],
    )?;
    let preview = source.document().preview_with_blocks(
        &edits(&batch, "alignments", alignment_edit)?,
        &edits(&batch, "tableInsertions", table_insertion)?,
        &edits(&batch, "headerImages", header_edit)?,
        &edits(&batch, "runText", |input, path| {
            let edit = Record::new(input, path, &["target", "text"])?;
            Ok(rocx::editor::RunTextEdit {
                target: crate::inspection::inline_anchor(
                    &edit.get("target")?,
                    &edit.path("target"),
                )?,
                text: b::string(&edit.get("text")?, &edit.path("text"))?,
            })
        })?,
        &edits(&batch, "paragraphInsertions", paragraph_insertion)?,
        &edits(&batch, "cellAlignments", cell_alignment_edit)?,
        &edits(&batch, "runRemovals", crate::inspection::inline_anchor)?,
        &edits(&batch, "blockEdits", block_edit)?,
    )?;
    Ok(b::handle_with_methods(
        "editPreview",
        preview,
        JsValue::UNDEFINED,
        &[
            ("changes", "editPreviewChanges"),
            ("blockChanges", "editPreviewBlockChanges"),
            ("cellAlignmentChanges", "editPreviewCellAlignmentChanges"),
            ("runTextChanges", "editPreviewRunTextChanges"),
            ("runRemovals", "editPreviewRunRemovals"),
            ("insertions", "editPreviewInsertions"),
            ("paragraphInsertions", "editPreviewParagraphInsertions"),
            ("headerChanges", "editPreviewHeaderChanges"),
            ("toDocx", "editPreviewToDocx"),
        ],
    ))
}

fn run_removals(input: &Wire) -> Result<JsValue> {
    let preview = borrow_target::<EditPreview>(&call(input)?, "preview")?;
    Ok(array(preview.run_removals().iter().map(|c| {
        b::record(&[
            ("bodyIndex", c.body_index.into()),
            ("inlineIndex", c.inline_index.into()),
            ("before", (&c.before).into()),
        ])
    })))
}

fn preview_header(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let edit = header_edit(&argument(&call, "edit")?, "edit")?;
    let preview = source.document().preview_header_image(&edit)?;
    Ok(b::handle_with_methods(
        "headerImagePreview",
        preview,
        JsValue::UNDEFINED,
        &[
            ("change", "headerImagePreviewChange"),
            ("toDocx", "headerImagePreviewToDocx"),
        ],
    ))
}

fn array(values: impl IntoIterator<Item = JsValue>) -> JsValue {
    values.into_iter().collect::<Array>().into()
}

fn cell_alignment_changes(input: &Wire) -> Result<JsValue> {
    let preview = borrow_target::<EditPreview>(&call(input)?, "preview")?;
    Ok(array(preview.cell_alignment_changes().iter().map(
        |change| {
            let after = match change.after {
                rocx::VerticalAlignment::Top => "top",
                rocx::VerticalAlignment::Center => "center",
                rocx::VerticalAlignment::Bottom => "bottom",
                rocx::VerticalAlignment::Both => "both",
            };
            let result = b::record(&[
                ("bodyIndex", change.body_index.into()),
                ("rowIndex", change.row_index.into()),
                ("cellIndex", change.cell_index.into()),
                ("after", after.into()),
            ]);
            if let Some(before) = &change.before {
                b::set(&result, "before", &before.into());
            }
            result
        },
    )))
}

fn changes(input: &Wire) -> Result<JsValue> {
    let preview = borrow_target::<EditPreview>(&call(input)?, "preview")?;
    Ok(array(preview.changes().iter().map(|change| {
        let result = b::record(&[
            ("bodyIndex", change.body_index.into()),
            ("after", alignment_view(&change.after)),
        ]);
        if let Some(before) = &change.before {
            b::set(&result, "before", &before.into());
        }
        result
    })))
}

fn insertions(input: &Wire) -> Result<JsValue> {
    let preview = borrow_target::<EditPreview>(&call(input)?, "preview")?;
    Ok(array(preview.insertions().iter().map(|change| {
        b::record(&[
            ("afterBodyIndex", change.after_body_index.into()),
            ("rows", change.rows.into()),
            ("columns", change.columns.into()),
        ])
    })))
}

fn paragraph_insertions(input: &Wire) -> Result<JsValue> {
    let preview = borrow_target::<EditPreview>(&call(input)?, "preview")?;
    Ok(array(
        preview
            .paragraph_insertions()
            .iter()
            .copied()
            .map(JsValue::from),
    ))
}

fn block_changes(input: &Wire) -> Result<JsValue> {
    let preview = borrow_target::<EditPreview>(&call(input)?, "preview")?;
    Ok(array(
        preview.block_changes().iter().copied().map(JsValue::from),
    ))
}

fn run_text_changes(input: &Wire) -> Result<JsValue> {
    let preview = borrow_target::<EditPreview>(&call(input)?, "preview")?;
    Ok(array(preview.run_text_changes().iter().map(|c| {
        b::record(&[
            ("bodyIndex", c.body_index.into()),
            ("inlineIndex", c.inline_index.into()),
            ("before", (&c.before).into()),
            ("after", (&c.after).into()),
        ])
    })))
}

fn header_view(change: &HeaderImageChange) -> JsValue {
    let result = b::record(&[
        ("sourcePart", (&change.source_part).into()),
        ("outputPart", (&change.output_part).into()),
        (
            "affectedSections",
            array(change.affected_sections.iter().copied().map(JsValue::from)),
        ),
        (
            "mode",
            match change.mode {
                HeaderEditMode::Shared => "shared",
                HeaderEditMode::SectionLocal => "sectionLocal",
            }
            .into(),
        ),
    ]);
    if let Some(section) = change.restored_section {
        b::set(&result, "restoredSection", &section.into());
    }
    result
}

fn header_changes(input: &Wire) -> Result<JsValue> {
    let preview = borrow_target::<EditPreview>(&call(input)?, "preview")?;
    Ok(array(preview.header_changes().iter().map(header_view)))
}

fn header_change(input: &Wire) -> Result<JsValue> {
    let preview = borrow_target::<HeaderImagePreview>(&call(input)?, "preview")?;
    Ok(header_view(preview.change()))
}

fn edit_docx(input: &Wire) -> Result<JsValue> {
    let preview = borrow_target::<EditPreview>(&call(input)?, "preview")?;
    Ok(Uint8Array::from(preview.to_docx()?.as_slice()).into())
}

fn header_docx(input: &Wire) -> Result<JsValue> {
    let preview = borrow_target::<HeaderImagePreview>(&call(input)?, "preview")?;
    Ok(Uint8Array::from(preview.to_docx()?.as_slice()).into())
}
