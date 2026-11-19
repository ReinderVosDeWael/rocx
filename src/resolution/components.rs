//! Common standalone resolution/await interface.
use crate::Result;
use crate::authoring::{
    Break, Document, Footer, Header, Hyperlink, ImageRun, Paragraph, Section, Tab, Table,
    TableCell, TableRow, TextRun,
};
use crate::document::{
    ResolvedDocument, ResolvedHeaderFooter, ResolvedInline, ResolvedParagraph, ResolvedSection,
    ResolvedTable, ResolvedTableCell, ResolvedTableRow,
};
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
                Box::pin(crate::resolution::in_pass(self.resolve_at($path.into())))
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
component!(TextRun, ResolvedInline, "text_run");
component!(Hyperlink, ResolvedInline, "hyperlink");
component!(crate::Field, ResolvedInline, "field");
component!(crate::RangeStart, ResolvedInline, "range_start");
component!(crate::RangeEnd, ResolvedInline, "range_end");
component!(crate::Note, crate::ResolvedNote, "note");
component!(crate::NoteReference, ResolvedInline, "note_reference");
component!(crate::Comment, crate::ResolvedCommentBody, "comment");
component!(ImageRun, ResolvedInline, "image_run");
component!(Tab, ResolvedInline, "tab");
component!(Break, ResolvedInline, "break");
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
