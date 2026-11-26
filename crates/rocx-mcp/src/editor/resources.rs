use std::collections::BTreeMap;

use rocx::Alignment;
use rocx::editor::{CustomValue, DefaultField, DefaultValue};
use serde_json::{Map, Value, json};

use super::Snapshot;
use crate::schema::resources::{METADATA_FIELDS, ResourceQuery};

impl Snapshot {
    pub(crate) fn inspect_resource_value(&self, query: ResourceQuery) -> Result<Value, String> {
        let doc = &self.document;
        let value = match query {
            ResourceQuery::Metadata { field } => {
                doc.metadata_field(field.into()).map(|value| json!(value))
            }
            ResourceQuery::DocumentDefaults {} => doc.document_defaults().map(defaults_value),
            ResourceQuery::DefaultAlignment {} => doc
                .default_alignment()
                .map(|value| json!(value.map(alignment_name))),
            ResourceQuery::CustomProperty { name } => doc.custom_property(&name).map(custom_value),
            ResourceQuery::AbstractStarts {} => doc.abstract_starts().map(|starts| {
                json!(starts.into_iter().map(|start| json!({
                    "abstract_id": start.abstract_id, "level": start.level, "start": start.start,
                })).collect::<Vec<_>>())
            }),
            ResourceQuery::NumberingInstances {} => {
                doc.numbering_instances().map(|instances| {
                    json!(instances.into_iter().map(|instance| json!({
                    "num_id": instance.num_id, "abstract_id": instance.abstract_id,
                    "replacement_levels": instance.replacement_levels,
                    "start_overrides": instance.start_overrides.into_iter().map(|start| {
                        json!({"level": start.level, "start": start.start})
                    }).collect::<Vec<_>>(),
                })).collect::<Vec<_>>())
                })
            }
        }
        .map_err(|error| error.to_string())?;
        Ok(json!({"value": value}))
    }

    pub(crate) fn inspect_resources(
        &self,
        custom_property: Option<String>,
    ) -> Result<Value, String> {
        let doc = &self.document;
        let defaults = doc
            .document_defaults()
            .map(defaults_value)
            .map_err(|error| error.to_string())?;
        let metadata: Map<_, _> = METADATA_FIELDS
            .iter()
            .map(|&(key, field)| {
                doc.metadata_field(field)
                    .map(|value| (key.into(), json!(value)))
            })
            .collect::<Result<_, _>>()
            .map_err(|error| error.to_string())?;

        let mut result = json!({"document_defaults": defaults, "metadata": metadata});
        // The typed reader supports direct OOXML justification values and
        // annotated leaves that the legacy alignment reader cannot represent.
        match doc.default_alignment() {
            Ok(alignment) => result["default_alignment"] = json!(alignment.map(alignment_name)),
            Err(error) => {
                result["default_alignment"] = Value::Null;
                result["default_alignment_error"] = json!(error.to_string());
            }
        }
        if let Some(name) = custom_property {
            result["custom_property"] = doc
                .custom_property(&name)
                .map(custom_value)
                .map_err(|error| error.to_string())?;
        }
        Ok(result)
    }
}

fn defaults_value(defaults: BTreeMap<DefaultField, DefaultValue>) -> Value {
    Value::Object(
        defaults
            .into_iter()
            .map(|(field, value)| (field.key().into(), default_value(value)))
            .collect(),
    )
}

fn custom_value(value: Option<CustomValue>) -> Value {
    match value {
        Some(CustomValue::Text(value)) => json!(value),
        Some(CustomValue::Boolean(value)) => json!(value),
        None => Value::Null,
    }
}

fn default_value(value: DefaultValue) -> Value {
    match value {
        DefaultValue::Bool(value) => json!(value),
        DefaultValue::Integer(value) => json!(value),
        DefaultValue::Text(value) => json!(value),
        DefaultValue::Tabs(tabs) => Value::Array(
            tabs.into_iter()
                .map(|tab| {
                    json!({
                        "position_twips": tab.position_twips,
                        "alignment": tab.alignment,
                        "leader": tab.leader,
                    })
                })
                .collect(),
        ),
    }
}

fn alignment_name(value: Alignment) -> &'static str {
    match value {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
        Alignment::Justify => "justify",
        Alignment::Distribute => "distribute",
        Alignment::JustifyMedium => "justify-medium",
        Alignment::JustifyHigh => "justify-high",
        Alignment::JustifyLow => "justify-low",
        Alignment::ThaiJustify => "thai-justify",
    }
}
