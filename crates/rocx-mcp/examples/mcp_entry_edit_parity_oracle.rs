//! Calls the public core editor with fixed fixture addresses, without MCP.
use rocx::Rule;
use rocx::editor::{EntryEdit, ImportedDocument};
use std::{env, fs};

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let [case, source, output] = args.as_slice() else {
        panic!("usage: mcp_entry_edit_parity_oracle CASE SOURCE OUTPUT");
    };
    use EntryEdit::{
        CommentAuthor, DeleteUnusedNote, InsertParagraph, RemoveParagraph, ReplaceParagraph,
    };
    let comments = "word/comments.xml";
    let endnotes = "word/notes/endnotes.xml";
    let text = " New 😀 & text ";
    let insert = |index| InsertParagraph {
        index,
        text: text.into(),
    };
    let replace = |index| ReplaceParagraph {
        index,
        text: text.into(),
    };
    let (owner, id, edit, rejected) = match case.as_str() {
        "insert-comment" => (comments, "0", insert(1), false),
        "replace-comment" => (comments, "0", replace(0), false),
        "remove-comment" => (comments, "0", RemoveParagraph(1), false),
        "delete-endnote" => (endnotes, "0008", DeleteUnusedNote, false),
        "last-block" => (comments, "1", RemoveParagraph(0), true),
        "marked-block" => (comments, "0", RemoveParagraph(0), true),
        "rich-block" => (comments, "0", replace(2), true),
        "protected-gap" => (comments, "2", insert(1), true),
        "rich-note" => (endnotes, "0009", DeleteUnusedNote, true),
        "live-note" => (endnotes, "007", DeleteUnusedNote, true),
        "field-gap" => (comments, "3", insert(1), true),
        "after-field" => (comments, "3", insert(2), false),
        "alias-0008" => (endnotes, "0008", DeleteUnusedNote, true),
        "alias-8" => (endnotes, "8", DeleteUnusedNote, true),
        "extended-author" => (comments, "0", CommentAuthor("Updated".into()), true),
        _ => panic!("unknown fixture case {case}"),
    };
    let document = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let result = document.edit_entry(owner, id, edit);
    if rejected {
        let error = result.expect_err("expected native entry edit rejection");
        assert_eq!(error.rule(), Some(Rule::UnsupportedEdit));
        println!("{error}");
    } else {
        fs::write(output, result.unwrap()).unwrap();
    }
}
