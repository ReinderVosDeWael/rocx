use std::ops::ControlFlow::{self, Break, Continue};

use rocx::editor::{
    BodyBlock, ImportedInline, ImportedInlineNode, ImportedParagraph, ImportedPropertyRevision,
    ImportedRevision, ImportedRun, InlineAnchor, TrackedParagraph, TrackedText,
};
use serde_json::{Value, json};

use super::{Snapshot, projected_run_text, report};
use crate::{diagnostics::ScopedEditError, schema::revisions::RevisionChange};

impl Snapshot {
    pub fn revision_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        if limit == 0 {
            return Err("revision page limit must be positive".into());
        }
        let mut rows = Vec::new();
        let mut count = 0;
        let mut visit = |body_index, path: &[usize], item: Revision<'_>| {
            let index = count;
            count += 1;
            if index >= start {
                if rows.len() == limit {
                    return Break(index);
                }
                rows.push(item.row(body_index, path));
            }
            Continue(())
        };
        let mut path = Vec::new();
        for (index, block) in self.document.body().iter().enumerate() {
            let BodyBlock::Paragraph(paragraph) = block else {
                continue;
            };
            if let Some(mark) = &paragraph.mark_revision
                && let Break(next) = visit(index, &[], Revision::ParagraphMark(mark))
            {
                return Ok((rows, Some(next)));
            }
            if let Break(next) =
                walk_inlines(index, &paragraph.inline_structure, &mut path, &mut visit)
            {
                return Ok((rows, Some(next)));
            }
        }
        Ok((rows, None))
    }

    // The session authorizes the issued opaque handles before passing local
    // coordinates here. Native staging performs the final semantic checks.
    pub fn preview_revision(
        &self,
        local: &str,
        paired: Option<&str>,
        edit: RevisionChange,
    ) -> Result<(Vec<u8>, Value), ScopedEditError> {
        use RevisionChange::*;
        if paired.is_some() && !matches!(edit, MoveText { .. } | Review { .. }) {
            return Err("paired_anchor is only allowed for a move or paired move review".into());
        }
        let doc = &self.document;
        let preview = match edit {
            InsertTextAfter { text, author, date } => doc.preview_tracked_text(
                self.proposal_run(local)?,
                TrackedText::InsertAfter(text),
                &author,
                date.as_deref(),
            ),
            ReplaceText { text, author, date } => doc.preview_tracked_text(
                self.proposal_run(local)?,
                TrackedText::Replace(text),
                &author,
                date.as_deref(),
            ),
            DeleteText { author, date } => doc.preview_tracked_text(
                self.proposal_run(local)?,
                TrackedText::Delete,
                &author,
                date.as_deref(),
            ),
            MoveText { name, author, date } => {
                let target = self.proposal_run(local)?;
                let after = self.proposal_run(paired.ok_or("move-text requires paired_anchor")?)?;
                doc.preview_tracked_move(target, after, &name, &author, &date)
            }
            SetBold { bold, author, date } => {
                doc.preview_tracked_bold(self.proposal_run(local)?, bold, &author, date.as_deref())
            }
            InsertParagraphBefore { text, author, date } => doc.preview_tracked_paragraph(
                &self.proposal_paragraph(local)?.anchor,
                TrackedParagraph::InsertBefore(text),
                &author,
                date.as_deref(),
            ),
            DeleteParagraph { author, date } => doc.preview_tracked_paragraph(
                &self.proposal_paragraph(local)?.anchor,
                TrackedParagraph::Delete,
                &author,
                date.as_deref(),
            ),
            Review { action } => {
                let (target, is_move) = self.review_inline(local)?;
                let paired = match (is_move, paired) {
                    (true, Some(other)) => {
                        let (other, other_is_move) = self.review_inline(other)?;
                        if !other_is_move {
                            return Err(wrong_type("a paired move wrapper"));
                        }
                        Some(other)
                    }
                    (true, None) => return Err("move review requires paired_anchor".into()),
                    (false, Some(_)) => {
                        return Err("paired_anchor is only allowed for a paired move review".into());
                    }
                    (false, None) => None,
                };
                doc.preview_revision(target, action.into(), paired)
            }
            ReviewParagraph { action } => {
                let paragraph = local
                    .strip_prefix("revision-mark:")
                    .or_else(|| local.strip_prefix("paragraph:"))
                    .and_then(|index| self.paragraph(index))
                    .filter(|p| p.mark_revision.is_some())
                    .ok_or_else(|| wrong_type("a pending paragraph mark"))?;
                doc.preview_paragraph_revision(&paragraph.anchor, action.into())
            }
        }?;
        Ok((preview.to_docx()?, report(&preview)))
    }

    fn paragraph(&self, index: &str) -> Option<&ImportedParagraph> {
        match self.document.body().get(index.parse::<usize>().ok()?)? {
            BodyBlock::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        }
    }

    fn direct_inline(&self, coords: &str) -> Option<&ImportedInlineNode> {
        let (body, inline) = coords.split_once(':')?;
        self.paragraph(body)?
            .inline_structure
            .get(inline.parse::<usize>().ok()?)
    }

    fn proposal_paragraph(&self, local: &str) -> Result<&ImportedParagraph, ScopedEditError> {
        local
            .strip_prefix("paragraph:")
            .and_then(|index| self.paragraph(index))
            .ok_or_else(|| wrong_type("a direct body paragraph"))
    }

    fn proposal_run(&self, local: &str) -> Result<&InlineAnchor, ScopedEditError> {
        match local
            .strip_prefix("run:")
            .and_then(|coords| self.direct_inline(coords))
        {
            Some(ImportedInlineNode::Run(run)) => Ok(&run.anchor),
            _ => Err(wrong_type("a direct body run")),
        }
    }

    fn review_inline(&self, local: &str) -> Result<(&InlineAnchor, bool), ScopedEditError> {
        let coords = local
            .strip_prefix("revision-inline:")
            .or_else(|| local.strip_prefix("run:"))
            .and_then(|coords| self.direct_inline(coords));
        match coords {
            Some(ImportedInlineNode::Revision(revision)) => Ok((
                &revision.anchor,
                matches!(revision.kind, "moveSource" | "moveDestination"),
            )),
            Some(ImportedInlineNode::Run(run)) if run.property_revision.is_some() => {
                Ok((&run.anchor, false))
            }
            _ => Err(wrong_type(
                "a pending inline revision or run property change",
            )),
        }
    }
}

