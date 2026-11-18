//! Formatting, style, section and numbering values in the current authoring API.
//!
//! Current public lengths follow the initial CMI-inspired units. Wire units and
//! lexical values are defined by ECMA-376 Part 1 (2016), including §§17.18.42
//! (half-points), 22.9.2.14 (twips), and 22.9.2.15 (universal measurements).
//! [Official Part 1](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip).

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
pub struct RunFormat {
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
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParagraphFormat {
    pub heading: Option<u8>,
    pub style: Option<String>,
    pub alignment: Option<Alignment>,
    pub spacing_before: Option<f64>,
    pub spacing_after: Option<f64>,
    pub line_spacing: Option<f64>,
    pub left_indent: Option<f64>,
    pub right_indent: Option<f64>,
    pub first_line_indent: Option<f64>,
    pub keep_together: Option<bool>,
    pub keep_with_next: Option<bool>,
    pub page_break_before: Option<bool>,
    pub widow_control: Option<bool>,
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
    pub left_indent: Option<f64>,
    pub right_indent: Option<f64>,
    pub first_line_indent: Option<f64>,
    pub keep_together: Option<bool>,
    pub keep_with_next: Option<bool>,
    pub page_break_before: Option<bool>,
    pub widow_control: Option<bool>,
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
    Paragraph(Box<ParagraphStyleDefinition>),
    Table(Box<TableStyleDefinition>),
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NumberingLevel {
    pub level: u8,
    pub start: u32,
    pub format: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NumberingDefinition {
    pub id: u32,
    pub levels: Vec<NumberingLevel>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NumberingReference {
    pub id: u32,
    pub level: u8,
}
