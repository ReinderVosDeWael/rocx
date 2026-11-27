use rmcp::schemars::{self, JsonSchema};
use rocx::{
    BookmarkName, BreakType, ColumnChartProperties, ColumnChartRun, FieldForm, FieldInstruction,
    FieldProperties, Hyperlink, HyperlinkProperties, HyperlinkTarget, ImageData, ImageRun,
    InlineFraction, ResolvedField, ResolvedHyperlink, ResolvedInline, RunFormat, ShapeProperties,
    ShapeRun, TextRun,
};
use serde::Deserialize;

use super::{drawing, format::RunFormatInput, resolve_component};
use crate::diagnostics::ScopedEditError;

const MAX_FIELD_TREE_NODES: usize = 16384;
const MAX_FIELD_TREE_CHILDREN: usize = 16384;
const MAX_FIELD_TREE_DEPTH: usize = 256;

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum InlineInput {
    Text {
        text: String,
        format: Option<RunFormatInput>,
    },
    Tab {
        format: Option<RunFormatInput>,
    },
    Break {
        #[serde(rename = "type", default)]
        break_type: BreakTypeInput,
        format: Option<RunFormatInput>,
    },
    Image {
        /// Complete encoded image as byte values, not a file path or URL.
        bytes: Vec<u8>,
        /// Drawing name metadata only; never used to load or identify the image.
        source_name: Option<String>,
        format: Option<RunFormatInput>,
        transformation: Option<Box<drawing::Transform>>,
        alt_text: Option<drawing::AltText>,
    },
    Shape {
        properties: Box<drawing::Shape>,
    },
    ColumnChart {
        /// Dimensions in points.
        width: f64,
        height: f64,
        /// Ordered labels and their corresponding finite values.
        #[schemars(length(min = 1))]
        categories: Vec<String>,
        #[schemars(length(min = 1))]
        values: Vec<f64>,
    },
    Hyperlink {
        target: LinkTargetInput,
        children: Vec<Self>,
        tooltip: Option<String>,
        history: Option<bool>,
    },
    Fraction {
        numerator: String,
        denominator: String,
    },
    Field {
        instruction: FieldInstructionInput,
        #[serde(default)]
        children: Vec<Self>,
        #[serde(default)]
        complex: bool,
        dirty: Option<bool>,
        locked: Option<bool>,
    },
    /// Flat field cache with nested fields, hyperlinks and leaf runs.
    /// Native XML limits still apply.
    FieldTree {
        /// Node 0 is the root. Each subsequent node must be referenced once
        /// by an earlier node. At most 256 container levels and 16,384 total
        /// child entries, including runs inside hyperlinks.
        #[schemars(length(min = 1, max = MAX_FIELD_TREE_NODES))]
        nodes: Vec<FieldTreeNode>,
    },
}

impl InlineInput {
    pub(crate) fn resolve(self) -> Result<ResolvedInline, ScopedEditError> {
        self.resolve_at_depth(0)
    }

