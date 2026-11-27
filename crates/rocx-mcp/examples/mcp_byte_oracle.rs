//! Direct core byte and report oracle for the independent fixtures in `tests/mcp`.
use rocx::{
    Alignment, BreakType, Component, Document, HeaderFooterType, ImageRun, Paragraph, Section,
    Table, TableCell, TableRow, VerticalAlignment,
    editor::{
        BlockEdit, BodyBlock, CellAlignmentEdit, EditBatch, EditPreview, EntryEdit, HeaderEditMode,
        HeaderImageEdit, ImportedDocument, ImportedInline, ImportedInlineNode, ImportedParagraph,
        ImportedStoryBlock, InlineAction, InlineEdit, ParagraphAlignmentEdit, ParagraphInsertion,
        RunTextEdit, TableInsertion, TextInsertion, TextRangeEdit,
    },
};
use std::{fs, future::Future, path::Path, task::Poll};

// These fixture declarations resolve without waiting on an external dependency.
fn resolve_now<T>(future: impl Future<Output = T>) -> T {
    match std::pin::pin!(future).poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
    {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("fixture unexpectedly requires an executor"),
    }
}

fn mixed(source: &Path, png: &Path, mode: HeaderEditMode) -> EditPreview {
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let paragraph = |index| -> &ImportedParagraph {
        let BodyBlock::Paragraph(p) = &doc.body()[index] else {
            panic!("expected fixture paragraph");
        };
        p
    };
    let run = |body, inline: usize| {
        let ImportedInlineNode::Run(run) = &paragraph(body).inline_structure[inline] else {
            panic!("expected fixture run");
        };
        run.anchor.clone()
    };
    let bold = |body, inline| InlineEdit {
        target: run(body, inline),
        action: InlineAction::SetBold(true),
    };
    let BodyBlock::Table(table) = &doc.body()[4] else {
        panic!("expected fixture table");
    };
    let header = doc
        .stories()
        .unwrap()
        .into_iter()
        .find(|s| {
            s.kind.as_str() == "header"
                && s.variant == HeaderFooterType::Default
                && s.section_index == 0
        })
        .unwrap();
    let inserted_table = Table::new(vec![
        TableRow::new(
            ["new A", "new B"]
                .into_iter()
                .map(|s| TableCell::new(vec![Paragraph::new(s).into()]).into())
                .collect(),
        )
        .into(),
    ]);
    doc.preview_edits(EditBatch {
        alignments: &[
            ParagraphAlignmentEdit {
                target: paragraph(0).anchor.clone(),
                alignment: Alignment::Center,
            },
            ParagraphAlignmentEdit {
                target: paragraph(7).anchor.clone(),
                alignment: Alignment::Right,
            },
        ],
        paragraphs: &[ParagraphInsertion {
            target: paragraph(3).anchor.clone(),
            paragraph: resolve_now(Paragraph::new("Inserted & placed").resolve())
                .unwrap()
                .unwrap(),
        }],
        tables: &[TableInsertion {
            target: paragraph(1).anchor.clone(),
            table: resolve_now(inserted_table.resolve()).unwrap().unwrap(),
        }],
        run_text: &[
            RunTextEdit {
                target: run(0, 0),
                text: " Edited ".into(),
            },
            RunTextEdit {
                target: run(7, 0),
                text: "Unchanged".into(),
            },
        ],
        run_removals: &[run(3, 1)],
        cells: &[
            CellAlignmentEdit {
                target: table.anchor.clone(),
                row_index: 0,
                cell_index: 1,
                alignment: VerticalAlignment::Bottom,
            },
            CellAlignmentEdit {
                target: table.anchor.clone(),
                row_index: 0,
                cell_index: 0,
                alignment: VerticalAlignment::Top,
            },
        ],
        blocks: &[
            BlockEdit::Remove(paragraph(6).anchor.clone()),
            BlockEdit::Remove(paragraph(5).anchor.clone()),
            BlockEdit::KeepWithNext(paragraph(2).anchor.clone(), true),
        ],
        inline_edits: &[bold(3, 0), bold(1, 0), bold(7, 1)],
        header_images: &[HeaderImageEdit {
            target: header.anchor.unwrap(),
            image: resolve_now(ImageRun::new(fs::read(png).unwrap()).resolve())
                .unwrap()
                .unwrap()
                .image()
                .unwrap()
                .clone(),
            mode,
        }],
        ..EditBatch::default()
    })
    .unwrap()
}

