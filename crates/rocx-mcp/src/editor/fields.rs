use rocx::editor::{BodyBlock, FieldEdit, ImportedField, ImportedInline, ImportedInlineNode};
use serde_json::{Value, json};

use super::Snapshot;
use crate::schema::fields::FieldChange;

impl Snapshot {
    pub fn field_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        if limit == 0 {
            return Err("field page limit must be positive".into());
        }
        // Enumerate all direct inline slots, not just the field candidates. The
        // native span length includes cache children and can cross paragraphs.
        let mut slots =
            self.document
                .body()
                .iter()
                .enumerate()
                .filter_map(|(body_index, block)| match block {
                    BodyBlock::Paragraph(paragraph) => Some((body_index, paragraph)),
                    _ => None,
                })
                .flat_map(|(body_index, paragraph)| {
                    paragraph.inline_structure.iter().enumerate().map(
                        move |(inline_index, node)| (body_index, inline_index, paragraph, node),
                    )
                });
        let mut fields = std::iter::from_fn(|| {
            while let Some((body_index, inline_index, paragraph, node)) = slots.next() {
                let complex = match node {
                    ImportedInlineNode::Opaque(name) if name == "w:fldSimple" => false,
                    ImportedInlineNode::Run(run)
                        if run.items.iter().any(
                            |item| matches!(item, ImportedInline::Opaque(name) if name == "w:fldChar"),
                        ) =>
                    {
                        true
                    }
                    _ => continue,
                };
                let field = self.document.field_at(&paragraph.anchor, inline_index);
                match field {
                    Ok(field) => {
                        for _ in 1..field.inline_count {
                            slots.next();
                        }
                        return Some((body_index, inline_index, Ok(field)));
                    }
                    // The projection preserves the marker name but not its
                    // type. An unrecognized marker may just be a separator
                    // or end, so it is not evidence of a distinct field.
                    Err(_) if complex => continue,
                    Err(error) => return Some((body_index, inline_index, Err(error))),
                }
            }
            None
        })
        .skip(start);
        let rows: Vec<_> = fields
            .by_ref()
            .take(limit)
            .map(|(body_index, inline_index, field)| field_row(body_index, inline_index, field))
            .collect();
        let next = fields.next().map(|_| start + rows.len());
        Ok((rows, next))
    }

    // Session checks that the local anchor was issued by the fields view.
    pub fn field_edit(&self, local: &str, edit: FieldChange) -> Result<FieldEdit, String> {
        let (body_index, inline_index) = local
            .strip_prefix("field:")
            .and_then(|address| address.split_once(':'))
            .and_then(|(body, inline)| Some((body.parse::<usize>().ok()?, inline.parse().ok()?)))
            .ok_or("wrong_anchor_type: use an anchor from the fields view")?;
        let Some(BodyBlock::Paragraph(paragraph)) = self.document.body().get(body_index) else {
            return Err("wrong_anchor_type: use an anchor from the fields view".into());
        };
        // Recheck recognizability, including read-only rows. The ordinary
        // native batch still checks overlap and action-specific restrictions.
        self.document
            .field_at(&paragraph.anchor, inline_index)
            .map_err(|error| error.to_string())?;
        Ok(FieldEdit {
            target: paragraph.anchor.clone(),
            inline_index,
            action: edit.resolve()?,
        })
    }
}

fn field_row(body_index: usize, inline_index: usize, field: rocx::Result<ImportedField>) -> Value {
    let anchor = format!("field:{body_index}:{inline_index}");
    match field {
        Ok(field) => json!({
            "anchor": anchor,
            "body_index": body_index,
            "inline_index": inline_index,
            "editable": true,
            "complex": field.complex,
            "instruction": field.instruction,
            "dirty_xml": field.dirty_xml,
            "locked_xml": field.locked_xml,
            "inline_count": field.inline_count,
        }),
        Err(error) => json!({
            "anchor": anchor,
            "body_index": body_index,
            "inline_index": inline_index,
            "editable": false,
            "complex": false,
            "reason": error.to_string(),
        }),
    }
}
