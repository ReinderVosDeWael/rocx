use std::collections::BTreeMap;

use rocx::editor::{BodyBlock, LayoutEdit, LayoutField, LayoutValue};
use serde_json::{Value, json};

use super::{Snapshot, report};
use crate::schema::layout::LayoutChange;

const PARAGRAPH: &str = "layout-paragraph:";
const SECTION: &str = "layout-section:";

impl Snapshot {
    pub fn layout_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        if limit == 0 {
            return Err("layout page limit must be positive".into());
        }
        let mut rows = Vec::new();
        let mut count = 0;
        let mut visit = |kind, prefix, index, layout| {
            let ordinal = count;
            count += 1;
            if ordinal < start {
                return None;
            }
            if rows.len() == limit {
                return Some(ordinal);
            }
            rows.push(json!({
                "kind": kind, "index": index, "anchor": format!("{prefix}{index}"),
                "layout": layout_values(layout),
            }));
            None
        };
        for (index, block) in self.document.body().iter().enumerate() {
            if let BodyBlock::Paragraph(paragraph) = block
                && let Some(next) = visit("paragraph", PARAGRAPH, index, &paragraph.layout)
            {
                return Ok((rows, Some(next)));
            }
        }
        let sections = self.document.sections().map_err(|e| e.to_string())?;
        for (index, section) in sections.iter().enumerate() {
            if let Some(next) = visit("section", SECTION, index, &section.layout) {
                return Ok((rows, Some(next)));
            }
        }
        Ok((rows, None))
    }

    pub fn preview_layout(&self, edits: Vec<LayoutChange>) -> Result<(Vec<u8>, Value), String> {
        let mut sections = None;
        let mut native = Vec::with_capacity(edits.len());
        for (i, edit) in edits.into_iter().enumerate() {
            let wrong_target = || {
                format!(
                    "wrong_anchor_type: edit {}: expected a layout paragraph or section",
                    i + 1
                )
            };
            let index = |prefix| edit.anchor.strip_prefix(prefix)?.parse::<usize>().ok();
            let field = edit.field.into();
            let value = edit.value.into_native();
            if let Some(index) = index(PARAGRAPH) {
                let Some(BodyBlock::Paragraph(paragraph)) = self.document.body().get(index) else {
                    return Err(wrong_target());
                };
                native.push(LayoutEdit::Paragraph(
                    paragraph.anchor.clone(),
                    field,
                    value,
                ));
            } else if let Some(index) = index(SECTION) {
                if sections.is_none() {
                    sections = Some(self.document.sections().map_err(|e| e.to_string())?);
                }
                let section = sections
                    .as_ref()
                    .and_then(|sections| sections.get(index))
                    .ok_or_else(wrong_target)?;
                native.push(LayoutEdit::Section(section.anchor.clone(), field, value));
            } else {
                return Err(wrong_target());
            }
        }
        let preview = self
            .document
            .preview_layout(&native)
            .map_err(|e| e.to_string())?;
        Ok((
            preview.to_docx().map_err(|e| e.to_string())?,
            report(&preview),
        ))
    }
}

fn layout_values(layout: &BTreeMap<LayoutField, LayoutValue>) -> Value {
    Value::Object(
        layout
            .iter()
            .map(|(field, value)| {
                let value = match value {
                LayoutValue::Integer(v) => json!(v),
                LayoutValue::Token(v) => json!(v),
                LayoutValue::Boolean(v) => json!(v),
                LayoutValue::Tabs(v) => Value::Array(v.iter().map(|tab| json!({
                    "position": tab.position, "alignment": tab.alignment, "leader": tab.leader,
                })).collect()),
                LayoutValue::Border(v) => json!({
                    "size": v.size, "space": v.space, "color": v.color,
                }),
            };
                (field.name(), value)
            })
            .collect(),
    )
}
