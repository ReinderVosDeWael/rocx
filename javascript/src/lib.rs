//! JavaScript authoring adapter. Resolution and DOCX generation stay in rocx.
use js_sys::{Error, JsString, Object, Reflect, Uint8Array};
use rocx::{
    AltText, Block, Document, ImageRun, Inline, Paragraph, RunFormat, Section, Table, TableCell,
    TableRow, TextRun, Value,
};
use serde::{Deserialize, Deserializer, de::Error as _};
use wasm_bindgen::{JsCast, prelude::*};

mod boundary;
mod drawings;
mod editor;
mod host;
mod inspection;
mod program;
mod resolution;
mod structured;
mod styles;
mod tables;
mod template;
mod text_layout;

struct CallerText(String);

impl<'de> Deserialize<'de> for CallerText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Keep the original JS string until validation; converting it to Rust
        // first would silently replace unpaired surrogates with U+FFFD.
        let text: JsString = serde_wasm_bindgen::preserve::deserialize(deserializer)?;
        if !text.is_well_formed() {
            return Err(D::Error::custom("malformed UTF-16: unpaired surrogate"));
        }
        Ok(Self(String::from(text)))
    }
}

#[derive(Deserialize)]
struct Declaration {
    blocks: Vec<BlockInput>,
}

#[derive(Deserialize)]
struct BlockInput(#[serde(with = "serde_wasm_bindgen::preserve")] JsValue);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum BlockKind {
    Paragraph,
    Table,
}

#[derive(Deserialize)]
struct BlockTag {
    kind: BlockKind,
}

#[derive(Deserialize)]
struct ParagraphInput {
    #[serde(rename = "kind")]
    _kind: BlockKind,
    runs: Vec<RunInput>,
}

#[derive(Deserialize)]
struct TableInput {
    #[serde(rename = "kind")]
    _kind: BlockKind,
    rows: Vec<Vec<CallerText>>,
}

// Extract the nested JS object before deserializing its variant so serde does
// not buffer a potentially large typed array as arbitrary JS/serde values.
#[derive(Deserialize)]
struct RunInput(#[serde(with = "serde_wasm_bindgen::preserve")] JsValue);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum RunKind {
    Text,
    Image,
}

#[derive(Deserialize)]
struct RunTag {
    kind: RunKind,
}