fn report_variants(source: &Path) -> EditPreview {
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let paragraph = |index| -> &ImportedParagraph {
        let BodyBlock::Paragraph(paragraph) = &doc.body()[index] else {
            panic!("expected report fixture paragraph");
        };
        paragraph
    };
    let run = |body, inline: usize| {
        let ImportedInlineNode::Run(run) = &paragraph(body).inline_structure[inline] else {
            panic!("expected report fixture run");
        };
        run.anchor.clone()
    };
    let BodyBlock::Table(table) = &doc.body()[5] else {
        panic!("expected report fixture table");
    };
    let alignments = [
        (10, Alignment::Right), // No-op.
        (9, Alignment::ThaiJustify),
        (8, Alignment::JustifyLow),
        (7, Alignment::JustifyHigh),
        (6, Alignment::JustifyMedium),
        (4, Alignment::Distribute),
        (3, Alignment::Justify),
        (2, Alignment::Right),
        (1, Alignment::Center),
        (0, Alignment::Left),
    ]
    .map(|(body, alignment)| ParagraphAlignmentEdit {
        target: paragraph(body).anchor.clone(),
        alignment,
    });
    let cells = [
        (1, 2, VerticalAlignment::Bottom), // No-op.
        (1, 1, VerticalAlignment::Both),
        (1, 0, VerticalAlignment::Bottom),
        (0, 2, VerticalAlignment::Center),
        (0, 0, VerticalAlignment::Top),
    ]
    .map(|(row_index, cell_index, alignment)| CellAlignmentEdit {
        target: table.anchor.clone(),
        row_index,
        cell_index,
        alignment,
    });
    let tables = [
        (8, &[&["wide A", "wide B", "wide C"][..]][..]),
        (0, &[&["tall A"][..], &["tall B"][..]][..]),
    ]
    .map(|(body, rows)| TableInsertion {
        target: paragraph(body).anchor.clone(),
        table: resolve_now(
            Table::new(
                rows.iter()
                    .map(|row| {
                        TableRow::new(
                            row.iter()
                                .map(|&text| {
                                    TableCell::new(vec![Paragraph::new(text).into()]).into()
                                })
                                .collect(),
                        )
                        .into()
                    })
                    .collect(),
            )
            .resolve(),
        )
        .unwrap()
        .unwrap(),
    });
    let run_text = [
        (7, 0, "from empty"),
        (0, 4, "Keep"), // No-op.
        (0, 3, ""),
        (0, 1, " new 😀 & "),
    ]
    .map(|(body, inline, text)| RunTextEdit {
        target: run(body, inline),
        text: text.into(),
    });
    doc.preview_edits(EditBatch {
        alignments: &alignments,
        cells: &cells,
        tables: &tables,
        run_text: &run_text,
        run_removals: &[run(7, 1), run(0, 2), run(0, 0)],
        text_insertions: &[TextInsertion {
            target: paragraph(9).anchor.clone(),
            position: 0,
            text: "runless".into(),
        }],
        ..EditBatch::default()
    })
    .unwrap()
}

