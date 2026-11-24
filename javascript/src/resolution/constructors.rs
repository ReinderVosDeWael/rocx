use crate::{
    boundary::{self as b, Failure, Operation, Pass, Record},
    drawings,
    host::Wire,
    program, structured, text_layout,
};
use rocx::{ResolvedComment, ResolvedInline, Result, RunFormat};
use wasm_bindgen::JsValue;

fn optional<T>(
    record: &Record,
    name: &str,
    decode: impl FnOnce(&Wire, &str) -> Result<T>,
) -> Result<Option<T>> {
    let value = record.get(name)?;
    (!value.js().is_undefined())
        .then(|| decode(&value, &record.path(name)))
        .transpose()
}

fn run_properties(record: &Record) -> Result<(RunFormat, Option<ResolvedComment>)> {
    Ok((
        optional(record, "format", text_layout::run_format)?.unwrap_or_default(),
        optional(record, "comment", structured::plain_comment)?,
    ))
}

fn construct_text(input: &Wire, pass: &Pass) -> Result<ResolvedInline> {
    let record = Record::new(
        input,
        "",
        &["kind", "text", "format", "comment", "revision"],
    )?;
    let (format, comment) = run_properties(&record)?;
    Ok(ResolvedInline::Text {
        text: b::string(&record.get("text")?, "text")?,
        format,
        comment,
        revision: optional(&record, "revision", |value, path| {
            structured::text_revision(value, pass, path)
        })?,
    })
}

pub(super) fn create_inline(input: &Wire) -> Result<JsValue> {
    let kind = b::string(&b::member(&input.js(), "kind", "")?, "kind")?;
    let pass = Operation::new().pass();
    let inline = if kind == "text" {
        construct_text(input, &pass)?
    } else if let Some(inline) =
        drawings::construct_inline(input, &kind, &pass, "", run_properties)?
    {
        inline
    } else if let Some(inline) = structured::construct_inline(input, &kind, &pass, "")? {
        inline
    } else {
        return Err(Failure::invalid("kind", "unknown resolved inline kind"));
    };
    // Containers use Rust checked constructors. Direct enum arms remain
    // independently constructible, just as in Rust; validate them separately.
    Ok(program::inline_handle(inline))
}
