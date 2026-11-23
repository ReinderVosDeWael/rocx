//! B0 bootstrap program; feature owners register extensions through `Hooks`.
use crate::{
    boundary::{self as b, Failure, Operation, Pass, Paths, Record},
    host::{self, Wire, field},
};
use futures_util::future::BoxFuture;
use js_sys::{Array, Uint8Array};
use rocx::{
    Block, Component, Document, ImageRun, Inline, OutputProfile, Paragraph, ResolvedBlock,
    ResolvedDocument, ResolvedInline, ResolvedParagraph, ResolvedSection, Result, Section, TextRun,
    Value, application::RenderOptions,
};
use wasm_bindgen::prelude::*;

pub(crate) type Decode<T> = fn(&Wire, &Pass, &str) -> Result<T>;
pub(crate) type Resolve = fn(Wire, Pass) -> BoxFuture<'static, Result<Option<Projection>>>;
pub(crate) type View<T> = fn(&T) -> Option<JsValue>;
pub(crate) type Dispatch = fn(&Wire) -> Result<JsValue>;
pub(crate) type Named<T> = &'static [(&'static str, T)];
pub(crate) type OperationHook = (Dispatch, Paths);

/// Rust-only future output. Construct JS projections after the pass ends.
pub(crate) struct Projection(Box<dyn FnOnce() -> JsValue + Send>);
impl Projection {
    pub(crate) fn new<T: Send + 'static>(value: T, project: fn(T) -> JsValue) -> Self {
        Self(Box::new(move || project(value)))
    }
    fn js(self) -> JsValue {
        (self.0)()
    }
}

pub(crate) struct Hooks {
    pub(crate) components: &'static [(&'static str, Option<&'static str>, Resolve)],
    pub(crate) blocks: Named<Decode<Block>>,
    pub(crate) inlines: Named<Decode<Inline>>,
    pub(crate) block_views: &'static [View<ResolvedBlock>],
    pub(crate) inline_views: &'static [View<ResolvedInline>],
    pub(crate) document_fields: &'static [&'static str],
    pub(crate) document: fn(&mut Document, &Record, &Pass) -> Result<()>,
    pub(crate) document_view: fn(&ResolvedDocument, &JsValue),
    pub(crate) section_view: fn(&ResolvedSection, &JsValue),
    pub(crate) paragraph_view: fn(&ResolvedParagraph, &JsValue),
    pub(crate) document_methods: &'static [(&'static str, &'static str)],
    pub(crate) operations: Named<OperationHook>,
}
impl Hooks {
    pub(crate) const EMPTY: Self = Self {
        components: &[],
        blocks: &[],
        inlines: &[],
        block_views: &[],
        inline_views: &[],
        document_fields: &[],
        document: |_, _, _| Ok(()),
        document_view: |_, _| {},
        section_view: |_, _| {},
        paragraph_view: |_, _| {},
        document_methods: &[],
        operations: &[],
    };
}

use crate::{
    drawings, editor, inspection, resolution, structured, styles, tables, template, text_layout,
};
const MODULES: &[Hooks] = &[
    text_layout::HOOKS,
    styles::HOOKS,
    tables::HOOKS,
    drawings::HOOKS,
    structured::HOOKS,
    resolution::HOOKS,
    template::HOOKS,
    inspection::HOOKS,
    editor::HOOKS,
];

fn find<T: Copy>(entries: Named<T>, key: &str) -> Option<T> {
    entries
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, value)| *value)
}

fn kind(input: &Wire, path: &str) -> Result<String> {
    b::string(
        &b::member(&input.js(), "kind", path)?,
        &b::child_path(path, "kind"),
    )
}

pub(crate) fn required<T: Send + 'static>(
    record: &Record,
    pass: &Pass,
    key: &str,
    decode: Decode<T>,
) -> Result<Value<T>> {
    Ok(field(
        record.get(key)?,
        pass.clone(),
        record.path(key),
        decode,
    ))
}

