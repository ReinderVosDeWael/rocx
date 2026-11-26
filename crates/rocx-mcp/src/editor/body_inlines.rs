use rocx::ImageRun;
use rocx::editor::{BodyBlock, ImportedInlineNode, InlineAction, InlineAnchor, InlineEdit};
use serde_json::{Value, json};

use super::{Snapshot, projected_run_text};
use crate::schema::{body_inlines::BodyInlineChange, payloads::resolve_component};

const PREFIX: &str = "body-inline:";

impl Snapshot {
    pub fn body_inline_page(
        &self,
        start: usize,
        limit: usize,
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
                let mut row = project(node);
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
        let action = match change {
            BodyInlineChange::InsertAfter { inline } => {
                InlineAction::InsertAfter(inline.resolve()?)
            }
            BodyInlineChange::Replace { inline } => InlineAction::Replace(inline.resolve()?),
            BodyInlineChange::RemoveHyperlink {} => InlineAction::RemoveHyperlink,
            BodyInlineChange::HyperlinkTooltip { value } => {
                InlineAction::HyperlinkTooltip(value.into_option())
            }
            BodyInlineChange::HyperlinkHistory { value } => {
                InlineAction::HyperlinkHistory(value.into_option())
            }
            BodyInlineChange::Format { action } => action.into(),
            BodyInlineChange::RemoveObject {} => InlineAction::RemoveObject,
            BodyInlineChange::DrawingDescription { value } => {
                InlineAction::SetDrawingDescription(value.into_option())
            }
            BodyInlineChange::ImageAsset { bytes } => InlineAction::SetImageAsset(
                resolve_component(ImageRun::new(bytes))?
                    .image()
                    .ok_or("expected resolved image")?
                    .clone(),
            ),
            BodyInlineChange::DrawingProperty { property, inline } => {
                InlineAction::PatchDrawing(property.into(), inline.resolve()?)
            }
        };
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

fn project(node: &ImportedInlineNode) -> Value {
    match node {
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
            row
        }
        ImportedInlineNode::Hyperlink(link) => json!({
            "kind":"hyperlink", "relationship_id":link.relationship_id, "bookmark":link.bookmark,
            "tooltip":link.tooltip, "history_xml":link.history_xml,
            "children":link.children.iter().map(project).collect::<Vec<_>>(),
        }),
        ImportedInlineNode::Fraction {
            numerator,
            denominator,
            ..
        } => json!({"kind":"fraction", "numerator":numerator, "denominator":denominator}),
        ImportedInlineNode::Revision(revision) => {
            json!({"kind":"revision", "revision_kind":revision.kind,"id":revision.id})
        }
        ImportedInlineNode::Opaque(name) => json!({"kind":"opaque","name":name}),
        _ => json!({"kind":"opaque"}),
    }
}
