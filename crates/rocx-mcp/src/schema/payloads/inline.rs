use rmcp::schemars::{self, JsonSchema};
use rocx::{
    BookmarkName, BreakType, ColumnChartProperties, ColumnChartRun, FieldForm, FieldInstruction,
    FieldProperties, Hyperlink, HyperlinkProperties, HyperlinkTarget, ImageData, ImageRun,
    InlineFraction, ResolvedField, ResolvedInline, RunFormat, ShapeProperties, ShapeRun, TextRun,
};
use serde::Deserialize;

use super::{drawing, format::RunFormatInput, resolve_component};

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
}

impl InlineInput {
    pub(crate) fn resolve(self) -> Result<ResolvedInline, String> {
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
                    .map(|inline| inline.resolve().map(Into::into))
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
            } => {
                let instruction = instruction.resolve()?;
                let children = children
                    .into_iter()
                    .map(Self::resolve)
                    .collect::<Result<_, _>>()?;
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
                .map_err(|error| error.to_string())
            }
        }
    }
}

fn leaf_format(format: Option<RunFormatInput>) -> Result<RunFormat, String> {
    let format = format.map(RunFormat::from).unwrap_or_default();
    format.validate().map_err(|error| error.to_string())?;
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
    fn resolve(self) -> Result<HyperlinkTarget, String> {
        match self {
            Self::Url { url } => HyperlinkTarget::external(url),
            Self::Bookmark { name } => BookmarkName::new(name).map(HyperlinkTarget::bookmark),
        }
        .map_err(|error| error.to_string())
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
    fn resolve(self) -> Result<FieldInstruction, String> {
        match self {
            Self::Page {} => Ok(FieldInstruction::Page),
            Self::NumPages {} => Ok(FieldInstruction::NumPages),
            Self::Reference { name } => BookmarkName::new(name).map(FieldInstruction::Reference),
            Self::Raw { text } => FieldInstruction::raw(text),
            Self::TableOfContents {} => Ok(FieldInstruction::TableOfContents),
        }
        .map_err(|error| error.to_string())
    }
}
