//! Shared resolved-value operations. Rust producer transport lives in `host`.
mod constructors;

use crate::{
    boundary::{self as b, Failure, Operation, Paths, Record},
    drawings,
    host::Wire,
    program::Hooks,
    structured, styles, tables, text_layout,
};
use rocx::{
    BookmarkName, RangeId, ResolvedCommentBody, ResolvedDocument, ResolvedInline, ResolvedNote,
    ResolvedParagraph, Result,
};
use wasm_bindgen::JsValue;

pub(crate) const HOOKS: Hooks = Hooks {
    operations: &[
        ("inlineText", (inline_text, Paths::Declaration)),
        (
            "createResolvedInline",
            (constructors::create_inline, Paths::Declaration),
        ),
        ("validateValue", (validate_value, Paths::Declaration)),
        (
            "validateAuthoringId",
            (validate_authoring_id, Paths::Declaration),
        ),
    ],
    ..Hooks::EMPTY
};

fn validate_value(input: &Wire) -> Result<JsValue> {
    let record = Record::new(input, "", &["kind", "value"])?;
    let kind = b::string(&record.get("kind")?, "kind")?;
    let path = "value";
    let value = record.get(path)?;
    match kind.as_str() {
        "resolvedInline" => b::borrow::<ResolvedInline>(&value, path)?.validate()?,
        "resolvedParagraph" => b::borrow::<ResolvedParagraph>(&value, path)?.validate()?,
        "resolvedDocument" => b::borrow::<ResolvedDocument>(&value, path)?.validate()?,
        "numberingDefinition" => return styles::validate_numbering_definition(&value, path),
        "imageTransformation" => {
            drawings::validate_transformation(&value, &Operation::new().pass(), path)?
        }
        "resolvedNote" => b::borrow::<ResolvedNote>(&value, path)?.validate()?,
        "resolvedCommentBody" => b::borrow::<ResolvedCommentBody>(&value, path)?.validate()?,
        "textRevision" => {
            structured::validate_text_revision(&value, &Operation::new().pass(), path)?
        }
        "commentMetadata" => {
            structured::validate_comment_metadata(&value, &Operation::new().pass(), path)?
        }
        _ => {
            let operation = format!("validateValue:{kind}");
            let (validate, _) = [text_layout::HOOKS.operations, tables::HOOKS.operations]
                .into_iter()
                .flatten()
                .find_map(|(name, hook)| (*name == operation).then_some(*hook))
                .ok_or_else(|| Failure::invalid("kind", "unknown validation kind"))?;
            return validate(input);
        }
    }
    Ok(JsValue::UNDEFINED)
}

fn validate_authoring_id(input: &Wire) -> Result<JsValue> {
    let record = Record::new(input, "", &["kind", "value"])?;
    let kind = b::string(&record.get("kind")?, "kind")?;
    let value = record.get("value")?;
    match kind.as_str() {
        "bookmarkName" => BookmarkName::new(b::string(&value, "value")?).map(drop),
        "rangeId" => RangeId::new(b::string(&value, "value")?).map(drop),
        "styleId" => return styles::validate_style_id(&value, "value"),
        _ => return Err(Failure::invalid("kind", "unknown authoring ID kind")),
    }
    .map_err(|diagnostic| diagnostic.at("value"))?;
    Ok(JsValue::UNDEFINED)
}

fn inline_text(input: &Wire) -> Result<JsValue> {
    let record = Record::new(input, "", &["handles"])?;
    let inlines = b::list(&record.get("handles")?, "handles")?
        .iter()
        .enumerate()
        .map(|(index, handle)| {
            b::borrow::<ResolvedInline>(handle, &format!("handles[{index}]"))
                .map(|value| (*value).clone())
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(rocx::inline_text(&inlines).into())
}
