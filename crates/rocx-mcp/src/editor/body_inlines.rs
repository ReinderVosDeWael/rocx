use rocx::editor::{BodyBlock, ImportedInlineNode, InlineAction, InlineEdit};
use serde_json::{Value, json};

use super::{Snapshot, projected_run_text};
use crate::schema::body_inlines::BodyInlineChange;

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

    pub fn body_inline_edit(
        &self,
        local: &str,
        change: BodyInlineChange,
    ) -> Result<InlineEdit, String> {
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
        let target = match paragraph.inline_structure.get(inline) {
            Some(ImportedInlineNode::Run(run)) => &run.anchor,
            Some(ImportedInlineNode::Hyperlink(link)) => &link.anchor,
            Some(ImportedInlineNode::Fraction { anchor, .. }) => anchor,
            _ => return Err(error()),
        };
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
        };
        Ok(InlineEdit {
            target: target.clone(),
            action,
        })
    }
}

fn project(node: &ImportedInlineNode) -> Value {
    match node {
        ImportedInlineNode::Run(run) => {
            let (text, has_opaque_content) = projected_run_text(&run.items);
            json!({"kind":"run", "text":text, "has_opaque_content":has_opaque_content,
                   "has_opaque_formatting":run.has_opaque_formatting})
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
