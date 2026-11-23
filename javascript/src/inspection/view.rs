use super::{Anchors, Failure, b};
use js_sys::Array;
use rocx::{
    Alignment, BreakType, Error, Result, SectionType,
    editor::{
        BodyBlock, ImportedCellBlock, ImportedHeader, ImportedHeaderBlock, ImportedInline,
        ImportedInlineNode, ImportedRowChild, ImportedSection, ImportedTableChild,
    },
};
use std::{cell::RefCell, collections::HashMap, hash::Hash};
use wasm_bindgen::JsValue;

pub(super) fn array(items: impl IntoIterator<Item = JsValue>) -> JsValue {
    items.into_iter().collect::<Array>().into()
}

pub(super) fn try_array(items: impl IntoIterator<Item = Result<JsValue>>) -> Result<JsValue> {
    let values = Array::new();
    for item in items {
        values.push(&item?);
    }
    Ok(values.into())
}

// Non-exhaustive Rust enums may gain variants in a future engine update.
// Reject an unmodeled variant instead of trapping the Wasm instance or
// inventing an XML name. Actual opaque XML uses the explicit Opaque variants.
fn unmodeled(kind: &str) -> Error {
    Failure::invalid("", format!("unmodeled Rust inspection variant: {kind}"))
}

fn record(kind: &str, fields: &[(&str, JsValue)]) -> JsValue {
    let value = b::record(fields);
    b::set(&value, "kind", &kind.into());
    value
}

fn opaque(name: &str) -> JsValue {
    record("opaque", &[("name", name.into())])
}

fn optional(result: &JsValue, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        b::set(result, key, &value.into());
    }
}

fn cached<K: Eq + Hash, T: Clone + 'static>(
    cache: &RefCell<HashMap<K, JsValue>>,
    key: K,
    kind: &str,
    anchor: &T,
) -> JsValue {
    cache
        .borrow_mut()
        .entry(key)
        .or_insert_with(|| b::handle(kind, anchor.clone(), JsValue::UNDEFINED))
        .clone()
}

pub(super) fn body_block(block: &BodyBlock, index: usize, anchors: &Anchors) -> Result<JsValue> {
    Ok(match block {
        BodyBlock::Paragraph(p) => {
            let result = record(
                "paragraph",
                &[
                    (
                        "anchor",
                        cached(&anchors.paragraphs, index, "paragraphAnchor", &p.anchor),
                    ),
                    ("text", p.text.as_str().into()),
                    ("hasOpaqueContent", p.has_opaque_content.into()),
                    ("inlineItems", items(&p.inline_items)?),
                    (
                        "inlineStructure",
                        inline_nodes(&p.inline_structure, anchors)?,
                    ),
                ],
            );
            optional(&result, "alignment", p.alignment.map(alignment));
            optional(&result, "alignmentXml", p.alignment_xml.as_deref());
            result
        }
        BodyBlock::Table(t) => record(
            "table",
            &[
                (
                    "anchor",
                    cached(&anchors.tables, index, "tableAnchor", &t.anchor),
                ),
                ("children", try_array(t.children.iter().map(table_child))?),
            ],
        ),
        BodyBlock::Opaque(name) => opaque(name),
        _ => return Err(unmodeled("body block")),
    })
}

fn table_child(child: &ImportedTableChild) -> Result<JsValue> {
    Ok(match child {
        ImportedTableChild::Row(row) => record(
            "row",
            &[("children", try_array(row.children.iter().map(row_child))?)],
        ),
        ImportedTableChild::Opaque(name) => opaque(name),
        _ => return Err(unmodeled("table child")),
    })
}

fn row_child(child: &ImportedRowChild) -> Result<JsValue> {
    Ok(match child {
        ImportedRowChild::Cell(cell) => record(
            "cell",
            &[("blocks", try_array(cell.blocks.iter().map(cell_block))?)],
        ),
        ImportedRowChild::Opaque(name) => opaque(name),
        _ => return Err(unmodeled("row child")),
    })
}

fn cell_block(block: &ImportedCellBlock) -> Result<JsValue> {
    match block {
        ImportedCellBlock::Paragraph(values) => flat_paragraph(values),
        ImportedCellBlock::Opaque(name) => Ok(opaque(name)),
        _ => Err(unmodeled("cell block")),
    }
}

