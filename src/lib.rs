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
pub use authoring::{
    Block, BlockChildren, Break, Document, Footer, Header, ImageData, ImageRun, Inline,
    InlineChildren, Paragraph, ParagraphContent, Section, Tab, Table, TableCell, TableRow, TextRun,
};
pub use document::{
    Alignment, AltText, AssetId, BorderLine, BorderSide, BreakType, CellBorder, CellBorderSide,
    Color, Columns, CommentId, DeclarationPath, Diagnostic, DrawingId, Emu, HalfPoints,
    HeaderFooterType, ImageTransformation, Metadata, NumberingDefinition, NumberingLevel,
    NumberingReference, Orientation, PageMargins, PageNumbering, PageSize, ParagraphFormat,
    ParagraphStyleDefinition, Points, ResolvedBlock, ResolvedComment, ResolvedDocument,
    ResolvedHeaderFooter, ResolvedImage, ResolvedInline, ResolvedParagraph, ResolvedSection,
    ResolvedTable, ResolvedTableCell, ResolvedTableRow, Rule, RunFormat, SectionProperties,
    SectionType, StoryId, StoryKind, StyleDefinition, StyleId, TableBorder, TableCellProperties,
    TableLayout, TableProperties, TableSectionFormat, TableStyleDefinition, ThemeColor, Twips,
    Underline, VerticalAlignment, VerticalMerge,
};
pub use error::{Error, Result};
pub use resolution::{Component, Condition, Deferred, Property, PropertyId, PropertyRef, Value};

/// Component imports corresponding to Python's `rv_docx.declarative` module.
pub mod declarative {
    pub use crate::document::{
        Alignment, AltText, BorderLine, BorderSide, BreakType, CellBorder, CellBorderSide, Color,
        Columns, Emu, HalfPoints, HeaderFooterType, ImageTransformation, Metadata,
        NumberingDefinition, NumberingLevel, NumberingReference, Orientation, PageMargins,
        PageNumbering, PageSize, ParagraphFormat, ParagraphStyleDefinition, Points, RunFormat,
        SectionProperties, SectionType, StyleDefinition, TableBorder, TableCellProperties,
        TableLayout, TableProperties, TableSectionFormat, TableStyleDefinition, ThemeColor, Twips,
        Underline, VerticalAlignment, VerticalMerge,
    };
    pub use crate::{
        Block, BlockChildren, Break, Component, Condition, Deferred, Document, Footer, Header,
        ImageData, ImageRun, Inline, InlineChildren, Paragraph, ParagraphContent, Property,
        PropertyRef, Section, Tab, Table, TableCell, TableRow, TextRun, Value,
    };
}

mod xml;

pub mod ooxml;
pub use ooxml::OutputProfile;
pub mod opc;
pub mod template;
pub use template::{EditScope, TemplateEdits};
