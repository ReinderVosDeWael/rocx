use rmcp::schemars::{self, JsonSchema, Schema, SchemaGenerator, json_schema};
use rocx::SectionType;
use rocx::editor::{ParagraphFlag, SectionFlag};
use serde::Deserialize;

use super::payloads::{BlockInput, ParagraphInput};

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(crate) enum BlockChange {
    /// Split a section using an anchor from the sections view.
    SplitSection {
        /// Zero-based, section-relative gap in the original body.
        offset: usize,
    },
    /// Merge a nonfinal section into its successor. Use a sections view anchor.
    MergeSection {},
    /// Replace a direct body paragraph at its compact body anchor.
    ReplaceParagraph {
        paragraph: ParagraphInput,
    },
    /// Remove a direct body paragraph at its compact body anchor.
    RemoveParagraph {},
    /// Replace a section's blocks. Use its first block's compact paragraph anchor.
    ReplaceSection {
        blocks: Vec<BlockInput>,
    },
    KeepWithNext {
        value: bool,
    },
    /// Edit a direct flag on a compact body paragraph anchor.
    ParagraphFlag {
        field: ParagraphFlagKey,
        /// Required; null clears the direct value.
        value: Nullable<bool>,
    },
    /// Edit a direct section flag. Use the first block's compact paragraph anchor.
    SectionFlag {
        field: SectionFlagKey,
        /// Required; null clears the direct value.
        value: Nullable<bool>,
    },
    /// Edit the explicit section type. Use the first block's compact paragraph anchor.
    SectionType {
        /// Required; null clears the direct value.
        value: Nullable<SectionTypeToken>,
    },
}

#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(inline)]
pub(crate) enum Nullable<T> {
    Clear(()),
    Set(T),
}

impl<T> Nullable<T> {
    pub(crate) fn into_option(self) -> Option<T> {
        match self {
            Self::Clear(()) => None,
            Self::Set(value) => Some(value),
        }
    }
}

macro_rules! flag_key {
    ($key:ident, $native:ty, $error:literal) => {
        #[derive(Deserialize)]
        #[serde(try_from = "String")]
        pub(crate) struct $key($native);

        impl JsonSchema for $key {
            fn inline_schema() -> bool {
                true
            }

            fn schema_name() -> std::borrow::Cow<'static, str> {
                stringify!($key).into()
            }

            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                let names: Vec<_> = <$native>::ALL.iter().map(|flag| flag.name()).collect();
                json_schema!({"type": "string", "enum": names})
            }
        }

        impl TryFrom<String> for $key {
            type Error = &'static str;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                <$native>::ALL
                    .iter()
                    .copied()
                    .find(|flag| flag.name() == value)
                    .map(Self)
                    .ok_or($error)
            }
        }

        impl From<$key> for $native {
            fn from(key: $key) -> Self {
                key.0
            }
        }
    };
}

flag_key!(ParagraphFlagKey, ParagraphFlag, "unknown paragraph flag");
flag_key!(SectionFlagKey, SectionFlag, "unknown section flag");

#[derive(Deserialize, JsonSchema)]
#[serde(try_from = "String")]
#[schemars(inline, schema_with = "section_type_schema")]
pub(crate) struct SectionTypeToken(SectionType);

const SECTION_TYPES: &[(SectionType, &str)] = &[
    (SectionType::NextPage, "nextPage"),
    (SectionType::NextColumn, "nextColumn"),
    (SectionType::Continuous, "continuous"),
    (SectionType::EvenPage, "evenPage"),
    (SectionType::OddPage, "oddPage"),
];

pub(crate) fn section_type_name(value: SectionType) -> &'static str {
    SECTION_TYPES
        .iter()
        .find_map(|&(native, token)| (native == value).then_some(token))
        .expect("native section type must have a token")
}

fn section_type_schema(_: &mut SchemaGenerator) -> Schema {
    let names: Vec<_> = SECTION_TYPES.iter().map(|&(_, token)| token).collect();
    json_schema!({"type": "string", "enum": names})
}

impl TryFrom<String> for SectionTypeToken {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        SECTION_TYPES
            .iter()
            .find_map(|&(native, token)| (token == value).then_some(Self(native)))
            .ok_or("unknown section type")
    }
}

impl From<SectionTypeToken> for SectionType {
    fn from(token: SectionTypeToken) -> Self {
        token.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn block_discovery_and_decoding_require_nullable_values() {
        let schema = serde_json::to_value(schemars::schema_for!(BlockChange)).unwrap();
        for (kind, field, sample) in [
            ("paragraph-flag", Some("keepTogether"), json!(false)),
            ("section-flag", Some("rtlGutter"), json!(true)),
            ("section-type", None, json!("continuous")),
        ] {
            let variant = schema["oneOf"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["properties"]["kind"]["const"] == kind)
                .unwrap();
            assert!(
                variant["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("value"))
            );
            assert_eq!(variant["additionalProperties"], false);
            let mut request = json!({"kind": kind});
            if let Some(field) = field {
                request["field"] = json!(field);
            }
            assert!(serde_json::from_value::<BlockChange>(request.clone()).is_err());
            for value in [json!(null), sample] {
                request["value"] = value;
                assert!(serde_json::from_value::<BlockChange>(request.clone()).is_ok());
            }
            request["value"] = json!(1);
            assert!(serde_json::from_value::<BlockChange>(request).is_err());
        }
    }

    #[test]
    fn block_requests_reject_unknown_fields_and_tokens() {
        for bad in [
            json!({"kind":"merge-section","offset":0}),
            json!({"kind":"remove-paragraph","paragraph":null}),
            json!({"kind":"split-section","offset":-1}),
            json!({"kind":"keep-with-next","value":null}),
            json!({"kind":"paragraph-flag","field":"titlePage","value":false}),
            json!({"kind":"section-flag","field":"keepTogether","value":null}),
            json!({"kind":"section-type","value":"even-page"}),
        ] {
            assert!(
                serde_json::from_value::<BlockChange>(bad.clone()).is_err(),
                "{bad}"
            );
        }
    }
}