fn flat_paragraph(values: &[ImportedInline]) -> Result<JsValue> {
    Ok(record("paragraph", &[("inlineItems", items(values)?)]))
}

fn items(items: &[ImportedInline]) -> Result<JsValue> {
    try_array(items.iter().map(|item| {
        Ok(match item {
            ImportedInline::Text(value) => record("text", &[("text", value.as_str().into())]),
            ImportedInline::Tab => record("tab", &[]),
            ImportedInline::Break(value) => record(
                "break",
                &[(
                    "breakType",
                    match value {
                        BreakType::Line => "line",
                        BreakType::Page => "page",
                        BreakType::Column => "column",
                        BreakType::TextWrapping => "textWrapping",
                    }
                    .into(),
                )],
            ),
            ImportedInline::Opaque(name) => opaque(name),
            _ => return Err(unmodeled("inline item")),
        })
    }))
}

pub(super) fn inline_nodes(nodes: &[ImportedInlineNode], anchors: &Anchors) -> Result<JsValue> {
    try_array(nodes.iter().map(|node| inline_node(node, anchors)))
}

pub(super) fn inline_node(node: &ImportedInlineNode, anchors: &Anchors) -> Result<JsValue> {
    // All three entry points borrow Rust's stored nodes: the immutable body,
    // the cached physical header shared by aliases, or Rust's inline_at
    // lookup into those same allocations. Use that canonical address solely
    // to intern envelopes; each token stores the actual Rust anchor.
    Ok(match node {
        ImportedInlineNode::Run(run) => record(
            "run",
            &[
                (
                    "anchor",
                    cached(&anchors.inlines, node, "inlineAnchor", &run.anchor),
                ),
                ("items", items(&run.items)?),
            ],
        ),
        ImportedInlineNode::Hyperlink(link) => {
            let result = record(
                "hyperlink",
                &[
                    (
                        "anchor",
                        cached(&anchors.inlines, node, "inlineAnchor", &link.anchor),
                    ),
                    ("children", inline_nodes(&link.children, anchors)?),
                ],
            );
            optional(&result, "relationshipId", link.relationship_id.as_deref());
            optional(&result, "bookmark", link.bookmark.as_deref());
            result
        }
        ImportedInlineNode::Opaque(name) => opaque(name),
        _ => return Err(unmodeled("inline node")),
    })
}

fn alignment(value: Alignment) -> &'static str {
    match value {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
        Alignment::Justify => "justify",
        Alignment::Distribute => "distribute",
        Alignment::JustifyMedium => "justifyMedium",
        Alignment::JustifyHigh => "justifyHigh",
        Alignment::JustifyLow => "justifyLow",
        Alignment::ThaiJustify => "thaiJustify",
    }
}

pub(super) fn section(section: &ImportedSection) -> JsValue {
    let result = b::record(&[
        ("bodyStart", section.body_start.into()),
        ("bodyEnd", section.body_end.into()),
    ]);
    optional(
        &result,
        "sectionType",
        section.section_type.map(|value| match value {
            SectionType::NextPage => "nextPage",
            SectionType::NextColumn => "nextColumn",
            SectionType::Continuous => "continuous",
            SectionType::EvenPage => "evenPage",
            SectionType::OddPage => "oddPage",
        }),
    );
    optional(
        &result,
        "sectionTypeXml",
        section.section_type_xml.as_deref(),
    );
    result
}

pub(super) fn header(header: &ImportedHeader, anchors: &Anchors) -> Result<JsValue> {
    Ok(b::record(&[
        (
            "anchor",
            cached(
                &anchors.headers,
                header.section_index,
                "headerAnchor",
                &header.anchor,
            ),
        ),
        ("sectionIndex", header.section_index.into()),
        ("partName", header.part_name.as_str().into()),
        ("inherited", header.inherited.into()),
        (
            "blocks",
            try_array(header.blocks.iter().map(|block| match block {
                ImportedHeaderBlock::Paragraph(values) => flat_paragraph(values),
                ImportedHeaderBlock::Opaque(name) => Ok(opaque(name)),
                _ => Err(unmodeled("header block")),
            }))?,
        ),
    ]))
}
