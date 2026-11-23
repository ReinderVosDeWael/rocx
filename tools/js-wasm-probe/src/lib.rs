//! Experimental byte-only authoring boundary, not a public API.
use rocx::{Document, Paragraph, Section};
use wasm_bindgen::prelude::*;

/// Resolve one paragraph and return a DOCX package, without filesystem access.
#[wasm_bindgen]
pub async fn paragraph_docx(text: String) -> Result<Vec<u8>, JsValue> {
    // wasm32-unknown-unknown has no std wall clock. Keep probe output deterministic.
    let document = Document::new(vec![Section::new(vec![Paragraph::new(text).into()]).into()])
        .with_timestamp(Some("2026-11-21T00:00:00Z".into()));
    let resolved = document
        .resolve()
        .await
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    resolved
        .to_docx()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}
