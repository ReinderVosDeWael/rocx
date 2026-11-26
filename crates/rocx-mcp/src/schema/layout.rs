use rmcp::schemars::{self, JsonSchema};
use rocx::editor::{LayoutBorder, LayoutField, LayoutTab, LayoutValue};
use serde::Deserialize;

const MIN_I32: i32 = i32::MIN;
const MAX_I32: i32 = i32::MAX;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LayoutChange {
    /// An issued paragraph or section anchor from the layout view.
    pub(crate) anchor: String,
    pub(crate) field: Field,
    /// Set a direct value, or pass null to clear. Omit the entire edit to keep.
    pub(crate) value: LayoutInput,
}

macro_rules! layout_fields {
    ($($field:ident),+ $(,)?) => {
        #[derive(Deserialize, JsonSchema)]
        #[serde(rename_all = "camelCase")]
        pub(crate) enum Field {
            $($field),+
        }

        impl From<Field> for LayoutField {
            fn from(field: Field) -> Self {
                match field {
                    $(Field::$field => Self::$field),+
                }
            }
        }
    };
}

layout_fields!(
    StyleId,
    Heading,
    KeepWithNext,
    KeepTogether,
    PageBreakBefore,
    WidowControl,
    SuppressLineNumbers,
    ParagraphBidi,
    SnapToGrid,
    ContextualSpacing,
    MirrorIndents,
    Alignment,
    OutlineLevel,
    SpaceBefore,
    SpaceAfter,
    LineHeight,
    LineRule,
    LeftIndent,
    RightIndent,
    FirstLineIndent,
    TabStops,
    PageWidth,
    PageHeight,
    Orientation,
    MarginTop,
    MarginBottom,
    MarginLeft,
    MarginRight,
    MarginHeader,
    MarginFooter,
    MarginGutter,
    ColumnCount,
    ColumnSpace,
    ColumnSeparator,
    GridMode,
    GridPitch,
    GridCharSpace,
    PageStart,
    PageFormat,
    LineStart,
    LineCountBy,
    LineDistance,
    LineRestart,
    VerticalAlign,
    SectionType,
    TitlePage,
    Bidi,
    RtlGutter,
    BorderOffset,
    BorderDisplay,
    BorderZOrder,
    BorderTop,
    BorderLeft,
    BorderBottom,
    BorderRight,
    Margins,
    LineNumbering,
);

/// Native direct layout values. The field determines which non-null shape is valid.
#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
pub(crate) enum LayoutInput {
    Clear(()),
    Integer(#[schemars(range(min = MIN_I32, max = MAX_I32))] i32),
    Token(String),
    Boolean(bool),
    Tabs(Vec<TabInput>),
    Border(BorderInput),
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TabInput {
    /// Position in twips.
    #[schemars(range(min = MIN_I32, max = MAX_I32))]
    position: i32,
    alignment: String,
    leader: Option<String>,
}

/// A single page border. Size is in eighth-points, space is in points.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct BorderInput {
    #[schemars(range(min = MIN_I32, max = MAX_I32))]
    size: Option<i32>,
    #[schemars(range(min = MIN_I32, max = MAX_I32))]
    space: Option<i32>,
    color: Option<String>,
}

impl LayoutInput {
    pub(crate) fn into_native(self) -> Option<LayoutValue> {
        Some(match self {
            Self::Clear(()) => return None,
            Self::Integer(value) => LayoutValue::Integer(value),
            Self::Token(value) => LayoutValue::Token(value),
            Self::Boolean(value) => LayoutValue::Boolean(value),
            Self::Tabs(tabs) => LayoutValue::Tabs(
                tabs.into_iter()
                    .map(|tab| LayoutTab {
                        position: tab.position,
                        alignment: tab.alignment,
                        leader: tab.leader,
                    })
                    .collect(),
            ),
            Self::Border(border) => LayoutValue::Border(LayoutBorder {
                size: border.size,
                space: border.space,
                color: border.color,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn discovered_field_names_match_the_native_inventory() {
        let schema = schemars::schema_for!(Field);
        let discovered = schema.as_object().unwrap()["enum"].as_array().unwrap();
        assert_eq!(discovered.len(), LayoutField::ALL.len());
        for &native in LayoutField::ALL {
            let name = json!(native.name());
            assert!(discovered.contains(&name), "{name}");
            let parsed: Field = serde_json::from_value(name).unwrap();
            assert_eq!(LayoutField::from(parsed), native);
        }
    }

    #[test]
    fn clear_is_explicit_and_wire_payloads_are_exact() {
        let parse = |value: Value| {
            serde_json::from_value::<LayoutChange>(
                json!({"anchor": "issued", "field": "tabStops", "value": value}),
            )
        };
        assert_eq!(parse(Value::Null).unwrap().value.into_native(), None);
        assert_eq!(
            parse(json!(-12)).unwrap().value.into_native(),
            Some(LayoutValue::Integer(-12))
        );
        for invalid in [
            json!(1.5),
            json!(i64::from(i32::MAX) + 1),
            json!({"leader": "dot"}),
            json!([{"position": 40, "alignment": "left", "extra": true}]),
            json!([{"alignment": "left"}]),
        ] {
            assert!(parse(invalid).is_err());
        }
        for invalid in [
            json!({"anchor":"issued","field":"tabStops"}),
            json!({"anchor":"issued","field":"tab_stops","value":null}),
            json!({"anchor":"issued","field":"tabStops","value":null,"extra":true}),
        ] {
            assert!(serde_json::from_value::<LayoutChange>(invalid).is_err());
        }
    }
}
