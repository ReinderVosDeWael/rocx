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
        ("importedEditStoryText", (story_text, Paths::Package)),
        (
            "importedEditStoryReference",
            (story_reference, Paths::Package),
        ),
        ("importedEditStoryBlocks", (story_blocks, Paths::Package)),
        ("importedEditEntryInlines", (entry_inlines, Paths::Package)),
        ("importedReplaceField", (replace_field, Paths::Package)),
        ("importedFieldAt", (field_at, Paths::Package)),
        ("importedPatchField", (patch_field, Paths::Package)),
        ("importedSetPairedRange", (paired_range, Paths::Package)),
        ("importedEditRunFormat", (format_runs, Paths::Package)),
        (
            "importedEditNoteReference",
            (note_reference, Paths::Package),
        ),
        ("importedEditEntry", (entry_edit, Paths::Package)),
        ("importedEditPartTables", (part_tables, Paths::Package)),
        ("importedPreviewTrackedText", (tracked_text, Paths::Package)),
        ("importedPreviewTrackedBold", (tracked_bold, Paths::Package)),
        (
            "importedPreviewTrackedParagraph",
            (tracked_paragraph, Paths::Package),
        ),
        (
            "importedPreviewParagraphRevision",
            (paragraph_revision, Paths::Package),
        ),
        ("importedPreviewTrackedMove", (tracked_move, Paths::Package)),
        ("importedPreviewRevision", (revision, Paths::Package)),
        ("importedPreview", (preview, Paths::Package)),
        ("importedPreviewLayout", (preview_layout, Paths::Package)),
        ("importedPreviewWrapControl", (wrap_control, Paths::Package)),
        (
            "importedPreviewReplaceControl",
            (replace_control, Paths::Package),
        ),
        (
            "importedPreviewUnwrapControl",
            (unwrap_control, Paths::Package),
        ),
        (
            "importedPreviewControlChoice",
            (control_choice, Paths::Package),
        ),
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
        ("editPreviewInlineChanges", (inline_changes, Paths::Package)),
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
        "remove" | "mergeSection" => &["kind", "target"],
        "splitSection" => &["kind", "target", "offset"],
        "replaceSection" => &["kind", "target", "blocks"],
        "keepWithNext" | "sectionType" => &["kind", "target", "value"],
        "paragraphFlag" | "sectionFlag" => &["kind", "target", "flag", "value"],
        _ => return Err(Failure::invalid(path, "unknown block edit")),
    };
    let edit = Record::new(input, path, fields)?;
    if matches!(kind.as_str(), "splitSection" | "mergeSection") {
        let target =
            (*borrow_target::<rocx::editor::SectionAnchor>(&edit, &edit.path("target"))?).clone();
        return Ok(if kind == "splitSection" {
            BlockEdit::SplitSection(target, index_field(&edit, "offset")?)
        } else {
            BlockEdit::MergeSection(target)
        });
    }
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
        "sectionType" => {
            let v = edit.get("value")?;
            BlockEdit::SectionType(
                target,
                if v.js().is_null() {
                    None
                } else {
                    Some(crate::text_layout::section_type(&v, &edit.path("value"))?)
                },
            )
        }
        "paragraphFlag" | "sectionFlag" => {
            let flag = b::string(&edit.get("flag")?, &edit.path("flag"))?;
            let v = edit.get("value")?.js();
            let value = if v.is_null() {
                None
            } else {
                Some(v.as_bool().ok_or_else(|| {
                    Failure::invalid(&edit.path("value"), "expected boolean or null")
                })?)
            };
            if kind == "paragraphFlag" {
                let f = rocx::editor::ParagraphFlag::ALL
                    .iter()
                    .find(|f| f.name() == flag)
                    .ok_or_else(|| {
                        Failure::invalid(&edit.path("flag"), "unknown paragraph flag")
                    })?;
                BlockEdit::ParagraphFlag(target, *f, value)
            } else {
                let f = rocx::editor::SectionFlag::ALL
                    .iter()
                    .find(|f| f.name() == flag)
                    .ok_or_else(|| Failure::invalid(&edit.path("flag"), "unknown section flag"))?;
                BlockEdit::SectionFlag(target, *f, value)
            }
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

fn inline_edit(input: &Wire, path: &str) -> Result<rocx::editor::InlineEdit> {
    use rocx::editor::{InlineAction, InlineEdit};
    let record = Record::new(
        input,
        path,
        &[
            "target", "kind", "inline", "bold", "value", "image", "property", "format", "clear",
        ],
    )?;
    let kind = b::string(&record.get("kind")?, &record.path("kind"))?;
    let edit = Record::new(
        input,
        path,
        if kind == "removeHyperlink" {
            &["target", "kind"]
        } else if matches!(kind.as_str(), "hyperlinkTooltip" | "hyperlinkHistory") {
            &["target", "kind", "value"]
        } else if kind == "setBold" {
            &["target", "kind", "bold"]
        } else if kind == "patchDrawing" {
            &["target", "kind", "property", "inline"]
        } else if kind == "setImageAsset" {
            &["target", "kind", "image"]
        } else if kind == "removeObject" {
            &["target", "kind"]
        } else if kind == "setDrawingDescription" {
            &["target", "kind", "value"]
        } else if kind == "setToggle" {
            &["target", "kind", "property", "value"]
        } else if kind == "patchFormat" {
            &["target", "kind", "format", "clear"]
        } else {
            &["target", "kind", "inline"]
        },
    )?;
    let payload = || -> Result<_> {
        Ok(
            (*b::borrow::<rocx::ResolvedInline>(&edit.get("inline")?, &edit.path("inline"))?)
                .clone(),
        )
    };
    Ok(InlineEdit {
        target: crate::inspection::inline_anchor(&edit.get("target")?, &edit.path("target"))?,
        action: match kind.as_str() {
            "removeHyperlink" => InlineAction::RemoveHyperlink,
            "hyperlinkTooltip" => {
                let v = edit.get("value")?;
                InlineAction::HyperlinkTooltip(if v.js().is_undefined() {
                    None
                } else {
                    Some(b::string(&v, &edit.path("value"))?)
                })
            }
            "hyperlinkHistory" => {
                let v = edit.get("value")?;
                InlineAction::HyperlinkHistory(if v.js().is_undefined() {
                    None
                } else {
                    Some(
                        v.js().as_bool().ok_or_else(|| {
                            Failure::invalid(&edit.path("value"), "expected boolean")
                        })?,
                    )
                })
            }
            "insertAfter" => InlineAction::InsertAfter(payload()?),
            "replace" => InlineAction::Replace(payload()?),
            "removeObject" => InlineAction::RemoveObject,
            "patchDrawing" => {
                let p = edit.path("property");
                let property =
                    rocx::editor::DrawingProperty::parse(&b::string(&edit.get("property")?, &p)?)
                        .ok_or_else(|| Failure::invalid(&p, "unknown drawing property"))?;
                InlineAction::PatchDrawing(property, payload()?)
            }
            "setImageAsset" => InlineAction::SetImageAsset(
                (*b::borrow::<ResolvedImage>(&edit.get("image")?, &edit.path("image"))?).clone(),
            ),
            "setDrawingDescription" => {
                let value = edit.get("value")?;
                InlineAction::SetDrawingDescription(if value.js().is_null() {
                    None
                } else {
                    Some(b::string(&value, &edit.path("value"))?)
                })
            }
            "patchFormat" => {
                let set = edit.get("format")?;
                let set = if set.js().is_undefined() {
                    Default::default()
                } else {
                    crate::text_layout::run_format(&set, &edit.path("format"))?
                };
                InlineAction::PatchFormat {
                    set: Box::new(set),
                    clear: edits(&edit, "clear", |wire, path| {
                        rocx::editor::RunFormatField::parse(&b::string(wire, path)?)
                            .ok_or_else(|| Failure::invalid(path, "unknown run format field"))
                    })?,
                }
            }
            "setToggle" => {
                let name = b::string(&edit.get("property")?, &edit.path("property"))?;
                let prop = rocx::editor::RunToggle::parse(&name).ok_or_else(|| {
                    Failure::invalid(&edit.path("property"), "unknown run toggle")
                })?;
                let v = edit.get("value")?;
                let value = if v.js().is_null() {
                    None
                } else {
                    Some(v.js().as_bool().ok_or_else(|| {
                        Failure::invalid(&edit.path("value"), "expected boolean or null")
                    })?)
                };
                InlineAction::SetToggle(prop, value)
            }
            "setBold" => InlineAction::SetBold(
                edit.get("bold")?
                    .js()
                    .as_bool()
                    .ok_or_else(|| Failure::invalid(&edit.path("bold"), "expected boolean"))?,
            ),
            _ => return Err(Failure::invalid(&edit.path("kind"), "unknown inline edit")),
        },
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

fn structure_edit(input: &Wire, path: &str) -> Result<rocx::editor::TableStructureEdit> {
    use rocx::editor::{TableOperation::*, TableStructureEdit};
    let edit = Record::new(
        input,
        path,
        &[
            "target", "kind", "index", "row", "cell", "blocks", "value", "column", "field",
            "source",
        ],
    )?;
    let kind = b::string(&edit.get("kind")?, &edit.path("kind"))?;
    let fields: &[&str] = if kind == "replaceCell" {
        &["target", "kind", "row", "cell", "blocks"]
    } else if kind == "patchProperty" {
        &["target", "kind", "row", "cell", "column", "field", "source"]
    } else if kind == "replaceNode" {
        &["target", "kind", "row", "cell", "source"]
    } else if kind == "removeTable" {
        &["target", "kind"]
    } else if kind == "layout" {
        &["target", "kind", "value"]
    } else if matches!(kind.as_str(), "insertCell" | "removeCell") {
        &["target", "kind", "row", "cell"]
    } else {
        &["target", "kind", "index"]
    };
    let edit = Record::new(input, path, fields)?;
    let index = |name: &str| -> Result<usize> {
        usize::try_from(b::u64_number(&edit.get(name)?, &edit.path(name))?)
            .map_err(|_| Failure::invalid(&edit.path(name), "index is too large"))
    };
    let operation = match kind.as_str() {
        "removeTable" => RemoveTable,
        "patchProperty" | "replaceNode" => {
            use rocx::editor::TableSite;
            let idx = |k| -> Result<Option<usize>> {
                if edit.get(k)?.js().is_undefined() {
                    Ok(None)
                } else {
                    index(k).map(Some)
                }
            };
            let site = match (idx("row")?, idx("cell")?, idx("column")?) {
                (None, None, None) => TableSite::Table,
                (Some(r), None, None) => TableSite::Row(r),
                (Some(r), Some(c), None) => TableSite::Cell(r, c),
                (None, None, Some(c)) => TableSite::Column(c),
                _ => return Err(Failure::invalid(path, "invalid table site coordinates")),
            };
            let wire = edit.get("source")?;
            let source = || -> Result<_> {
                Ok(Box::new(
                    (*b::borrow::<ResolvedTable>(&wire, &edit.path("source"))?).clone(),
                ))
            };
            if kind == "patchProperty" {
                Property {
                    site,
                    field: b::string(&edit.get("field")?, &edit.path("field"))?,
                    source: if wire.js().is_null() {
                        None
                    } else {
                        Some(source()?)
                    },
                }
            } else {
                ReplaceNode {
                    site,
                    source: source()?,
                }
            }
        }
        "layout" => {
            let v = edit.get("value")?;
            Layout(if v.js().is_null() {
                None
            } else {
                Some(match b::string(&v, &edit.path("value"))?.as_str() {
                    "fixed" => rocx::TableLayout::Fixed,
                    "autofit" => rocx::TableLayout::Autofit,
                    _ => {
                        return Err(Failure::invalid(
                            &edit.path("value"),
                            "unknown table layout",
                        ));
                    }
                })
            })
        }
        "insertRow" => InsertRow(index("index")?),
        "removeRow" => RemoveRow(index("index")?),
        "insertColumn" => InsertColumn(index("index")?),
        "removeColumn" => RemoveColumn(index("index")?),
        "insertCell" => InsertCell {
            row: index("row")?,
            cell: index("cell")?,
        },
        "removeCell" => RemoveCell {
            row: index("row")?,
            cell: index("cell")?,
        },
        "replaceCell" => ReplaceCell {
            row: index("row")?,
            cell: index("cell")?,
            blocks: b::list(&edit.get("blocks")?, &edit.path("blocks"))?
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    crate::structured::block_value(item, &format!("{}[{i}]", edit.path("blocks")))
                })
                .collect::<Result<_>>()?,
        },
        _ => {
            return Err(Failure::invalid(
                &edit.path("kind"),
                "unknown table operation",
            ));
        }
    };
    Ok(TableStructureEdit {
        target: (*borrow_target::<TableAnchor>(&edit, &edit.path("target"))?).clone(),
        operation,
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

fn run_text_edit(input: &Wire, path: &str) -> Result<rocx::editor::RunTextEdit> {
    let edit = Record::new(input, path, &["target", "text"])?;
    Ok(rocx::editor::RunTextEdit {
        target: crate::inspection::inline_anchor(&edit.get("target")?, &edit.path("target"))?,
        text: b::string(&edit.get("text")?, &edit.path("text"))?,
    })
}

fn note_reference(input: &Wire) -> Result<JsValue> {
    use rocx::editor::NoteReferenceEdit::*;
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let w = argument(&call, "edit")?;
    let kind = b::string(&b::member(&w.js(), "kind", "edit")?, "edit.kind")?;
    let fields: &[&str] = match kind.as_str() {
        "create" => &["ownerPart", "target", "kind", "id", "text"],
        "delete" | "detach" => &["ownerPart", "target", "kind"],
        "attach" | "retarget" | "renumber" => &["ownerPart", "target", "kind", "id"],
        _ => return Err(Failure::invalid("edit.kind", "unknown note reference edit")),
    };
    let r = Record::new(&w, "edit", fields)?;
    let owner = b::string(&r.get("ownerPart")?, "edit.ownerPart")?;
    let target = crate::inspection::inline_anchor(&r.get("target")?, "edit.target")?;
    let id = || -> Result<u32> {
        u32::try_from(b::u64_number(&r.get("id")?, "edit.id")?)
            .map_err(|_| Failure::invalid("edit.id", "expected an unsigned 32-bit ID"))
    };
    let edit = match kind.as_str() {
        "create" => Create {
            id: id()?,
            text: b::string(&r.get("text")?, "edit.text")?,
        },
        "retarget" => Retarget(id()?),
        "attach" => Attach(id()?),
        "detach" => Detach,
        "renumber" => Renumber(id()?),
        _ => Delete,
    };
    Ok(Uint8Array::from(
        source
            .document()
            .edit_note_reference(&owner, &target, edit)?
            .as_slice(),
    )
    .into())
}

fn entry_edit(input: &Wire) -> Result<JsValue> {
    use rocx::editor::EntryEdit::*;
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let w = argument(&call, "edit")?;
    let kind = b::string(&b::member(&w.js(), "kind", "edit")?, "edit.kind")?;
    let fields: &[&str] = match kind.as_str() {
        "insertParagraph" | "replaceParagraph" => &["ownerPart", "id", "kind", "index", "value"],
        "removeParagraph" => &["ownerPart", "id", "kind", "index"],
        "author" | "initials" | "date" => &["ownerPart", "id", "kind", "value"],
        "clearDate" | "clearInitials" | "deleteUnusedNote" => &["ownerPart", "id", "kind"],
        _ => return Err(Failure::invalid("edit.kind", "unknown entry edit")),
    };
    let r = Record::new(&w, "edit", fields)?;
    let string = |name: &str| b::string(&r.get(name)?, &r.path(name));
    let index = || -> Result<usize> {
        usize::try_from(b::u64_number(&r.get("index")?, "edit.index")?)
            .map_err(|_| Failure::invalid("edit.index", "index too large"))
    };
    let edit = match kind.as_str() {
        "insertParagraph" => InsertParagraph {
            index: index()?,
            text: string("value")?,
        },
        "replaceParagraph" => ReplaceParagraph {
            index: index()?,
            text: string("value")?,
        },
        "removeParagraph" => RemoveParagraph(index()?),
        "author" => CommentAuthor(string("value")?),
        "initials" => CommentInitials(Some(string("value")?)),
        "date" => CommentDate(Some(string("value")?)),
        "clearDate" => CommentDate(None),
        "deleteUnusedNote" => DeleteUnusedNote,
        _ => CommentInitials(None),
    };
    Ok(Uint8Array::from(
        source
            .document()
            .edit_entry(&string("ownerPart")?, &string("id")?, edit)?
            .as_slice(),
    )
    .into())
}

fn story_payload(r: &Record) -> Result<Vec<rocx::ResolvedBlock>> {
    b::list(&r.get("blocks")?, &r.path("blocks"))?
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let path = format!("{}[{i}]", r.path("blocks"));
            b::borrow::<ResolvedParagraph>(w, &path)
                .map(|p| rocx::ResolvedBlock::Paragraph((*p).clone()))
                .or_else(|_| {
                    b::borrow::<ResolvedTable>(w, &path)
                        .map(|t| rocx::ResolvedBlock::Table((*t).clone()))
                })
        })
        .collect()
}

fn index_field(r: &Record, key: &str) -> Result<usize> {
    usize::try_from(b::u64_number(&r.get(key)?, &r.path(key))?)
        .map_err(|_| Failure::invalid(&r.path(key), "index out of bounds"))
}

fn story_reference(input: &Wire) -> Result<JsValue> {
    use rocx::editor::{SectionStoryKind, StoryReferenceEdit::*};
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let w = argument(&call, "edit")?;
    let action = b::string(&b::member(&w.js(), "action", "edit")?, "edit.action")?;
    let fields: &[&str] = match action.as_str() {
        "create" => &["sectionIndex", "storyKind", "variant", "action", "blocks"],
        "link" => &["sectionIndex", "storyKind", "variant", "action", "partName"],
        "remove" => &["sectionIndex", "storyKind", "variant", "action"],
        _ => {
            return Err(Failure::invalid(
                "edit.action",
                "unknown story reference edit",
            ));
        }
    };
    let r = Record::new(&w, "edit", fields)?;
    let string = |key: &str| b::string(&r.get(key)?, &r.path(key));
    let kind = match string("storyKind")?.as_str() {
        "header" => SectionStoryKind::Header,
        "footer" => SectionStoryKind::Footer,
        _ => {
            return Err(Failure::invalid(
                "edit.storyKind",
                "expected header or footer",
            ));
        }
    };
    let variant = match string("variant")?.as_str() {
        "default" => rocx::HeaderFooterType::Default,
        "first" => rocx::HeaderFooterType::First,
        "even" => rocx::HeaderFooterType::Even,
        _ => return Err(Failure::invalid("edit.variant", "unknown story variant")),
    };
    let edit = match action.as_str() {
        "create" => Create(story_payload(&r)?),
        "link" => Link(string("partName")?),
        _ => Remove,
    };
    Ok(Uint8Array::from(
        source
            .document()
            .edit_story_reference(index_field(&r, "sectionIndex")?, kind, variant, edit)?
            .as_slice(),
    )
    .into())
}

fn story_blocks(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let w = argument(&call, "edit")?;
    let r = Record::new(&w, "edit", &["target", "mode", "index", "remove", "blocks"])?;
    let target = borrow_target::<HeaderAnchor>(&r, "edit.target")?;
    let mode = match b::string(&r.get("mode")?, "edit.mode")?.as_str() {
        "shared" => HeaderEditMode::Shared,
        "sectionLocal" => HeaderEditMode::SectionLocal,
        _ => return Err(Failure::invalid("edit.mode", "unknown story edit mode")),
    };
    Ok(Uint8Array::from(
        source
            .document()
            .edit_story_blocks(
                &target,
                mode,
                index_field(&r, "index")?,
                index_field(&r, "remove")?,
                &story_payload(&r)?,
            )?
            .as_slice(),
    )
    .into())
}

fn entry_inlines(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let batch = Record::new(
        &argument(&call, "batch")?,
        "batch",
        &["ownerPart", "inlineEdits"],
    )?;
    let owner = b::string(&batch.get("ownerPart")?, "batch.ownerPart")?;
    Ok(Uint8Array::from(
        source
            .document()
            .edit_entry_inlines(&owner, &edits(&batch, "inlineEdits", inline_edit)?)?
            .as_slice(),
    )
    .into())
}

fn part_tables(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let batch = Record::new(
        &argument(&call, "batch")?,
        "batch",
        &["ownerPart", "tableStructure"],
    )?;
    let owner = b::string(&batch.get("ownerPart")?, "batch.ownerPart")?;
    Ok(Uint8Array::from(
        source
            .document()
            .edit_part_tables(&owner, &edits(&batch, "tableStructure", structure_edit)?)?
            .as_slice(),
    )
    .into())
}

fn field_at(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let batch = Record::new(
        &argument(&call, "batch")?,
        "batch",
        &["target", "inlineIndex"],
    )?;
    let target = borrow_target::<ParagraphAnchor>(&batch, "batch.target")?;
    let index = usize::try_from(b::u64_number(
        &batch.get("inlineIndex")?,
        "batch.inlineIndex",
    )?)
    .map_err(|_| Failure::invalid("batch.inlineIndex", "index too large"))?;
    let f = source.document().field_at(&target, index)?;
    let view = b::record(&[
        ("complex", f.complex.into()),
        ("instruction", f.instruction.into()),
        ("inlineCount", (f.inline_count as u32).into()),
    ]);
    if let Some(v) = f.dirty_xml {
        b::set(&view, "dirtyXml", &v.into());
    }
    if let Some(v) = f.locked_xml {
        b::set(&view, "lockedXml", &v.into());
    }
    Ok(view)
}
fn patch_field(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let batch = Record::new(
        &argument(&call, "batch")?,
        "batch",
        &["target", "inlineIndex", "property", "value"],
    )?;
    let target = borrow_target::<ParagraphAnchor>(&batch, "batch.target")?;
    let index = usize::try_from(b::u64_number(
        &batch.get("inlineIndex")?,
        "batch.inlineIndex",
    )?)
    .map_err(|_| Failure::invalid("batch.inlineIndex", "index too large"))?;
    let patch = decode_field_patch(&batch)?;
    Ok(Uint8Array::from(
        source
            .document()
            .patch_field(&target, index, patch)?
            .as_slice(),
    )
    .into())
}
fn decode_field_patch(batch: &Record) -> Result<rocx::editor::FieldPatch> {
    use rocx::editor::FieldPatch;
    let v = batch.get("value")?;
    let flag = || {
        if v.js().is_undefined() {
            Ok(None)
        } else {
            v.js()
                .as_bool()
                .map(Some)
                .ok_or_else(|| Failure::invalid(&batch.path("value"), "expected boolean"))
        }
    };
    Ok(
        match b::string(&batch.get("property")?, &batch.path("property"))?.as_str() {
            "instruction" => FieldPatch::Instruction(b::string(&v, &batch.path("value"))?),
            "dirty" => FieldPatch::Dirty(flag()?),
            "locked" => FieldPatch::Locked(flag()?),
            _ => {
                return Err(Failure::invalid(
                    &batch.path("property"),
                    "unknown field property",
                ));
            }
        },
    )
}
fn field_edit(input: &Wire, path: &str) -> Result<rocx::editor::FieldEdit> {
    use rocx::editor::{FieldAction, FieldEdit};
    let r = Record::new(
        input,
        path,
        &[
            "target",
            "inlineIndex",
            "op",
            "property",
            "value",
            "field",
            "children",
        ],
    )?;
    let op = b::string(&r.get("op")?, &r.path("op"))?;
    let fields: &[&str] = match op.as_str() {
        "patch" => &["target", "inlineIndex", "op", "property", "value"],
        "replace" => &["target", "inlineIndex", "op", "field"],
        "cache" => &["target", "inlineIndex", "op", "children"],
        _ => return Err(Failure::invalid(&r.path("op"), "unknown field edit")),
    };
    let r = Record::new(input, path, fields)?;
    let target = (*borrow_target::<ParagraphAnchor>(&r, &r.path("target"))?).clone();
    let inline_index = usize::try_from(b::u64_number(
        &r.get("inlineIndex")?,
        &r.path("inlineIndex"),
    )?)
    .map_err(|_| Failure::invalid(&r.path("inlineIndex"), "index too large"))?;
    let action = match op.as_str() {
        "patch" => FieldAction::Patch(decode_field_patch(&r)?),
        "cache" => FieldAction::Cache(
            b::list(&r.get("children")?, &r.path("children"))?
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    Ok((*b::borrow::<rocx::ResolvedInline>(
                        v,
                        &format!("{}[{i}]", r.path("children")),
                    )?)
                    .clone())
                })
                .collect::<Result<_>>()?,
        ),
        _ => {
            let f = r.get("field")?;
            if f.js().is_undefined() {
                FieldAction::Remove
            } else {
                FieldAction::Replace(Box::new(
                    (*b::borrow::<rocx::ResolvedInline>(&f, &r.path("field"))?).clone(),
                ))
            }
        }
    };
    Ok(FieldEdit {
        target,
        inline_index,
        action,
    })
}
fn replace_field(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let batch = Record::new(
        &argument(&call, "batch")?,
        "batch",
        &["target", "inlineIndex", "field"],
    )?;
    let target = borrow_target::<ParagraphAnchor>(&batch, "batch.target")?;
    let index = usize::try_from(b::u64_number(
        &batch.get("inlineIndex")?,
        "batch.inlineIndex",
    )?)
    .map_err(|_| Failure::invalid("batch.inlineIndex", "index too large"))?;
    let field = batch.get("field")?;
    let field = if field.js().is_undefined() {
        None
    } else {
        Some(b::borrow::<rocx::ResolvedInline>(&field, "batch.field")?)
    };
    Ok(Uint8Array::from(
        source
            .document()
            .replace_field(&target, index, field.as_deref())?
            .as_slice(),
    )
    .into())
}
fn paired_range(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let e = paired_edit(&argument(&call, "batch")?, "batch")?;
    Ok(Uint8Array::from(
        source
            .document()
            .set_paired_range(
                e.id,
                e.bookmark.as_deref(),
                e.endpoints.as_ref().map(|(a, b)| (a, b)),
            )?
            .as_slice(),
    )
    .into())
}
fn paired_edit(input: &Wire, path: &str) -> Result<rocx::editor::PairedRangeEdit> {
    let batch = Record::new(input, path, &["id", "bookmark", "endpoints"])?;
    let id = u32::try_from(b::u64_number(&batch.get("id")?, &batch.path("id"))?)
        .map_err(|_| Failure::invalid(&batch.path("id"), "id too large"))?;
    let name = batch.get("bookmark")?;
    let name = if name.js().is_undefined() {
        None
    } else {
        Some(b::string(&name, &batch.path("bookmark"))?)
    };
    let endpoints = batch.get("endpoints")?;
    let endpoints = if endpoints.js().is_undefined() {
        None
    } else {
        let items = b::list(&endpoints, &batch.path("endpoints"))?;
        let [a, z] = items.as_slice() else {
            return Err(Failure::invalid(
                &batch.path("endpoints"),
                "expected two inline anchors",
            ));
        };
        Some((
            crate::inspection::inline_anchor(a, &format!("{path}.endpoints[0]"))?,
            crate::inspection::inline_anchor(z, &format!("{path}.endpoints[1]"))?,
        ))
    };
    Ok(rocx::editor::PairedRangeEdit {
        id,
        bookmark: name,
        endpoints,
    })
}
fn format_runs(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let batch = Record::new(
        &argument(&call, "batch")?,
        "batch",
        &["ownerPart", "inlineEdits"],
    )?;
    let owner = batch.get("ownerPart")?;
    let owner = (!owner.js().is_undefined())
        .then(|| b::string(&owner, "batch.ownerPart"))
        .transpose()?;
    Ok(Uint8Array::from(
        source
            .document()
            .edit_run_format(
                owner.as_deref(),
                &edits(&batch, "inlineEdits", inline_edit)?,
            )?
            .as_slice(),
    )
    .into())
}

fn story_text(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let batch = Record::new(
        &argument(&call, "batch")?,
        "batch",
        &["ownerPart", "runText"],
    )?;
    let owner = b::string(&batch.get("ownerPart")?, "batch.ownerPart")?;
    Ok(Uint8Array::from(
        source
            .document()
            .edit_story_text(&owner, &edits(&batch, "runText", run_text_edit)?)?
            .as_slice(),
    )
    .into())
}

fn range_edit(input: &Wire, path: &str) -> Result<rocx::editor::TextRangeEdit> {
    let edit = Record::new(input, path, &["target", "start", "end", "text"])?;
    let offset = |name: &str| -> Result<usize> {
        usize::try_from(b::u64_number(&edit.get(name)?, &edit.path(name))?)
            .map_err(|_| Failure::invalid(&edit.path(name), "offset is too large"))
    };
    Ok(rocx::editor::TextRangeEdit {
        target: (*borrow_target::<ParagraphAnchor>(&edit, &edit.path("target"))?).clone(),
        start: offset("start")?,
        end: offset("end")?,
        text: b::string(&edit.get("text")?, &edit.path("text"))?,
    })
}

fn text_insertion(input: &Wire, path: &str) -> Result<rocx::editor::TextInsertion> {
    let edit = Record::new(input, path, &["target", "position", "text"])?;
    let position = usize::try_from(b::u64_number(
        &edit.get("position")?,
        &edit.path("position"),
    )?)
    .map_err(|_| Failure::invalid(&edit.path("position"), "offset is too large"))?;
    Ok(rocx::editor::TextInsertion {
        target: (*borrow_target::<ParagraphAnchor>(&edit, &edit.path("target"))?).clone(),
        position,
        text: b::string(&edit.get("text")?, &edit.path("text"))?,
    })
}

fn control_edit(input: &Wire, path: &str) -> Result<rocx::editor::ControlEdit> {
    let edit = Record::new(input, path, &["target", "alias", "tag", "text"])?;
    let string = |field: &str| -> Result<Option<String>> {
        let value = edit.get(field)?;
        if value.js().is_undefined() {
            Ok(None)
        } else {
            b::string(&value, &edit.path(field)).map(Some)
        }
    };
    Ok(rocx::editor::ControlEdit {
        target: (*borrow_target::<rocx::editor::ControlAnchor>(&edit, &edit.path("target"))?)
            .clone(),
        alias: string("alias")?,
        tag: string("tag")?,
        text: string("text")?,
    })
}

fn resource_edit(input: &Wire, path: &str) -> Result<rocx::editor::ResourceEdit> {
    use rocx::editor::{AbstractStart, ResourceEdit};
    let edit = Record::new(
        input,
        path,
        &[
            "kind",
            "value",
            "abstractId",
            "numId",
            "level",
            "levels",
            "definition",
            "target",
            "field",
            "name",
        ],
    )?;
    let kind = b::string(&edit.get("kind")?, &edit.path("kind"))?;
    Record::new(
        input,
        path,
        match kind.as_str() {
            "metadata" | "documentDefault" => &["kind", "field", "value"],
            "customProperty" => &["kind", "name", "value"],
            "replaceInstance" => &["kind", "definition"],
            "abstractStart" => &["kind", "abstractId", "level", "value"],
            "addAbstract" | "replaceAbstract" => &["kind", "abstractId", "levels"],
            "abstractLevel" => &["kind", "abstractId", "level", "value"],
            "removeAbstract" => &["kind", "abstractId"],
            "addInstance" | "relinkInstance" => &["kind", "abstractId", "numId"],
            "removeInstance" => &["kind", "numId"],
            "instanceStart" | "instanceLevel" => &["kind", "numId", "level", "value"],
            "paragraphNumbering" => &["kind", "target", "value"],
            "storyParagraphNumbering" => &["kind", "target", "value"],
            _ => &["kind", "value"],
        },
    )?;
    let v = edit.get("value")?;
    let clear = v.js().is_null();
    let vp = edit.path("value");
    let uint = |wire: &Wire, p: &str| -> Result<u32> {
        b::u64_number(wire, p)?
            .try_into()
            .map_err(|_| Failure::invalid(p, "expected u32"))
    };
    use rocx::editor::NumberingEdit as N;
    let id = |field| uint(&edit.get(field)?, &edit.path(field));
    let level = |n: u32| {
        u8::try_from(n).map_err(|_| Failure::invalid(&edit.path("level"), "invalid level"))
    };
    Ok(match kind.as_str() {
        "coreTitle" | "coreSubject" => {
            let value = if clear {
                None
            } else {
                Some(b::string(&v, &vp)?)
            };
            if kind == "coreTitle" {
                ResourceEdit::CoreTitle(value)
            } else {
                ResourceEdit::CoreSubject(value)
            }
        }
        "documentDefault" => {
            use rocx::editor::{DefaultField, DefaultTab, DefaultValue as V};
            let key = b::string(&edit.get("field")?, &edit.path("field"))?;
            let f = DefaultField::from_key(&key)
                .ok_or_else(|| Failure::invalid(&edit.path("field"), "unknown document default"))?;
            let value = if clear {
                None
            } else if let Some(b) = v.js().as_bool() {
                Some(V::Bool(b))
            } else if v.js().as_f64().is_some() {
                Some(V::Integer(b::i64_number(&v, &vp)?))
            } else if v.js().is_string() {
                Some(V::Text(b::string(&v, &vp)?))
            } else {
                let mut tabs = Vec::new();
                for (i, item) in b::list(&v, &vp)?.iter().enumerate() {
                    let tab = Record::new(
                        item,
                        &format!("{vp}[{i}]"),
                        &["positionTwips", "alignment", "leader"],
                    )?;
                    let leader = tab.get("leader")?;
                    tabs.push(DefaultTab {
                        position_twips: b::i64_number(
                            &tab.get("positionTwips")?,
                            &tab.path("positionTwips"),
                        )?
                        .try_into()
                        .map_err(|_| Failure::invalid(&vp, "expected i32 tab position"))?,
                        alignment: b::string(&tab.get("alignment")?, &tab.path("alignment"))?,
                        leader: if leader.js().is_undefined() {
                            None
                        } else {
                            Some(b::string(&leader, &tab.path("leader"))?)
                        },
                    });
                }
                Some(V::Tabs(tabs))
            };
            ResourceEdit::Default(f, value)
        }
        "metadata" => ResourceEdit::Metadata(
            b::string(&edit.get("field")?, &edit.path("field"))?.parse()?,
            if clear {
                None
            } else {
                Some(b::string(&v, &vp)?)
            },
        ),
        "customProperty" => ResourceEdit::CustomProperty(
            b::string(&edit.get("name")?, &edit.path("name"))?,
            if clear {
                None
            } else if let Some(b) = v.js().as_bool() {
                Some(rocx::editor::CustomValue::Boolean(b))
            } else {
                Some(rocx::editor::CustomValue::Text(b::string(&v, &vp)?))
            },
        ),
        "replaceInstance" => ResourceEdit::Numbering(N::ReplaceInstance(Box::new(
            crate::styles::numbering_definition(
                &edit.get("definition")?,
                &b::Operation::new().pass(),
                &edit.path("definition"),
            )?,
        ))),
        "addAbstract" | "replaceAbstract" => {
            let abstract_id = id("abstractId")?;
            let levels = b::list(&edit.get("levels")?, &edit.path("levels"))?
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    crate::styles::numbering_level(
                        l,
                        &b::Operation::new().pass(),
                        &format!("{path}.levels[{i}]"),
                    )
                })
                .collect::<Result<_>>()?;
            ResourceEdit::Numbering(if kind == "addAbstract" {
                N::AddAbstract {
                    abstract_id,
                    levels,
                }
            } else {
                N::ReplaceAbstract {
                    abstract_id,
                    levels,
                }
            })
        }
        "abstractLevel" | "instanceLevel" => {
            let level = level(id("level")?)?;
            let value = if clear {
                None
            } else {
                Some(Box::new(crate::styles::numbering_level(
                    &v,
                    &b::Operation::new().pass(),
                    &vp,
                )?))
            };
            ResourceEdit::Numbering(if kind == "abstractLevel" {
                N::AbstractLevel {
                    abstract_id: id("abstractId")?,
                    level,
                    value,
                }
            } else {
                N::InstanceLevel {
                    num_id: id("numId")?,
                    level,
                    value,
                }
            })
        }
        "relinkInstance" => ResourceEdit::Numbering(N::RelinkInstance {
            num_id: id("numId")?,
            abstract_id: id("abstractId")?,
        }),
        "removeAbstract" => ResourceEdit::Numbering(N::RemoveAbstract {
            abstract_id: id("abstractId")?,
        }),
        "addInstance" => ResourceEdit::Numbering(N::AddInstance {
            num_id: id("numId")?,
            abstract_id: id("abstractId")?,
        }),
        "removeInstance" => ResourceEdit::Numbering(N::RemoveInstance {
            num_id: id("numId")?,
        }),
        "instanceStart" => ResourceEdit::Numbering(N::InstanceStart {
            num_id: id("numId")?,
            level: level(id("level")?)?,
            start: if clear { None } else { Some(uint(&v, &vp)?) },
        }),
        "paragraphNumbering" | "storyParagraphNumbering" => {
            let value = if clear {
                None
            } else {
                let r = Record::new(&v, &vp, &["id", "level"])?;
                Some(rocx::NumberingReference {
                    id: uint(&r.get("id")?, &r.path("id"))?,
                    level: level(uint(&r.get("level")?, &r.path("level"))?)?,
                })
            };
            ResourceEdit::Numbering(if kind == "paragraphNumbering" {
                N::Paragraph {
                    target: (*borrow_target::<ParagraphAnchor>(&edit, &edit.path("target"))?)
                        .clone(),
                    value,
                }
            } else {
                N::StoryParagraph {
                    target: (*borrow_target::<rocx::editor::StoryParagraphAnchor>(
                        &edit,
                        &edit.path("target"),
                    )?)
                    .clone(),
                    value,
                }
            })
        }
        "defaultAlignment" => ResourceEdit::DefaultAlignment(if clear {
            None
        } else {
            Some(alignment(&v, &vp)?)
        }),
        "abstractStart" => ResourceEdit::AbstractStart(AbstractStart {
            abstract_id: uint(&edit.get("abstractId")?, &edit.path("abstractId"))?,
            level: uint(&edit.get("level")?, &edit.path("level"))?
                .try_into()
                .map_err(|_| Failure::invalid(&edit.path("level"), "invalid level"))?,
            start: if clear { None } else { Some(uint(&v, &vp)?) },
        }),
        _ => {
            return Err(Failure::invalid(
                &edit.path("kind"),
                "unknown resource edit",
            ));
        }
    })
}

