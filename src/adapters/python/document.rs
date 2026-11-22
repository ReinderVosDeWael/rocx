//! Native conversion into resolved domain values.
use super::*;
use pyo3::types::{PyBool, PyString};

pub(super) fn document(o: &Bound<'_, PyAny>) -> PyResult<ResolvedDocument> {
    let custom = o.getattr("custom_properties")?;
    let mut custom_properties = std::collections::BTreeMap::new();
    let mut custom_boolean_properties = std::collections::BTreeMap::new();
    if !custom.is_none() {
        for item in custom.call_method0("items")?.try_iter()? {
            let (name, value) = item?.extract::<(String, Bound<'_, PyAny>)>()?;
            if value.is_instance_of::<PyBool>() {
                custom_boolean_properties.insert(name, value.extract::<bool>()?);
            } else if value.is_instance_of::<PyString>() {
                custom_properties.insert(name, value.extract::<String>()?);
            } else {
                return Err(PyTypeError::new_err(
                    "Custom property values must be strings or booleans",
                ));
            }
        }
    }
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
            custom_properties,
            custom_boolean_properties,
            company: field!(o, "company", Option<String>),
            manager: field!(o, "manager", Option<String>),
        },
        comment_author: field!(o, "comment_author", Option<String>),
        styles: list(&o.getattr("styles")?, |v| {
            Ok(Some(match kind(v)?.as_str() {
                "ParagraphStyleDefinition" => paragraphstyledefinition(v)?.into(),
                "CharacterStyleDefinition" => character_style(v)?.into(),
                "NumberingStyleDefinition" => crate::NumberingStyleDefinition {
                    name: field!(v, "name", String),
                    numbering_id: field!(v, "numbering_id", u32),
                }
                .into(),
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
