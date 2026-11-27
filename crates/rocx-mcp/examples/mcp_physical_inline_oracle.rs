//! Typed inspection of independently authored physical paragraphs via public Rust APIs.
use rocx::editor::{
    BodyBlock, ImportedDocument, ImportedInline, ImportedInlineNode, ImportedRowChild,
    ImportedTableChild, SectionStoryKind,
};
use rocx::{BreakType, Color, HeaderFooterType};
use serde_json::{Value, json};

fn project(doc: &ImportedDocument, node: &ImportedInlineNode) -> Value {
    let (anchor, value) = match node {
        ImportedInlineNode::Run(run) => {
            let items: Vec<_> = run
                .items
                .iter()
                .map(|item| match item {
                    ImportedInline::Text(text) => json!({"kind": "text", "text": text}),
                    ImportedInline::Tab => json!({"kind": "tab"}),
                    ImportedInline::Break(BreakType::TextWrapping) => {
                        json!({"kind": "break", "break_type": "text-wrapping"})
                    }
                    ImportedInline::Opaque(name) => json!({"kind": "opaque", "name": name}),
                    other => panic!("unexpected fixture run content: {other:?}"),
                })
                .collect();
            let color = match run.format.color {
                Some(Color::Rgb(rgb)) => Some(rgb),
                None => None,
                other => panic!("unexpected fixture color: {other:?}"),
            };
            let mut value = json!({
                "kind": "run", "items": items, "bold": run.bold, "italic": run.italic,
                "has_opaque_formatting": run.has_opaque_formatting,
                "format": {"size": run.format.size, "color": color},
            });
            if let Some(drawing) = &run.drawing {
                value["drawing"] = json!({
                    "kind": drawing.kind, "placement": drawing.placement,
                    "description": drawing.description, "relationship_id": drawing.relationship_id,
                    "width_emu": drawing.width_emu, "height_emu": drawing.height_emu,
                });
            }
            (&run.anchor, value)
        }
        ImportedInlineNode::Hyperlink(link) => (
            &link.anchor,
            json!({
                "kind": "hyperlink", "relationship_id": link.relationship_id,
                "tooltip": link.tooltip, "history_xml": link.history_xml,
                "children": link.children.iter().map(|child| project(doc, child)).collect::<Vec<_>>(),
            }),
        ),
        ImportedInlineNode::Opaque(name) => return json!({"kind": "opaque", "name": name}),
        other => panic!("unexpected fixture inline: {other:?}"),
    };
    assert!(std::ptr::eq(doc.inline_at(anchor).unwrap(), node));
    value
}

fn paragraph(doc: &ImportedDocument, nodes: &[ImportedInlineNode]) -> Vec<Value> {
    nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let mut value = project(doc, node);
            value["inline_index"] = json!(index);
            value
        })
        .collect()
}

fn main() -> rocx::Result<()> {
    let input = std::env::args_os().nth(1).expect("expected fixture path");
    let doc = ImportedDocument::open(&std::fs::read(input).unwrap())?;
    let stories = doc.stories()?;
    let read_story = |kind| {
        let aliases: Vec<_> = stories
            .iter()
            .filter(|story| story.kind == kind && story.variant == HeaderFooterType::Default)
            .collect();
        let [first, second] = aliases.as_slice() else {
            panic!("expected two sections sharing a story");
        };
        assert!(!first.inherited && second.inherited);
        assert_eq!(first.part_name, second.part_name);
        let nodes = first.inline_structure(2).unwrap();
        assert!(std::ptr::eq(nodes, second.inline_structure(2).unwrap()));
        assert!(first.inline_structure(1).is_none());
        assert!(first.inline_structure(3).unwrap().is_empty());
        paragraph(&doc, nodes)
    };

    let BodyBlock::Table(table) = &doc.body()[1] else {
        panic!("expected fixture body table");
    };
    let [ImportedTableChild::Row(row)] = table.children.as_slice() else {
        panic!("expected one physical row");
    };
    let [ImportedRowChild::Cell(merged), ImportedRowChild::Cell(cell)] = row.children.as_slice()
    else {
        panic!("expected two physical cells in a three-column grid");
    };
    assert_eq!(merged.grid_span, Some(2));
    assert!(cell.inline_structure(1).is_none());
    assert!(cell.inline_structure(3).unwrap().is_empty());
    println!(
        "{}",
        json!({
            "header": read_story(SectionStoryKind::Header),
            "footer": read_story(SectionStoryKind::Footer),
            "cell": paragraph(&doc, cell.inline_structure(2).unwrap()),
            "adjacent_cell": paragraph(&doc, merged.inline_structure(0).unwrap()),
        })
    );
    Ok(())
}