fn tracked_text(input: &Wire) -> Result<JsValue> {
    use rocx::editor::TrackedText::*;
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let arg = argument(&call, "edit")?;
    let edit = Record::new(&arg, "edit", &["target", "kind", "author", "date", "text"])?;
    let string = |k: &str| b::string(&edit.get(k)?, &edit.path(k));
    let target = crate::inspection::inline_anchor(&edit.get("target")?, "edit.target")?;
    let change = match string("kind")?.as_str() {
        "replace" => Replace(string("text")?),
        "insertAfter" => InsertAfter(string("text")?),
        "delete" if edit.get("text")?.js().is_undefined() => Delete,
        _ => {
            return Err(Failure::invalid(
                "edit.kind",
                "expected insertAfter/replace with text or delete without text",
            ));
        }
    };
    let date = if edit.get("date")?.js().is_undefined() {
        None
    } else {
        Some(string("date")?)
    };
    preview_handle(source.document().preview_tracked_text(
        &target,
        change,
        &string("author")?,
        date.as_deref(),
    )?)
}

fn tracked_bold(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let arg = argument(&call, "edit")?;
    let edit = Record::new(&arg, "edit", &["target", "bold", "author", "date"])?;
    let string = |k: &str| b::string(&edit.get(k)?, &edit.path(k));
    let date = (!edit.get("date")?.js().is_undefined())
        .then(|| string("date"))
        .transpose()?;
    preview_handle(
        source.document().preview_tracked_bold(
            &crate::inspection::inline_anchor(&edit.get("target")?, "edit.target")?,
            edit.get("bold")?
                .js()
                .as_bool()
                .ok_or_else(|| Failure::invalid("edit.bold", "expected boolean"))?,
            &string("author")?,
            date.as_deref(),
        )?,
    )
}

