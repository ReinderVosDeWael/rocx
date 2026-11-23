//! A13–A15 authoring and structured resolved-value operations.
mod operations;
mod values;
mod views;

use crate::{
    boundary::{Failure, Pass, Paths, Record},
    host::{self, Wire},
    program::{self as p, Hooks},
};
use rocx::{
    Block, BlockContentControl, BookmarkName, Comment, Condition, Deferred, Diagnostic, Document,
    Field, Hyperlink, Inline, InlineContentControl, Note, NoteReference, RangeEnd, RangeStart,
    Result, Value,
};

pub(crate) use values::{content_control_properties, text_revision};
pub(crate) use views::{content_control_properties_view, plain_comment_view, text_revision_view};

macro_rules! component {
    ($resolve:ident, $decode:ident, $path:literal, $project:path) => {
        fn $resolve(
            input: Wire,
            pass: Pass,
        ) -> futures_util::future::BoxFuture<'static, Result<Option<p::Projection>>> {
            Box::pin(async move { p::resolve($decode(&input, &pass, $path)?, $project).await })
        }
    };
}

component!(resolve_link, hyperlink, "hyperlink", p::inline_handle);
component!(resolve_field, field, "field", p::inline_handle);
component!(resolve_note, note, "note", views::note_handle);
component!(
    resolve_note_reference,
    note_reference,
    "noteReference",
    p::inline_handle
);
component!(resolve_comment, comment, "comment", views::comment_handle);
component!(resolve_start, range_start, "rangeStart", p::inline_handle);
component!(resolve_end, range_end, "rangeEnd", p::inline_handle);
component!(
    resolve_inline_control,
    inline_control,
    "contentControl",
    p::inline_handle
);
component!(
    resolve_block_control,
    block_control,
    "contentControl",
    p::block_handle
);

pub(crate) const HOOKS: Hooks = Hooks {
    components: &[
        ("hyperlink", None, resolve_link),
        ("field", None, resolve_field),
        ("note", None, resolve_note),
        ("noteReference", None, resolve_note_reference),
        ("comment", None, resolve_comment),
        ("rangeStart", None, resolve_start),
        ("rangeEnd", None, resolve_end),
        ("contentControl", Some("inline"), resolve_inline_control),
        ("contentControl", Some("block"), resolve_block_control),
    ],
    blocks: &[("contentControl", |input, pass, path| {
        block_control(input, pass, path).map(Block::from)
    })],
    inlines: &[
        ("hyperlink", |i, p, l| hyperlink(i, p, l).map(Inline::from)),
        ("field", |i, p, l| field(i, p, l).map(Inline::from)),
        ("noteReference", |i, p, l| {
            note_reference(i, p, l).map(Inline::from)
        }),
        ("rangeStart", |i, p, l| {
            range_start(i, p, l).map(Inline::from)
        }),
        ("rangeEnd", |i, p, l| range_end(i, p, l).map(Inline::from)),
        ("contentControl", |i, p, l| {
            inline_control(i, p, l).map(Inline::from)
        }),
    ],
    inline_views: &[views::inline_view],
    block_views: &[views::block_view],
    document_fields: &["notes"],
    document: document_notes,
    document_view: views::document_view,
    operations: &[
        (
            "createResolvedBlockControl",
            (operations::create_block_control, Paths::Declaration),
        ),
        (
            "bookmarkStart",
            (operations::bookmark_start, Paths::Declaration),
        ),
        (
            "bookmarkEnd",
            (operations::bookmark_end, Paths::Declaration),
        ),
    ],
    ..Hooks::EMPTY
};

pub(crate) use operations::{construct_inline, validate_comment_metadata, validate_text_revision};
pub(crate) use values::plain_comment;

fn guard(record: &Record, pass: &Pass) -> Result<Condition> {
    pass.condition(record.get("condition")?, record.path("condition"))
}

fn hyperlink(i: &Wire, pass: &Pass, path: &str) -> Result<Hyperlink> {
    let r = Record::new(
        i,
        path,
        &["kind", "target", "children", "properties", "condition"],
    )?;
    Ok(Hyperlink {
        target: p::required(&r, pass, "target", |i, pass, path| {
            relative(values::hyperlink_target(i, pass, path), path)
        })?,
        children: p::required(&r, pass, "children", p::inlines)?,
        properties: p::optional(&r, pass, "properties", values::hyperlink_properties)?
            .unwrap_or_default(),
        condition: guard(&r, pass)?,
    })
}

fn field(i: &Wire, pass: &Pass, path: &str) -> Result<Field> {
    let r = Record::new(
        i,
        path,
        &["kind", "instruction", "children", "properties", "condition"],
    )?;
    Ok(Field {
        instruction: p::required(&r, pass, "instruction", |i, pass, path| {
            relative(values::instruction(i, pass, path), path)
        })?,
        children: p::required(&r, pass, "children", p::inlines)?,
        properties: p::optional(&r, pass, "properties", values::field_properties)?
            .unwrap_or_default(),
        condition: guard(&r, pass)?,
    })
}

