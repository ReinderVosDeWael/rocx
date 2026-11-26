use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;

use super::super::ParagraphAlignment;

// Each wire choice maps to the identically named native variant. Explicit wire
// names preserve CLI spellings for nested formatting choices.
macro_rules! choices {
    ($name:ident => $native:ident { $($variant:ident = $wire:literal),+ $(,)? }) => {
        #[derive(Deserialize, JsonSchema)]
        enum $name { $(#[serde(rename = $wire)] $variant),+ }
        impl From<$name> for rocx::$native {
            fn from(value: $name) -> Self {
                match value { $($name::$variant => Self::$variant),+ }
            }
        }
    };
}

choices!(UnderlineInput => Underline {
    None = "none", Single = "single", Double = "double", Dotted = "dotted",
    Dash = "dash", Wave = "wave", Words = "words",
});
choices!(ThemeColorInput => ThemeColor {
    Dark1 = "dark1", Light1 = "light1", Dark2 = "dark2", Light2 = "light2",
    Accent1 = "accent1", Accent2 = "accent2", Accent3 = "accent3", Accent4 = "accent4",
    Accent5 = "accent5", Accent6 = "accent6", Hyperlink = "hyperlink",
    FollowedHyperlink = "followedHyperlink", Background1 = "background1", Text1 = "text1",
    Background2 = "background2", Text2 = "text2",
});
choices!(TabAlignmentInput => TabAlignment {
    Left = "left", Center = "center", Right = "right", Decimal = "decimal",
    Bar = "bar", Clear = "clear", Start = "start", End = "end",
});
choices!(TabLeaderInput => TabLeader {
    None = "none", Dot = "dot", Hyphen = "hyphen", Underscore = "underscore",
    Heavy = "heavy", MiddleDot = "middleDot",
});

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RunLanguageInput {
    latin: Option<String>,
    east_asia: Option<String>,
    bidi: Option<String>,
}

impl From<RunLanguageInput> for rocx::RunLanguage {
    fn from(value: RunLanguageInput) -> Self {
        Self {
            latin: value.latin,
            east_asia: value.east_asia,
            bidi: value.bidi,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
enum AutomaticColor {
    #[serde(rename = "auto")]
    Auto,
}

#[derive(Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
enum ColorInput {
    Auto(AutomaticColor),
    Rgb([u8; 3]),
    Theme {
        theme: ThemeColorInput,
        tint: Option<u8>,
        shade: Option<u8>,
    },
}

impl From<ColorInput> for rocx::Color {
    fn from(value: ColorInput) -> Self {
        match value {
            ColorInput::Auto(AutomaticColor::Auto) => Self::Auto,
            ColorInput::Rgb(rgb) => Self::Rgb(rgb),
            ColorInput::Theme { theme, tint, shade } => Self::Theme {
                name: theme.into(),
                tint,
                shade,
            },
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunFormatInput {
    style: Option<String>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<UnderlineInput>,
    font: Option<String>,
    /// Font size in points.
    size: Option<f64>,
    color: Option<ColorInput>,
    superscript: Option<bool>,
    subscript: Option<bool>,
    strike: Option<bool>,
    all_caps: Option<bool>,
    small_caps: Option<bool>,
    snap_to_grid: Option<bool>,
    language: Option<RunLanguageInput>,
    bold_complex_script: Option<bool>,
    italic_complex_script: Option<bool>,
    /// Complex-script font size in points.
    size_complex_script: Option<f64>,
    rtl: Option<bool>,
    complex_script: Option<bool>,
    font_east_asia: Option<String>,
    font_complex_script: Option<String>,
}

impl From<RunFormatInput> for rocx::RunFormat {
    fn from(value: RunFormatInput) -> Self {
        Self {
            style: value.style,
            bold: value.bold,
            italic: value.italic,
            underline: value.underline.map(Into::into),
            font: value.font,
            size: value.size,
            color: value.color.map(Into::into),
            superscript: value.superscript,
            subscript: value.subscript,
            strike: value.strike,
            all_caps: value.all_caps,
            small_caps: value.small_caps,
            snap_to_grid: value.snap_to_grid,
            language: value.language.map(Into::into),
            bold_complex_script: value.bold_complex_script,
            italic_complex_script: value.italic_complex_script,
            size_complex_script: value.size_complex_script,
            rtl: value.rtl,
            complex_script: value.complex_script,
            font_east_asia: value.font_east_asia,
            font_complex_script: value.font_complex_script,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
enum LineHeightInput {
    /// Multiple of the consumer's single-line height.
    Multiple(f64),
    /// Exact height in points.
    Exact(f64),
    /// Minimum height in points.
    AtLeast(f64),
}

impl From<LineHeightInput> for rocx::LineHeight {
    fn from(value: LineHeightInput) -> Self {
        match value {
            LineHeightInput::Multiple(value) => Self::Multiple(value),
            LineHeightInput::Exact(value) => Self::Exact(value),
            LineHeightInput::AtLeast(value) => Self::AtLeast(value),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TabStopInput {
    /// Signed position in points.
    position: f64,
    alignment: TabAlignmentInput,
    leader: Option<TabLeaderInput>,
}

impl From<TabStopInput> for rocx::TabStop {
    fn from(value: TabStopInput) -> Self {
        Self {
            position: value.position,
            alignment: value.alignment.into(),
            leader: value.leader.map(Into::into),
        }
    }
}

/// Direct paragraph properties. Spacing and indent lengths are in points.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ParagraphFormatInput {
    bidi: Option<bool>,
    heading: Option<u8>,
    style: Option<String>,
    alignment: Option<ParagraphAlignment>,
    spacing_before: Option<f64>,
    spacing_after: Option<f64>,
    /// Legacy single-line-height multiple; mutually exclusive with line_height.
    line_spacing: Option<f64>,
    line_height: Option<LineHeightInput>,
    left_indent: Option<f64>,
    right_indent: Option<f64>,
    first_line_indent: Option<f64>,
    keep_together: Option<bool>,
    keep_with_next: Option<bool>,
    page_break_before: Option<bool>,
    widow_control: Option<bool>,
    suppress_line_numbers: Option<bool>,
    contextual_spacing: Option<bool>,
    mirror_indents: Option<bool>,
    snap_to_grid: Option<bool>,
    #[serde(default)]
    tab_stops: Vec<TabStopInput>,
}

impl From<ParagraphFormatInput> for rocx::ParagraphFormat {
    fn from(value: ParagraphFormatInput) -> Self {
        Self {
            bidi: value.bidi,
            heading: value.heading,
            style: value.style,
            alignment: value.alignment.map(Into::into),
            spacing_before: value.spacing_before,
            spacing_after: value.spacing_after,
            line_spacing: value.line_spacing,
            line_height: value.line_height.map(Into::into),
            left_indent: value.left_indent,
            right_indent: value.right_indent,
            first_line_indent: value.first_line_indent,
            keep_together: value.keep_together,
            keep_with_next: value.keep_with_next,
            page_break_before: value.page_break_before,
            widow_control: value.widow_control,
            suppress_line_numbers: value.suppress_line_numbers,
            contextual_spacing: value.contextual_spacing,
            mirror_indents: value.mirror_indents,
            snap_to_grid: value.snap_to_grid,
            tab_stops: value.tab_stops.into_iter().map(Into::into).collect(),
        }
    }
}
