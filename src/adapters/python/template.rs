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
    template.edits.scope = match field!(o, "scope", String).as_str() {
        "body" => EditScope::Body,
        "body_headers_footers" => EditScope::BodyHeadersFooters,
        "supported_stories" => EditScope::SupportedStories,
        _ => return Err(PyValueError::new_err("invalid template edit scope")),
    };
    Ok(template)
}

use crate::{DocumentTemplate, EditScope};