pub(crate) fn optional<T: Send + 'static>(
    record: &Record,
    pass: &Pass,
    key: &str,
    decode: Decode<T>,
) -> Result<Option<Value<T>>> {
    let value = record.get(key)?;
    Ok((!value.js().is_undefined()).then(|| field(value, pass.clone(), record.path(key), decode)))
}

pub(crate) fn values<T: Send + 'static>(
    input: &Wire,
    pass: &Pass,
    path: &str,
    decode: Decode<T>,
) -> Result<Vec<Value<T>>> {
    Ok(b::list(input, path)?
        .into_iter()
        .enumerate()
        .map(|(index, wire)| field(wire, pass.clone(), format!("{path}[{index}]"), decode))
        .collect())
}

fn guard(record: &Record, pass: &Pass) -> Result<rocx::Condition> {
    pass.condition(record.get("condition")?, record.path("condition"))
}

fn timestamp(input: &Wire, _: &Pass, path: &str) -> Result<Option<String>> {
    if input.js().is_undefined() {
        return Ok(None);
    }
    let value = b::string(input, path)?;
    host::invoke("timestamp", &Array::of1(&value.as_str().into()))
        .map_err(|_| Failure::invalid(path, "timestamp must be a valid UTC ISO timestamp"))?;
    Ok(Some(value))
}

fn optional_text(input: &Wire, pass: &Pass, path: &str) -> Result<Option<String>> {
    if input.js().is_undefined() {
        Ok(None)
    } else {
        b::text(input, pass, path).map(Some)
    }
}

fn document(input: &Wire, pass: &Pass, clock: String) -> Result<Document> {
    let mut fields = vec!["sections", "timestamp", "commentAuthor"];
    fields.extend(MODULES.iter().flat_map(|module| module.document_fields));
    let record = Record::new(input, "", &fields)?;
    let mut document = Document::new(values(
        &record.get("sections")?,
        pass,
        "sections",
        decode_section,
    )?);
    document.timestamp =
        required(&record, pass, "timestamp", timestamp)?.map(move |date| date.or(Some(clock)));
    document.comment_author = required(&record, pass, "commentAuthor", optional_text)?;
    for module in MODULES {
        (module.document)(&mut document, &record, pass)?;
    }
    Ok(document)
}

fn decode_section(input: &Wire, pass: &Pass, path: &str) -> Result<Section> {
    if let Some(decode) = text_layout::SECTION {
        return decode(input, pass, path);
    }
    let record = Record::new(input, path, &["kind", "children", "condition"])?;
    if kind(input, path)? != "section" {
        return Err(Failure::invalid(path, "expected a section"));
    }
    let mut section = Section::default().when(guard(&record, pass)?);
    if let Some(children) = optional(&record, pass, "children", blocks)? {
        section = section.with_children(children);
    }
    Ok(section)
}

pub(crate) fn blocks(input: &Wire, pass: &Pass, path: &str) -> Result<Vec<Value<Block>>> {
    values(input, pass, path, block)
}

fn block(input: &Wire, pass: &Pass, path: &str) -> Result<Block> {
    let kind = kind(input, path)?;
    if let Some(decode) = MODULES.iter().find_map(|module| find(module.blocks, &kind)) {
        return decode(input, pass, path);
    }
    match kind.as_str() {
        "paragraph" => paragraph(input, pass, path).map(Block::from),
        "resolved" => {
            let record = Record::new(input, path, &["kind", "value"])?;
            let value = record.get("value")?;
            let path = record.path("value");
            let block = b::borrow::<ResolvedBlock>(&value, &path)
                .map(|v| (*v).clone())
                .or_else(|_| {
                    b::borrow::<rocx::ResolvedTable>(&value, &path)
                        .map(|v| ResolvedBlock::Table((*v).clone()))
                })
                .or_else(|_| {
                    b::borrow::<ResolvedParagraph>(&value, &path)
                        .map(|v| ResolvedBlock::Paragraph((*v).clone()))
                })?;
            Ok(Block::Resolved(block))
        }
        _ => Err(Failure::invalid(
            path,
            format!("unsupported block kind `{kind}`"),
        )),
    }
}

