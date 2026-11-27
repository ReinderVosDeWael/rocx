//! Read the acceptance fixtures through the public Rust editor, not the MCP adapter.
use std::{collections::BTreeMap, env, fs};

use rocx::{
    Alignment, BreakType, Color, HeaderFooterType, NoteKind, RunFormat, SectionType,
    editor::{
        BodyBlock, ImportedCellBlock, ImportedDocument, ImportedInline, ImportedInlineNode,
        ImportedPropertyRevision, ImportedRowChild, ImportedStoryBlock, ImportedTableChild,
        LayoutField, LayoutValue,
    },
};
use serde_json::{Value, json};

fn items(items: &[ImportedInline]) -> Vec<Value> {
    items
        .iter()
        .map(|item| match item {
            ImportedInline::Text(text) => json!({"kind":"text", "text":text}),
            ImportedInline::NoteReference { kind, id } => json!({
                "kind":"note-reference", "id":id, "note_kind":match kind {
                    NoteKind::Footnote => "footnote",
                    NoteKind::Endnote => "endnote",
                },
            }),
            ImportedInline::Tab => json!({"kind":"tab"}),
            ImportedInline::Break(kind) => json!({
                "kind":"break", "break_type":match kind {
                    BreakType::Line => "line",
                    BreakType::TextWrapping => "text-wrapping",
                    BreakType::Page => "page",
                    BreakType::Column => "column",
                },
            }),
            ImportedInline::Opaque(name) => json!({"kind":"opaque", "name":name}),
            _ => panic!("unexpected imported inline: {item:?}"),
        })
        .collect()
}

fn property_revision(revision: Option<&ImportedPropertyRevision>) -> Value {
    json!(revision.map(|revision| json!({
        "kind": revision.kind, "id": revision.id, "author": revision.author, "date": revision.date,
    })))
}

fn layout(fields: &BTreeMap<LayoutField, LayoutValue>) -> Value {
    let fields: BTreeMap<_, _> = fields
        .iter()
        .map(|(field, value)| {
            let value = match value {
                LayoutValue::Integer(value) => json!(value),
                LayoutValue::Boolean(value) => json!(value),
                LayoutValue::Token(value) => json!(value),
                LayoutValue::Tabs(tabs) => json!(tabs.iter().map(|tab| json!({
                "position": tab.position, "alignment": tab.alignment, "leader": tab.leader,
            })).collect::<Vec<_>>()),
                LayoutValue::Border(border) => json!({
                    "size": border.size, "space": border.space, "color": border.color,
                }),
            };
            (field.name(), value)
        })
        .collect();
    json!(fields)
}

fn format(format: &RunFormat) -> Value {
    json!({
        "style": format.style, "bold": format.bold, "italic": format.italic,
        "underline": format.underline.map(|value| format!("{value:?}").to_ascii_lowercase()),
        "font": format.font, "size": format.size,
        "color": match format.color {
            Some(Color::Rgb(rgb)) => json!(rgb),
            Some(other) => panic!("unexpected fixture color: {other:?}"),
            None => Value::Null,
        },
        "superscript": format.superscript, "subscript": format.subscript,
        "strike": format.strike, "all_caps": format.all_caps, "small_caps": format.small_caps,
        "snap_to_grid": format.snap_to_grid, "language": format.language.as_ref().map(|lang| json!({
            "latin":lang.latin, "east_asia":lang.east_asia, "bidi":lang.bidi,
        })),
        "bold_complex_script": format.bold_complex_script,
        "italic_complex_script": format.italic_complex_script,
        "size_complex_script": format.size_complex_script, "rtl": format.rtl,
        "complex_script": format.complex_script, "font_east_asia": format.font_east_asia,
        "font_complex_script": format.font_complex_script,
    })
}