// Use only the public native getters; never call the MCP report projector.
fn mixed_report(preview: &EditPreview) -> serde_json::Value {
    use serde_json::{Value, json};

    fn records<T>(items: &[T], record: impl FnMut(&T) -> Value) -> Vec<Value> {
        items.iter().map(record).collect()
    }

    json!({
        "changes": records(preview.changes(), |c| json!({
            "target": c.body_index, "before": c.before, "after": format!("{:?}", c.after),
        })),
        "table_insertions": records(preview.insertions(), |c| json!({
            "target": c.after_body_index, "rows": c.rows, "columns": c.columns,
        })),
        "paragraph_insertions": preview.paragraph_insertions(),
        "block_changes": preview.block_changes(),
        "inline_changes": preview.inline_changes(),
        "header_images": records(preview.header_changes(), |c| json!({
            "source_part": c.source_part, "output_part": c.output_part,
            "mode": match c.mode {
                HeaderEditMode::Shared => "shared",
                HeaderEditMode::SectionLocal => "section-local",
            },
            "affected_sections": c.affected_sections, "restored_section": c.restored_section,
        })),
        "run_text": records(preview.run_text_changes(), |c| json!({
            "target": c.body_index, "inline_index": c.inline_index,
            "before": c.before, "after": c.after,
        })),
        "run_removals": records(preview.run_removals(), |c| json!({
            "target": c.body_index, "inline_index": c.inline_index, "before": c.before,
        })),
        "cell_alignments": records(preview.cell_alignment_changes(), |c| json!({
            "target": c.body_index, "row_index": c.row_index, "cell_index": c.cell_index,
            "before": c.before,
            "after": match c.after {
                VerticalAlignment::Top => "top",
                VerticalAlignment::Center => "center",
                VerticalAlignment::Bottom => "bottom",
                VerticalAlignment::Both => "both",
            },
        })),
    })
}

fn scalar(source: &Path) -> Vec<u8> {
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let [
        BodyBlock::Paragraph(formatted),
        BodyBlock::Paragraph(empty),
        ..,
    ] = doc.body()
    else {
        panic!("expected formatted and runless fixture paragraphs");
    };
    doc.preview_edits(EditBatch {
        ranges: &[
            TextRangeEdit {
                target: formatted.anchor.clone(),
                start: 0,
                end: 1,
                text: String::new(),
            },
            TextRangeEdit {
                target: formatted.anchor.clone(),
                start: 4,
                end: 7,
                text: "🦊&".into(),
            },
        ],
        text_insertions: &[
            TextInsertion {
                target: formatted.anchor.clone(),
                position: 3,
                text: " + ".into(),
            },
            TextInsertion {
                target: empty.anchor.clone(),
                position: 0,
                text: "runless".into(),
            },
            TextInsertion {
                target: formatted.anchor.clone(),
                position: 8,
                text: "!".into(),
            },
        ],
        ..EditBatch::default()
    })
    .unwrap()
    .to_docx()
    .unwrap()
}

fn entry(source: &Path, case: &str) -> Vec<u8> {
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let footnote = "word/notes/footnotes.xml";
    let endnote = "word/notes/endnotes.xml";
    let comment = "word/comments.xml";
    let (owner, id, edit) = match case {
        "delete-unused" => (footnote, "8", EntryEdit::DeleteUnusedNote),
        "replace-first" => (
            footnote,
            "007",
            EntryEdit::ReplaceParagraph {
                index: 0,
                text: "New footnote 😀".into(),
            },
        ),
        "insert-last" => (
            endnote,
            "007",
            EntryEdit::InsertParagraph {
                index: 2,
                text: "New endnote & tail".into(),
            },
        ),
        "remove-last" => (endnote, "007", EntryEdit::RemoveParagraph(1)),
        "author" => (comment, "0", EntryEdit::CommentAuthor("Café & Co.".into())),
        "same-author" => (comment, "0", EntryEdit::CommentAuthor("Original".into())),
        "initials-set" => (comment, "0", EntryEdit::CommentInitials(Some("CC".into()))),
        "initials-clear" => (comment, "0", EntryEdit::CommentInitials(None)),
        "date-set" => (
            comment,
            "0",
            EntryEdit::CommentDate(Some("2030-02-03T04:05:06Z".into())),
        ),
        "date-clear" => (comment, "0", EntryEdit::CommentDate(None)),
        _ => panic!("unknown entry fixture case: {case}"),
    };
    doc.edit_entry(owner, id, edit).unwrap()
}

