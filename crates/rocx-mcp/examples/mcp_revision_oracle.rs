//! Independent byte oracle: call the native revision API, without MCP handles.
use std::{env, error::Error, fs};

use rocx::TextRevisionAction;
use rocx::editor::{
    BodyBlock, ImportedDocument, ImportedInlineNode, ImportedParagraph, InlineAnchor,
    TrackedParagraph, TrackedText,
};
use serde_json::json;

const AUTHOR: &str = "Ada & Co.";
const DATE: &str = "2030-02-03T04:05:06Z";
const TEXT: &str = "New 😀 & text";
const MOVE_NAME: &str = "Move1";

fn paragraph(doc: &ImportedDocument, body: usize) -> &ImportedParagraph {
    let BodyBlock::Paragraph(paragraph) = &doc.body()[body] else {
        panic!("expected body paragraph");
    };
    paragraph
}

fn inline(doc: &ImportedDocument, body: usize, index: usize) -> &InlineAnchor {
    match &paragraph(doc, body).inline_structure[index] {
        ImportedInlineNode::Run(run) => &run.anchor,
        ImportedInlineNode::Revision(revision) => &revision.anchor,
        _ => panic!("expected run or revision wrapper"),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [operation, output, source, rest @ ..] = args.as_slice() else {
        return Err("usage: mcp_revision_oracle OPERATION OUTPUT SOURCE [arguments]".into());
    };
    let doc = ImportedDocument::open(&fs::read(source)?)?;
    if operation == "inspect" && rest.is_empty() {
        let mut rows = vec![];
        for (body, block) in doc.body().iter().enumerate() {
            let BodyBlock::Paragraph(paragraph) = block else {
                continue;
            };
            if let Some(mark) = &paragraph.mark_revision {
                rows.push(json!({
                    "body_index": body, "inline_path": [], "kind": mark.kind,
                    "id": mark.id, "author": mark.author, "date": mark.date,
                }));
            }
            for (index, inline) in paragraph.inline_structure.iter().enumerate() {
                let (kind, id, author, date) = match inline {
                    ImportedInlineNode::Revision(r) => (r.kind, &r.id, &r.author, &r.date),
                    ImportedInlineNode::Run(run) => {
                        let Some(r) = &run.property_revision else {
                            continue;
                        };
                        (r.kind, &r.id, &r.author, &r.date)
                    }
                    _ => continue,
                };
                rows.push(json!({
                    "body_index": body, "inline_path": [index], "kind": kind,
                    "id": id, "author": author, "date": date,
                }));
            }
        }
        fs::write(output, serde_json::to_vec(&rows)?)?;
        return Ok(());
    }
    let action = |value: &str| match value {
        "accept" => TextRevisionAction::Accept,
        "reject" => TextRevisionAction::Reject,
        _ => panic!("invalid disposition"),
    };
    let coordinate =
        |body: &str, index: &str| inline(&doc, body.parse().unwrap(), index.parse().unwrap());
    let preview = match (operation.as_str(), rest) {
        ("insert-text-after", []) => doc.preview_tracked_text(
            inline(&doc, 1, 0),
            TrackedText::InsertAfter(TEXT.into()),
            AUTHOR,
            None,
        ),
        ("replace-text", []) => doc.preview_tracked_text(
            inline(&doc, 1, 0),
            TrackedText::Replace(TEXT.into()),
            AUTHOR,
            Some(DATE),
        ),
        ("replace-identical", []) => doc.preview_tracked_text(
            inline(&doc, 1, 0),
            TrackedText::Replace("Source".into()),
            AUTHOR,
            None,
        ),
        ("delete-text", []) => {
            doc.preview_tracked_text(inline(&doc, 1, 0), TrackedText::Delete, AUTHOR, None)
        }
        ("move-text", []) => doc.preview_tracked_move(
            inline(&doc, 1, 0),
            inline(&doc, 2, 0),
            MOVE_NAME,
            AUTHOR,
            DATE,
        ),
        ("set-bold", []) => doc.preview_tracked_bold(inline(&doc, 1, 0), false, AUTHOR, Some(DATE)),
        ("insert-paragraph-before", []) => doc.preview_tracked_paragraph(
            &paragraph(&doc, 1).anchor,
            TrackedParagraph::InsertBefore(TEXT.into()),
            AUTHOR,
            Some(DATE),
        ),
        ("delete-paragraph", []) => doc.preview_tracked_paragraph(
            &paragraph(&doc, 1).anchor,
            TrackedParagraph::Delete,
            AUTHOR,
            None,
        ),
        ("review", [disposition, body, index]) => {
            doc.preview_revision(coordinate(body, index), action(disposition), None)
        }
        ("review", [disposition, body, index, paired_body, paired_index]) => doc.preview_revision(
            coordinate(body, index),
            action(disposition),
            Some(coordinate(paired_body, paired_index)),
        ),
        ("review-paragraph", [disposition, body]) => doc.preview_paragraph_revision(
            &paragraph(&doc, body.parse()?).anchor,
            action(disposition),
        ),
        _ => return Err("invalid oracle operation or arguments".into()),
    }?;
    fs::write(output, preview.to_docx()?)?;
    Ok(())
}
