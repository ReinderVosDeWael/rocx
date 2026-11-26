//! Direct core oracle: owners, original coordinates, and edits are fixture constants.
use rocx::editor::{BodyBlock, ImportedDocument, ImportedInlineNode, NoteReferenceEdit};
use std::{env, fs};

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let [case, source, output] = args.as_slice() else {
        panic!("usage: mcp_note_oracle CASE SOURCE OUTPUT");
    };
    use NoteReferenceEdit::{Attach, Create, Delete, Detach, Renumber, Retarget};
    let footnotes = "word/notes/footnotes.xml";
    let endnotes = "word/notes/endnotes.xml";
    let create = || Create {
        id: 13,
        text: "New note 😀 & tail".into(),
    };
    let (owner, inline_index, edit) = match case.as_str() {
        "create" | "reserved-only" => (endnotes, 0, create()),
        "first-part" | "empty-owner" => ("word/footnotes.xml", 0, create()),
        "attach" => (footnotes, 3, Attach(8)),
        "detach" => (footnotes, 4, Detach),
        "delete" => (footnotes, 4, Delete),
        "retarget" => (footnotes, 4, Retarget(8)),
        "renumber" => (endnotes, 5, Renumber(12)),
        _ => panic!("unknown fixture case {case}"),
    };
    let source = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let BodyBlock::Paragraph(paragraph) = &source.body()[0] else {
        panic!("expected the first direct paragraph");
    };
    let ImportedInlineNode::Run(run) = &paragraph.inline_structure[inline_index] else {
        panic!("expected a direct run at fixture index {inline_index}");
    };
    fs::write(
        output,
        source
            .edit_note_reference(owner, &run.anchor, edit)
            .unwrap(),
    )
    .unwrap();
}
