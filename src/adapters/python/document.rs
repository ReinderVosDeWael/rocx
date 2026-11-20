//! Native conversion into resolved domain values.
use super::*;
pub(super) fn document(o: &Bound<'_, PyAny>) -> PyResult<ResolvedDocument> {
    Ok(ResolvedDocument {
        sections: list(&o.getattr("sections")?, |v| Ok(Some(section(v)?)))?,
        notes: list(&o.getattr("notes")?, note)?,
        metadata: crate::document::Metadata {
            creator: field!(o, "creator", Option<String>),
            title: field!(o, "title", Option<String>),
            subject: field!(o, "subject", Option<String>),
            description: field!(o, "description", Option<String>),
            keywords: field!(o, "keywords", Option<String>),
            category: field!(o, "category", Option<String>),
            comments: field!(o, "comments", Option<String>),
            version: field!(o, "version", Option<String>),
        },
        comment_author: field!(o, "comment_author", Option<String>),
        styles: list(&o.getattr("styles")?, |v| {
            Ok(Some(match kind(v)?.as_str() {
                "ParagraphStyleDefinition" => paragraphstyledefinition(v)?.into(),
                "CharacterStyleDefinition" => character_style(v)?.into(),
                "TableStyleDefinition" => tablestyledefinition(v)?.into(),
                _ => return Err(PyTypeError::new_err("Unknown style definition")),
            }))
        })?,
        numbering: list(&o.getattr("numbering")?, |n| {
            Ok(Some(numbering_definition(n)?))
        })?,
        timestamp: field!(o, "timestamp", String),
        defaults: optional_record(o, "defaults", document_defaults)?,
    })
}

use crate::ResolvedDocument;
