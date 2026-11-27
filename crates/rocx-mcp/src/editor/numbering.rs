use rocx::{
    NumberingReference,
    editor::{ImportedNumberingCatalog, ParagraphAnchor},
};
use serde_json::{Value, json};

use super::{Snapshot, Target};

mod physical;

fn assignment_value(value: Option<NumberingReference>) -> Value {
    json!(value.map(|value| json!({"id": value.id, "level": value.level})))
}

fn window<T: Clone>(items: &[T], start: usize, limit: usize) -> Vec<T> {
    items
        .get(start..start.saturating_add(limit).min(items.len()))
        .unwrap_or_default()
        .to_vec()
}

impl Snapshot {
    pub fn numbering_page(
        &self,
        abstracts: bool,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        let catalog = self
            .numbering
            .get_or_init(|| self.document.numbering_catalog().map_err(|e| e.to_string()))
            .as_ref()
            .map_err(Clone::clone)?;
        let mut page = ImportedNumberingCatalog::default();
        let (key, len) = if abstracts {
            page.abstracts = window(&catalog.abstracts, start, limit);
            ("abstracts", catalog.abstracts.len())
        } else {
            page.instances = window(&catalog.instances, start, limit);
            ("instances", catalog.instances.len())
        };
        // Reuse the core's CLI/Python wire model, but serialize only this page.
        let Value::Array(rows) = serde_json::to_value(page).map_err(|e| e.to_string())?[key].take()
        else {
            return Err("native numbering catalog did not serialize to an array".into());
        };
        let end = start + rows.len();
        Ok((rows, (end < len).then_some(end)))
    }

    pub fn numbering_paragraph_anchor(&self, local: &str) -> Result<ParagraphAnchor, String> {
        match self
            .issued_inspection()
            .and_then(|inspection| inspection.anchors.get(local))
        {
            Some(Target::Paragraph(anchor)) => Ok(anchor.clone()),
            _ => Err("wrong_anchor_type: expected a compact body paragraph anchor".into()),
        }
    }

    pub fn paragraph_numbering(&self, local: &str) -> Result<Value, String> {
        let anchor = self.numbering_paragraph_anchor(local)?;
        let value = self
            .document
            .paragraph_numbering(&anchor)
            .map_err(|e| e.to_string())?;
        Ok(assignment_value(value))
    }
}
