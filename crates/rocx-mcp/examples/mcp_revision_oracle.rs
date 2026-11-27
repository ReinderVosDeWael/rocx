//! Independent native revision and diagnostic oracle, without MCP handles.
use std::{env, error::Error, fs};

use rocx::editor::{
    BodyBlock, ImportedDocument, ImportedInline, ImportedInlineNode, ImportedParagraph,
    ImportedPropertyRevision, ImportedRevision, ImportedRun, InlineAnchor, TrackedParagraph,
    TrackedText,
};
use rocx::{BreakType, Rule, TextRevisionAction};
use serde_json::{Value, json};

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

fn property(revision: &ImportedPropertyRevision, kind: &str) -> Value {
    json!({"type": kind, "kind": revision.kind, "id": revision.id,
           "author": revision.author, "date": revision.date})
}

fn run_value(run: &ImportedRun) -> Value {
    let mut text = String::new();
    let mut opaque = false;
    let items: Vec<_> = run
        .items
        .iter()
        .map(|item| {
            match item {
                ImportedInline::Text(value) => text.push_str(value),
                ImportedInline::Tab => text.push('\t'),
                ImportedInline::Break(BreakType::TextWrapping | BreakType::Line) => text.push('\n'),
                _ => opaque = true,
            }
            match item {
                ImportedInline::Text(text) => json!({"type": "text", "text": text}),
                ImportedInline::Tab => json!({"type": "tab"}),
                ImportedInline::Break(kind) => {
                    json!({"type": "break", "kind": format!("{kind:?}")})
                }
                ImportedInline::NoteReference { kind, id } => {
                    json!({"type": "note-reference", "kind": format!("{kind:?}"), "id": id})
                }
                ImportedInline::Opaque(name) => json!({"type": "opaque", "name": name}),
                other => panic!("unqualified item: {other:?}"),
            }
        })
        .collect();
    json!({"type": "run", "text": text, "items": items, "bold": run.bold,
           "has_opaque_content": opaque, "has_opaque_formatting": run.has_opaque_formatting})
}

fn wrapper(r: &ImportedRevision) -> Value {
    json!({"type": "inline-wrapper", "kind": r.kind, "id": r.id,
           "author": r.author, "date": r.date, "children": child_values(&r.children)})
}

fn child_values(children: &[ImportedInlineNode]) -> Vec<Value> {
    children
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let mut value = match node {
                ImportedInlineNode::Run(run) => run_value(run),
                ImportedInlineNode::Revision(r) => wrapper(r),
                ImportedInlineNode::Hyperlink(link) => {
                    json!({"type": "hyperlink", "children": child_values(&link.children)})
                }
                ImportedInlineNode::Fraction {
                    numerator,
                    denominator,
                    ..
                } => {
                    json!({"type": "fraction", "numerator": numerator, "denominator": denominator})
                }
                ImportedInlineNode::Opaque(name) => json!({"type": "opaque", "name": name}),
                other => panic!("unqualified child: {other:?}"),
            };
            value["inline_index"] = json!(i);
            value
        })
        .collect()
}

fn revision_rows(
    rows: &mut Vec<Value>,
    body: usize,
    prefix: &[usize],
    children: &[ImportedInlineNode],
) {
    for (index, child) in children.iter().enumerate() {
        let mut path = prefix.to_vec();
        path.push(index);
        let mut row = match child {
            ImportedInlineNode::Revision(r) => wrapper(r),
            ImportedInlineNode::Run(run) => {
                let Some(r) = &run.property_revision else {
                    continue;
                };
                let mut value = property(r, "run-properties");
                value["run"] = run_value(run);
                value
            }
            ImportedInlineNode::Hyperlink(link) => {
                revision_rows(rows, body, &path, &link.children);
                continue;
            }
            _ => continue,
        };
        row["body_index"] = json!(body);
        row["inline_path"] = json!(path);
        rows.push(row);
        if let ImportedInlineNode::Revision(r) = child {
            revision_rows(rows, body, &path, &r.children);
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [operation, output, source, rest @ ..] = args.as_slice() else {
        return Err("usage: mcp_revision_oracle OPERATION OUTPUT SOURCE [arguments]".into());
    };
    let bytes = fs::read(source)?;
    if operation == "invalid-member-crc" && rest.is_empty() {
        let Err(error) = ImportedDocument::open(&bytes) else {
            panic!("opening the archive with a bad member CRC must fail");
        };
        assert!(matches!(&error, rocx::Error::Io(_)), "{error:?}");
        assert_eq!(error.rule(), Some(Rule::Io));
        fs::write(
            output,
            serde_json::to_vec(&json!({
                "rule": Rule::Io.code(), "path": error.path(), "message": error.to_string(),
            }))?,
        )?;
        return Ok(());
    }
    let doc = ImportedDocument::open(&bytes)?;
    if operation == "inspect" && rest.is_empty() {
        let mut rows = vec![];
        for (body, block) in doc.body().iter().enumerate() {
            let BodyBlock::Paragraph(paragraph) = block else {
                continue;
            };
            if let Some(mark) = &paragraph.mark_revision {
                let mut value = property(mark, "paragraph-mark");
                value["body_index"] = json!(body);
                value["inline_path"] = json!([]);
                rows.push(value);
            }
            revision_rows(&mut rows, body, &[], &paragraph.inline_structure);
        }
        fs::write(output, serde_json::to_vec(&rows)?)?;
        return Ok(());
    }
    if operation == "opaque-review" && rest.is_empty() {
        let error = doc
            .preview_revision(inline(&doc, 1, 1), TextRevisionAction::Accept, None)
            .expect_err("opaque revision must be rejected");
        fs::write(output, error.to_string())?;
        return Ok(());
    }
    if operation == "invalid-xml-author" && rest.is_empty() {
        let error = doc
            .preview_tracked_text(
                inline(&doc, 0, 0),
                TrackedText::Replace("Updated".into()),
                "é😀\0",
                None,
            )
            .expect_err("invalid XML author must be rejected");
        let rocx::Error::InvalidXmlCharacter {
            character,
            byte_offset,
        } = &error
        else {
            panic!("expected the native XML character error, got {error:?}");
        };
        fs::write(
            output,
            serde_json::to_vec(&json!({
                "character": character, "byte_offset": byte_offset,
                "rule": error.rule().map(rocx::Rule::code),
                "path": error.path(), "message": error.to_string(),
            }))?,
        )?;
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
