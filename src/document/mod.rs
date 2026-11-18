//! Pure document domain. This module depends only on the standard library.
mod content;
mod hyperlink;
pub use hyperlink::{HyperlinkProperties, HyperlinkTarget, ResolvedHyperlink, inline_text};
mod resources;
mod table;
mod validation;
mod values;
pub use content::Metadata;
pub use content::{ResolvedDocument, ResolvedInline, ResolvedParagraph, ResolvedSection};
pub use resources::{ResolvedComment, ResolvedHeaderFooter, ResolvedImage};
pub use table::{ResolvedBlock, ResolvedTable, ResolvedTableCell, ResolvedTableRow};
pub use table::{TableCellProperties, TableProperties};
pub use values::{
    Alignment, AltText, BorderLine, BorderSide, BreakType, CellBorder, CellBorderSide, Columns,
    HeaderFooterType, ImageTransformation, NumberingDefinition, NumberingLevel, NumberingReference,
    Orientation, PageMargins, PageNumbering, PageSize, ParagraphFormat, ParagraphStyleDefinition,
    RunFormat, SectionProperties, SectionType, StyleDefinition, TableBorder, TableLayout,
    TableSectionFormat, TableStyleDefinition, VerticalAlignment, VerticalMerge,
};

mod diagnostic;
mod measure;
pub use diagnostic::{DeclarationPath, Diagnostic, Rule};
pub use measure::{Emu, HalfPoints, Points, Twips};

mod color;
pub use color::{Color, ThemeColor, Underline};

mod identity;
pub use identity::{AssetId, CommentId, DrawingId, StoryId, StoryKind, StyleId};