fn inspect_entries(source: &Path) -> Vec<u8> {
    use serde_json::json;

    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let entries: Vec<_> = doc
        .entry_stories()
        .unwrap()
        .iter()
        .map(|entry| {
            let blocks: Vec<_> = entry
                .blocks()
                .iter()
                .enumerate()
                .map(|(index, block)| match block {
                    ImportedStoryBlock::Paragraph(items) => {
                        let text_items: Vec<_> = items
                            .iter()
                            .filter_map(|item| match item {
                                ImportedInline::Text(text) => Some(text.as_str()),
                                ImportedInline::Tab => Some("\t"),
                                ImportedInline::Break(
                                    BreakType::Line | BreakType::TextWrapping,
                                ) => Some("\n"),
                                _ => None,
                            })
                            .collect();
                        let ordered: Vec<_> = items
                            .iter()
                            .map(|item| match item {
                                ImportedInline::Text(text) => json!({"kind": "text", "text": text}),
                                ImportedInline::Tab => json!({"kind": "tab"}),
                                ImportedInline::Break(kind) => json!({
                                    "kind": "break", "break_type": match kind {
                                        BreakType::Line => "line",
                                        BreakType::TextWrapping => "text-wrapping",
                                        BreakType::Page => "page",
                                        BreakType::Column => "column",
                                    }
                                }),
                                ImportedInline::NoteReference { kind, id } => json!({
                                    "kind": "note-reference", "note_kind": match kind {
                                        rocx::NoteKind::Footnote => "footnote",
                                        rocx::NoteKind::Endnote => "endnote",
                                    }, "id": id,
                                }),
                                ImportedInline::Opaque(name) => {
                                    json!({"kind": "opaque", "name": name})
                                }
                                _ => panic!("unaccounted-for native inline item"),
                            })
                            .collect();
                        json!({
                            "index": index, "type": "paragraph", "text": text_items.concat(),
                            "has_opaque_content": text_items.len() != items.len(),
                            "inline_items": ordered,
                        })
                    }
                    ImportedStoryBlock::Opaque(name) => {
                        json!({"index": index, "type": "opaque", "name": name})
                    }
                    _ => panic!("unexpected block in the entry fixture"),
                })
                .collect();
            json!({
                "kind": entry.kind.as_str(), "part_name": entry.part_name, "id": entry.id,
                "note_type": entry.note_type, "author": entry.author, "date": entry.date,
                "initials": entry.initials, "blocks": blocks,
            })
        })
        .collect();
    serde_json::to_vec(&entries).unwrap()
}

fn authored(timestamp: &str, paragraphs: &[String]) -> Vec<u8> {
    let blocks = if paragraphs.is_empty() {
        vec![Paragraph::new("").into()]
    } else {
        paragraphs
            .iter()
            .map(|text| Paragraph::new(text.as_str()).into())
            .collect()
    };
    resolve_now(
        Document::new(vec![Section::new(blocks).into()])
            .with_timestamp(Some(timestamp.to_owned()))
            .resolve(),
    )
    .unwrap()
    .to_docx()
    .unwrap()
}

fn story_image(source: &Path, png: &Path, slot: &str, mode: &str) -> Vec<u8> {
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let (kind, variant) = slot.split_once(':').unwrap();
    let variant = match variant {
        "default" => HeaderFooterType::Default,
        "first" => HeaderFooterType::First,
        "even" => HeaderFooterType::Even,
        _ => panic!("invalid story variant"),
    };
    let target = doc
        .stories()
        .unwrap()
        .into_iter()
        .find(|s| s.section_index == 1 && s.kind.as_str() == kind && s.variant == variant)
        .unwrap()
        .anchor
        .unwrap();
    let image = resolve_now(ImageRun::new(fs::read(png).unwrap()).resolve())
        .unwrap()
        .unwrap()
        .image()
        .unwrap()
        .clone();
    let edit = HeaderImageEdit {
        target,
        mode: match mode {
            "shared" => HeaderEditMode::Shared,
            "section-local" => HeaderEditMode::SectionLocal,
            _ => panic!("invalid story image mode"),
        },
        image,
    };
    doc.preview_header_image(&edit).unwrap().to_docx().unwrap()
}

