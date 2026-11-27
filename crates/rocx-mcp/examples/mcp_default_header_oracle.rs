use std::{env, fs};

use rocx::{
    BreakType,
    editor::{ImportedDocument, ImportedHeaderBlock, ImportedInline, ImportedInlineNode},
};
use serde_json::{Value, json};

fn items(items: &[ImportedInline]) -> Vec<Value> {
    items
        .iter()
        .map(|item| match item {
            ImportedInline::Text(text) => json!({"kind":"text", "text":text}),
            ImportedInline::Tab => json!({"kind":"tab"}),
            ImportedInline::Break(BreakType::Page) => json!({"kind":"break", "break_type":"page"}),
            ImportedInline::Opaque(name) => json!({"kind":"opaque", "name":name}),
            _ => panic!("unexpected inline item: {item:?}"),
        })
        .collect()
}

fn typed(node: &ImportedInlineNode) -> Value {
    match node {
        ImportedInlineNode::Run(run) => json!({
            "kind":"run", "bold":run.bold, "items":items(&run.items),
        }),
        ImportedInlineNode::Hyperlink(link) => json!({
            "kind":"hyperlink", "bookmark":link.bookmark, "tooltip":link.tooltip,
            "history_xml":link.history_xml, "children":link.children.iter().map(typed).collect::<Vec<_>>(),
        }),
        ImportedInlineNode::Revision(revision) => json!({
            "kind":"revision", "revision_kind":revision.kind, "id":revision.id, "author":revision.author,
            "children":revision.children.iter().map(typed).collect::<Vec<_>>(),
        }),
        _ => panic!("unexpected typed inline: {node:?}"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = env::args_os().nth(1).expect("source path");
    let document = ImportedDocument::open(&fs::read(source)?)?;
    let rows: Vec<_> = document
        .default_headers()?
        .iter()
        .map(|header| {
            let blocks: Vec<_> = header
                .blocks
                .iter()
                .enumerate()
                .map(|(index, block)| match block {
                    ImportedHeaderBlock::Paragraph(flat) => {
                        let native = header.inline_structure(index).expect("paragraph");
                        let nodes: Vec<_> = native.iter().map(typed).collect();
                        if let Some(ImportedInlineNode::Run(run)) = native.first() {
                            assert_eq!(nodes[0], typed(document.inline_at(&run.anchor).unwrap()));
                        }
                        json!({
                            "index":index, "type":"paragraph", "inline_items":items(flat), "inlines":nodes,
                        })
                    }
                    ImportedHeaderBlock::Opaque(name) => {
                        assert!(header.inline_structure(index).is_none());
                        json!({"index":index, "type":"opaque", "name":name})
                    }
                    _ => panic!("unexpected header block: {block:?}"),
                })
                .collect();
            json!({
                "section_index":header.section_index, "part_name":header.part_name,
                "inherited":header.inherited, "blocks":blocks,
            })
        })
        .collect();
    println!(
        "{}",
        json!({"default_headers":rows, "stories_ok":document.stories().is_ok()})
    );
    Ok(())
}
