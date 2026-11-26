use rmcp::schemars::{self, JsonSchema};
use rocx::opc::ReadLimits;
use serde::{Deserialize, Deserializer, de};

// These are strings on the wire so a client need not round a 64-bit budget.
fn decimal_budget(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    serde_json::json!({
        "type": "string",
        "pattern": "^(0|[1-9][0-9]{0,19})$",
    })
    .try_into()
    .expect("valid import budget schema")
}

fn budget<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: std::str::FromStr,
{
    let text = String::deserialize(deserializer)?;
    if text.is_empty()
        || !text.bytes().all(|b| b.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err(de::Error::custom(
            "expected a canonical unsigned decimal string",
        ));
    }
    text.parse()
        .map(Some)
        .map_err(|_| de::Error::custom("import budget is out of range on this host"))
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(inline)]
pub(super) struct ImportLimits {
    /// Maximum ZIP members; unsigned decimal string, bounded by the host's usize.
    #[serde(default, deserialize_with = "budget")]
    #[schemars(
        schema_with = "decimal_budget",
        skip_serializing_if = "Option::is_none"
    )]
    max_parts: Option<usize>,
    /// Maximum expanded bytes in one member; unsigned 64-bit decimal string.
    #[serde(default, deserialize_with = "budget")]
    #[schemars(
        schema_with = "decimal_budget",
        skip_serializing_if = "Option::is_none"
    )]
    max_part_bytes: Option<u64>,
    /// Maximum expanded bytes across members; unsigned 64-bit decimal string.
    #[serde(default, deserialize_with = "budget")]
    #[schemars(
        schema_with = "decimal_budget",
        skip_serializing_if = "Option::is_none"
    )]
    max_expanded_bytes: Option<u64>,
}

impl From<ImportLimits> for ReadLimits {
    fn from(input: ImportLimits) -> Self {
        let defaults = Self::default();
        Self {
            max_parts: input.max_parts.unwrap_or(defaults.max_parts),
            max_part_bytes: input.max_part_bytes.unwrap_or(defaults.max_part_bytes),
            max_expanded_bytes: input
                .max_expanded_bytes
                .unwrap_or(defaults.max_expanded_bytes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_value, json};

    #[test]
    fn preserves_full_byte_budget_and_host_part_count_without_rounding() {
        let limits: ReadLimits = from_value::<ImportLimits>(json!({
            "max_parts": usize::MAX.to_string(),
            "max_part_bytes": u64::MAX.to_string(),
        }))
        .unwrap()
        .into();
        assert_eq!(limits.max_parts, usize::MAX);
        assert_eq!(limits.max_part_bytes, u64::MAX);
        assert_eq!(
            limits.max_expanded_bytes,
            ReadLimits::default().max_expanded_bytes
        );
    }
}
