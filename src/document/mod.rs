//! Pure document domain. This module depends only on the standard library.
// Keep setter declarations beside their structs while sharing assignment semantics.
macro_rules! setters {
    ($owner:ty { $($method:ident => $field:ident : $kind:ident <$ty:ty>),* $(,)? }) => {
        impl $owner {
            $(setters!(@method $method, $field, $kind, $ty);)*
        }
    };
    (@method $method:ident, $field:ident, optional, $ty:ty) => {
        #[doc = concat!("Set `", stringify!($field), "` to `Some(value)`. Other fields are unchanged.")]
        #[must_use = "builders return the updated value"]
        pub fn $method(mut self, value: $ty) -> Self {
            self.$field = Some(value);
            self
        }
    };
    (@method $method:ident, $field:ident, value, $ty:ty) => {
        #[doc = concat!("Replace `", stringify!($field), "`. Other fields are unchanged.")]
        #[must_use = "builders return the updated value"]
        pub fn $method(mut self, value: $ty) -> Self {
            self.$field = value;
            self
        }
    };
    (@method $method:ident, $field:ident, optional_into, $ty:ty) => {
        #[doc = concat!("Set `", stringify!($field), "` to `Some(value)`. Other fields are unchanged.")]
        #[must_use = "builders return the updated value"]
        pub fn $method(mut self, value: impl Into<$ty>) -> Self {
            self.$field = Some(value.into());
            self
        }
    };
    (@method $method:ident, $field:ident, into, $ty:ty) => {
        #[doc = concat!("Replace `", stringify!($field), "`. Other fields are unchanged.")]
        #[must_use = "builders return the updated value"]
        pub fn $method(mut self, value: impl Into<$ty>) -> Self {
            self.$field = value.into();
            self
        }
    };
}
pub(crate) use setters;

mod content;
mod field;
pub use field::{FieldForm, FieldInstruction, FieldProperties, RawFieldInstruction, ResolvedField};
mod hyperlink;
mod notes;
mod range;
pub use hyperlink::{HyperlinkProperties, HyperlinkTarget, ResolvedHyperlink, inline_text};
pub use notes::{CommentMetadata, NoteKind, NoteTarget, ResolvedCommentBody, ResolvedNote};
pub use range::{BookmarkName, RangeId, RangeKind};
pub(crate) use range::{story_inlines, validate_ranges};
mod resources;
mod table;
mod validation;
mod values;
pub use content::Metadata;
pub use content::{ResolvedDocument, ResolvedInline, ResolvedParagraph, ResolvedSection};
pub use resources::{ResolvedComment, ResolvedHeaderFooter, ResolvedImage};
pub use table::TableAlignment;
pub use table::TableLook;
pub use table::{CellMargins, CellTextDirection};
pub use table::{ResolvedBlock, ResolvedTable, ResolvedTableCell, ResolvedTableRow};
pub use table::{RowHeight, TableRowProperties};
pub use table::{TableCellProperties, TableProperties};
pub use values::CharacterStyleDefinition;
pub use values::DocumentDefaults;
pub use values::NumberingAlignment;
pub use values::NumberingStartOverride;
pub use values::NumberingSuffix;
pub use values::RunLanguage;
pub use values::{
    Alignment, AltText, BorderLine, BorderSide, BreakType, CellBorder, CellBorderSide, Columns,
    HeaderFooterType, ImageTransformation, LineHeight, NumberingDefinition, NumberingLevel,
    NumberingReference, Orientation, PageMargins, PageNumbering, PageSize, ParagraphFormat,
    ParagraphStyleDefinition, RunFormat, SectionProperties, SectionType, StyleDefinition,
    TabAlignment, TabLeader, TabStop, TableBorder, TableLayout, TableSectionFormat,
    TableStyleDefinition, VerticalAlignment, VerticalMerge,
};

mod diagnostic;
mod measure;
pub use diagnostic::{DeclarationPath, Diagnostic, Rule};
pub use measure::{Emu, HalfPoints, Points, Twips};

mod color;
pub use color::{Color, ThemeColor, Underline};

mod identity;
pub use identity::{AssetId, CommentId, DrawingId, NoteId, StoryId, StoryKind, StyleId};
