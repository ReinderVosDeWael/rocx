//! Direct core byte oracle for the independent fixtures in `tests/mcp`.
use rocx::{
    Alignment, BreakType, Component, Document, HeaderFooterType, ImageRun, Paragraph, Section,
    Table, TableCell, TableRow, VerticalAlignment,
    editor::{
        BodyBlock, CellAlignmentEdit, EditBatch, EntryEdit, HeaderEditMode, HeaderImageEdit,
        ImportedDocument, ImportedInline, ImportedInlineNode, ImportedParagraph,
        ImportedStoryBlock, ParagraphAlignmentEdit, ParagraphInsertion, RunTextEdit,
        TableInsertion, TextInsertion, TextRangeEdit,
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

fn mixed(source: &Path, png: &Path, mode: HeaderEditMode) -> Vec<u8> {
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let paragraph = |index| -> &ImportedParagraph {
        let BodyBlock::Paragraph(p) = &doc.body()[index] else {
            panic!("expected fixture paragraph");
        };
        p
    };
    let title = paragraph(0);
    let tail = paragraph(3);
    let ImportedInlineNode::Run(title_run) = &title.inline_structure[0] else {
        panic!("expected fixture title run");
    };
    let ImportedInlineNode::Run(drop_run) = &tail.inline_structure[1] else {
        panic!("expected fixture drop run");
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
        alignments: &[ParagraphAlignmentEdit {
            target: title.anchor.clone(),
            alignment: Alignment::Center,
        }],
        paragraphs: &[ParagraphInsertion {
            target: tail.anchor.clone(),
            paragraph: resolve_now(Paragraph::new("Inserted & placed").resolve())
                .unwrap()
                .unwrap(),
        }],
        tables: &[TableInsertion {
            target: paragraph(1).anchor.clone(),
            table: resolve_now(inserted_table.resolve()).unwrap().unwrap(),
        }],
        run_text: &[RunTextEdit {
            target: title_run.anchor.clone(),
            text: " Edited ".into(),
        }],
        run_removals: std::slice::from_ref(&drop_run.anchor),
        cells: &[CellAlignmentEdit {
            target: table.anchor.clone(),
            row_index: 0,
            cell_index: 1,
            alignment: VerticalAlignment::Bottom,
        }],
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
    .to_docx()
    .unwrap()
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
                        json!({
                            "index": index, "type": "paragraph", "text": text_items.concat(),
                            "has_opaque_content": text_items.len() != items.len(),
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
            (output, mixed(Path::new(source), Path::new(png), mode))
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
            "usage: mcp_byte_oracle create OUTPUT TIMESTAMP [TEXT ...] | mixed OUTPUT SOURCE PNG MODE | scalar OUTPUT SOURCE | entry OUTPUT SOURCE CASE | inspect-entries OUTPUT SOURCE | no-op OUTPUT SOURCE | open-with-limits OUTPUT SOURCE COUNT MEMBER TOTAL"
        ),
    };
    fs::write(output, bytes).unwrap();
}
