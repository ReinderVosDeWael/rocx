use rocx::{
    Alignment, BreakType, NoteKind, TableLayout,
    editor::{
        ImportedCellBlock, ImportedInline, ImportedParagraph, ImportedRowChild, ImportedTable,
        ImportedTableChild,
    },
};
use serde_json::{Map, Value, json};

use super::{layout::layout_values, projected_run_text};

pub(super) fn inline_items(items: &[ImportedInline]) -> Value {
    Value::Array(
        items
            .iter()
            .map(|item| match item {
                ImportedInline::Text(text) => json!({"kind": "text", "text": text}),
                ImportedInline::NoteReference { kind, id } => {
                    let kind = match kind {
                        NoteKind::Footnote => "footnote",
                        NoteKind::Endnote => "endnote",
                    };
                    json!({"kind": "note-reference", "note_kind": kind, "id": id})
                }
                ImportedInline::Tab => json!({"kind": "tab"}),
                ImportedInline::Break(kind) => {
                    let kind = match kind {
                        BreakType::Line => "line",
                        BreakType::TextWrapping => "text-wrapping",
                        BreakType::Page => "page",
                        BreakType::Column => "column",
                    };
                    json!({"kind": "break", "break_type": kind})
                }
                ImportedInline::Opaque(name) => json!({"kind": "opaque", "name": name}),
                _ => json!({"kind": "opaque"}),
            })
            .collect(),
    )
}

pub(super) fn paragraph_fields(paragraph: &ImportedParagraph, value: &mut Value) {
    value["alignment"] = json!(paragraph.alignment.map(|alignment| match alignment {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
        Alignment::Justify => "justify",
        Alignment::Distribute => "distribute",
        Alignment::JustifyMedium => "justify-medium",
        Alignment::JustifyHigh => "justify-high",
        Alignment::JustifyLow => "justify-low",
        Alignment::ThaiJustify => "thai-justify",
    }));
    value["flags"] = Value::Object(
        paragraph
            .flags
            .iter()
            .map(|(flag, enabled)| (flag.name().to_owned(), json!(enabled)))
            .collect::<Map<_, _>>(),
    );
    value["layout"] = layout_values(&paragraph.layout);
    value["mark_revision"] = json!(paragraph.mark_revision.as_ref().map(|revision| {
        json!({
            "kind": revision.kind, "id": revision.id,
            "author": revision.author, "date": revision.date,
        })
    }));
    value["inline_items"] = inline_items(&paragraph.inline_items);
}

pub(super) fn table_fields(table: &ImportedTable, value: &mut Value) {
    value["properties"] = json!(table.properties);
    value["grid_widths"] = json!(table.grid_widths);
    value["layout"] = json!(table.layout.map(|layout| match layout {
        TableLayout::Fixed => "fixed",
        TableLayout::Autofit => "autofit",
    }));
    let mut row_index = 0;
    value["children"] = Value::Array(
        table
            .children
            .iter()
            .enumerate()
            .map(|(index, child)| match child {
                ImportedTableChild::Row(row) => {
                    let row_value = json!({
                        "type": "row", "index": index, "row_index": row_index,
                        "properties": row.properties, "children": row_children(&row.children),
                    });
                    row_index += 1;
                    row_value
                }
                ImportedTableChild::Opaque(name) => opaque(index, name),
                _ => opaque(index, "unknown"),
            })
            .collect(),
    );
}

fn row_children(children: &[ImportedRowChild]) -> Vec<Value> {
    // Structural selectors count only direct rows/cells. The `index` fields
    // instead describe positions in the native ordered lists, including opaque
    // siblings (like the indices in compact cell alignment anchors).
    let mut cell_index = 0;
    children
        .iter()
        .enumerate()
        .map(|(index, child)| match child {
            ImportedRowChild::Cell(cell) => {
                let blocks: Vec<_> = cell
                    .blocks
                    .iter()
                    .enumerate()
                    .map(|(index, block)| match block {
                        ImportedCellBlock::Paragraph(items) => {
                            let (text, has_opaque_content) = projected_run_text(items);
                            json!({
                                "type": "paragraph", "index": index, "text": text,
                                "has_opaque_content": has_opaque_content,
                                "inline_items": inline_items(items),
                            })
                        }
                        ImportedCellBlock::Opaque(name) => opaque(index, name),
                        _ => opaque(index, "unknown"),
                    })
                    .collect();
                let cell_value = json!({
                    "type": "cell", "index": index, "cell_index": cell_index,
                    "properties": cell.properties, "grid_span": cell.grid_span,
                    "vertical_merge": cell.vertical_merge, "blocks": blocks,
                });
                cell_index += 1;
                cell_value
            }
            ImportedRowChild::Opaque(name) => opaque(index, name),
            _ => opaque(index, "unknown"),
        })
        .collect()
}

fn opaque(index: usize, name: &str) -> Value {
    json!({"type": "opaque", "index": index, "name": name})
}