pub(crate) fn inlines(input: &Wire, pass: &Pass, path: &str) -> Result<Vec<Value<Inline>>> {
    values(input, pass, path, inline)
}

fn inline(input: &Wire, pass: &Pass, path: &str) -> Result<Inline> {
    let kind = kind(input, path)?;
    if let Some(decode) = MODULES
        .iter()
        .find_map(|module| find(module.inlines, &kind))
    {
        return decode(input, pass, path);
    }
    match kind.as_str() {
        "text" => text_run(input, pass, path).map(Inline::from),
        "image" => image_run(input, pass, path).map(Inline::from),
        "resolved" => {
            let record = Record::new(input, path, &["kind", "value"])?;
            Ok(Inline::Resolved(
                (*b::borrow::<ResolvedInline>(&record.get("value")?, &record.path("value"))?)
                    .clone(),
            ))
        }
        _ => Err(Failure::invalid(
            path,
            format!("unsupported inline kind `{kind}`"),
        )),
    }
}

fn paragraph(input: &Wire, pass: &Pass, path: &str) -> Result<Paragraph> {
    let record = Record::new(
        input,
        path,
        &[
            "kind",
            "text",
            "runs",
            "condition",
            "commentText",
            "commentAuthor",
        ],
    )?;
    let text = optional(&record, pass, "text", b::text)?;
    let runs = optional(&record, pass, "runs", inlines)?;
    if text.is_some() && runs.is_some() {
        return Err(Failure::invalid(
            path,
            "paragraph accepts either text or runs, not both",
        ));
    }
    let mut paragraph = Paragraph::default().when(guard(&record, pass)?);
    if let Some(text) = text {
        paragraph = paragraph.with_text(text);
    }
    if let Some(runs) = runs {
        pass.alias(b::child_path(path, "children"), record.path("runs"));
        paragraph = paragraph.with_children(runs);
    }
    paragraph.comment_text = optional(&record, pass, "commentText", b::text)?;
    paragraph.comment_author = optional(&record, pass, "commentAuthor", b::text)?;
    Ok(paragraph)
}

fn text_run(input: &Wire, pass: &Pass, path: &str) -> Result<TextRun> {
    let record = Record::new(
        input,
        path,
        &["kind", "text", "condition", "commentText", "commentAuthor"],
    )?;
    let mut text =
        TextRun::new(required(&record, pass, "text", b::text)?).when(guard(&record, pass)?);
    text.comment_text = optional(&record, pass, "commentText", b::text)?;
    text.comment_author = optional(&record, pass, "commentAuthor", b::text)?;
    Ok(text)
}

fn image_run(input: &Wire, pass: &Pass, path: &str) -> Result<ImageRun> {
    let record = Record::new(input, path, &["kind", "data", "condition"])?;
    let data = required(&record, pass, "data", b::bytes)?.map(rocx::ImageData::Bytes);
    let mut image = ImageRun::new(Vec::<u8>::new()).when(guard(&record, pass)?);
    image.data = data;
    Ok(image)
}

#[wasm_bindgen]
pub async fn resolve_program(
    input: JsValue,
    clock: String,
    operation: &Operation,
) -> std::result::Result<JsValue, JsValue> {
    let pass = operation.pass();
    let input = Wire::new(input);
    let resolved = pass
        .run(async { document(&input, &pass, clock)?.resolve().await })
        .await?;
    let view = b::record(&[
        ("timestamp", resolved.timestamp().into()),
        (
            "sections",
            projected(resolved.sections().iter().cloned().map(section_handle)),
        ),
    ]);
    if let Some(author) = resolved.comment_author() {
        b::set(&view, "commentAuthor", &author.into());
    }
    for module in MODULES {
        (module.document_view)(&resolved, &view);
    }
    let methods: Vec<_> = MODULES
        .iter()
        .flat_map(|module| module.document_methods)
        .copied()
        .collect();
    Ok(b::handle_with_methods("document", resolved, view, &methods))
}