fn entry_runs(source: &Path, kind: &str) -> Vec<u8> {
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let entries = doc.entry_stories().unwrap();
    let ids: &[&str] = match kind {
        "footnote" => &["007", "8"],
        "endnote" => &["007"],
        "comment" => &["0"],
        _ => panic!("invalid fixture entry kind"),
    };
    let selected: Vec<_> = ids
        .iter()
        .map(|id| {
            entries
                .iter()
                .find(|entry| entry.kind.as_str() == kind && entry.id == *id)
                .unwrap()
        })
        .collect();
    let edits: Vec<_> = selected
        .iter()
        .map(|entry| {
            // Each fixture begins with a note mark or comment annotation reference.
            let Some(ImportedInlineNode::Run(run)) = entry.inline_structure(0).unwrap().get(1)
            else {
                panic!("expected fixture entry run");
            };
            RunTextEdit {
                target: run.anchor.clone(),
                text: format!("Revised {kind} {}", entry.id),
            }
        })
        .collect();
    doc.edit_story_text(&selected[0].part_name, &edits).unwrap()
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let (output, bytes) = match args.as_slice() {
        [command, output, timestamp, paragraphs @ ..] if command == "create" => {
            (output, authored(timestamp, paragraphs))
        }
        [command, output, source, png, mode] if command == "mixed" => {
            let mode = match mode.as_str() {
                "shared" => HeaderEditMode::Shared,
                "section-local" => HeaderEditMode::SectionLocal,
                _ => panic!("invalid header mode"),
            };
            let preview = mixed(Path::new(source), Path::new(png), mode);
            println!("{}", mixed_report(&preview));
            (output, preview.to_docx().unwrap())
        }
        [command, output, source] if command == "report-variants" => {
            let preview = report_variants(Path::new(source));
            println!("{}", mixed_report(&preview));
            (output, preview.to_docx().unwrap())
        }
        [command, output, source, png, slot, mode] if command == "story-image" => (
            output,
            story_image(Path::new(source), Path::new(png), slot, mode),
        ),
        [command, output, source, kind] if command == "entry-runs" => {
            (output, entry_runs(Path::new(source), kind))
        }
        [command, output, source] if command == "scalar" => (output, scalar(Path::new(source))),
        [command, output, source, case] if command == "entry" => {
            (output, entry(Path::new(source), case))
        }
        [command, output, source] if command == "inspect-entries" => {
            (output, inspect_entries(Path::new(source)))
        }
        [command, output, source] if command == "no-op" => {
            let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
            (
                output,
                doc.preview_edits(EditBatch::default())
                    .unwrap()
                    .to_docx()
                    .unwrap(),
            )
        }
        [command, output, source, count, member, total] if command == "open-with-limits" => {
            let limits = rocx::opc::ReadLimits {
                max_parts: count.parse().unwrap(),
                max_part_bytes: member.parse().unwrap(),
                max_expanded_bytes: total.parse().unwrap(),
            };
            let input = fs::read(source).unwrap();
            let result = match ImportedDocument::open_with_limits(&input, limits) {
                Ok(doc) => serde_json::json!({"blocks": doc.body().len()}),
                Err(error) => serde_json::json!({"error": error.to_string()}),
            };
            (output, serde_json::to_vec(&result).unwrap())
        }
        _ => panic!(
            "usage: mcp_byte_oracle create OUTPUT TIMESTAMP [TEXT ...] | mixed OUTPUT SOURCE PNG MODE | report-variants OUTPUT SOURCE | story-image OUTPUT SOURCE PNG SLOT MODE | entry-runs OUTPUT SOURCE KIND | scalar OUTPUT SOURCE | entry OUTPUT SOURCE CASE | inspect-entries OUTPUT SOURCE | no-op OUTPUT SOURCE | open-with-limits OUTPUT SOURCE COUNT MEMBER TOTAL"
        ),
    };
    fs::write(output, bytes).unwrap();
}
