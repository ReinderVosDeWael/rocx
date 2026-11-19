//! Declarative DOCX generation from a declarative, asynchronous document tree.
//!
//! ```
//! use rv_docx_rs::{Document, Paragraph, Section};
//!
//! # futures::executor::block_on(async {
//! let document = Document::new(vec![
//!     Section::new(vec![Paragraph::new("Hello, world!").into()]).into(),
//! ]);
//! let resolved = document.resolve().await?;
//! let bytes = resolved.to_docx()?;
//! # Ok::<(), rv_docx_rs::Error>(())
//! # }).unwrap();
//! ```
//! Resolution consumes producers once. Resolved documents can be rendered repeatedly.
//! No runtime is required by this crate; poll resolution with your chosen executor.
//!
//! CMI inspired the initial interface; API identity is not a future requirement.
//! OOXML vocabulary and packaging are specified in
//! [ECMA-376](https://ecma-international.org/publications-and-standards/standards/ecma-376/).
//! The repository's `docs/OOXML.md` maps implemented features to edition-specific
//! clauses; `docs/MODULES.md` records the implemented domain-driven boundaries.
//! Implemented features and reference comparisons do not establish full OOXML
//! conformance.

mod adapters;
pub mod application;
pub mod authoring;
pub mod document;
mod error;
pub mod resolution;

pub use application::DocumentTemplate;
pub use authoring::Field;
pub use authoring::{
    Block, BlockChildren, Break, Comment, Document, Footer, Header, Hyperlink, ImageData, ImageRun,
    Inline, InlineChildren, Note, NoteReference, Paragraph, ParagraphContent, RangeEnd, RangeStart,
    Section, Tab, Table, TableCell, TableRow, TextRun,
};
pub use document::{
    Alignment, AltText, AssetId, BookmarkName, BorderLine, BorderSide, BreakType, CellBorder,
    CellBorderSide, Color, Columns, CommentId, CommentMetadata, DeclarationPath, Diagnostic,
    DrawingId, Emu, HalfPoints, HeaderFooterType, HyperlinkProperties, HyperlinkTarget,
    ImageTransformation, Metadata, NoteId, NoteKind, NoteTarget, NumberingDefinition,
    NumberingLevel, NumberingReference, Orientation, PageMargins, PageNumbering, PageSize,
    ParagraphFormat, ParagraphStyleDefinition, Points, RangeId, RangeKind, ResolvedBlock,
    ResolvedComment, ResolvedCommentBody, ResolvedDocument, ResolvedHeaderFooter,
    ResolvedHyperlink, ResolvedImage, ResolvedInline, ResolvedNote, ResolvedParagraph,
    ResolvedSection, ResolvedTable, ResolvedTableCell, ResolvedTableRow, Rule, RunFormat,
    SectionProperties, SectionType, StoryId, StoryKind, StyleDefinition, StyleId, TableBorder,
    TableCellProperties, TableLayout, TableProperties, TableSectionFormat, TableStyleDefinition,
    ThemeColor, Twips, Underline, VerticalAlignment, VerticalMerge, inline_text,
};
pub use document::{
    FieldForm, FieldInstruction, FieldProperties, RawFieldInstruction, ResolvedField,
};
pub use error::{Error, Result};
pub use resolution::{Component, Condition, Deferred, Property, PropertyId, PropertyRef, Value};

/// Component imports corresponding to Python's `rv_docx.declarative` module.
pub mod declarative {
    pub use crate::document::{
        Alignment, AltText, BookmarkName, BorderLine, BorderSide, BreakType, CellBorder,
        CellBorderSide, Color, Columns, CommentMetadata, Emu, HalfPoints, HeaderFooterType,
        HyperlinkProperties, HyperlinkTarget, ImageTransformation, Metadata, NoteId, NoteKind,
        NoteTarget, NumberingDefinition, NumberingLevel, NumberingReference, Orientation,
        PageMargins, PageNumbering, PageSize, ParagraphFormat, ParagraphStyleDefinition, Points,
        RangeId, RangeKind, RunFormat, SectionProperties, SectionType, StyleDefinition,
        TableBorder, TableCellProperties, TableLayout, TableProperties, TableSectionFormat,
        TableStyleDefinition, ThemeColor, Twips, Underline, VerticalAlignment, VerticalMerge,
    };
    pub use crate::{
        Block, BlockChildren, Break, Comment, Component, Condition, Deferred, Document, Footer,
        Header, Hyperlink, ImageData, ImageRun, Inline, InlineChildren, Note, NoteReference,
        Paragraph, ParagraphContent, Property, PropertyRef, RangeEnd, RangeStart, Section, Tab,
        Table, TableCell, TableRow, TextRun, Value,
    };
    pub use crate::{Field, FieldForm, FieldInstruction, FieldProperties};
}

mod xml;

pub mod ooxml;
pub use ooxml::OutputProfile;
pub mod opc;
pub mod template;
pub use template::{EditScope, TemplateEdits};