fn note(i: &Wire, pass: &Pass, path: &str) -> Result<Note> {
    let r = Record::new(i, path, &["kind", "target", "children", "condition"])?;
    values::expect_kind(&r, "note")?;
    Ok(Note::new(
        p::required(&r, pass, "target", values::note_target)?,
        p::required(&r, pass, "children", p::blocks)?,
    )
    .when(guard(&r, pass)?))
}

fn document_notes(doc: &mut Document, r: &Record, pass: &Pass) -> Result<()> {
    if let Some(notes) = p::optional(r, pass, "notes", |input, pass, path| {
        p::values(input, pass, path, note)
    })? {
        doc.notes = notes;
    }
    Ok(())
}

fn note_reference(i: &Wire, pass: &Pass, path: &str) -> Result<NoteReference> {
    let r = Record::new(i, path, &["kind", "target", "condition"])?;
    Ok(
        NoteReference::new(p::required(&r, pass, "target", values::note_target)?)
            .when(guard(&r, pass)?),
    )
}

fn comment(i: &Wire, pass: &Pass, path: &str) -> Result<Comment> {
    let r = Record::new(i, path, &["kind", "children", "metadata", "condition"])?;
    values::expect_kind(&r, "comment")?;
    Ok(Comment {
        children: p::required(&r, pass, "children", p::blocks)?,
        metadata: p::optional(&r, pass, "metadata", values::comment_metadata)?.unwrap_or_default(),
        condition: guard(&r, pass)?,
    })
}

fn range_start(i: &Wire, pass: &Pass, path: &str) -> Result<RangeStart> {
    let r = Record::new(i, path, &["kind", "id", "range", "condition"])?;
    let id = Deferred::new(p::required(&r, pass, "id", range_id_field)?);
    let comment_id = id.clone();
    let input = r.get("range")?;
    let path = r.path("range");
    pass.alias(r.path("kind"), path.clone());
    let range_pass = pass.clone();
    let range = Value::Async(Box::pin(async move {
        let value = host::resolve_value(input).await?;
        relative(
            values::range(&value, &range_pass, &path, comment_id).await,
            &path,
        )
        .map(Value::ready)
    }));
    Ok(RangeStart::new(id, range).when(guard(&r, pass)?))
}

fn range_end(i: &Wire, pass: &Pass, path: &str) -> Result<RangeEnd> {
    let r = Record::new(i, path, &["kind", "id", "condition"])?;
    Ok(RangeEnd::new(p::required(&r, pass, "id", range_id_field)?).when(guard(&r, pass)?))
}

fn range_id_field(i: &Wire, pass: &Pass, path: &str) -> Result<rocx::RangeId> {
    relative(values::range_id(i, pass, path), path)
}

fn inline_control(i: &Wire, pass: &Pass, path: &str) -> Result<InlineContentControl> {
    let r = Record::new(i, path, &["kind", "children", "properties", "condition"])?;
    Ok(InlineContentControl {
        children: p::required(&r, pass, "children", p::inlines)?,
        properties: p::optional(&r, pass, "properties", content_control_properties)?
            .unwrap_or_default(),
        condition: guard(&r, pass)?,
    })
}

fn block_control(i: &Wire, pass: &Pass, path: &str) -> Result<BlockContentControl> {
    let r = Record::new(i, path, &["kind", "children", "properties", "condition"])?;
    Ok(BlockContentControl {
        children: p::required(&r, pass, "children", p::blocks)?,
        properties: p::optional(&r, pass, "properties", content_control_properties)?
            .unwrap_or_default(),
        condition: guard(&r, pass)?,
    })
}

fn located<T>(value: std::result::Result<T, Diagnostic>, path: &str) -> Result<T> {
    value.map_err(|error| error.at(path).into())
}

// Adapter shape failures already have absolute paths. Rust prepends the field
// path to domain diagnostics, so make only those relative for deferred use.
fn relative<T>(value: Result<T>, path: &str) -> Result<T> {
    value.map_err(|error| {
        let rocx::Error::Diagnostic(mut diagnostic) = error else {
            return error;
        };
        if let Some(location) = diagnostic.location.as_deref() {
            if location == path {
                diagnostic.location = None;
            } else if let Some(suffix) = location
                .strip_prefix(path)
                .and_then(|s| s.strip_prefix('.'))
            {
                diagnostic.location = Some(suffix.to_owned());
            }
        }
        diagnostic.into()
    })
}

fn bookmark(i: &Wire, path: &str) -> Result<BookmarkName> {
    located(BookmarkName::new(crate::boundary::string(i, path)?), path)
}

fn unknown(r: &Record, key: &str, label: &str) -> rocx::Error {
    Failure::invalid(&r.path(key), format!("unknown {label}"))
}
