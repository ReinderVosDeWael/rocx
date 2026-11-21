//! Native conversion into resolved domain values.
use super::*;
pub(super) fn template(o: &Bound<'_, PyAny>) -> PyResult<DocumentTemplate> {
    let mut template = DocumentTemplate::new(field!(o, "path", std::path::PathBuf));
    template.edits.paragraph_index = field!(o, "paragraph_index", Option<usize>);
    let replacements = o.getattr("replacements")?;
    if !replacements.is_none() {
        for (k, v) in items(&replacements)? {
            template
                .edits
                .replacements
                .push((k.extract()?, v.extract()?));
        }
    }
    let controls = o.getattr("content_control_replacements")?;
    if !controls.is_none() {
        for (key, value) in items(&controls)? {
            template
                .edits
                .content_control_replacements
                .insert(key.extract()?, value.extract()?);
        }
    }
    template.edits.text_revision_action =
        match field!(o, "text_revision_action", Option<String>).as_deref() {
            None => None,
            Some("accept") => Some(crate::TextRevisionAction::Accept),
            Some("reject") => Some(crate::TextRevisionAction::Reject),
            _ => return Err(PyValueError::new_err("invalid text revision action")),
        };
    template.edits.scope = match field!(o, "scope", String).as_str() {
        "body" => EditScope::Body,
        "body_headers_footers" => EditScope::BodyHeadersFooters,
        "supported_stories" => EditScope::SupportedStories,
        _ => return Err(PyValueError::new_err("invalid template edit scope")),
    };
    Ok(template)
}

use crate::{DocumentTemplate, EditScope};
