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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageSize {
    pub width: Option<i64>,
    pub height: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageMargins {
    pub top: Option<i64>,
    pub bottom: Option<i64>,
    pub left: Option<i64>,
    pub right: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageNumbering {
    pub start: Option<u32>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Columns {
    pub count: Option<u32>,
    pub space: Option<u32>,
    pub separator: Option<bool>,
}

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

#[derive(Debug, Clone, PartialEq)]
pub struct TableBorder {
    pub side: BorderSide,
    pub sz: u32,
    pub color: [u8; 3],
    pub val: BorderLine,
}

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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AltText {
    pub title: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
}

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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NumberingDefinition {
    pub id: u32,
    pub levels: Vec<NumberingLevel>,
    pub start_overrides: Vec<NumberingStartOverride>,
    /// Concrete-instance replacements using the same supported level subset.
    /// May coexist with a start override for the same defined level.
    pub level_overrides: Vec<NumberingLevel>,
}

/// Concrete list starting value, independent of its abstract level's start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberingStartOverride {
    pub level: u8,
    pub start: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NumberingReference {
    pub id: u32,
    pub level: u8,
}
