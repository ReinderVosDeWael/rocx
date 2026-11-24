use super::{Anchors, Failure, InlineKey, InlineSource, StoryPosition, b};
use js_sys::Array;
use rocx::{
    Alignment, BreakType, Error, HeaderFooterType, Result, SectionType,
    editor::{
        BodyBlock, ImportedCellBlock, ImportedHeader, ImportedHeaderBlock, ImportedInline,
        ImportedInlineNode, ImportedRowChild, ImportedSection, ImportedStory, ImportedTableChild,
        InlineAnchor,
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

pub(super) fn cached<K: Eq + Hash, T: Clone + 'static>(
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
                        inline_nodes(&p.inline_structure, anchors, None)?,
                    ),
                ],
            );
            optional(&result, "alignment", p.alignment.map(alignment));
            optional(&result, "alignmentXml", p.alignment_xml.as_deref());
            revision_metadata(&result, "markRevision", p.mark_revision.as_ref());
            b::set(
                &result,
                "flags",
                &b::record(
                    &p.flags
                        .iter()
                        .map(|(f, v)| (f.name(), (*v).into()))
                        .collect::<Vec<_>>(),
                ),
            );
            result
        }
        BodyBlock::Table(t) => {
            let result = record(
                "table",
                &[
                    (
                        "anchor",
                        cached(&anchors.tables, index, "tableAnchor", &t.anchor),
                    ),
                    (
                        "gridWidths",
                        array(
                            t.grid_widths
                                .iter()
                                .map(|w| w.map_or(JsValue::NULL, JsValue::from)),
                        ),
                    ),
                    (
                        "children",
                        try_array(t.children.iter().map(|child| table_child(child, anchors)))?,
                    ),
                ],
            );
            optional(
                &result,
                "layout",
                t.layout.map(|v| match v {
                    rocx::TableLayout::Fixed => "fixed",
                    rocx::TableLayout::Autofit => "autofit",
                }),
            );
            result
        }
        BodyBlock::ContentControl(control) => {
            let result = record("contentControl", &[]);
            optional(&result, "tag", control.tag.as_deref());
            optional(&result, "alias", control.alias.as_deref());
            optional(&result, "lock", control.lock.as_deref());
            result
        }
        BodyBlock::Opaque(name) => opaque(name),
        _ => return Err(unmodeled("body block")),
    })
}

fn table_child(child: &ImportedTableChild, anchors: &Anchors) -> Result<JsValue> {
    Ok(match child {
        ImportedTableChild::Row(row) => record(
            "row",
            &[(
                "children",
                try_array(row.children.iter().map(|child| row_child(child, anchors)))?,
            )],
        ),
        ImportedTableChild::Opaque(name) => opaque(name),
        _ => return Err(unmodeled("table child")),
    })
}

fn row_child(child: &ImportedRowChild, anchors: &Anchors) -> Result<JsValue> {
    Ok(match child {
        ImportedRowChild::Cell(cell) => {
            let result = record(
                "cell",
                &[(
                    "blocks",
                    try_array(
                        cell.blocks
                            .iter()
                            .enumerate()
                            .map(|(i, block)| match block {
                                ImportedCellBlock::Paragraph(values) => {
                                    let nodes = cell.inline_structure(i).ok_or_else(|| {
                                        Failure::invalid(
                                            "",
                                            "missing Rust cell paragraph inline structure",
                                        )
                                    })?;
                                    Ok(record(
                                        "paragraph",
                                        &[
                                            ("inlineItems", items(values)?),
                                            (
                                                "inlineStructure",
                                                inline_nodes(nodes, anchors, None)?,
                                            ),
                                        ],
                                    ))
                                }
                                ImportedCellBlock::Opaque(name) => Ok(opaque(name)),
                                _ => Err(unmodeled("cell block")),
                            }),
                    )?,
                )],
            );
            if let Some(span) = cell.grid_span {
                b::set(&result, "gridSpan", &span.into());
            }
            optional(&result, "verticalMerge", cell.vertical_merge.as_deref());
            result
        }
        ImportedRowChild::Opaque(name) => opaque(name),
        _ => return Err(unmodeled("row child")),
    })
}

fn flat_paragraph(values: &[ImportedInline]) -> Result<JsValue> {
    Ok(record("paragraph", &[("inlineItems", items(values)?)]))
}

