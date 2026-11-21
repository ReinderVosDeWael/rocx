//! Formatting, style, section and numbering values in the current authoring API.
//!
//! Current public lengths follow the initial CMI-inspired units. Wire units and
//! lexical values are defined by ECMA-376 Part 1 (2016), including §§17.18.42
//! (half-points), 22.9.2.14 (twips), and 22.9.2.15 (universal measurements).
//! [Official Part 1](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip).

/// Explicit document-default overrides, not computed effective formatting.
/// Paragraph lengths use points, as for direct paragraphs. Style references and
/// heading selection are not permitted in defaults. Omission retains templates.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocumentDefaults {
    pub run: Option<RunFormat>,
    pub paragraph: Option<ParagraphFormat>,
}
setters!(DocumentDefaults {
    with_run => run: optional<RunFormat>,
    with_paragraph => paragraph: optional<ParagraphFormat>,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Alignment {
    Left,
    Center,
    Right,
    Justify,
    Distribute,
    JustifyMedium,
    JustifyHigh,
    JustifyLow,
    ThaiJustify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BreakType {
    Line,
    Page,
    Column,
    TextWrapping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Orientation {
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HeaderFooterType {
    Default,
    First,
    Even,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SectionType {
    NextPage,
    NextColumn,
    Continuous,
    EvenPage,
    OddPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VerticalMerge {
    Restart,
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TableLayout {
    Fixed,
    Autofit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BorderSide {
    Top,
    Left,
    Bottom,
    Right,
    InsideH,
    InsideV,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CellBorderSide {
    Top,
    Bottom,
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BorderLine {
    Single,
    Dashed,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RunLanguage {
    pub latin: Option<String>,
    pub east_asia: Option<String>,
    pub bidi: Option<String>,
}
setters!(RunLanguage {
    with_latin => latin: optional_into<String>,
    with_east_asia => east_asia: optional_into<String>,
    with_bidi => bidi: optional_into<String>,
});

/// Optional run properties. Builders set explicit values without resolving inheritance.
///
/// ```
/// use rocx::{RunFormat, TextRun};
/// let run = TextRun::new("Participant (REDCap ID): ")
///     .with_format(RunFormat::default().with_bold(true));
/// assert_eq!(RunFormat::default().bold, None);
/// assert_eq!(RunFormat::default().with_bold(false).bold, Some(false));
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RunFormat {
    pub style: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<super::Underline>,
    pub font: Option<String>,
    pub size: Option<f64>,
    pub color: Option<super::Color>,
    pub superscript: Option<bool>,
    pub subscript: Option<bool>,
    pub strike: Option<bool>,
    pub all_caps: Option<bool>,
    pub small_caps: Option<bool>,
    pub language: Option<RunLanguage>,
    pub bold_complex_script: Option<bool>,
    pub italic_complex_script: Option<bool>,
    pub size_complex_script: Option<f64>,
    pub rtl: Option<bool>,
    pub complex_script: Option<bool>,
    pub font_east_asia: Option<String>,
    pub font_complex_script: Option<String>,
}
setters!(RunFormat {
    with_style => style: optional_into<String>,
    with_bold => bold: optional<bool>,
    with_italic => italic: optional<bool>,
    with_underline => underline: optional<super::Underline>,
    with_font => font: optional_into<String>,
    with_size => size: optional<f64>,
    with_color => color: optional<super::Color>,
    with_superscript => superscript: optional<bool>,
    with_subscript => subscript: optional<bool>,
    with_strike => strike: optional<bool>,
    with_all_caps => all_caps: optional<bool>,
    with_small_caps => small_caps: optional<bool>,
    with_language => language: optional<RunLanguage>,
    with_bold_complex_script => bold_complex_script: optional<bool>,
    with_italic_complex_script => italic_complex_script: optional<bool>,
    with_size_complex_script => size_complex_script: optional<f64>,
    with_rtl => rtl: optional<bool>,
    with_complex_script => complex_script: optional<bool>,
    with_font_east_asia => font_east_asia: optional_into<String>,
    with_font_complex_script => font_complex_script: optional_into<String>,
});

/// Line height, separate from space before/after a paragraph (P1 §17.3.1.33).
/// Point values and multiples must be finite and nonnegative. Conversion rounds
/// to the nearest twip or 1/240 line; no layout or font metrics are calculated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineHeight {
    /// Multiple of the consumer's single-line height.
    Multiple(f64),
    /// Exact height in points; tall content may be clipped by the consumer.
    Exact(f64),
    /// Minimum height in points; the consumer may expand it for content.
    AtLeast(f64),
}

/// Custom tab alignment; legacy numbering tabs are not exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabAlignment {
    #[default]
    Left,
    Center,
    Right,
    Decimal,
    Bar,
    Clear,
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabLeader {
    None,
    Dot,
    Hyphen,
    Underscore,
    Heavy,
    MiddleDot,
}

/// A custom tab stop at a signed point position, rounded to the nearest twip.
/// A clear stop removes an inherited stop at that position, not all defaults.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TabStop {
    pub position: f64,
    pub alignment: TabAlignment,
    pub leader: Option<TabLeader>,
}
setters!(TabStop {
    with_position => position: value<f64>,
    with_alignment => alignment: value<TabAlignment>,
    with_leader => leader: optional<TabLeader>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParagraphFormat {
    pub bidi: Option<bool>,
    pub heading: Option<u8>,
    pub style: Option<String>,
    pub alignment: Option<Alignment>,
    pub spacing_before: Option<f64>,
    pub spacing_after: Option<f64>,
    pub line_spacing: Option<f64>,
    /// Typed line height; mutually exclusive with legacy `line_spacing`.
    pub line_height: Option<LineHeight>,
    pub left_indent: Option<f64>,
    pub right_indent: Option<f64>,
    pub first_line_indent: Option<f64>,
    pub keep_together: Option<bool>,
    pub keep_with_next: Option<bool>,
    pub page_break_before: Option<bool>,
    pub widow_control: Option<bool>,
    /// Empty means no local tab overrides; use explicit clear stops to inherit less.
    pub tab_stops: Vec<TabStop>,
}
setters!(ParagraphFormat {
    with_bidi => bidi: optional<bool>,
    with_heading => heading: optional<u8>,
    with_style => style: optional_into<String>,
    with_alignment => alignment: optional<Alignment>,
    with_spacing_before => spacing_before: optional<f64>,
    with_spacing_after => spacing_after: optional<f64>,
    with_line_spacing => line_spacing: optional<f64>,
    with_line_height => line_height: optional<LineHeight>,
    with_left_indent => left_indent: optional<f64>,
    with_right_indent => right_indent: optional<f64>,
    with_first_line_indent => first_line_indent: optional<f64>,
    with_keep_together => keep_together: optional<bool>,
    with_keep_with_next => keep_with_next: optional<bool>,
    with_page_break_before => page_break_before: optional<bool>,
    with_widow_control => widow_control: optional<bool>,
    with_tab_stops => tab_stops: value<Vec<TabStop>>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageSize {
    pub width: Option<i64>,
    pub height: Option<i64>,
}
setters!(PageSize {
    with_width => width: optional<i64>,
    with_height => height: optional<i64>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageMargins {
    pub top: Option<i64>,
    pub bottom: Option<i64>,
    pub left: Option<i64>,
    pub right: Option<i64>,
}
setters!(PageMargins {
    with_top => top: optional<i64>,
    with_bottom => bottom: optional<i64>,
    with_left => left: optional<i64>,
    with_right => right: optional<i64>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageNumbering {
    pub start: Option<u32>,
    pub format: Option<String>,
}
setters!(PageNumbering {
    with_start => start: optional<u32>,
    with_format => format: optional_into<String>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Columns {
    pub count: Option<u32>,
    pub space: Option<u32>,
    pub separator: Option<bool>,
}
setters!(Columns {
    with_count => count: optional<u32>,
    with_space => space: optional<u32>,
    with_separator => separator: optional<bool>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SectionProperties {
    pub page_size: Option<PageSize>,
    pub page_orientation: Option<Orientation>,
    pub page_margins: Option<PageMargins>,
    pub page_numbering: Option<PageNumbering>,
    pub columns: Option<Columns>,
    pub vertical_align: Option<VerticalAlignment>,
    pub title_page: Option<bool>,
    pub r#type: Option<SectionType>,
    /// Section page direction only; does not set paragraph or run direction.
    pub bidi: Option<bool>,
    /// Gutter placement request only; independent of page/text direction.
    pub rtl_gutter: Option<bool>,
}
setters!(SectionProperties {
    with_page_size => page_size: optional<PageSize>,
    with_page_orientation => page_orientation: optional<Orientation>,
    with_page_margins => page_margins: optional<PageMargins>,
    with_page_numbering => page_numbering: optional<PageNumbering>,
    with_columns => columns: optional<Columns>,
    with_vertical_align => vertical_align: optional<VerticalAlignment>,
    with_title_page => title_page: optional<bool>,
    with_type => r#type: optional<SectionType>,
    with_bidi => bidi: optional<bool>,
    with_rtl_gutter => rtl_gutter: optional<bool>,
});

#[derive(Debug, Clone, PartialEq)]
pub struct TableBorder {
    pub side: BorderSide,
    pub sz: u32,
    pub color: [u8; 3],
    pub val: BorderLine,
}
setters!(TableBorder {
    with_side => side: value<BorderSide>,
    with_sz => sz: value<u32>,
    with_color => color: value<[u8; 3]>,
    with_val => val: value<BorderLine>,
});

impl Default for TableBorder {
    fn default() -> Self {
        Self {
            side: BorderSide::Top,
            sz: 1,
            color: [0, 0, 0],
            val: BorderLine::Single,
        }
    }
}
impl TableBorder {
    pub fn hex_color(&self) -> String {
        format!(
            "{:02X}{:02X}{:02X}",
            self.color[0], self.color[1], self.color[2]
        )
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct CellBorder {
    pub side: CellBorderSide,
    pub sz: u32,
    pub color: [u8; 3],
    pub val: BorderLine,
}
setters!(CellBorder {
    with_side => side: value<CellBorderSide>,
    with_sz => sz: value<u32>,
    with_color => color: value<[u8; 3]>,
    with_val => val: value<BorderLine>,
});

impl Default for CellBorder {
    fn default() -> Self {
        Self {
            side: CellBorderSide::Top,
            sz: 1,
            color: [0, 0, 0],
            val: BorderLine::Single,
        }
    }
}
impl CellBorder {
    pub fn hex_color(&self) -> String {
        format!(
            "{:02X}{:02X}{:02X}",
            self.color[0], self.color[1], self.color[2]
        )
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParagraphStyleDefinition {
    pub bidi: Option<bool>,
    pub name: String,
    pub base_style: Option<String>,
    pub next_paragraph_style: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub font: Option<String>,
    pub font_size: Option<f64>,
    pub color: Option<[u8; 3]>,
    pub alignment: Option<Alignment>,
    pub spacing_before: Option<f64>,
    pub spacing_after: Option<f64>,
    pub line_spacing: Option<f64>,
    pub line_height: Option<LineHeight>,
    pub left_indent: Option<f64>,
    pub right_indent: Option<f64>,
    pub first_line_indent: Option<f64>,
    pub keep_together: Option<bool>,
    pub keep_with_next: Option<bool>,
    pub page_break_before: Option<bool>,
    pub widow_control: Option<bool>,
    pub tab_stops: Vec<TabStop>,
    pub language: Option<RunLanguage>,
    pub bold_complex_script: Option<bool>,
    pub italic_complex_script: Option<bool>,
    pub size_complex_script: Option<f64>,
    pub rtl: Option<bool>,
    pub complex_script: Option<bool>,
    pub font_east_asia: Option<String>,
    pub font_complex_script: Option<String>,
    /// Explicit character-style pairing; does not copy formatting or author a backlink.
    pub linked_style: Option<String>,
}
setters!(ParagraphStyleDefinition {
    with_name => name: into<String>,
    with_bidi => bidi: optional<bool>,
    with_base_style => base_style: optional_into<String>,
    with_next_paragraph_style => next_paragraph_style: optional_into<String>,
    with_bold => bold: optional<bool>,
    with_italic => italic: optional<bool>,
    with_underline => underline: optional<bool>,
    with_font => font: optional_into<String>,
    with_font_size => font_size: optional<f64>,
    with_color => color: optional<[u8; 3]>,
    with_alignment => alignment: optional<Alignment>,
    with_spacing_before => spacing_before: optional<f64>,
    with_spacing_after => spacing_after: optional<f64>,
    with_line_spacing => line_spacing: optional<f64>,
    with_line_height => line_height: optional<LineHeight>,
    with_left_indent => left_indent: optional<f64>,
    with_right_indent => right_indent: optional<f64>,
    with_first_line_indent => first_line_indent: optional<f64>,
    with_keep_together => keep_together: optional<bool>,
    with_keep_with_next => keep_with_next: optional<bool>,
    with_page_break_before => page_break_before: optional<bool>,
    with_widow_control => widow_control: optional<bool>,
    with_tab_stops => tab_stops: value<Vec<TabStop>>,
    with_language => language: optional<RunLanguage>,
    with_bold_complex_script => bold_complex_script: optional<bool>,
    with_italic_complex_script => italic_complex_script: optional<bool>,
    with_size_complex_script => size_complex_script: optional<f64>,
    with_rtl => rtl: optional<bool>,
    with_complex_script => complex_script: optional<bool>,
    with_font_east_asia => font_east_asia: optional_into<String>,
    with_font_complex_script => font_complex_script: optional_into<String>,
    with_linked_style => linked_style: optional_into<String>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableSectionFormat {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub font: Option<String>,
    pub font_size: Option<f64>,
    pub color: Option<[u8; 3]>,
    pub background: Option<[u8; 3]>,
    pub alignment: Option<Alignment>,
    pub spacing_before: Option<f64>,
    pub spacing_after: Option<f64>,
    pub line_height: Option<LineHeight>,
    pub tab_stops: Vec<TabStop>,
    pub bidi: Option<bool>,
    pub language: Option<RunLanguage>,
    pub bold_complex_script: Option<bool>,
    pub italic_complex_script: Option<bool>,
    pub size_complex_script: Option<f64>,
    pub rtl: Option<bool>,
    pub complex_script: Option<bool>,
    pub font_east_asia: Option<String>,
    pub font_complex_script: Option<String>,
}
setters!(TableSectionFormat {
    with_bold => bold: optional<bool>,
    with_italic => italic: optional<bool>,
    with_underline => underline: optional<bool>,
    with_font => font: optional_into<String>,
    with_font_size => font_size: optional<f64>,
    with_color => color: optional<[u8; 3]>,
    with_background => background: optional<[u8; 3]>,
    with_alignment => alignment: optional<Alignment>,
    with_spacing_before => spacing_before: optional<f64>,
    with_spacing_after => spacing_after: optional<f64>,
    with_line_height => line_height: optional<LineHeight>,
    with_tab_stops => tab_stops: value<Vec<TabStop>>,
    with_bidi => bidi: optional<bool>,
    with_language => language: optional<RunLanguage>,
    with_bold_complex_script => bold_complex_script: optional<bool>,
    with_italic_complex_script => italic_complex_script: optional<bool>,
    with_size_complex_script => size_complex_script: optional<f64>,
    with_rtl => rtl: optional<bool>,
    with_complex_script => complex_script: optional<bool>,
    with_font_east_asia => font_east_asia: optional_into<String>,
    with_font_complex_script => font_complex_script: optional_into<String>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableStyleDefinition {
    pub name: String,
    pub base_style: Option<String>,
    pub whole_table: Option<TableSectionFormat>,
    pub first_row: Option<TableSectionFormat>,
    pub last_row: Option<TableSectionFormat>,
    pub first_column: Option<TableSectionFormat>,
    pub last_column: Option<TableSectionFormat>,
    pub banding_1_row: Option<TableSectionFormat>,
    pub banding_2_row: Option<TableSectionFormat>,
    pub banding_1_column: Option<TableSectionFormat>,
    pub banding_2_column: Option<TableSectionFormat>,
    pub top_left_cell: Option<TableSectionFormat>,
    pub top_right_cell: Option<TableSectionFormat>,
    pub bottom_left_cell: Option<TableSectionFormat>,
    pub bottom_right_cell: Option<TableSectionFormat>,
}
setters!(TableStyleDefinition {
    with_name => name: into<String>,
    with_base_style => base_style: optional_into<String>,
    with_whole_table => whole_table: optional<TableSectionFormat>,
    with_first_row => first_row: optional<TableSectionFormat>,
    with_last_row => last_row: optional<TableSectionFormat>,
    with_first_column => first_column: optional<TableSectionFormat>,
    with_last_column => last_column: optional<TableSectionFormat>,
    with_banding_1_row => banding_1_row: optional<TableSectionFormat>,
    with_banding_2_row => banding_2_row: optional<TableSectionFormat>,
    with_banding_1_column => banding_1_column: optional<TableSectionFormat>,
    with_banding_2_column => banding_2_column: optional<TableSectionFormat>,
    with_top_left_cell => top_left_cell: optional<TableSectionFormat>,
    with_top_right_cell => top_right_cell: optional<TableSectionFormat>,
    with_bottom_left_cell => bottom_left_cell: optional<TableSectionFormat>,
    with_bottom_right_cell => bottom_right_cell: optional<TableSectionFormat>,
});

#[derive(Debug, Clone, PartialEq)]
pub enum StyleDefinition {
    Character(Box<CharacterStyleDefinition>),
    Paragraph(Box<ParagraphStyleDefinition>),
    Table(Box<TableStyleDefinition>),
}
/// A character style reuses run formatting; inheritance uses `base_style`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CharacterStyleDefinition {
    pub name: String,
    pub base_style: Option<String>,
    pub format: RunFormat,
    /// Explicit paragraph-style pairing; omission preserves existing links.
    pub linked_style: Option<String>,
}
setters!(CharacterStyleDefinition {
    with_name => name: into<String>,
    with_base_style => base_style: optional_into<String>,
    with_format => format: value<RunFormat>,
    with_linked_style => linked_style: optional_into<String>,
});
impl From<CharacterStyleDefinition> for StyleDefinition {
    fn from(value: CharacterStyleDefinition) -> Self {
        Self::Character(Box::new(value))
    }
}
impl From<ParagraphStyleDefinition> for StyleDefinition {
    fn from(v: ParagraphStyleDefinition) -> Self {
        Self::Paragraph(Box::new(v))
    }
}
impl From<TableStyleDefinition> for StyleDefinition {
    fn from(v: TableStyleDefinition) -> Self {
        Self::Table(Box::new(v))
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImageTransformation {
    pub width: Option<f64>,
    pub height: Option<f64>,
}
setters!(ImageTransformation {
    with_width => width: optional<f64>,
    with_height => height: optional<f64>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AltText {
    pub title: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
}
setters!(AltText {
    with_title => title: optional_into<String>,
    with_description => description: optional_into<String>,
    with_name => name: optional_into<String>,
});

/// Numbering-label alignment, independent of the paragraph's text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberingAlignment {
    Left,
    Center,
    Right,
    Start,
    End,
}

/// Content requested between the generated numbering label and paragraph text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberingSuffix {
    Tab,
    Space,
    Nothing,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NumberingLevel {
    pub level: u8,
    pub start: u32,
    pub format: String,
    pub text: String,
    /// Request decimal display for all referenced levels, without changing numFmt.
    pub is_legal: Option<bool>,
    pub suffix: Option<NumberingSuffix>,
    /// One-based restart trigger; zero requests never restarting. Not a counter.
    pub restart: Option<u32>,
    pub alignment: Option<NumberingAlignment>,
}
setters!(NumberingLevel {
    with_level => level: value<u8>,
    with_start => start: value<u32>,
    with_format => format: into<String>,
    with_text => text: into<String>,
    with_is_legal => is_legal: optional<bool>,
    with_suffix => suffix: optional<NumberingSuffix>,
    with_restart => restart: optional<u32>,
    with_alignment => alignment: optional<NumberingAlignment>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NumberingDefinition {
    pub id: u32,
    pub levels: Vec<NumberingLevel>,
    pub start_overrides: Vec<NumberingStartOverride>,
    /// Concrete-instance replacements using the same supported level subset.
    /// May coexist with a start override for the same defined level.
    pub level_overrides: Vec<NumberingLevel>,
}
setters!(NumberingDefinition {
    with_id => id: value<u32>,
    with_levels => levels: value<Vec<NumberingLevel>>,
    with_start_overrides => start_overrides: value<Vec<NumberingStartOverride>>,
    with_level_overrides => level_overrides: value<Vec<NumberingLevel>>,
});

/// Concrete list starting value, independent of its abstract level's start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberingStartOverride {
    pub level: u8,
    pub start: u32,
}
setters!(NumberingStartOverride {
    with_level => level: value<u8>,
    with_start => start: value<u32>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NumberingReference {
    pub id: u32,
    pub level: u8,
}
setters!(NumberingReference {
    with_id => id: value<u32>,
    with_level => level: value<u8>,
});
