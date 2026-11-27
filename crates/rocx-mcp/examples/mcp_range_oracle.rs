//! Independent native inspection and byte oracle for the MCP range fixture.
use rocx::editor::{
    BodyBlock, EditBatch, ImportedDocument, ImportedInlineNode, InlineAnchor, PairedRangeEdit,
    RunTextEdit,
};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let [source, action, output] = args.as_slice() else {
        return Err("usage: mcp_range_oracle SOURCE inspect|move|remove|create OUTPUT".into());
    };
    let document = ImportedDocument::open(&std::fs::read(source)?)?;
    if action == "inspect" {
        let rows: Vec<_> = document
            .body_range_markers()?
            .into_iter()
            .map(|marker| {
                json!([
                    marker.body_index,
                    marker.inline_index,
                    marker.kind,
                    marker.id,
                    marker.name
                ])
            })
            .collect();
        std::fs::write(output, serde_json::to_vec(&rows)?)?;
        return Ok(());
    }
    let anchor = |body_index: usize, inline_index: usize| -> InlineAnchor {
        let BodyBlock::Paragraph(paragraph) = &document.body()[body_index] else {
            panic!("expected fixture paragraph");
        };
        match &paragraph.inline_structure[inline_index] {
            ImportedInlineNode::Run(run) => run.anchor.clone(),
            ImportedInlineNode::Hyperlink(link) => link.anchor.clone(),
            _ => panic!("expected fixture run or hyperlink"),
        }
    };
    let pair = |id, bookmark: Option<&str>, endpoints| PairedRangeEdit {
        id,
        bookmark: bookmark.map(str::to_owned),
        endpoints,
    };
    let (pairs, text) = match action.as_str() {
        "move" => (
            [
                pair(7, Some("OldPlace"), Some((anchor(2, 1), anchor(3, 0)))),
                pair(0, None, Some((anchor(5, 0), anchor(5, 0)))),
            ],
            vec![RunTextEdit {
                target: anchor(6, 0),
                text: "ordinary edited".into(),
            }],
        ),
        "remove" => (
            [pair(7, Some("OldPlace"), None), pair(0, None, None)],
            vec![],
        ),
        "create" => (
            [
                pair(8, Some("NewPlace"), Some((anchor(1, 0), anchor(1, 0)))),
                pair(0, None, Some((anchor(4, 0), anchor(4, 0)))),
            ],
            vec![],
        ),
        _ => return Err("unknown action".into()),
    };
    let preview = document.preview_edits(EditBatch {
        paired_ranges: &pairs,
        run_text: &text,
        ..Default::default()
    })?;
    std::fs::write(output, preview.to_docx()?)?;
    println!("{}", serde_json::to_string(preview.inline_changes())?);
    Ok(())
}