fn items(items: &[ImportedInline]) -> Result<JsValue> {
    try_array(items.iter().map(|item| {
        Ok(match item {
            ImportedInline::Text(value) => record("text", &[("text", value.as_str().into())]),
            ImportedInline::NoteReference { kind, id } => record(
                match kind {
                    rocx::NoteKind::Footnote => "footnoteReference",
                    rocx::NoteKind::Endnote => "endnoteReference",
                },
                &[("id", id.as_str().into())],
            ),
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

pub(super) fn inline_nodes(
    nodes: &[ImportedInlineNode],
    anchors: &Anchors,
    parent: Option<&StoryPosition>,
) -> Result<JsValue> {
    try_array(nodes.iter().enumerate().map(|(index, node)| {
        let story = parent.cloned().map(|mut position| {
            position.path.push(index);
            position
        });
        inline_node(node, anchors, story.as_ref())
    }))
}

pub(super) fn inline_node(
    node: &ImportedInlineNode,
    anchors: &Anchors,
    story: Option<&StoryPosition>,
) -> Result<JsValue> {
    let token = |anchor: &InlineAnchor| {
        // Body paragraphs and cells borrow nodes from one immutable allocation;
        // story aliases may use either of two Rust caches for the same part.
        let key = story.map_or(InlineKey::Body(node), |position| {
            InlineKey::Story(position.clone())
        });
        anchors
            .inlines
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| {
                b::handle(
                    "inlineAnchor",
                    InlineSource {
                        anchor: anchor.clone(),
                        story: story.cloned(),
                    },
                    JsValue::UNDEFINED,
                )
            })
            .clone()
    };
    Ok(match node {
        ImportedInlineNode::Fraction {
            anchor,
            numerator,
            denominator,
        } => record(
            "fraction",
            &[
                ("anchor", token(anchor)),
                ("numerator", numerator.as_str().into()),
                ("denominator", denominator.as_str().into()),
            ],
        ),
        ImportedInlineNode::Run(run) => {
            let result = record(
                "run",
                &[
                    ("anchor", token(&run.anchor)),
                    ("hasOpaqueFormatting", run.has_opaque_formatting.into()),
                    ("format", crate::text_layout::run_format_view(&run.format)),
                    ("items", items(&run.items)?),
                ],
            );
            for (key, value) in [
                ("bold", run.bold),
                ("italic", run.italic),
                ("strike", run.strike),
                ("hidden", run.hidden),
            ] {
                if let Some(value) = value {
                    b::set(&result, key, &value.into());
                }
            }
            revision_metadata(&result, "propertyRevision", run.property_revision.as_ref());
            if let Some(d) = &run.drawing {
                let drawing = record(d.kind, &[("placement", d.placement.into())]);
                optional(&drawing, "relationshipId", d.relationship_id.as_deref());
                optional(&drawing, "description", d.description.as_deref());
                for (key, value) in [
                    ("title", &d.title),
                    ("name", &d.name),
                    ("geometry", &d.geometry),
                    ("shapeText", &d.shape_text),
                ] {
                    optional(&drawing, key, value.as_deref());
                }
                if let Some(crop) = d.crop {
                    b::set(
                        &drawing,
                        "crop",
                        &array(crop.into_iter().map(JsValue::from)),
                    );
                }
                if let Some(rot) = d.rotation {
                    b::set(&drawing, "rotation", &rot.into());
                }
                for (key, value) in [
                    ("flipHorizontal", d.flip_horizontal),
                    ("flipVertical", d.flip_vertical),
                ] {
                    if let Some(v) = value {
                        b::set(&drawing, key, &v.into());
                    }
                }
                for (key, v) in [("widthEmu", d.width_emu), ("heightEmu", d.height_emu)] {
                    if let Some(v) = v {
                        b::set(&drawing, key, &(v as f64).into());
                    }
                }
                b::set(&result, "drawing", &drawing);
            }
            result
        }
        ImportedInlineNode::Hyperlink(link) => {
            let result = record(
                "hyperlink",
                &[
                    ("anchor", token(&link.anchor)),
                    ("children", inline_nodes(&link.children, anchors, story)?),
                ],
            );
            optional(&result, "relationshipId", link.relationship_id.as_deref());
            optional(&result, "bookmark", link.bookmark.as_deref());
            optional(&result, "tooltip", link.tooltip.as_deref());
            optional(&result, "historyXml", link.history_xml.as_deref());
            result
        }
        ImportedInlineNode::Revision(revision) => {
            let result = record(
                "revision",
                &[
                    ("anchor", token(&revision.anchor)),
                    ("revisionKind", JsValue::from_str(revision.kind)),
                    (
                        "children",
                        inline_nodes(&revision.children, anchors, story)?,
                    ),
                ],
            );
            optional(&result, "id", revision.id.as_deref());
            optional(&result, "author", revision.author.as_deref());
            optional(&result, "date", revision.date.as_deref());
            result
        }
        ImportedInlineNode::Opaque(name) => opaque(name),
        _ => return Err(unmodeled("inline node")),
    })
}

fn revision_metadata(
    result: &JsValue,
    key: &str,
    change: Option<&rocx::editor::ImportedPropertyRevision>,
) {
    if let Some(change) = change {
        let metadata = record(change.kind, &[]);
        optional(&metadata, "author", change.author.as_deref());
        optional(&metadata, "id", change.id.as_deref());
        optional(&metadata, "date", change.date.as_deref());
        b::set(result, key, &metadata);
    }
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
        (
            "flags",
            b::record(
                &section
                    .flags
                    .iter()
                    .map(|(f, v)| (f.name(), (*v).into()))
                    .collect::<Vec<_>>(),
            ),
        ),
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
                (
                    header.section_index,
                    rocx::editor::SectionStoryKind::Header,
                    0,
                ),
                "headerAnchor",
                &header.anchor,
            ),
        ),
        ("sectionIndex", header.section_index.into()),
        ("partName", header.part_name.as_str().into()),
        ("inherited", header.inherited.into()),
        ("blocks", story_blocks(&header.blocks)?),
    ]))
}

