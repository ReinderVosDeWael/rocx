//! Plain-text DOCX generation from a declarative, asynchronous document tree.
//!
//! ```
//! use cmi_docx::{Document, Paragraph, Section};
//!
//! # futures::executor::block_on(async {
//! let document = Document::new(vec![
//!     Section::new(vec![Paragraph::new("Hello, world!").into()]).into(),
//! ]);
//! let resolved = document.resolve().await?;
//! let bytes = resolved.to_docx()?;
//! # Ok::<(), cmi_docx::Error>(())
//! # }).unwrap();
//! ```
//! Resolution consumes producers once. Resolved documents can be rendered repeatedly.
//! No runtime is required by this crate; poll resolution with your chosen executor.

mod error;
mod model;
mod render;
mod value;

pub use error::{Error, Result};
pub use model::{
    Block, BlockChildren, Document, Paragraph, ResolvedDocument, ResolvedParagraph,
    ResolvedSection, Section,
};
pub use value::{Condition, Value};

/// Component imports corresponding to Python's `cmi_docx.declarative` module.
pub mod declarative {
    pub use crate::{Block, BlockChildren, Condition, Document, Paragraph, Section, Value};
}