#[derive(Deserialize)]
struct TextInput {
    #[serde(rename = "kind")]
    _kind: RunKind,
    text: CallerText,
    bold: Option<bool>,
    italic: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageInput {
    #[serde(rename = "kind")]
    _kind: RunKind,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    bytes: JsValue,
    alt_text: Option<CallerText>,
}

fn error(message: impl std::fmt::Display) -> Error {
    Error::new(&message.to_string())
}

fn with_property(error: Error, key: &str, value: &str) -> Error {
    // Diagnostics are optional; keep the original error if setting one fails.
    let _ = Reflect::set(&error, &key.into(), &value.into());
    error
}

fn at(error: Error, path: &str) -> Error {
    with_property(error, "path", path)
}

fn core_error(source: rocx::Error) -> Error {
    let error = error(&source);
    // Core declaration paths can name synthetic sections and other Rust-only
    // fields. Don't expose them as paths into the public JS input.
    match source.rule() {
        Some(rule) => with_property(error, "rule", rule.code()),
        None => error,
    }
}

fn decode<T: serde::de::DeserializeOwned>(value: JsValue) -> Result<T, Error> {
    serde_wasm_bindgen::from_value(value).map_err(|err| error(format!("Invalid document: {err}")))
}

fn check_fields(value: &JsValue, allowed: &[&str]) -> Result<(), Error> {
    if !value.is_object() {
        return Err(error("Invalid document: expected an object"));
    }
    // serde-wasm-bindgen's struct decoder only visits fields it knows.
    for key in Object::keys(value.unchecked_ref::<Object>()).iter() {
        if let Some(key) = key
            .as_string()
            .filter(|key| !allowed.contains(&key.as_str()))
        {
            return Err(error(format!("Invalid document: unknown field `{key}`")));
        }
    }
    Ok(())
}

fn decode_fields<T: serde::de::DeserializeOwned>(
    value: JsValue,
    allowed: &[&str],
) -> Result<T, Error> {
    check_fields(&value, allowed)?;
    decode(value)
}

impl RunInput {
    fn into_inline(self) -> Result<Value<Inline>, Error> {
        match decode::<RunTag>(self.0.clone())?.kind {
            RunKind::Text => {
                let TextInput {
                    text: CallerText(text),
                    bold,
                    italic,
                    ..
                } = decode_fields(self.0, &["kind", "text", "bold", "italic"])?;
                Ok(TextRun::new(text)
                    .with_format(RunFormat {
                        bold,
                        italic,
                        ..Default::default()
                    })
                    .into())
            }
            RunKind::Image => {
                let ImageInput {
                    bytes, alt_text, ..
                } = decode_fields(self.0, &["kind", "bytes", "altText"])?;
                Ok(ImageRun::new(image_bytes(&bytes)?)
                    .with_alt_text(alt_text.map(|CallerText(description)| AltText {
                        description: Some(description),
                        ..Default::default()
                    }))
                    .into())
            }
        }
    }
}

fn image_bytes(bytes: &JsValue) -> Result<Vec<u8>, Error> {
    copy_image_view(bytes).map(|bytes| bytes.to_vec())
}

fn copy_image_view(bytes: &JsValue) -> Result<Uint8Array, Error> {
    use js_sys::{Array, Reflect, Symbol};

    let uint8_prototype = Object::get_prototype_of(&Uint8Array::new_with_length(0));
    let typed_array_prototype = Object::get_prototype_of(&uint8_prototype);
    // Call the intrinsic getter directly: it reads the internal typed-array
    // brand across realms without consulting properties on the input.
    let brand = Object::get_own_property_descriptor_symbol(
        &typed_array_prototype,
        &Symbol::to_string_tag(),
    )
    .ok()
    .and_then(|descriptor| descriptor.get_get())
    .and_then(|getter| getter.call0(bytes).ok());
    if brand.and_then(|brand| brand.as_string()).as_deref() != Some("Uint8Array") {
        return Err(error("Image bytes must be a Uint8Array"));
    }

    // The typed-array constructor copies exactly the source view, ignoring
    // overridden properties. Reflect catches exceptions from unusable views.
    let copy = Reflect::construct(&uint8_prototype.constructor(), &Array::of1(bytes))
        .map_err(|_| error("Cannot copy image bytes from Uint8Array"))?;
    Ok(copy.unchecked_into::<Uint8Array>())
}

impl BlockInput {
    fn into_block(self, index: usize) -> Result<Value<Block>, Error> {
        let path = format!("blocks[{index}]");
        match decode::<BlockTag>(self.0.clone())
            .map_err(|err| at(err, &path))?
            .kind
        {
            BlockKind::Paragraph => {
                let ParagraphInput { runs, .. } =
                    decode_fields(self.0, &["kind", "runs"]).map_err(|err| at(err, &path))?;
                Ok(Paragraph::from_children(
                    runs.into_iter()
                        .enumerate()
                        .map(|(index, run)| {
                            run.into_inline()
                                .map_err(|err| at(err, &format!("{path}.runs[{index}]")))
                        })
                        .collect::<Result<_, _>>()?,
                )
                .into())
            }
            BlockKind::Table => {
                let TableInput { rows, .. } =
                    decode_fields(self.0, &["kind", "rows"]).map_err(|err| at(err, &path))?;
                let invalid_grid = || error("Table rows must form a non-empty rectangular grid");
                let Some(first) = rows.first() else {
                    return Err(at(invalid_grid(), &format!("{path}.rows")));
                };
                if let Some(index) = rows
                    .iter()
                    .position(|row| row.is_empty() || row.len() != first.len())
                {
                    return Err(at(invalid_grid(), &format!("{path}.rows[{index}]")));
                }
                Ok(Table::new(
                    rows.into_iter()
                        .map(|row| {
                            TableRow::new(
                                row.into_iter()
                                    .map(|CallerText(text)| {
                                        TableCell::new(vec![Paragraph::new(text).into()]).into()
                                    })
                                    .collect(),
                            )
                            .into()
                        })
                        .collect(),
                )
                .into())
            }
        }
    }
}

#[wasm_bindgen]
pub async fn render_docx(declaration: JsValue, timestamp: String) -> Result<Vec<u8>, JsValue> {
    check_fields(&declaration, &["blocks"])?;
    let declaration: Declaration = decode(declaration).map_err(|err| at(err, "blocks"))?;
    let blocks = declaration
        .blocks
        .into_iter()
        .enumerate()
        .map(|(index, block)| block.into_block(index))
        .collect::<Result<_, _>>()?;
    // wasm32-unknown-unknown has no std wall clock; the JS entry supplies one.
    let resolved = Document::new(vec![Section::new(blocks).into()])
        .with_timestamp(Some(timestamp))
        .resolve()
        .await
        .map_err(core_error)?;
    resolved.to_docx().map_err(|err| core_error(err).into())
}