    fn resolve_at_depth(self, depth: usize) -> Result<ResolvedInline, ScopedEditError> {
        match self {
            Self::Text { text, format } => {
                let mut run = TextRun::new(text);
                if let Some(format) = format {
                    run = run.with_format(RunFormat::from(format));
                }
                resolve_component(run)
            }
            Self::Tab { format } => Ok(ResolvedInline::Tab {
                format: leaf_format(format)?,
                comment: None,
            }),
            Self::Break { break_type, format } => Ok(ResolvedInline::Break {
                kind: break_type.into(),
                format: leaf_format(format)?,
                comment: None,
            }),
            Self::Image {
                bytes,
                source_name,
                format,
                transformation,
                alt_text,
            } => {
                let data = match source_name {
                    Some(source_name) => ImageData::NamedBytes { bytes, source_name },
                    None => ImageData::Bytes(bytes),
                };
                let ResolvedInline::Image { image, comment, .. } = resolve_component(
                    ImageRun::new(data)
                        .with_transformation(transformation.map(|value| (*value).into()))
                        .with_alt_text(alt_text.map(Into::into)),
                )?
                else {
                    unreachable!("ImageRun resolves to an image")
                };
                Ok(ResolvedInline::Image {
                    image,
                    format: leaf_format(format)?,
                    comment,
                })
            }
            Self::Shape { properties } => {
                resolve_component(ShapeRun::new(ShapeProperties::from(*properties)))
            }
            Self::ColumnChart {
                width,
                height,
                categories,
                values,
            } => resolve_component(ColumnChartRun::new(ColumnChartProperties {
                width,
                height,
                categories,
                values,
            })),
            Self::Hyperlink {
                target,
                children,
                tooltip,
                history,
            } => {
                let target = target.resolve()?;
                let children = children
                    .into_iter()
                    .map(|inline| inline.resolve_at_depth(depth + 1).map(Into::into))
                    .collect::<Result<Vec<_>, _>>()?;
                resolve_component(
                    Hyperlink::new(target, children)
                        .with_properties(HyperlinkProperties { tooltip, history }),
                )
            }
            Self::Fraction {
                numerator,
                denominator,
            } => resolve_component(InlineFraction::new(numerator, denominator)),
            Self::Field {
                instruction,
                children,
                complex,
                dirty,
                locked,
            } => resolve_field(
                instruction.resolve()?,
                children
                    .into_iter()
                    .map(|inline| inline.resolve_at_depth(depth + 1))
                    .collect::<Result<_, _>>()?,
                complex,
                dirty,
                locked,
            ),
            Self::FieldTree { nodes } => resolve_field_tree(nodes, depth),
        }
    }
}

