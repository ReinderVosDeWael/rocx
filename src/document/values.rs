//! Formatting, style, section and numbering values in the current authoring API.
//!
//! Public lengths use documented authoring units. Wire units and lexical values
//! are defined by ECMA-376 Part 1 (2016), including §§17.18.42 (half-points),
//! 22.9.2.14 (twips), and 22.9.2.15 (universal measurements).
//! [Official Part 1](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip).

/// Explicit document-default overrides, not computed effective formatting.
/// Paragraph lengths use points, as for direct paragraphs. Style references and
/// heading selection are not permitted in defaults. Omission retains templates.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocumentDefaults {
    pub run: Option<RunFormat>,
    pub paragraph: Option<ParagraphFormat>,
    pub footnote_numbering: Option<super::NoteNumbering>,
    pub endnote_numbering: Option<super::NoteNumbering>,
    pub footnote_position: Option<super::FootnotePosition>,
    pub endnote_position: Option<super::EndnotePosition>,
    pub mirror_margins: Option<bool>,
    pub gutter_at_top: Option<bool>,
    pub track_revisions: Option<bool>,
}
setters!(DocumentDefaults {
    with_run => run: optional<RunFormat>,
    with_paragraph => paragraph: optional<ParagraphFormat>,
    with_footnote_numbering => footnote_numbering: optional<super::NoteNumbering>,
    with_endnote_numbering => endnote_numbering: optional<super::NoteNumbering>,
    with_footnote_position => footnote_position: optional<super::FootnotePosition>,
    with_endnote_position => endnote_position: optional<super::EndnotePosition>,
    with_mirror_margins => mirror_margins: optional<bool>,
    with_gutter_at_top => gutter_at_top: optional<bool>,
    with_track_revisions => track_revisions: optional<bool>,
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
    pub snap_to_grid: Option<bool>,
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
    with_snap_to_grid => snap_to_grid: optional<bool>,
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
    pub suppress_line_numbers: Option<bool>,
    pub contextual_spacing: Option<bool>,
    pub mirror_indents: Option<bool>,
    pub snap_to_grid: Option<bool>,
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
    with_suppress_line_numbers => suppress_line_numbers: optional<bool>,
    with_contextual_spacing => contextual_spacing: optional<bool>,
    with_mirror_indents => mirror_indents: optional<bool>,
    with_snap_to_grid => snap_to_grid: optional<bool>,
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
    /// Page edge to header distance, in EMUs.
    pub header: Option<i64>,
    /// Page edge to footer distance, in EMUs.
    pub footer: Option<i64>,
}
setters!(PageMargins {
    with_top => top: optional<i64>,
    with_bottom => bottom: optional<i64>,
    with_left => left: optional<i64>,
    with_right => right: optional<i64>,
    with_header => header: optional<i64>,
    with_footer => footer: optional<i64>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineNumberRestart {
    NewPage,
    NewSection,
    Continuous,
}
impl LineNumberRestart {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::NewPage => "newPage",
            Self::NewSection => "newSection",
            Self::Continuous => "continuous",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LineNumbering {
    /// Remove the section line-numbering settings.
    pub clear: bool,
    pub count_by: Option<u32>,
    pub start: Option<u32>,
    /// Gap between the line number and text, in twips.
    pub distance: Option<u32>,
    pub restart: Option<LineNumberRestart>,
}
setters!(LineNumbering {
    with_clear => clear: value<bool>,
    with_count_by => count_by: optional<u32>,
    with_start => start: optional<u32>,
    with_distance => distance: optional<u32>,
    with_restart => restart: optional<LineNumberRestart>,
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

/// A line border on one physical edge of a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageBorder {
    pub style: BorderLine,
    /// Line width in eighths of a point.
    pub size: Option<u32>,
    /// Offset in points.
    pub space: Option<u32>,
    pub color: Option<[u8; 3]>,
}
setters!(PageBorder {
    with_style => style: value<BorderLine>,
    with_size => size: optional<u32>,
    with_space => space: optional<u32>,
    with_color => color: optional<[u8; 3]>,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageBorderOffset {
    Page,
    Text,
}
impl PageBorderOffset {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Text => "text",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageBorderDisplay {
    AllPages,
    FirstPage,
    NotFirstPage,
}
impl PageBorderDisplay {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::AllPages => "allPages",
            Self::FirstPage => "firstPage",
            Self::NotFirstPage => "notFirstPage",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageBorderZOrder {
    Front,
    Back,
}
impl PageBorderZOrder {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::Front => "front",
            Self::Back => "back",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PageBorders {
    pub offset_from: Option<PageBorderOffset>,
    pub display: Option<PageBorderDisplay>,
    pub z_order: Option<PageBorderZOrder>,
    pub top: Option<PageBorder>,
    pub left: Option<PageBorder>,
    pub bottom: Option<PageBorder>,
    pub right: Option<PageBorder>,
}
setters!(PageBorders {
    with_offset_from => offset_from: optional<PageBorderOffset>,
    with_display => display: optional<PageBorderDisplay>,
    with_z_order => z_order: optional<PageBorderZOrder>,
    with_top => top: optional<PageBorder>,
    with_left => left: optional<PageBorder>,
    with_bottom => bottom: optional<PageBorder>,
    with_right => right: optional<PageBorder>,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentGridMode {
    Default,
    Lines,
    LinesAndChars,
    SnapToChars,
}
impl DocumentGridMode {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Lines => "lines",
            Self::LinesAndChars => "linesAndChars",
            Self::SnapToChars => "snapToChars",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentGrid {
    pub mode: Option<DocumentGridMode>,
    /// Distance between grid lines, in twips.
    pub line_pitch: Option<u32>,
    /// Signed difference from Normal-font character pitch, in 1/4096 point units.
    pub character_pitch_delta: Option<i32>,
}
setters!(DocumentGrid {
    with_mode => mode: optional<DocumentGridMode>,
    with_line_pitch => line_pitch: optional<u32>,
    with_character_pitch_delta => character_pitch_delta: optional<i32>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SectionProperties {
    pub page_size: Option<PageSize>,
    pub page_orientation: Option<Orientation>,
    pub page_margins: Option<PageMargins>,
    pub page_borders: Option<PageBorders>,
    pub document_grid: Option<DocumentGrid>,
    pub page_numbering: Option<PageNumbering>,
    pub line_numbering: Option<LineNumbering>,
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
    with_page_borders => page_borders: optional<PageBorders>,
    with_document_grid => document_grid: optional<DocumentGrid>,
    with_page_numbering => page_numbering: optional<PageNumbering>,
    with_line_numbering => line_numbering: optional<LineNumbering>,
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
    pub suppress_line_numbers: Option<bool>,
    pub contextual_spacing: Option<bool>,
    pub mirror_indents: Option<bool>,
    pub snap_to_grid: Option<bool>,
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
    with_suppress_line_numbers => suppress_line_numbers: optional<bool>,
    with_contextual_spacing => contextual_spacing: optional<bool>,
    with_mirror_indents => mirror_indents: optional<bool>,
    with_snap_to_grid => snap_to_grid: optional<bool>,
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
    Numbering(NumberingStyleDefinition),
    Paragraph(Box<ParagraphStyleDefinition>),
    Table(Box<TableStyleDefinition>),
}
/// A named numbering style associated with an authored concrete instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberingStyleDefinition {
    pub name: String,
    pub numbering_id: u32,
}
setters!(NumberingStyleDefinition {
    with_name => name: into<String>,
    with_numbering_id => numbering_id: value<u32>,
});
impl From<NumberingStyleDefinition> for StyleDefinition {
    fn from(value: NumberingStyleDefinition) -> Self {
        Self::Numbering(value)
    }
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
/// Insets as percentages of the original picture, from each physical edge.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImageCrop {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}
setters!(ImageCrop {
    with_left => left: value<f64>,
    with_top => top: value<f64>,
    with_right => right: value<f64>,
    with_bottom => bottom: value<f64>,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageAnchorReference {
    #[default]
    Page,
    Margin,
    Column,
    Character,
    LeftMargin,
    RightMargin,
    Paragraph,
    Line,
    TopMargin,
    BottomMargin,
    InsideMargin,
    OutsideMargin,
}
impl ImageAnchorReference {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Margin => "margin",
            Self::Column => "column",
            Self::Character => "character",
            Self::LeftMargin => "leftMargin",
            Self::RightMargin => "rightMargin",
            Self::Paragraph => "paragraph",
            Self::Line => "line",
            Self::TopMargin => "topMargin",
            Self::BottomMargin => "bottomMargin",
            Self::InsideMargin => "insideMargin",
            Self::OutsideMargin => "outsideMargin",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageWrap {
    #[default]
    None,
    Square,
    TopAndBottom,
    Tight,
    Through,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageWrapSide {
    BothSides,
    Left,
    Right,
    Largest,
}
impl ImageWrapSide {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::BothSides => "bothSides",
            Self::Left => "left",
            Self::Right => "right",
            Self::Largest => "largest",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageAnchorHorizontalAlignment {
    Left,
    Center,
    Right,
    Inside,
    Outside,
}
impl ImageAnchorHorizontalAlignment {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
            Self::Inside => "inside",
            Self::Outside => "outside",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageAnchorVerticalAlignment {
    Top,
    Center,
    Bottom,
    Inside,
    Outside,
}
impl ImageAnchorVerticalAlignment {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Center => "center",
            Self::Bottom => "bottom",
            Self::Inside => "inside",
            Self::Outside => "outside",
        }
    }
}
/// Floating placement for pictures and shapes. Offsets use points; relative height
/// controls drawing stacking order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImageAnchor {
    pub horizontal_reference: ImageAnchorReference,
    pub vertical_reference: ImageAnchorReference,
    pub horizontal_alignment: Option<ImageAnchorHorizontalAlignment>,
    pub vertical_alignment: Option<ImageAnchorVerticalAlignment>,
    pub x: f64,
    pub y: f64,
    pub wrap: ImageWrap,
    pub wrap_side: Option<ImageWrapSide>,
    /// Ordered polygon vertices in DrawingML wrap-path coordinates.
    pub wrap_polygon: Vec<(i32, i32)>,
    /// Wrap distances from the drawing edges in points.
    pub distance_top: Option<f64>,
    pub distance_bottom: Option<f64>,
    pub distance_left: Option<f64>,
    pub distance_right: Option<f64>,
    pub relative_height: u32,
    pub behind_text: bool,
    pub locked: Option<bool>,
    pub layout_in_cell: Option<bool>,
    pub allow_overlap: Option<bool>,
}
setters!(ImageAnchor {
    with_horizontal_reference => horizontal_reference: value<ImageAnchorReference>,
    with_vertical_reference => vertical_reference: value<ImageAnchorReference>,
    with_horizontal_alignment => horizontal_alignment: optional<ImageAnchorHorizontalAlignment>,
    with_vertical_alignment => vertical_alignment: optional<ImageAnchorVerticalAlignment>,
    with_x => x: value<f64>,
    with_y => y: value<f64>,
    with_wrap => wrap: value<ImageWrap>,
    with_wrap_side => wrap_side: optional<ImageWrapSide>,
    with_wrap_polygon => wrap_polygon: value<Vec<(i32, i32)>>,
    with_distance_top => distance_top: optional<f64>,
    with_distance_bottom => distance_bottom: optional<f64>,
    with_distance_left => distance_left: optional<f64>,
    with_distance_right => distance_right: optional<f64>,
    with_relative_height => relative_height: value<u32>,
    with_behind_text => behind_text: value<bool>,
    with_locked => locked: optional<bool>,
    with_layout_in_cell => layout_in_cell: optional<bool>,
    with_allow_overlap => allow_overlap: optional<bool>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImageTransformation {
    pub anchor: Option<ImageAnchor>,
    pub crop: Option<ImageCrop>,
    /// Clockwise rotation in degrees.
    pub rotation: Option<f64>,
    pub flip_horizontal: Option<bool>,
    pub flip_vertical: Option<bool>,
    pub width: Option<f64>,
    pub height: Option<f64>,
}
setters!(ImageTransformation {
    with_anchor => anchor: optional<ImageAnchor>,
    with_crop => crop: optional<ImageCrop>,
    with_rotation => rotation: optional<f64>,
    with_flip_horizontal => flip_horizontal: optional<bool>,
    with_flip_vertical => flip_vertical: optional<bool>,
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
    /// Direct run properties for the generated numbering label.
    pub label_format: Option<RunFormat>,
    /// Paragraph properties applied through the numbering level. Lengths use points.
    pub paragraph_format: Option<ParagraphFormat>,
    /// Paragraph style name or ID associated with this level.
    pub paragraph_style: Option<String>,
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
    with_label_format => label_format: optional<RunFormat>,
    with_paragraph_format => paragraph_format: optional<ParagraphFormat>,
    with_paragraph_style => paragraph_style: optional_into<String>,
});

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NumberingDefinition {
    pub id: u32,
    /// Explicit abstract identity shared by instances with identical levels.
    pub abstract_id: Option<u32>,
    /// Inherit levels through an authored numbering style.
    pub inherit_style: Option<String>,
    pub levels: Vec<NumberingLevel>,
    pub start_overrides: Vec<NumberingStartOverride>,
    /// Concrete-instance replacements using the same supported level subset.
    /// May coexist with a start override for the same defined level.
    pub level_overrides: Vec<NumberingLevel>,
}
setters!(NumberingDefinition {
    with_id => id: value<u32>,
    with_abstract_id => abstract_id: optional<u32>,
    with_inherit_style => inherit_style: optional_into<String>,
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