fn tracked_move(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let arg = argument(&call, "edit")?;
    let edit = Record::new(&arg, "edit", &["target", "after", "name", "author", "date"])?;
    let string = |k: &str| b::string(&edit.get(k)?, &edit.path(k));
    let anchor = |k: &str| crate::inspection::inline_anchor(&edit.get(k)?, &edit.path(k));
    preview_handle(source.document().preview_tracked_move(
        &anchor("target")?,
        &anchor("after")?,
        &string("name")?,
        &string("author")?,
        &string("date")?,
    )?)
}

fn tracked_paragraph(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let doc = borrow_target::<ImportedSource>(&call, "document")?;
    let arg = argument(&call, "edit")?;
    let edit = Record::new(&arg, "edit", &["target", "kind", "text", "author", "date"])?;
    let string = |k: &str| b::string(&edit.get(k)?, &edit.path(k));
    let date = (!edit.get("date")?.js().is_undefined())
        .then(|| string("date"))
        .transpose()?;
    let target = borrow_target::<ParagraphAnchor>(&edit, "edit.target")?;
    let change = match string("kind")?.as_str() {
        "insertBefore" => rocx::editor::TrackedParagraph::InsertBefore(string("text")?),
        "delete" if edit.get("text")?.js().is_undefined() => rocx::editor::TrackedParagraph::Delete,
        _ => {
            return Err(Failure::invalid(
                "edit.kind",
                "expected insertBefore with text or delete without text",
            ));
        }
    };
    preview_handle(doc.document().preview_tracked_paragraph(
        &target,
        change,
        &string("author")?,
        date.as_deref(),
    )?)
}

