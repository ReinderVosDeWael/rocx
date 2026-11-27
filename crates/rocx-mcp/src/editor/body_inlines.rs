use std::collections::HashMap;

use rocx::editor::{BodyBlock, ImportedInlineNode, InlineAnchor, InlineEdit};
use serde_json::{Value, json};

use super::run_format::format_value;
use super::{Snapshot, block_readback::inline_items, projected_run_text};
use crate::schema::body_inlines::BodyInlineChange;

const PREFIX: &str = "body-inline:";

/// Readback registry writer for one open snapshot. Local keys describe original
/// source coordinates; only projected, addressable nodes enter the registry.
pub(crate) struct InlineHandles<'a> {
    pub document: &'a str,
    pub issued: &'a mut HashMap<String, InlineAnchor>,
}

impl Snapshot {
    pub fn body_inline_page(
        &self,
        start: usize,
        limit: usize,
        handles: &mut InlineHandles<'_>,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        let mut nodes = self
            .document
            .body()
            .iter()
            .enumerate()
            .flat_map(|(body, block)| {
                let slice = match block {
                    BodyBlock::Paragraph(paragraph) => paragraph.inline_structure.as_slice(),
                    _ => &[],
                };
                slice
                    .iter()
                    .enumerate()
                    .map(move |(inline, node)| (body, inline, node))
            })
            .skip(start);
        let rows = nodes
            .by_ref()
            .take(limit)
            .map(|(body, inline, node)| {
                let mut row = project(node, &format!("inline:body:{body}:{inline}"), handles);
                row["body_index"] = json!(body);
                row["inline_index"] = json!(inline);
                if matches!(
                    node,
                    ImportedInlineNode::Run(_)
                        | ImportedInlineNode::Hyperlink(_)
                        | ImportedInlineNode::Fraction { .. }
                ) {
                    row["anchor"] = json!(format!("{PREFIX}{body}:{inline}"));
                }
                row
            })
            .collect::<Vec<_>>();
        let next = nodes.next().map(|_| start + rows.len());
        Ok((rows, next))
    }

    pub fn inspect_inline(
        &self,
        anchor: &InlineAnchor,
        local: &str,
        handles: &mut InlineHandles<'_>,
    ) -> Result<Value, String> {
        let node = self.document.inline_at(anchor).map_err(|e| e.to_string())?;
        Ok(project(node, local, handles))
    }

    pub(super) fn body_inline_anchor(&self, local: &str) -> Result<&InlineAnchor, String> {
        let error =
            || "wrong_anchor_type: expected an anchor from the body-inlines view".to_owned();
        let (body, inline) = local
            .strip_prefix(PREFIX)
            .and_then(|s| s.split_once(':'))
            .and_then(|(a, b)| Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?)))
            .ok_or_else(error)?;
        let Some(BodyBlock::Paragraph(paragraph)) = self.document.body().get(body) else {
            return Err(error());
        };
        match paragraph.inline_structure.get(inline) {
            Some(ImportedInlineNode::Run(run)) => Ok(&run.anchor),
            Some(ImportedInlineNode::Hyperlink(link)) => Ok(&link.anchor),
            Some(ImportedInlineNode::Fraction { anchor, .. }) => Ok(anchor),
            _ => Err(error()),
        }
    }

    pub fn body_inline_edit(
        &self,
        local: &str,
        change: BodyInlineChange,
    ) -> Result<InlineEdit, String> {
        let target = self.body_inline_anchor(local)?.clone();
        let action = change.resolve()?;
        Ok(InlineEdit { target, action })
    }

    pub fn chart_series(&self, local: &str) -> Result<Value, String> {
        let anchor = self.body_inline_anchor(local)?;
        let (categories, values) = self
            .document
            .chart_series(anchor)
            .map_err(|error| error.to_string())?;
        Ok(json!({"categories": categories, "values": values}))
    }
}

pub(super) fn typed_inline_page(
    nodes: &[ImportedInlineNode],
    start: usize,
    limit: usize,
    source: &str,
    handles: &mut InlineHandles<'_>,
) -> (Vec<Value>, Option<usize>) {
    let end = start.saturating_add(limit).min(nodes.len());
    let rows = (start..end)
        .map(|index| {
            let mut row = project(&nodes[index], &format!("{source}:{index}"), handles);
            row["inline_index"] = json!(index);
            row
        })
        .collect();
    (rows, (end < nodes.len()).then_some(end))
}

pub(super) fn project(
    node: &ImportedInlineNode,
    local: &str,
    handles: &mut InlineHandles<'_>,
) -> Value {
    let mut children = |nodes: &[ImportedInlineNode]| {
        nodes
            .iter()
            .enumerate()
            .map(|(index, node)| project(node, &format!("{local}:{index}"), handles))
            .collect::<Vec<_>>()
    };
    let (mut row, anchor) = match node {
        ImportedInlineNode::Run(run) => {
            let (text, has_opaque_content) = projected_run_text(&run.items);
            let mut row = json!({"kind":"run", "text":text, "has_opaque_content":has_opaque_content,
                   "has_opaque_formatting":run.has_opaque_formatting});
            if let Some(drawing) = &run.drawing {
                row["drawing"] = json!({
                    "kind": drawing.kind, "placement": drawing.placement,
                    "description": drawing.description, "title": drawing.title, "name": drawing.name,
                    "geometry": drawing.geometry, "shape_text": drawing.shape_text,
                    "crop": drawing.crop, "rotation": drawing.rotation,
                    "flip_horizontal": drawing.flip_horizontal, "flip_vertical": drawing.flip_vertical,
                    "relationship_id": drawing.relationship_id, "width_emu": drawing.width_emu,
                    "height_emu": drawing.height_emu,
                });
            }
            row["items"] = inline_items(&run.items);
            row["format"] = format_value(&run.format);
            row["bold"] = json!(run.bold);
            row["italic"] = json!(run.italic);
            row["strike"] = json!(run.strike);
            row["hidden"] = json!(run.hidden);
            row["property_revision"] = json!(run.property_revision.as_ref().map(|revision| {
                json!({"kind":revision.kind,"id":revision.id,
                       "author":revision.author,"date":revision.date})
            }));
            (row, &run.anchor)
        }
        ImportedInlineNode::Hyperlink(link) => (
            json!({
                "kind":"hyperlink", "relationship_id":link.relationship_id, "bookmark":link.bookmark,
                "tooltip":link.tooltip, "history_xml":link.history_xml,
                "children":children(&link.children),
            }),
            &link.anchor,
        ),
        ImportedInlineNode::Fraction {
            numerator,
            denominator,
            anchor,
        } => (
            json!({"kind":"fraction", "numerator":numerator, "denominator":denominator}),
            anchor,
        ),
        ImportedInlineNode::Revision(revision) => (
            json!({
                "kind":"revision", "revision_kind":revision.kind,"id":revision.id,
                "author":revision.author,"date":revision.date,"children":children(&revision.children),
            }),
            &revision.anchor,
        ),
        ImportedInlineNode::Opaque(name) => return json!({"kind":"opaque","name":name}),
        _ => return json!({"kind":"opaque"}),
    };
    let issued = handles
        .issued
        .entry(local.to_owned())
        .or_insert_with(|| anchor.clone());
    debug_assert_eq!(issued, anchor, "inline handle source key must be unique");
    row["inline_handle"] = json!(format!("{}/{local}", handles.document));
    row
}
