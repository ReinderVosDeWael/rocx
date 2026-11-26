use rmcp::schemars::{self, JsonSchema, Schema, SchemaGenerator, json_schema};
use rocx::editor::{
    CustomValue, DefaultField, DefaultTab, DefaultValue, MetadataField, ResourceEdit,
};
use serde::{Deserialize, Deserializer};

use super::ParagraphAlignment;

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(crate) enum ResourceQuery {
    /// Read a single direct core or extended metadata field.
    Metadata { field: MetadataKey },
    /// Read direct run, paragraph, and settings defaults without style inheritance.
    DocumentDefaults {},
    /// Read direct paragraph justification using the legacy alignment names.
    DefaultAlignment {},
    /// Read direct abstract level starts independently of the full catalog.
    AbstractStarts {},
    /// Read concrete IDs and override coordinates independently of modeled level details.
    NumberingInstances {},
    /// Read one custom property by its exact, nonblank name.
    CustomProperty { name: String },
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(crate) enum ResourceChange {
    /// Set the core title. Aliases the title metadata field.
    CoreTitle {
        /// Required; pass null to clear or an empty string to store an empty title.
        value: Nullable<String>,
    },
    /// Set the core subject. Aliases the subject metadata field.
    CoreSubject {
        /// Required; pass null to clear.
        value: Nullable<String>,
    },
    /// Set direct paragraph justification using the legacy alignment names.
    /// Aliases the justification document default; "justify" maps to "both".
    DefaultAlignment {
        /// Required; pass null to clear.
        value: Nullable<ParagraphAlignment>,
    },
    /// Set one direct run, paragraph, or settings default.
    Default {
        field: DefaultKey,
        /// Required; pass null to clear. The value type and range depend on the field.
        value: Nullable<DefaultScalar>,
    },
    /// Set a direct core or extended metadata field. Comments aliases description.
    Metadata {
        field: MetadataKey,
        /// Required; pass null to clear. Set dates with a UTC YYYY-MM-DDTHH:MM:SSZ string.
        value: Nullable<String>,
    },
    /// Set a named custom property (an unlinked string or boolean).
    CustomProperty {
        /// Exact, nonblank property name.
        name: String,
        /// Required; pass null to clear. False and empty text are stored values.
        value: Nullable<CustomScalar>,
    },
}

// The outer newtype makes the member required while the inner Option accepts null.
#[derive(Deserialize, JsonSchema)]
#[schemars(inline)]
pub(crate) struct Nullable<T>(Option<T>);

#[derive(Deserialize, JsonSchema)]
#[serde(try_from = "String")]
#[schemars(inline, schema_with = "default_key_schema")]
pub(crate) struct DefaultKey(DefaultField);

impl TryFrom<String> for DefaultKey {
    type Error = &'static str;

    fn try_from(key: String) -> Result<Self, Self::Error> {
        DefaultField::from_key(&key)
            .map(Self)
            .ok_or("unknown document default field")
    }
}

fn default_key_schema(_: &mut SchemaGenerator) -> Schema {
    let keys: Vec<_> = DefaultField::ALL.iter().map(|field| field.key()).collect();
    json_schema!({"type": "string", "enum": keys})
}

// MetadataField has native FromStr but no public key inventory. Keep one list
// for schema validation and readback, including the native Comments alias.
pub(crate) const METADATA_FIELDS: &[(&str, MetadataField)] = &[
    ("title", MetadataField::Title),
    ("subject", MetadataField::Subject),
    ("creator", MetadataField::Creator),
    ("description", MetadataField::Description),
    ("comments", MetadataField::Comments),
    ("keywords", MetadataField::Keywords),
    ("category", MetadataField::Category),
    ("version", MetadataField::Version),
    ("created", MetadataField::Created),
    ("modified", MetadataField::Modified),
    ("lastPrinted", MetadataField::LastPrinted),
    ("lastModifiedBy", MetadataField::LastModifiedBy),
    ("revision", MetadataField::Revision),
    ("identifier", MetadataField::Identifier),
    ("language", MetadataField::Language),
    ("contentStatus", MetadataField::ContentStatus),
    ("company", MetadataField::Company),
    ("manager", MetadataField::Manager),
];

#[derive(Deserialize, JsonSchema)]
#[serde(try_from = "String")]
#[schemars(inline, schema_with = "metadata_key_schema")]
pub(crate) struct MetadataKey(MetadataField);

impl TryFrom<String> for MetadataKey {
    type Error = rocx::Error;

    fn try_from(key: String) -> Result<Self, Self::Error> {
        key.parse().map(Self)
    }
}

impl From<MetadataKey> for MetadataField {
    fn from(key: MetadataKey) -> Self {
        key.0
    }
}

fn metadata_key_schema(_: &mut SchemaGenerator) -> Schema {
    let keys: Vec<_> = METADATA_FIELDS.iter().map(|(key, _)| *key).collect();
    json_schema!({"type": "string", "enum": keys})
}

// DefaultValue uses i64, but every modeled native integer field is bounded
// by i32 or u32. This enclosing range is exact in JavaScript JSON numbers.
const MIN_DEFAULT_INTEGER: i64 = i32::MIN as i64;
const MAX_DEFAULT_INTEGER: i64 = u32::MAX as i64;

fn default_integer<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
    let value = i64::deserialize(deserializer)?;
    if !(MIN_DEFAULT_INTEGER..=MAX_DEFAULT_INTEGER).contains(&value) {
        return Err(serde::de::Error::custom(
            "document default integer out of range",
        ));
    }
    Ok(value)
}

