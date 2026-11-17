//! Declarative DOCX generation from a declarative, asynchronous document tree.
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
    Block, BlockChildren, Document, Inline, InlineChildren, Paragraph, ResolvedDocument,
    ResolvedParagraph, ResolvedSection, ResolvedTextRun, Section, TextRun,
};
pub use value::{Condition, Value};

/// Component imports corresponding to Python's `cmi_docx.declarative` module.
pub mod declarative {
    pub use crate::types::*;
    pub use crate::{
        Block, BlockChildren, Break, Component, Condition, Document, Footer, Header, ImageData,
        ImageRun, Inline, InlineChildren, Paragraph, Section, Tab, Table, TableCell, TableRow,
        TextRun, Value,
    };
}

mod components;
mod image;
mod table;
mod types;
mod xml;
pub use components::*;
pub use table::*;
pub use types::*;

mod format;
mod package;
mod styles;
mod template;

/// Common resolution interface for conditional declarative components.
/// Resolution consumes the declaration; false conditions return `None`.
pub trait Component: Send + Sized + 'static {
    type Resolved: Send;
    fn resolve(self) -> futures_util::future::BoxFuture<'static, Result<Option<Self::Resolved>>>;
}
macro_rules! component {
    ($t:ty,$resolved:ty,$path:literal) => {
        impl Component for $t {
            type Resolved = $resolved;
            fn resolve(
                self,
            ) -> futures_util::future::BoxFuture<'static, Result<Option<Self::Resolved>>> {
                Box::pin(self.resolve_at($path.into()))
            }
        }
        impl std::future::IntoFuture for $t {
            type Output = Result<Option<$resolved>>;
            type IntoFuture = futures_util::future::BoxFuture<'static, Self::Output>;
            fn into_future(self) -> Self::IntoFuture {
                Component::resolve(self)
            }
        }
    };
}
component!(Section, ResolvedSection, "section");
component!(Paragraph, ResolvedParagraph, "paragraph");
component!(TextRun, ResolvedTextRun, "text_run");
component!(ImageRun, ResolvedTextRun, "image_run");
component!(Tab, ResolvedTextRun, "tab");
component!(Break, ResolvedTextRun, "break");
component!(Table, ResolvedTable, "table");
component!(TableRow, ResolvedTableRow, "table_row");
component!(TableCell, ResolvedTableCell, "table_cell");
component!(Header, ResolvedHeaderFooter, "header");
component!(Footer, ResolvedHeaderFooter, "footer");
impl std::future::IntoFuture for Document {
    type Output = Result<ResolvedDocument>;
    type IntoFuture = futures_util::future::BoxFuture<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.resolve())
    }
}

#[cfg(feature = "python-bindings")]
mod python_bindings;
