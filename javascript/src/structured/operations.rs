use super::{bookmark, content_control_properties, values};
use crate::{
    boundary::{self as b, Operation, Pass, Record},
    host::Wire,
    program,
};
use rocx::{
    ResolvedBlock, ResolvedBlockContentControl, ResolvedField, ResolvedHyperlink, ResolvedInline,
    ResolvedInlineContentControl, ResolvedParagraph, ResolvedTable, Result,
};
use wasm_bindgen::JsValue;

fn handles<T>(r: &Record, key: &str, decode: impl Fn(&Wire, &str) -> Result<T>) -> Result<Vec<T>> {
    let path = r.path(key);
    b::list(&r.get(key)?, &path)?
        .iter()
        .enumerate()
        .map(|(index, value)| decode(value, &format!("{path}[{index}]")))
        .collect()
}

fn inline_handles(r: &Record) -> Result<Vec<ResolvedInline>> {
    handles(r, "children", inline_value)
}

fn inline_value(i: &Wire, path: &str) -> Result<ResolvedInline> {
    b::borrow::<ResolvedInline>(i, path).map(|v| (*v).clone())
}

fn block_value(i: &Wire, path: &str) -> Result<ResolvedBlock> {
    b::borrow::<ResolvedBlock>(i, path)
        .map(|v| (*v).clone())
        .or_else(|_| {
            b::borrow::<ResolvedParagraph>(i, path).map(|v| ResolvedBlock::Paragraph((*v).clone()))
        })
        .or_else(|_| {
            b::borrow::<ResolvedTable>(i, path).map(|v| ResolvedBlock::Table((*v).clone()))
        })
}

fn decode_properties<T: Default>(r: &Record, pass: &Pass, decode: program::Decode<T>) -> Result<T> {
    let input = r.get("properties")?;
    if input.js().is_undefined() {
        Ok(T::default())
    } else {
        decode(&input, pass, &r.path("properties"))
    }
}

pub(crate) fn construct_inline(
    i: &Wire,
    kind: &str,
    pass: &Pass,
    path: &str,
) -> Result<Option<ResolvedInline>> {
    Ok(Some(match kind {
        "hyperlink" => {
            let r = Record::new(i, path, &["kind", "target", "children", "properties"])?;
            ResolvedInline::Hyperlink(ResolvedHyperlink::new(
                values::hyperlink_target(&r.get("target")?, pass, &r.path("target"))?,
                inline_handles(&r)?,
                decode_properties(&r, pass, values::hyperlink_properties)?,
            )?)
        }
        "field" => {
            let r = Record::new(i, path, &["kind", "instruction", "children", "properties"])?;
            ResolvedInline::Field(ResolvedField::new(
                values::instruction(&r.get("instruction")?, pass, &r.path("instruction"))?,
                inline_handles(&r)?,
                decode_properties(&r, pass, values::field_properties)?,
            )?)
        }
        "contentControl" => {
            let r = Record::new(i, path, &["kind", "children", "properties"])?;
            ResolvedInline::ContentControl(ResolvedInlineContentControl::new(
                inline_handles(&r)?,
                decode_properties(&r, pass, content_control_properties)?,
            )?)
        }
        "noteReference" => {
            let r = Record::new(i, path, &["kind", "target"])?;
            ResolvedInline::NoteReference(values::note_target(
                &r.get("target")?,
                pass,
                &r.path("target"),
            )?)
        }
        "rangeStart" => {
            let r = Record::new(i, path, &["kind", "id", "range"])?;
            ResolvedInline::RangeStart {
                id: values::range_id(&r.get("id")?, pass, &r.path("id"))?,
                kind: values::resolved_range(&r.get("range")?, &r.path("range"))?,
            }
        }
        "rangeEnd" => {
            let r = Record::new(i, path, &["kind", "id"])?;
            ResolvedInline::RangeEnd(values::range_id(&r.get("id")?, pass, &r.path("id"))?)
        }
        _ => return Ok(None),
    }))
}

pub(super) fn create_block_control(input: &Wire) -> Result<JsValue> {
    let r = Record::new(input, "", &["properties", "children"])?;
    let children = handles(&r, "children", block_value)?;
    let properties = decode_properties(&r, &Operation::new().pass(), content_control_properties)?;
    let control = ResolvedBlockContentControl::new(children, properties)?;
    Ok(program::block_handle(ResolvedBlock::ContentControl(
        control,
    )))
}

fn bookmark_marker(input: &Wire, start: bool) -> Result<JsValue> {
    let r = Record::new(input, "", &["name"])?;
    let name = bookmark(&r.get("name")?, &r.path("name"))?;
    let id = name.range_id();
    let view = b::record(&[
        ("kind", if start { "rangeStart" } else { "rangeEnd" }.into()),
        ("id", id.as_str().into()),
    ]);
    if start {
        let range = b::record(&[("kind", "bookmark".into()), ("name", name.as_str().into())]);
        b::set(&view, "range", &range);
    }
    Ok(view)
}

pub(super) fn bookmark_start(input: &Wire) -> Result<JsValue> {
    bookmark_marker(input, true)
}

pub(super) fn bookmark_end(input: &Wire) -> Result<JsValue> {
    bookmark_marker(input, false)
}

pub(crate) fn validate_text_revision(i: &Wire, pass: &Pass, path: &str) -> Result<()> {
    Ok(values::text_revision(i, pass, path)?.validate()?)
}

pub(crate) fn validate_comment_metadata(i: &Wire, pass: &Pass, path: &str) -> Result<()> {
    Ok(values::comment_metadata(i, pass, path)?.validate()?)
}