fn wrong_type(expected: &str) -> ScopedEditError {
    ScopedEditError::WrongAnchorType(format!("revision operation expects {expected}"))
}

enum Revision<'a> {
    ParagraphMark(&'a ImportedPropertyRevision),
    Wrapper(&'a ImportedRevision),
    RunProperties(&'a ImportedRun, &'a ImportedPropertyRevision),
}

impl Revision<'_> {
    fn row(&self, body: usize, path: &[usize]) -> Value {
        let mut row = match self {
            Self::ParagraphMark(mark) => {
                let mut value = property(mark);
                value["type"] = json!("paragraph-mark");
                value["anchor"] = json!(format!("revision-mark:{body}"));
                value
            }
            Self::Wrapper(revision) => {
                let mut value = wrapper(revision);
                value["type"] = json!("inline-wrapper");
                value
            }
            Self::RunProperties(run, revision) => {
                let mut value = property(revision);
                value["type"] = json!("run-properties");
                value["run"] = run_value(run);
                value
            }
        };
        row["body_index"] = json!(body);
        row["inline_path"] = json!(path);
        if let [index] = path {
            row["anchor"] = json!(format!("revision-inline:{body}:{index}"));
        }
        row
    }
}

fn walk_inlines(
    body: usize,
    inlines: &[ImportedInlineNode],
    path: &mut Vec<usize>,
    visit: &mut impl FnMut(usize, &[usize], Revision<'_>) -> ControlFlow<usize>,
) -> ControlFlow<usize> {
    for (index, inline) in inlines.iter().enumerate() {
        path.push(index);
        match inline {
            ImportedInlineNode::Revision(revision) => {
                visit(body, path, Revision::Wrapper(revision))?;
                walk_inlines(body, &revision.children, path, visit)?;
            }
            ImportedInlineNode::Run(run) => {
                if let Some(revision) = &run.property_revision {
                    visit(body, path, Revision::RunProperties(run, revision))?;
                }
            }
            ImportedInlineNode::Hyperlink(link) => walk_inlines(body, &link.children, path, visit)?,
            _ => {}
        }
        path.pop();
    }
    Continue(())
}

fn property(revision: &ImportedPropertyRevision) -> Value {
    json!({
        "kind": revision.kind, "id": revision.id, "author": revision.author, "date": revision.date,
    })
}

fn wrapper(revision: &ImportedRevision) -> Value {
    json!({
        "kind": revision.kind, "id": revision.id, "author": revision.author, "date": revision.date,
        "children": children(&revision.children),
    })
}

fn run_value(run: &ImportedRun) -> Value {
    let (text, has_opaque_content) = projected_run_text(&run.items);
    let items: Vec<_> = run
        .items
        .iter()
        .map(|item| match item {
            ImportedInline::Text(text) => json!({"type": "text", "text": text}),
            ImportedInline::Tab => json!({"type": "tab"}),
            ImportedInline::Break(kind) => json!({"type": "break", "kind": format!("{kind:?}")}),
            ImportedInline::NoteReference { kind, id } => {
                json!({"type": "note-reference", "kind": format!("{kind:?}"), "id": id})
            }
            ImportedInline::Opaque(name) => json!({"type": "opaque", "name": name}),
            _ => json!({"type": "opaque"}),
        })
        .collect();
    json!({
        "type": "run", "text": text, "items": items, "bold": run.bold,
        "has_opaque_content": has_opaque_content, "has_opaque_formatting": run.has_opaque_formatting,
    })
}

fn children(inlines: &[ImportedInlineNode]) -> Vec<Value> {
    inlines
        .iter()
        .enumerate()
        .map(|(index, inline)| {
            let mut child = match inline {
                ImportedInlineNode::Run(run) => run_value(run),
                ImportedInlineNode::Revision(revision) => {
                    let mut child = wrapper(revision);
                    child["type"] = json!("inline-wrapper");
                    child
                }
                ImportedInlineNode::Hyperlink(link) => {
                    json!({"type": "hyperlink", "children": children(&link.children)})
                }
                ImportedInlineNode::Fraction {
                    numerator,
                    denominator,
                    ..
                } => {
                    json!({"type": "fraction", "numerator": numerator, "denominator": denominator})
                }
                ImportedInlineNode::Opaque(name) => json!({"type": "opaque", "name": name}),
                _ => json!({"type": "opaque"}),
            };
            child["inline_index"] = json!(index);
            child
        })
        .collect()
}