pub(super) fn story(story: &ImportedStory, anchors: &Anchors) -> Result<JsValue> {
    let result = b::record(&[
        ("sectionIndex", story.section_index.into()),
        ("kind", story.kind.as_str().into()),
        (
            "variant",
            match story.variant {
                HeaderFooterType::Default => "default",
                HeaderFooterType::First => "first",
                HeaderFooterType::Even => "even",
            }
            .into(),
        ),
        ("partName", story.part_name.as_str().into()),
        ("inherited", story.inherited.into()),
        ("blocks", story_blocks(&story.blocks)?),
    ]);
    if let Some(anchor) = &story.anchor {
        b::set(
            &result,
            "anchor",
            &cached(
                &anchors.headers,
                (story.section_index, story.kind, story.variant as usize),
                "headerAnchor",
                anchor,
            ),
        );
    }
    Ok(result)
}

fn story_blocks(blocks: &[ImportedHeaderBlock]) -> Result<JsValue> {
    try_array(blocks.iter().map(|block| match block {
        ImportedHeaderBlock::Paragraph(values) => flat_paragraph(values),
        ImportedHeaderBlock::Opaque(name) => Ok(opaque(name)),
        _ => Err(unmodeled("story block")),
    }))
}

pub(super) fn entry(entry: &rocx::editor::ImportedEntry, anchors: &Anchors) -> Result<JsValue> {
    let blocks = try_array(
        entry
            .blocks()
            .iter()
            .enumerate()
            .map(|(i, block)| match block {
                ImportedHeaderBlock::Paragraph(values) => {
                    let paragraph = flat_paragraph(values)?;
                    let nodes = entry
                        .inline_structure(i)
                        .ok_or_else(|| unmodeled("entry paragraph"))?;
                    b::set(
                        &paragraph,
                        "inlineStructure",
                        &inline_nodes(nodes, anchors, None)?,
                    );
                    Ok(paragraph)
                }
                ImportedHeaderBlock::Opaque(name) => Ok(opaque(name)),
                _ => Err(unmodeled("entry block")),
            }),
    )?;
    let result = record(
        entry.kind.as_str(),
        &[
            ("partName", (&entry.part_name).into()),
            ("id", (&entry.id).into()),
            ("blocks", blocks),
        ],
    );
    for (key, value) in [
        ("noteType", &entry.note_type),
        ("author", &entry.author),
        ("initials", &entry.initials),
        ("date", &entry.date),
    ] {
        optional(&result, key, value.as_deref());
    }
    Ok(result)
}
