use std::{env, fs};

use rocx::editor::{
    BodyBlock, ImportedCellBlock, ImportedDocument, ImportedInline, ImportedRowChild,
    ImportedStoryBlock, ImportedTableChild, LayoutField, LayoutValue,
};
use serde_json::{Value, json};

fn items(items: &[ImportedInline]) -> Vec<Value> {
    items
        .iter()
        .map(|item| match item {
            ImportedInline::Text(text) => json!(["text", text]),
            ImportedInline::NoteReference { kind, id } => {
                json!(["note-reference", format!("{kind:?}"), id])
            }
            ImportedInline::Tab => json!(["tab"]),
            ImportedInline::Break(kind) => json!(["break", format!("{kind:?}")]),
            ImportedInline::Opaque(name) => json!(["opaque", name]),
            _ => panic!("unexpected imported inline: {item:?}"),
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = env::args_os().nth(1).expect("source path");
    let doc = ImportedDocument::open(&fs::read(source)?)?;
    let [
        BodyBlock::Paragraph(paragraph),
        BodyBlock::Paragraph(lexical),
        BodyBlock::Table(table),
    ] = doc.body()
    else {
        panic!("expected two body paragraphs and a table");
    };
    let flags: Vec<_> = paragraph
        .flags
        .iter()
        .map(|(flag, value)| (flag.name(), *value))
        .collect();
    let Some(LayoutValue::Integer(space_before)) = paragraph.layout.get(&LayoutField::SpaceBefore)
    else {
        panic!("expected spacing before the paragraph");
    };
    let children: Vec<_> = table
        .children
        .iter()
        .map(|row| match row {
            ImportedTableChild::Opaque(name) => json!(name),
            ImportedTableChild::Row(row) => {
                let cells: Vec<_> = row
                    .children
                    .iter()
                    .map(|cell| match cell {
                        ImportedRowChild::Opaque(name) => json!(name),
                        ImportedRowChild::Cell(cell) => {
                            let blocks: Vec<_> = cell
                                .blocks
                                .iter()
                                .map(|block| match block {
                                    ImportedCellBlock::Paragraph(inlines) => json!(items(inlines)),
                                    ImportedCellBlock::Opaque(name) => json!(name),
                                    _ => panic!("unexpected cell block: {block:?}"),
                                })
                                .collect();
                            json!({
                                "properties": cell.properties, "grid_span": cell.grid_span,
                                "vertical_merge": cell.vertical_merge, "blocks": blocks,
                            })
                        }
                        _ => panic!("unexpected row child: {cell:?}"),
                    })
                    .collect();
                json!({"properties": row.properties, "cells": cells})
            }
            _ => panic!("unexpected table child: {row:?}"),
        })
        .collect();
    let stories = doc.stories()?;
    let header = stories.first().expect("header story");
    let story: Vec<_> = header
        .blocks
        .iter()
        .map(|block| match block {
            ImportedStoryBlock::Paragraph(inlines) => json!(items(inlines)),
            ImportedStoryBlock::Opaque(name) => json!(name),
            _ => panic!("unexpected story block: {block:?}"),
        })
        .collect();
    let mark = paragraph
        .mark_revision
        .as_ref()
        .expect("paragraph revision");
    println!(
        "{}",
        json!({
            "text": paragraph.text, "has_opaque_content": paragraph.has_opaque_content,
            "alignment": paragraph.alignment.map(|alignment| format!("{alignment:?}")),
            "alignment_xml": paragraph.alignment_xml, "flags": flags, "space_before": space_before,
            "inline_items": items(&paragraph.inline_items),
            "mark_revision": [Some(mark.kind), mark.id.as_deref(), mark.author.as_deref(), mark.date.as_deref()],
            "lexical_alignment_xml": lexical.alignment_xml,
            "table": {
                "layout": table.layout.map(|layout| format!("{layout:?}")),
                "grid_widths": table.grid_widths, "properties": table.properties,
                "children": children,
            },
            "story": story,
        }),
    );
    Ok(())
}
