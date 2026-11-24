//! Strict, direct numbering declarations using the same spelling as CLI readback.
use serde::Deserialize;

macro_rules! choices {
    ($vis:vis $name:ident => $core:ident { $($variant:ident = $value:literal),+ $(,)? }) => {
        #[derive(Deserialize)]
        $vis enum $name { $(#[serde(rename = $value)] $variant),+ }
        impl From<$name> for rocx::$core {
            fn from(value: $name) -> Self {
                match value { $($name::$variant => Self::$variant),+ }
            }
        }
    };
}
choices!(pub(super) NumberingSuffix => NumberingSuffix {
    Tab = "tab", Space = "space", Nothing = "nothing",
});
choices!(pub(super) NumberingAlignment => NumberingAlignment {
    Left = "left", Center = "center", Right = "right", Start = "start", End = "end",
});
choices!(Underline => Underline {
    None = "none", Single = "single", Double = "double", Dotted = "dotted",
    Dash = "dash", Wave = "wave", Words = "words",
});
choices!(Theme => ThemeColor {
    Dark1 = "dark1", Light1 = "light1", Dark2 = "dark2", Light2 = "light2",
    Accent1 = "accent1", Accent2 = "accent2", Accent3 = "accent3", Accent4 = "accent4",
    Accent5 = "accent5", Accent6 = "accent6", Hyperlink = "hyperlink",
    FollowedHyperlink = "followedHyperlink", Background1 = "background1", Text1 = "text1",
    Background2 = "background2", Text2 = "text2",
});
choices!(Alignment => Alignment {
    Left = "left", Center = "center", Right = "right", Justify = "both",
    Distribute = "distribute", JustifyMedium = "mediumKashida", JustifyHigh = "highKashida",
    JustifyLow = "lowKashida", ThaiJustify = "thaiDistribute",
});
choices!(TabAlignment => TabAlignment {
    Left = "left", Center = "center", Right = "right", Decimal = "decimal",
    Bar = "bar", Clear = "clear", Start = "start", End = "end",
});
choices!(TabLeader => TabLeader {
    None = "none", Dot = "dot", Hyphen = "hyphen", Underscore = "underscore",
    Heavy = "heavy", MiddleDot = "middleDot",
});

macro_rules! formatting {
    ($wire:ident => $core:ident { $($field:ident: $t:ty),* ; $($convert:ident: $u:ty),* }) => {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        pub(super) struct $wire {
            $($field: Option<$t>,)*
            $($convert: Option<$u>,)*
        }
        impl From<$wire> for rocx::$core {
            fn from(f: $wire) -> Self {
                Self { $($field: f.$field,)* $($convert: f.$convert.map(Into::into),)* }
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Language {
    latin: Option<String>,
    east_asia: Option<String>,
    bidi: Option<String>,
}
impl From<Language> for rocx::RunLanguage {
    fn from(f: Language) -> Self {
        Self {
            latin: f.latin,
            east_asia: f.east_asia,
            bidi: f.bidi,
        }
    }
}

#[derive(Deserialize)]
enum Auto {
    #[serde(rename = "auto")]
    Auto,
}

#[derive(Deserialize)]
#[serde(untagged, deny_unknown_fields)]
enum Color {
    Auto(Auto),
    Rgb([u8; 3]),
    Theme {
        theme: Theme,
        tint: Option<u8>,
        shade: Option<u8>,
    },
}
impl From<Color> for rocx::Color {
    fn from(c: Color) -> Self {
        match c {
            Color::Auto(_) => Self::Auto,
            Color::Rgb(rgb) => Self::Rgb(rgb),
            Color::Theme { theme, tint, shade } => Self::Theme {
                name: theme.into(),
                tint,
                shade,
            },
        }
    }
}
formatting!(LabelFormat => RunFormat {
    style: String, bold: bool, italic: bool, font: String, size: f64,
    superscript: bool, subscript: bool, strike: bool, all_caps: bool, small_caps: bool,
    snap_to_grid: bool, bold_complex_script: bool, italic_complex_script: bool,
    size_complex_script: f64, rtl: bool, complex_script: bool, font_east_asia: String,
    font_complex_script: String;
    underline: Underline, color: Color, language: Language
});

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
enum LineHeight {
    Multiple(f64),
    Exact(f64),
    AtLeast(f64),
}
impl From<LineHeight> for rocx::LineHeight {
    fn from(h: LineHeight) -> Self {
        match h {
            LineHeight::Multiple(v) => Self::Multiple(v),
            LineHeight::Exact(v) => Self::Exact(v),
            LineHeight::AtLeast(v) => Self::AtLeast(v),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TabStop {
    position: f64,
    alignment: TabAlignment,
    leader: Option<TabLeader>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ParagraphFormat {
    bidi: Option<bool>,
    heading: Option<u8>,
    style: Option<String>,
    alignment: Option<Alignment>,
    spacing_before: Option<f64>,
    spacing_after: Option<f64>,
    line_height: Option<LineHeight>,
    line_spacing: Option<f64>,
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
    tab_stops: Vec<TabStop>,
}
impl From<ParagraphFormat> for rocx::ParagraphFormat {
    fn from(f: ParagraphFormat) -> Self {
        Self {
            bidi: f.bidi,
            heading: f.heading,
            style: f.style,
            alignment: f.alignment.map(Into::into),
            spacing_before: f.spacing_before,
            spacing_after: f.spacing_after,
            line_height: f.line_height.map(Into::into),
            line_spacing: f.line_spacing,
            left_indent: f.left_indent,
            right_indent: f.right_indent,
            first_line_indent: f.first_line_indent,
            keep_together: f.keep_together,
            keep_with_next: f.keep_with_next,
            page_break_before: f.page_break_before,
            widow_control: f.widow_control,
            suppress_line_numbers: f.suppress_line_numbers,
            contextual_spacing: f.contextual_spacing,
            mirror_indents: f.mirror_indents,
            snap_to_grid: f.snap_to_grid,
            tab_stops: f
                .tab_stops
                .into_iter()
                .map(|t| rocx::TabStop {
                    position: t.position,
                    alignment: t.alignment.into(),
                    leader: t.leader.map(Into::into),
                })
                .collect(),
        }
    }
}