fn paragraph_revision(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let doc = borrow_target::<ImportedSource>(&call, "document")?;
    let arg = argument(&call, "edit")?;
    let edit = Record::new(&arg, "edit", &["target", "action"])?;
    let target = borrow_target::<ParagraphAnchor>(&edit, "edit.target")?;
    preview_handle(
        doc.document()
            .preview_paragraph_revision(&target, review_action(&edit)?)?,
    )
}

fn review_action(edit: &Record) -> Result<rocx::TextRevisionAction> {
    Ok(
        match b::string(&edit.get("action")?, "edit.action")?.as_str() {
            "accept" => rocx::TextRevisionAction::Accept,
            "reject" => rocx::TextRevisionAction::Reject,
            _ => return Err(Failure::invalid("edit.action", "expected accept or reject")),
        },
    )
}

fn revision(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let arg = argument(&call, "edit")?;
    let edit = Record::new(&arg, "edit", &["target", "action", "paired"])?;
    let anchor = |k: &str| crate::inspection::inline_anchor(&edit.get(k)?, &edit.path(k));
    let action = review_action(&edit)?;
    let paired = if edit.get("paired")?.js().is_undefined() {
        None
    } else {
        Some(anchor("paired")?)
    };
    preview_handle(source.document().preview_revision(
        &anchor("target")?,
        action,
        paired.as_ref(),
    )?)
}