fn structured(
    doc: &ImportedDocument,
    reopened: &ImportedDocument,
    nodes: &[ImportedInlineNode],
) -> Vec<Value> {
    nodes.iter().map(|node| {
        let children = |nodes| structured(doc, reopened, nodes);
        let (anchor, value) = match node {
            ImportedInlineNode::Run(run) => {
                let mut value = json!({
                    "kind":"run", "items":items(&run.items), "format":format(&run.format),
                    "bold":run.bold, "italic":run.italic, "strike":run.strike, "hidden":run.hidden,
                    "has_opaque_formatting":run.has_opaque_formatting,
                    "property_revision":property_revision(run.property_revision.as_ref()),
                });
                if let Some(d) = &run.drawing {
                    value["drawing"] = json!({
                        "kind":d.kind, "placement":d.placement, "description":d.description,
                        "title":d.title, "name":d.name, "geometry":d.geometry,
                        "shape_text":d.shape_text, "crop":d.crop, "rotation":d.rotation,
                        "flip_horizontal":d.flip_horizontal, "flip_vertical":d.flip_vertical,
                        "relationship_id":d.relationship_id, "width_emu":d.width_emu, "height_emu":d.height_emu,
                    });
                }
                (&run.anchor, value)
            }
            ImportedInlineNode::Hyperlink(link) => (&link.anchor, json!({
                "kind":"hyperlink", "relationship_id":link.relationship_id, "bookmark":link.bookmark,
                "tooltip":link.tooltip, "history_xml":link.history_xml, "children":children(&link.children),
            })),
            ImportedInlineNode::Revision(revision) => (&revision.anchor, json!({
                "kind":"revision", "revision_kind":revision.kind, "id":revision.id,
                "author":revision.author, "date":revision.date, "children":children(&revision.children),
            })),
            ImportedInlineNode::Fraction { anchor, numerator, denominator } => (anchor, json!({
                "kind":"fraction", "numerator":numerator, "denominator":denominator,
            })),
            ImportedInlineNode::Opaque(name) => return json!({"kind":"opaque", "name":name}),
            _ => panic!("unexpected fixture node: {node:?}"),
        };
        assert!(std::ptr::eq(doc.inline_at(anchor).unwrap(), node));
        assert!(reopened.inline_at(anchor).is_err());
        value
    }).collect()
}

fn block_readback(doc: &ImportedDocument, reopened: &ImportedDocument) -> rocx::Result<Value> {
    let [
        BodyBlock::Paragraph(paragraph),
        BodyBlock::Paragraph(lexical),
        BodyBlock::Table(table),
        BodyBlock::Opaque(opaque),
    ] = doc.body()
    else {
        panic!("expected two body paragraphs, a table and an opaque block");
    };
    let paragraphs = [paragraph, lexical].map(|p| {
        let flags: BTreeMap<_, _> = p.flags.iter().map(|(flag, value)| (flag.name(), *value)).collect();
        json!({
            "text":p.text, "has_opaque_content":p.has_opaque_content, "flags":flags,
            "alignment": p.alignment.map(|value| match value {
                Alignment::JustifyMedium => "justify-medium",
                other => panic!("unexpected fixture alignment: {other:?}"),
            }),
            "alignment_xml":p.alignment_xml, "layout":layout(&p.layout),
            "mark_revision":property_revision(p.mark_revision.as_ref()), "inline_items":items(&p.inline_items),
        })
    });
    let sections: Vec<_> = doc.sections()?.iter().map(|s| {
        let flags: BTreeMap<_, _> = s.flags.iter().map(|(flag, value)| (flag.name(), *value)).collect();
        json!({
            "body_start":s.body_start, "body_end":s.body_end, "flags":flags, "layout":layout(&s.layout),
            "section_type_xml":s.section_type_xml, "section_type":s.section_type.map(|value| match value {
                SectionType::Continuous => "continuous",
                other => panic!("unexpected fixture section type: {other:?}"),
            }),
        })
    }).collect();
    let children: Vec<_> = table.children.iter().map(|row| match row {
        ImportedTableChild::Opaque(name) => json!(name),
        ImportedTableChild::Row(row) => {
            let cells: Vec<_> = row.children.iter().map(|cell| match cell {
                ImportedRowChild::Opaque(name) => json!(name),
                ImportedRowChild::Cell(cell) => {
                    let blocks: Vec<_> = cell.blocks.iter().map(|block| match block {
                        ImportedCellBlock::Paragraph(inlines) => json!(items(inlines)),
                        ImportedCellBlock::Opaque(name) => json!(name),
                        _ => panic!("unexpected cell block: {block:?}"),
                    }).collect();
                    let inlines: Vec<_> = (0..=cell.blocks.len()).map(|index| cell.inline_structure(index)
                        .map(|nodes| structured(doc, reopened, nodes))).collect();
                    json!({
                        "properties":cell.properties, "grid_span":cell.grid_span,
                        "vertical_merge":cell.vertical_merge, "blocks":blocks, "inlines":inlines,
                    })
                }
                _ => panic!("unexpected row child: {cell:?}"),
            }).collect();
            json!({"properties":row.properties, "cells":cells})
        }
        _ => panic!("unexpected table child: {row:?}"),
    }).collect();
    let stories = doc.stories()?;
    let story: Vec<_> = stories[0]
        .blocks
        .iter()
        .map(|block| match block {
            ImportedStoryBlock::Paragraph(inlines) => json!(items(inlines)),
            ImportedStoryBlock::Opaque(name) => json!(name),
            _ => panic!("unexpected story block: {block:?}"),
        })
        .collect();
    Ok(json!({
        "paragraphs":paragraphs, "sections":sections, "story":story,
        "opaque_body_block":opaque,
        "table":{
            "layout":table.layout.map(|value| format!("{value:?}").to_ascii_lowercase()),
            "grid_widths":table.grid_widths, "properties":table.properties, "children":children,
        },
    }))
}

