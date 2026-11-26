//! Direct core byte oracle for the independent fixtures in `tests/mcp`.
use rocx::{
    Alignment, Component, Document, HeaderFooterType, ImageRun, Paragraph, Section, Table,
    TableCell, TableRow, VerticalAlignment,
    editor::{
        BodyBlock, CellAlignmentEdit, EditBatch, HeaderEditMode, HeaderImageEdit, ImportedDocument,
        ImportedInlineNode, ImportedParagraph, ParagraphAlignmentEdit, ParagraphInsertion,
        RunTextEdit, TableInsertion,
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
        _ => panic!(
            "usage: mcp_byte_oracle create OUTPUT TIMESTAMP [TEXT ...] | mixed OUTPUT SOURCE PNG MODE"
        ),
    };
    fs::write(output, bytes).unwrap();
}
