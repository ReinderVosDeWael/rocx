//! Direct core oracle: owners, original coordinates, and edits are fixture constants.
use rocx::editor::{
    BodyBlock, ImportedDocument, ImportedInline, ImportedInlineNode, NoteReferenceEdit,
};
use serde_json::json;
use std::{env, fs};

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let [case, source, output] = args.as_slice() else {
        panic!("usage: mcp_note_oracle CASE SOURCE OUTPUT");
    };
    use NoteReferenceEdit::{Attach, Create, Delete, Detach, Renumber, Retarget};
    let source = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    if case == "inspect" {
        let mut rows = Vec::new();
        for (body_index, block) in source.body().iter().enumerate() {
            let BodyBlock::Paragraph(paragraph) = block else {
                continue;
            };
            for (inline_index, node) in paragraph.inline_structure.iter().enumerate() {
                let ImportedInlineNode::Run(run) = node else {
                    continue;
                };
                let mut row = json!({"body_index": body_index, "inline_index": inline_index});
                match run.items.as_slice() {
                    [ImportedInline::NoteReference { kind, id }] => {
                        row["kind"] = json!("reference");
                        row["note_kind"] = json!(kind.as_str());
                        row["id"] = json!(id);
                    }
                    [ImportedInline::Text(_) | ImportedInline::Tab | ImportedInline::Break(_)] => {
                        row["kind"] = json!("insertion-candidate");
                    }
                    _ => continue,
                }
                rows.push(row);
            }
        }
        fs::write(output, serde_json::to_vec(&rows).unwrap()).unwrap();
        return;
    }
    let footnotes = "word/notes/footnotes.xml";
    let endnotes = "word/notes/endnotes.xml";
    let create = || Create {
        id: 13,
        text: "New note 😀 & tail".into(),
    };
    let (owner, inline_index, edit) = match case.as_str() {
        "create" | "reserved-only" | "relocated-empty-endnote" => (endnotes, 0, create()),
        "first-part" | "empty-owner" => ("word/footnotes.xml", 0, create()),
        "relocated-empty-footnote" => (footnotes, 0, create()),
        "attach" => (footnotes, 3, Attach(8)),
        "detach" => (footnotes, 4, Detach),
        "delete" => (footnotes, 4, Delete),
        "retarget" => (footnotes, 4, Retarget(8)),
        "missing-target" => (footnotes, 4, Retarget(99)),
        "renumber" => (endnotes, 5, Renumber(12)),
        _ => panic!("unknown fixture case {case}"),
    };
    let BodyBlock::Paragraph(paragraph) = &source.body()[0] else {
        panic!("expected the first direct paragraph");
    };
    let ImportedInlineNode::Run(run) = &paragraph.inline_structure[inline_index] else {
        panic!("expected a direct run at fixture index {inline_index}");
    };
    let result = source.edit_note_reference(owner, &run.anchor, edit);
    let bytes = if case == "missing-target" {
        result
            .expect_err("unknown note ID must be rejected")
            .to_string()
            .into_bytes()
    } else {
        result.unwrap()
    };
    fs::write(output, bytes).unwrap();
}