fn resolve_field(
    instruction: FieldInstruction,
    children: Vec<ResolvedInline>,
    complex: bool,
    dirty: Option<bool>,
    locked: Option<bool>,
) -> Result<ResolvedInline, ScopedEditError> {
    ResolvedField::new(
        instruction,
        children,
        FieldProperties {
            form: if complex {
                FieldForm::Complex
            } else {
                FieldForm::Simple
            },
            dirty,
            locked,
        },
    )
    .map(ResolvedInline::Field)
    .map_err(Into::into)
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct FieldTreeNode {
    instruction: FieldInstructionInput,
    /// Ordered cached inlines or references to nested field nodes.
    #[serde(default)]
    #[schemars(length(max = MAX_FIELD_TREE_CHILDREN))]
    children: Vec<FieldTreeChild>,
    #[serde(default)]
    complex: bool,
    dirty: Option<bool>,
    locked: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum FieldTreeChild {
    Field {
        /// Zero-based index of a later node in this tree.
        node: usize,
    },
    Hyperlink {
        target: LinkTargetInput,
        /// Ordered display runs. Native cache hyperlinks cannot contain fields
        /// or other containers.
        #[schemars(length(max = MAX_FIELD_TREE_CHILDREN))]
        children: Vec<FieldTreeLeaf>,
        tooltip: Option<String>,
        history: Option<bool>,
    },
    Text(CachedText),
    Tab(CachedTab),
    Break(CachedBreak),
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum FieldTreeLeaf {
    Text(CachedText),
    Tab(CachedTab),
    Break(CachedBreak),
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(inline)]
struct CachedText {
    text: String,
    format: Option<RunFormatInput>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(inline)]
struct CachedTab {
    format: Option<RunFormatInput>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(inline)]
struct CachedBreak {
    #[serde(rename = "type", default)]
    break_type: BreakTypeInput,
    format: Option<RunFormatInput>,
}

impl FieldTreeChild {
    fn entry_count(&self) -> usize {
        match self {
            Self::Hyperlink { children, .. } => 1 + children.len(),
            _ => 1,
        }
    }

    fn resolve(
        self,
        fields: &mut [Option<ResolvedInline>],
    ) -> Result<ResolvedInline, ScopedEditError> {
        match self {
            Self::Field { node } => Ok(fields[node].take().expect("validated field-tree")),
            Self::Hyperlink {
                target,
                children,
                tooltip,
                history,
            } => ResolvedHyperlink::new(
                target.resolve()?,
                children
                    .into_iter()
                    .map(FieldTreeLeaf::resolve)
                    .collect::<Result<_, _>>()?,
                HyperlinkProperties { tooltip, history },
            )
            .map(ResolvedInline::Hyperlink)
            .map_err(Into::into),
            Self::Text(run) => FieldTreeLeaf::Text(run).resolve(),
            Self::Tab(run) => FieldTreeLeaf::Tab(run).resolve(),
            Self::Break(run) => FieldTreeLeaf::Break(run).resolve(),
        }
    }
}

impl FieldTreeLeaf {
    fn resolve(self) -> Result<ResolvedInline, ScopedEditError> {
        let inline = match self {
            Self::Text(CachedText { text, format }) => InlineInput::Text { text, format },
            Self::Tab(CachedTab { format }) => InlineInput::Tab { format },
            Self::Break(CachedBreak { break_type, format }) => {
                InlineInput::Break { break_type, format }
            }
        };
        inline.resolve()
    }
}

fn resolve_field_tree(
    nodes: Vec<FieldTreeNode>,
    depth: usize,
) -> Result<ResolvedInline, ScopedEditError> {
    if nodes.is_empty() || nodes.len() > MAX_FIELD_TREE_NODES {
        return Err(format!("field-tree requires 1..={MAX_FIELD_TREE_NODES} nodes").into());
    }
    if nodes
        .iter()
        .flat_map(|node| &node.children)
        .map(FieldTreeChild::entry_count)
        .sum::<usize>()
        > MAX_FIELD_TREE_CHILDREN
    {
        return Err(
            format!("field-tree exceeds {MAX_FIELD_TREE_CHILDREN} total child entries").into(),
        );
    }

    // Forward references exclude cycles. Requiring exactly one parent also
    // excludes disconnected nodes and prevents shared subtrees from expanding.
    let mut depths = vec![0; nodes.len()];
    depths[0] = depth + 1;
    for (index, node) in nodes.iter().enumerate() {
        let depth = depths[index];
        if depth == 0 {
            return Err(format!("field-tree node {index} has no parent").into());
        }
        if depth > MAX_FIELD_TREE_DEPTH {
            return Err(format!("field-tree exceeds depth {MAX_FIELD_TREE_DEPTH}").into());
        }
        for child in &node.children {
            match child {
                FieldTreeChild::Field { node: target } => {
                    let target = *target;
                    if target <= index || target >= nodes.len() {
                        return Err(format!(
                            "field-tree node {index} must reference a later node, got {target}"
                        )
                        .into());
                    }
                    if depths[target] != 0 {
                        return Err(format!(
                            "field-tree node {target} is referenced more than once"
                        )
                        .into());
                    }
                    depths[target] = depth + 1;
                }
                FieldTreeChild::Hyperlink { .. } if depth == MAX_FIELD_TREE_DEPTH => {
                    return Err(format!("field-tree exceeds depth {MAX_FIELD_TREE_DEPTH}").into());
                }
                _ => {}
            }
        }
    }

    // Build children before parents, moving (not cloning) each resolved field.
    let mut fields: Vec<Option<ResolvedInline>> = (0..nodes.len()).map(|_| None).collect();
    for (index, node) in nodes.into_iter().enumerate().rev() {
        let instruction = node.instruction.resolve()?;
        let children = node
            .children
            .into_iter()
            .map(|child| child.resolve(&mut fields))
            .collect::<Result<_, _>>()?;
        fields[index] = Some(resolve_field(
            instruction,
            children,
            node.complex,
            node.dirty,
            node.locked,
        )?);
    }
    Ok(fields[0].take().expect("validated field-tree root"))
}

fn leaf_format(format: Option<RunFormatInput>) -> Result<RunFormat, ScopedEditError> {
    let format = format.map(RunFormat::from).unwrap_or_default();
    format.validate()?;
    Ok(format)
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum BreakTypeInput {
    #[default]
    Line,
    Page,
    Column,
    TextWrapping,
}

impl From<BreakTypeInput> for BreakType {
    fn from(value: BreakTypeInput) -> Self {
        match value {
            BreakTypeInput::Line => Self::Line,
            BreakTypeInput::Page => Self::Page,
            BreakTypeInput::Column => Self::Column,
            BreakTypeInput::TextWrapping => Self::TextWrapping,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum LinkTargetInput {
    Url { url: String },
    Bookmark { name: String },
}

impl LinkTargetInput {
    fn resolve(self) -> Result<HyperlinkTarget, ScopedEditError> {
        match self {
            Self::Url { url } => HyperlinkTarget::external(url),
            Self::Bookmark { name } => BookmarkName::new(name).map(HyperlinkTarget::bookmark),
        }
        .map_err(Into::into)
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum FieldInstructionInput {
    Page {},
    NumPages {},
    Reference { name: String },
    Raw { text: String },
    TableOfContents {},
}

impl FieldInstructionInput {
    fn resolve(self) -> Result<FieldInstruction, ScopedEditError> {
        match self {
            Self::Page {} => Ok(FieldInstruction::Page),
            Self::NumPages {} => Ok(FieldInstruction::NumPages),
            Self::Reference { name } => BookmarkName::new(name).map(FieldInstruction::Reference),
            Self::Raw { text } => FieldInstruction::raw(text),
            Self::TableOfContents {} => Ok(FieldInstruction::TableOfContents),
        }
        .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flat_field_tree_resolves_ordered_native_children() {
        let inline: InlineInput = serde_json::from_value(json!({
            "kind": "field-tree",
            "nodes": [
                {
                    "instruction": {"kind": "page"},
                    "children": [
                        {"kind": "field", "node": 2},
                        {"kind": "tab"},
                        {"kind": "field", "node": 1},
                        {"kind": "break", "type": "line"}
                    ]
                },
                {"instruction": {"kind": "num-pages"}, "complex": true, "locked": false},
                {
                    "instruction": {"kind": "raw", "text": " DATE "},
                    "children": [{"kind": "text", "text": "cached"}]
                }
            ]
        }))
        .unwrap();
        let ResolvedInline::Field(root) = inline.resolve().unwrap() else {
            panic!("expected field");
        };
        let [
            ResolvedInline::Field(first),
            ResolvedInline::Tab { .. },
            ResolvedInline::Field(second),
            ResolvedInline::Break {
                kind: BreakType::Line,
                ..
            },
        ] = root.children()
        else {
            panic!("expected children in cache order");
        };
        assert_eq!(root.instruction(), &FieldInstruction::Page);
        assert_eq!(first.instruction().text(), " DATE ");
        assert_eq!(first.text(), "cached");
        assert_eq!(second.instruction(), &FieldInstruction::NumPages);
        assert_eq!(second.properties().form, FieldForm::Complex);
        assert_eq!(second.properties().locked, Some(false));
    }

    #[test]
    fn flat_field_tree_does_not_deserialize_unsupported_cache_children() {
        for child in [
            json!({"kind": "fraction", "numerator": "1", "denominator": "2"}),
            json!({
                "kind": "hyperlink",
                "target": {"kind": "url", "url": "https://example.org"},
                "children": [{"kind": "field", "node": 1}]
            }),
        ] {
            let wire = json!({
                "kind": "field-tree",
                "nodes": [{"instruction": {"kind": "page"}, "children": [child]}]
            })
            .to_string();
            assert!(
                serde_json::from_str::<InlineInput>(&wire).is_err(),
                "{wire}"
            );
        }
    }

    #[test]
    fn recursive_field_syntax_keeps_the_json_parser_boundary() {
        // Each field adds two JSON containers. The text run is container 127,
        // and its optional format object would be container 128.
        for format in [serde_json::Value::Null, json!({})] {
            let expected_ok = format.is_null();
            let mut inline = json!({"kind": "text", "text": "cached", "format": format});
            for _ in 0..63 {
                inline = json!({
                    "kind": "field", "instruction": {"kind": "page"}, "children": [inline]
                });
            }
            match serde_json::from_str::<InlineInput>(&inline.to_string()) {
                Ok(_) => assert!(expected_ok, "accepted 128 JSON containers"),
                Err(error) => {
                    assert!(!expected_ok, "rejected 127 JSON containers: {error}");
                    assert!(
                        error.to_string().contains("recursion limit exceeded"),
                        "{error}"
                    );
                }
            }
        }
    }
}
