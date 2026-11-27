//! Calls the public core editor with fixed fixture addresses, without MCP.
use rocx::editor::{EntryEdit, ImportedDocument};
use std::{env, fs};

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let [case, source, output] = args.as_slice() else {
        panic!("usage: mcp_entry_edit_parity_oracle CASE SOURCE OUTPUT");
    };
    use EntryEdit::{DeleteUnusedNote, InsertParagraph, RemoveParagraph, ReplaceParagraph};
    let comments = "word/comments.xml";
    let endnotes = "word/notes/endnotes.xml";
    let text = " New 😀 & text ";
    let insert = || InsertParagraph {
        index: 1,
        text: text.into(),
    };
    let replace = |index| ReplaceParagraph {
        index,
        text: text.into(),
    };
    let (owner, id, edit, rejected) = match case.as_str() {
        "insert-comment" => (comments, "0", insert(), false),
        "replace-comment" => (comments, "0", replace(0), false),
        "remove-comment" => (comments, "0", RemoveParagraph(1), false),
        "delete-endnote" => (endnotes, "0008", DeleteUnusedNote, false),
        "last-block" => (comments, "1", RemoveParagraph(0), true),
        "marked-block" => (comments, "0", RemoveParagraph(0), true),
        "rich-block" => (comments, "0", replace(2), true),
        "protected-gap" => (comments, "2", insert(), true),
        "rich-note" => (endnotes, "0009", DeleteUnusedNote, true),
        "live-note" => (endnotes, "007", DeleteUnusedNote, true),
        _ => panic!("unknown fixture case {case}"),
    };
    let document = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let result = document.edit_entry(owner, id, edit);
    if rejected {
        println!(
            "{}",
            result.expect_err("expected native entry edit rejection")
        );
    } else {
        fs::write(output, result.unwrap()).unwrap();
    }
}