#[wasm_bindgen]
pub async fn resolve_component(
    input: JsValue,
    scope: Option<String>,
    operation: &Operation,
) -> std::result::Result<JsValue, JsValue> {
    let pass = operation.pass();
    let input = Wire::new(input);
    let result = pass
        .run(async {
            let kind = kind(&input, "")?;
            if let Some(resolve) = MODULES
                .iter()
                .flat_map(|module| module.components)
                .find(|(name, expected, _)| *name == kind && *expected == scope.as_deref())
                .map(|(_, _, resolve)| *resolve)
            {
                return resolve(input, pass.clone()).await;
            }
            if scope.is_some() {
                return Err(Failure::invalid(
                    "scope",
                    "scope is not valid for this component",
                ));
            }
            match kind.as_str() {
                "section" => {
                    resolve(decode_section(&input, &pass, "section")?, section_handle).await
                }
                "paragraph" => {
                    resolve(paragraph(&input, &pass, "paragraph")?, paragraph_handle).await
                }
                "text" => resolve(text_run(&input, &pass, "textRun")?, inline_handle).await,
                "image" => resolve(image_run(&input, &pass, "imageRun")?, inline_handle).await,
                _ => Err(Failure::invalid(
                    "kind",
                    format!("unsupported component kind `{kind}`"),
                )),
            }
        })
        .await?;
    Ok(result.map_or(JsValue::UNDEFINED, Projection::js))
}

pub(crate) async fn resolve<C: Component>(
    component: C,
    view: fn(C::Resolved) -> JsValue,
) -> Result<Option<Projection>>
where
    C::Resolved: 'static,
{
    Ok(component
        .resolve()
        .await?
        .map(|value| Projection::new(value, view)))
}

#[wasm_bindgen]
pub async fn resolve_input(
    input: JsValue,
    operation: &Operation,
) -> std::result::Result<JsValue, JsValue> {
    operation
        .pass()
        .run(host::resolve_value(Wire::new(input)))
        .await
        .map(|value| value.output())
}

fn projected(items: impl Iterator<Item = JsValue>) -> JsValue {
    items.collect::<Array>().into()
}

pub(crate) fn section_handle(section: ResolvedSection) -> JsValue {
    let view = b::record(&[
        (
            "blocks",
            projected(section.blocks().iter().cloned().map(block_handle)),
        ),
        (
            "paragraphs",
            projected(
                section
                    .paragraphs()
                    .into_iter()
                    .cloned()
                    .map(paragraph_handle),
            ),
        ),
    ]);
    for module in MODULES {
        (module.section_view)(&section, &view);
    }
    b::handle("section", section, view)
}

pub(crate) fn paragraph_handle(paragraph: ResolvedParagraph) -> JsValue {
    let view = b::record(&[
        ("text", paragraph.text().into()),
        (
            "runs",
            projected(paragraph.runs().iter().cloned().map(inline_handle)),
        ),
    ]);
    for module in MODULES {
        (module.paragraph_view)(&paragraph, &view);
    }
    b::handle("paragraph", paragraph, view)
}

pub(crate) fn block_handle(block: ResolvedBlock) -> JsValue {
    if let Some(view) = MODULES
        .iter()
        .flat_map(|module| module.block_views)
        .find_map(|view| view(&block))
    {
        return view;
    }
    match block {
        ResolvedBlock::Paragraph(paragraph) => paragraph_handle(paragraph),
        _ => b::handle("block", block, b::record(&[])),
    }
}

