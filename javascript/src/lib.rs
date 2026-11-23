//! JavaScript authoring adapter. Resolution and DOCX generation stay in rocx.
use js_sys::{Error, Object, Uint8Array};
use rocx::{
    AltText, Block, Document, ImageRun, Inline, Paragraph, RunFormat, Section, Table, TableCell,
    TableRow, TextRun, Value,
};
use serde::Deserialize;
use wasm_bindgen::{JsCast, prelude::*};

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
    rows: Vec<Vec<String>>,
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
    text: String,
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
    alt_text: Option<String>,
}

fn error(message: impl std::fmt::Display) -> Error {
    Error::new(&message.to_string())
}

fn decode<T: serde::de::DeserializeOwned>(value: JsValue) -> Result<T, Error> {
    serde_wasm_bindgen::from_value(value).map_err(|err| error(format!("Invalid document: {err}")))
}

fn decode_fields<T: serde::de::DeserializeOwned>(
    value: JsValue,
    allowed: &[&str],
) -> Result<T, Error> {
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
    decode(value)
}

impl RunInput {
    fn into_inline(self) -> Result<Value<Inline>, Error> {
        match decode::<RunTag>(self.0.clone())?.kind {
            RunKind::Text => {
                let TextInput {
                    text, bold, italic, ..
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
                    .with_alt_text(alt_text.map(|description| AltText {
                        description: Some(description),
                        ..Default::default()
                    }))
                    .into())
            }
        }
    }
}

fn image_bytes(bytes: &JsValue) -> Result<Vec<u8>, Error> {
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
    Ok(copy.unchecked_into::<Uint8Array>().to_vec())
}

impl BlockInput {
    fn into_block(self) -> Result<Value<Block>, Error> {
        match decode::<BlockTag>(self.0.clone())?.kind {
            BlockKind::Paragraph => {
                let ParagraphInput { runs, .. } = decode_fields(self.0, &["kind", "runs"])?;
                Ok(Paragraph::from_children(
                    runs.into_iter()
                        .map(RunInput::into_inline)
                        .collect::<Result<_, _>>()?,
                )
                .into())
            }
            BlockKind::Table => {
                let TableInput { rows, .. } = decode_fields(self.0, &["kind", "rows"])?;
                let width = rows.first().map_or(0, Vec::len);
                if width == 0 || rows.iter().any(|row| row.len() != width) {
                    return Err(error("Table rows must form a non-empty rectangular grid"));
                }
                Ok(Table::new(
                    rows.into_iter()
                        .map(|row| {
                            TableRow::new(
                                row.into_iter()
                                    .map(|text| {
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
    let declaration: Declaration = decode_fields(declaration, &["blocks"])?;
    let blocks = declaration
        .blocks
        .into_iter()
        .map(BlockInput::into_block)
        .collect::<Result<_, _>>()?;
    // wasm32-unknown-unknown has no std wall clock; the JS entry supplies one.
    let resolved = Document::new(vec![Section::new(blocks).into()])
        .with_timestamp(Some(timestamp))
        .resolve()
        .await
        .map_err(error)?;
    resolved.to_docx().map_err(|err| error(err).into())
}
