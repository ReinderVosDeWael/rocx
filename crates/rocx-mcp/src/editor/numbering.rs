use rocx::{
    NumberingReference,
    editor::{ImportedNumberingCatalog, ParagraphAnchor},
};
use serde_json::{Value, json};

use super::{ReadError, Snapshot, cached_read};

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
    ) -> Result<(Vec<Value>, Option<usize>), ReadError> {
        let catalog = cached_read(&self.numbering, || self.document.numbering_catalog())?;
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

    pub fn paragraph_numbering(&self, anchor: &ParagraphAnchor) -> rocx::Result<Value> {
        let value = self.document.paragraph_numbering(anchor)?;
        Ok(assignment_value(value))
    }
}