pub(crate) fn inline_handle(inline: ResolvedInline) -> JsValue {
    if let Some(view) = MODULES
        .iter()
        .flat_map(|module| module.inline_views)
        .find_map(|view| view(&inline))
    {
        return view;
    }
    let display = rocx::inline_text(std::slice::from_ref(&inline));
    let view = match &inline {
        ResolvedInline::Text { text, .. } => b::record(&[
            ("kind", "text".into()),
            ("text", text.as_str().into()),
            ("displayText", display.into()),
        ]),
        ResolvedInline::Image { image, .. } => {
            let copy = Uint8Array::from(image.data()).into();
            let image = b::handle("image", image.clone(), b::record(&[("bytes", copy)]));
            b::record(&[
                ("kind", "image".into()),
                ("displayText", display.into()),
                ("image", image),
            ])
        }
        _ => b::record(&[("displayText", display.into())]),
    };
    b::handle("inline", inline, view)
}

pub(crate) fn render_options(value: Wire) -> Result<RenderOptions> {
    let mut result = RenderOptions::default();
    if value.js().is_undefined() {
        return Ok(result);
    }
    let options = Record::new(&value, "options", &["timestamp", "profile", "readLimits"])?;
    result.timestamp = timestamp(
        &options.get("timestamp")?,
        &Operation::new().pass(),
        "options.timestamp",
    )?;
    let profile = options.get("profile")?;
    if !profile.js().is_undefined() {
        result.profile = match b::string(&profile, "options.profile")?.as_str() {
            "transitional" => OutputProfile::Transitional,
            "strict" => OutputProfile::Strict,
            _ => {
                return Err(Failure::invalid(
                    "options.profile",
                    "unknown output profile",
                ));
            }
        };
    }
    result.read_limits = read_limits(&options.get("readLimits")?, "options.readLimits")?;
    Ok(result)
}

pub(crate) fn read_limits(input: &Wire, path: &str) -> Result<rocx::opc::ReadLimits> {
    let mut result = rocx::opc::ReadLimits::default();
    if input.js().is_undefined() {
        return Ok(result);
    }
    let limits = Record::new(
        input,
        path,
        &["maxParts", "maxPartBytes", "maxExpandedBytes"],
    )?;
    let read = |key| {
        let value = limits.get(key)?;
        if value.js().is_undefined() {
            Ok(None)
        } else {
            b::u64_number(&value, &limits.path(key)).map(Some)
        }
    };
    if let Some(value) = read("maxParts")? {
        result.max_parts = usize::try_from(value)
            .map_err(|_| Failure::invalid(&limits.path("maxParts"), "outside usize bounds"))?;
    }
    if let Some(value) = read("maxPartBytes")? {
        result.max_part_bytes = value;
    }
    if let Some(value) = read("maxExpandedBytes")? {
        result.max_expanded_bytes = value;
    }
    Ok(result)
}

#[wasm_bindgen]
pub fn render_document(
    document: JsValue,
    options: JsValue,
) -> std::result::Result<Vec<u8>, JsValue> {
    let render = || -> Result<Vec<u8>> {
        let document = b::borrow::<ResolvedDocument>(&Wire::new(document), "document")?;
        document.to_docx_with_options(&render_options(Wire::new(options))?)
    };
    render().map_err(|error| b::js_error(&error, Paths::Package))
}

#[wasm_bindgen]
pub fn dispatch(operation: String, input: JsValue) -> std::result::Result<JsValue, JsValue> {
    let wire = Wire::new(input);
    if let Some((call, paths)) = MODULES
        .iter()
        .find_map(|module| find(module.operations, &operation))
    {
        call(&wire).map_err(|error| b::js_error(&error, paths))
    } else {
        Err(b::js_error(
            &Failure::invalid("", format!("unsupported engine operation `{operation}`")),
            Paths::Declaration,
        ))
    }
}