fn inline_readback(doc: &ImportedDocument, reopened: &ImportedDocument) -> rocx::Result<Value> {
    let read = |nodes| structured(doc, reopened, nodes);
    let mut body = BTreeMap::new();
    let mut cell = None;
    for (index, block) in doc.body().iter().enumerate() {
        match block {
            BodyBlock::Paragraph(p) if !p.inline_structure.is_empty() => {
                body.insert(index, read(&p.inline_structure));
            }
            BodyBlock::Table(table) => {
                let [ImportedTableChild::Row(row)] = table.children.as_slice() else {
                    panic!("expected one row");
                };
                let [ImportedRowChild::Cell(first)] = row.children.as_slice() else {
                    panic!("expected one cell");
                };
                cell = Some(read(first.inline_structure(0).unwrap()));
            }
            _ => (),
        }
    }
    let mut entries = Vec::new();
    for (entry_index, entry) in doc.entry_stories()?.iter().enumerate() {
        for block_index in 0..entry.blocks().len() {
            if let Some(nodes) = entry.inline_structure(block_index) {
                for (inline_index, node) in read(nodes).into_iter().enumerate() {
                    entries.push(json!({
                        "entry_index":entry_index, "entry_kind":entry.kind.as_str(), "entry_id":entry.id,
                        "block_index":block_index, "inline_index":inline_index, "node":node,
                    }));
                }
            }
        }
    }
    let stories = doc.stories()?;
    let headers = doc.default_headers()?;
    let story = |kind| {
        let aliases: Vec<_> = stories
            .iter()
            .filter(|s| s.kind == kind && s.variant == HeaderFooterType::Default)
            .collect();
        let [direct, inherited] = aliases.as_slice() else {
            panic!("expected two aliases");
        };
        let nodes = direct.inline_structure(0).unwrap();
        assert!(std::ptr::eq(nodes, inherited.inline_structure(0).unwrap()));
        read(nodes)
    };
    let header = story(rocx::editor::SectionStoryKind::Header);
    let [first, second] = headers.as_slice() else {
        panic!("expected two default headers")
    };
    let [ImportedInlineNode::Hyperlink(story_link)] = stories[0].inline_structure(0).unwrap()
    else {
        panic!("expected story hyperlink");
    };
    for header in [first, second] {
        let [ImportedInlineNode::Hyperlink(link)] = header.inline_structure(0).unwrap() else {
            panic!("expected default-header hyperlink");
        };
        assert_eq!(link.anchor, story_link.anchor);
    }
    Ok(json!({
        "body":body, "cell":cell.unwrap(), "entries":entries, "header":header,
        "footer":story(rocx::editor::SectionStoryKind::Footer),
    }))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = env::args_os().nth(1).expect("source path");
    let bytes = fs::read(source)?;
    let doc = ImportedDocument::open(&bytes)?;
    let reopened = ImportedDocument::open(&bytes)?;
    let result = match env::args().nth(2).as_deref() {
        None => block_readback(&doc, &reopened)?,
        Some("inline") => inline_readback(&doc, &reopened)?,
        Some(mode) => panic!("unknown fixture mode: {mode}"),
    };
    println!("{result}");
    Ok(())
}