#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(inline)]
pub(crate) enum DefaultScalar {
    Flag(bool),
    Integer(
        #[serde(deserialize_with = "default_integer")]
        #[schemars(range(min = MIN_DEFAULT_INTEGER, max = MAX_DEFAULT_INTEGER))]
        i64,
    ),
    Text(String),
    Tabs(Vec<Tab>),
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(inline)]
pub(crate) struct Tab {
    /// Signed position in twips.
    position_twips: i32,
    /// OOXML tab alignment, such as "left", "center", or "decimal".
    alignment: String,
    /// OOXML leader, such as "dot"; omit or pass null for no explicit leader.
    leader: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(inline)]
pub(crate) enum CustomScalar {
    Text(String),
    Flag(bool),
}

impl From<DefaultScalar> for DefaultValue {
    fn from(value: DefaultScalar) -> Self {
        match value {
            DefaultScalar::Flag(value) => Self::Bool(value),
            DefaultScalar::Integer(value) => Self::Integer(value),
            DefaultScalar::Text(value) => Self::Text(value),
            DefaultScalar::Tabs(tabs) => Self::Tabs(
                tabs.into_iter()
                    .map(|tab| DefaultTab {
                        position_twips: tab.position_twips,
                        alignment: tab.alignment,
                        leader: tab.leader,
                    })
                    .collect(),
            ),
        }
    }
}

impl From<CustomScalar> for CustomValue {
    fn from(value: CustomScalar) -> Self {
        match value {
            CustomScalar::Text(value) => Self::Text(value),
            CustomScalar::Flag(value) => Self::Boolean(value),
        }
    }
}

impl From<ResourceChange> for ResourceEdit {
    fn from(edit: ResourceChange) -> Self {
        match edit {
            ResourceChange::CoreTitle { value } => Self::CoreTitle(value.0),
            ResourceChange::CoreSubject { value } => Self::CoreSubject(value.0),
            ResourceChange::DefaultAlignment { value } => {
                Self::DefaultAlignment(value.0.map(Into::into))
            }
            ResourceChange::Default { field, value } => {
                Self::Default(field.0, value.0.map(Into::into))
            }
            ResourceChange::Metadata { field, value } => Self::Metadata(field.0, value.0),
            ResourceChange::CustomProperty { name, value } => {
                Self::CustomProperty(name, value.0.map(Into::into))
            }
        }
    }
}