fn preview_layout(input: &Wire) -> Result<JsValue> {
    use rocx::editor::{LayoutEdit, LayoutField, LayoutTab, LayoutValue, SectionAnchor};
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let arg = argument(&call, "edits")?;
    let integer = |v: &Wire, p: &str| -> Result<i32> {
        b::i64_number(v, p)?
            .try_into()
            .map_err(|_| Failure::invalid(p, "expected signed 32-bit integer"))
    };
    let edits = b::list(&arg, "edits")?
        .iter()
        .enumerate()
        .map(|(i, wire)| {
            let path = format!("edits[{i}]");
            let edit = Record::new(wire, &path, &["target", "field", "value"])?;
            let name = b::string(&edit.get("field")?, &edit.path("field"))?;
            let f = LayoutField::ALL
                .iter()
                .copied()
                .find(|f| f.name() == name)
                .ok_or_else(|| Failure::invalid(&edit.path("field"), "unknown layout field"))?;
            let wire = edit.get("value")?;
            let v = wire.js();
            let vp = edit.path("value");
            let val = if v.is_null() {
                None
            } else {
                Some(if f == LayoutField::TabStops {
                    LayoutValue::Tabs(
                        b::list(&wire, &vp)?
                            .iter()
                            .enumerate()
                            .map(|(j, t)| {
                                let p = format!("{vp}[{j}]");
                                let d = Record::new(t, &p, &["position", "alignment", "leader"])?;
                                let l = d.get("leader")?;
                                Ok(LayoutTab {
                                    position: integer(&d.get("position")?, &d.path("position"))?,
                                    alignment: b::string(
                                        &d.get("alignment")?,
                                        &d.path("alignment"),
                                    )?,
                                    leader: if l.js().is_undefined() {
                                        None
                                    } else {
                                        Some(b::string(&l, &d.path("leader"))?)
                                    },
                                })
                            })
                            .collect::<Result<_>>()?,
                    )
                } else if matches!(
                    f,
                    LayoutField::BorderTop
                        | LayoutField::BorderLeft
                        | LayoutField::BorderBottom
                        | LayoutField::BorderRight
                ) {
                    let d = Record::new(&wire, &vp, &["size", "space", "color"])?;
                    let num = |k: &str| -> Result<Option<i32>> {
                        let w = d.get(k)?;
                        if w.js().is_undefined() {
                            Ok(None)
                        } else {
                            integer(&w, &d.path(k)).map(Some)
                        }
                    };
                    let rgb = d.get("color")?;
                    LayoutValue::Border(rocx::editor::LayoutBorder {
                        size: num("size")?,
                        space: num("space")?,
                        color: if rgb.js().is_undefined() {
                            None
                        } else {
                            Some(b::string(&rgb, &d.path("color"))?)
                        },
                    })
                } else if let Some(b) = v.as_bool() {
                    LayoutValue::Boolean(b)
                } else if v.is_string() {
                    LayoutValue::Token(b::string(&wire, &vp)?)
                } else {
                    LayoutValue::Integer(integer(&wire, &vp)?)
                })
            };
            Ok(if f.is_section() {
                LayoutEdit::Section(
                    (*borrow_target::<SectionAnchor>(&edit, &edit.path("target"))?).clone(),
                    f,
                    val,
                )
            } else {
                LayoutEdit::Paragraph(
                    (*borrow_target::<ParagraphAnchor>(&edit, &edit.path("target"))?).clone(),
                    f,
                    val,
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    preview_handle(source.document().preview_layout(&edits)?)
}

fn wrap_control(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let edit = Record::new(&argument(&call, "edit")?, "edit", &["target", "tag"])?;
    let site = borrow_target::<rocx::editor::ControlSite>(&edit, "edit.target")?;
    preview_handle(
        source
            .document()
            .preview_wrap_control(&site, &b::string(&edit.get("tag")?, "edit.tag")?)?,
    )
}
fn replace_control(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let edit = Record::new(&argument(&call, "edit")?, "edit", &["target", "properties"])?;
    let anchor = borrow_target::<rocx::editor::ControlAnchor>(&edit, "edit.target")?;
    let props = crate::structured::content_control_properties(
        &edit.get("properties")?,
        &b::Operation::new().pass(),
        "edit.properties",
    )?;
    preview_handle(source.document().preview_replace_control(&anchor, &props)?)
}

fn unwrap_control(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let anchor = b::borrow::<rocx::editor::ControlAnchor>(&argument(&call, "target")?, "target")?;
    preview_handle(source.document().preview_unwrap_control(&anchor)?)
}
fn control_choice(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = borrow_target::<ImportedSource>(&call, "document")?;
    let edit = Record::new(
        &argument(&call, "edit")?,
        "edit",
        &["target", "selected", "choices"],
    )?;
    let anchor = borrow_target::<rocx::editor::ControlAnchor>(&edit, "edit.target")?;
    let choices = if edit.get("choices")?.js().is_undefined() {
        None
    } else {
        Some(edits(&edit, "choices", |v, path| {
            let item = Record::new(v, path, &["displayText", "value"])?;
            Ok(rocx::ContentControlListItem {
                display_text: b::string(&item.get("displayText")?, &item.path("displayText"))?,
                value: b::string(&item.get("value")?, &item.path("value"))?,
            })
        })?)
    };
    preview_handle(source.document().preview_control_choice(
        &anchor,
        &b::string(&edit.get("selected")?, "edit.selected")?,
        choices.as_deref(),
    )?)
}

fn wrapper_edit(input: &Wire, path: &str) -> Result<rocx::editor::WrapperEdit> {
    use rocx::editor::{ControlAnchor, ControlSite, WrapperEdit as E};
    let kind = b::string(
        &b::member(&input.js(), "kind", path)?,
        &b::child_path(path, "kind"),
    )?;
    let fields: &[&str] = match kind.as_str() {
        "wrap" => &["kind", "target", "last", "tag"],
        "unwrap" => &["kind", "target"],
        "define" => &["kind", "target", "properties"],
        "choice" => &["kind", "target", "selected", "choices"],
        "replaceInlines" => &["kind", "target", "inlines"],
        "replaceBlocks" => &["kind", "target", "blocks"],
        _ => return Err(Failure::invalid(path, "unknown wrapper edit")),
    };
    let r = Record::new(input, path, fields)?;
    let anchor = || borrow_target::<ControlAnchor>(&r, &r.path("target")).map(|a| (*a).clone());
    Ok(match kind.as_str() {
        "wrap" => E::Wrap {
            first: (*borrow_target::<ControlSite>(&r, &r.path("target"))?).clone(),
            last: if r.get("last")?.js().is_undefined() {
                None
            } else {
                Some((*b::borrow::<ControlSite>(&r.get("last")?, &r.path("last"))?).clone())
            },
            tag: b::string(&r.get("tag")?, &r.path("tag"))?,
        },
        "unwrap" => E::Unwrap(anchor()?),
        "define" => E::Define(
            anchor()?,
            crate::structured::content_control_properties(
                &r.get("properties")?,
                &b::Operation::new().pass(),
                &r.path("properties"),
            )?,
        ),
        "choice" => E::Choice {
            target: anchor()?,
            selected: b::string(&r.get("selected")?, &r.path("selected"))?,
            items: if r.get("choices")?.js().is_undefined() {
                None
            } else {
                Some(edits(&r, "choices", |v, p| {
                    let item = Record::new(v, p, &["displayText", "value"])?;
                    Ok(rocx::ContentControlListItem {
                        display_text: b::string(
                            &item.get("displayText")?,
                            &item.path("displayText"),
                        )?,
                        value: b::string(&item.get("value")?, &item.path("value"))?,
                    })
                })?)
            },
        },
        "replaceInlines" => E::ReplaceInlines(
            anchor()?,
            edits(&r, "inlines", |v, p| {
                Ok((*b::borrow::<rocx::ResolvedInline>(v, p)?).clone())
            })?,
        ),
        _ => E::ReplaceBlocks(
            anchor()?,
            edits(&r, "blocks", crate::structured::block_value)?,
        ),
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
            "textRanges",
            "textInsertions",
            "runRemovals",
            "paragraphInsertions",
            "cellAlignments",
            "blockEdits",
            "inlineEdits",
            "fields",
            "pairedRanges",
            "tableStructure",
            "styleEdits",
            "resources",
            "controls",
            "wrappers",
        ],
    )?;
    let preview = source.document().preview_edits(rocx::editor::EditBatch {
        alignments: &edits(&batch, "alignments", alignment_edit)?,
        tables: &edits(&batch, "tableInsertions", table_insertion)?,
        header_images: &edits(&batch, "headerImages", header_edit)?,
        run_text: &edits(&batch, "runText", run_text_edit)?,
        paragraphs: &edits(&batch, "paragraphInsertions", paragraph_insertion)?,
        cells: &edits(&batch, "cellAlignments", cell_alignment_edit)?,
        run_removals: &edits(&batch, "runRemovals", crate::inspection::inline_anchor)?,
        ranges: &edits(&batch, "textRanges", range_edit)?,
        text_insertions: &edits(&batch, "textInsertions", text_insertion)?,
        blocks: &edits(&batch, "blockEdits", block_edit)?,
        inline_edits: &edits(&batch, "inlineEdits", inline_edit)?,
        fields: &edits(&batch, "fields", field_edit)?,
        paired_ranges: &edits(&batch, "pairedRanges", paired_edit)?,
        structural: &edits(&batch, "tableStructure", structure_edit)?,
        styles: &edits(&batch, "styleEdits", style_edit)?,
        resources: &edits(&batch, "resources", resource_edit)?,
        controls: &edits(&batch, "controls", control_edit)?,
        wrappers: &edits(&batch, "wrappers", wrapper_edit)?,
    })?;
    preview_handle(preview)
}

fn preview_handle(preview: EditPreview) -> Result<JsValue> {
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
            ("inlineChanges", "editPreviewInlineChanges"),
            ("paragraphInsertions", "editPreviewParagraphInsertions"),
            ("headerChanges", "editPreviewHeaderChanges"),
            ("toDocx", "editPreviewToDocx"),
        ],
    ))
}

fn style_edit(input: &Wire, path: &str) -> Result<rocx::editor::StyleEdit> {
    use rocx::editor::{StyleAnchor, StyleEdit};
    let op = b::string(
        &b::member(&input.js(), "op", path)?,
        &b::child_path(path, "op"),
    )?;
    let fields: &[&str] = match op.as_str() {
        "add" => &["op", "definition"],
        "field" => &["op", "target", "region", "field", "value"],
        "region" => &["op", "target", "region", "format"],
        "patch" => &["op", "target", "bold", "italic"],
        "replace" => &["op", "target", "definition"],
        "remove" => &["op", "target"],
        "retarget" => &["op", "target", "replacement"],
        _ => return Err(Failure::invalid(path, "unknown style edit")),
    };
    let record = Record::new(input, path, fields)?;
    let target = || -> Result<_> {
        Ok((*borrow_target::<StyleAnchor>(&record, &record.path("target"))?).clone())
    };
    let definition = || {
        crate::styles::style_definition(
            &record.get("definition")?,
            &b::Operation::new().pass(),
            &record.path("definition"),
        )
    };
    let flag = |key| -> Result<_> {
        let value = record.get(key)?.js();
        if value.is_undefined() {
            Ok(None)
        } else {
            value
                .as_bool()
                .map(Some)
                .ok_or_else(|| Failure::invalid(&record.path(key), "expected a boolean"))
        }
    };
    Ok(match op.as_str() {
        "add" => StyleEdit::Add(definition()?),
        "field" => {
            let region = record.get("region")?;
            let value = record.get("value")?;
            StyleEdit::Field {
                target: target()?,
                region: (!region.js().is_undefined())
                    .then(|| b::string(&region, &record.path("region")))
                    .transpose()?,
                field: b::string(&record.get("field")?, &record.path("field"))?,
                value: (!value.js().is_null())
                    .then(|| b::string(&value, &record.path("value")))
                    .transpose()?,
            }
        }
        "region" => {
            let format = record.get("format")?;
            StyleEdit::Region {
                target: target()?,
                region: b::string(&record.get("region")?, &record.path("region"))?,
                format: (!format.js().is_null())
                    .then(|| {
                        crate::styles::table_region(
                            &format,
                            &b::Operation::new().pass(),
                            &record.path("format"),
                        )
                    })
                    .transpose()?
                    .map(Box::new),
            }
        }
        "replace" => StyleEdit::Replace {
            target: target()?,
            definition: definition()?,
        },
        "patch" => StyleEdit::Patch {
            target: target()?,
            bold: flag("bold")?,
            italic: flag("italic")?,
        },
        "retarget" => StyleEdit::Retarget {
            target: target()?,
            replacement: (*b::borrow::<StyleAnchor>(
                &record.get("replacement")?,
                &record.path("replacement"),
            )?)
            .clone(),
        },
        _ => StyleEdit::Remove(target()?),
    })
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

fn inline_changes(input: &Wire) -> Result<JsValue> {
    let preview = borrow_target::<EditPreview>(&call(input)?, "preview")?;
    Ok(array(preview.inline_changes().iter().map(
        |&(body, inline)| array([JsValue::from(body), JsValue::from(inline)]),
    )))
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
