//! A01–A06: declarations and resolved projections for text and page layout.
use crate::{
    boundary::{self as b, Failure, Pass, Paths, Record},
    host::{self, Wire},
    program::{self, Decode, Hooks, Resolve},
    structured, styles,
};
use js_sys::Array;
use rocx::{
    Block, Document, Footer, Header, HeaderFooterType, Inline, Paragraph, ResolvedDocument,
    ResolvedHeaderFooter, ResolvedInline, ResolvedParagraph, ResolvedSection, Result, Section,
    TextRevision, TextRun, Value,
};
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;

mod dto;
pub(crate) use dto::{
    alignment, alignment_view, document_defaults, line_height, line_height_view, paragraph_format,
    paragraph_format_view, rgb, rgb_view, run_format, run_format_view, run_language,
    run_language_view, section_properties, tab_stop, tab_stop_view,
};

pub(crate) const SECTION: Option<Decode<Section>> = Some(section);
pub(crate) const HOOKS: Hooks = Hooks {
    components: &[
        ("paragraph", None, resolve_paragraph as Resolve),
        ("text", None, resolve_text as Resolve),
        ("header", None, resolve_header as Resolve),
        ("footer", None, resolve_footer as Resolve),
    ],
    blocks: &[("paragraph", |input, pass, path| {
        paragraph(input, pass, path).map(Block::from)
    })],
    inlines: &[("text", |input, pass, path| {
        text_run(input, pass, path).map(Inline::from)
    })],
    document_fields: &["metadata", "defaults"],
    document,
    document_view,
    section_view,
    paragraph_view,
    operations: &[
        (
            "validateValue:runFormat",
            (validate_run_format, Paths::Declaration),
        ),
        (
            "validateValue:paragraphFormat",
            (validate_paragraph_format, Paths::Declaration),
        ),
        (
            "validateValue:sectionProperties",
            (validate_section_properties, Paths::Declaration),
        ),
        (
            "validateValue:documentDefaults",
            (validate_document_defaults, Paths::Declaration),
        ),
    ],
    ..Hooks::EMPTY
};

fn document(document: &mut Document, record: &Record, pass: &Pass) -> Result<()> {
    if let Some(value) = program::optional(record, pass, "metadata", |value, _, path| {
        dto::metadata(value, path)
    })? {
        document.metadata = value;
    }
    if let Some(value) = program::optional(record, pass, "defaults", |value, _, path| {
        maybe(value, path, document_defaults)
    })? {
        document.defaults = value;
    }
    Ok(())
}

fn maybe<T>(value: &Wire, path: &str, read: fn(&Wire, &str) -> Result<T>) -> Result<Option<T>> {
    (!value.js().is_undefined())
        .then(|| read(value, path))
        .transpose()
}

fn guard(record: &Record, pass: &Pass) -> Result<rocx::Condition> {
    pass.condition(record.get("condition")?, record.path("condition"))
}

fn declaration(input: &Wire, path: &str, kind: &str, keys: &[&str]) -> Result<Record> {
    let record = Record::new(input, path, keys)?;
    let kind_path = record.path("kind");
    if b::string(&record.get("kind")?, &kind_path)? != kind {
        return Err(Failure::invalid(&kind_path, format!("expected `{kind}`")));
    }
    Ok(record)
}

const STORY_TYPES: [(&str, HeaderFooterType); 3] = [
    ("default", HeaderFooterType::Default),
    ("first", HeaderFooterType::First),
    ("even", HeaderFooterType::Even),
];

fn stories<T: Send + 'static>(
    record: &Record,
    pass: &Pass,
    key: &str,
    read: Decode<T>,
) -> Result<BTreeMap<HeaderFooterType, Value<T>>> {
    let input = record.get(key)?;
    if input.js().is_undefined() {
        return Ok(BTreeMap::new());
    }
    let record = Record::new(&input, &record.path(key), &["default", "first", "even"])?;
    let mut stories = BTreeMap::new();
    for (name, kind) in STORY_TYPES {
        let value = record.get(name)?;
        if !value.js().is_undefined() {
            stories.insert(
                kind,
                host::field(value, pass.clone(), record.path(name), read),
            );
        }
    }
    Ok(stories)
}

fn section(input: &Wire, pass: &Pass, path: &str) -> Result<Section> {
    let record = declaration(
        input,
        path,
        "section",
        &[
            "kind",
            "properties",
            "children",
            "headers",
            "footers",
            "condition",
        ],
    )?;
    let mut value = Section::default().when(guard(&record, pass)?);
    if let Some(properties) = program::optional(&record, pass, "properties", |value, _, path| {
        maybe(value, path, section_properties)
    })? {
        value.properties = properties;
    }
    if let Some(children) = program::optional(&record, pass, "children", program::blocks)? {
        value.children = children;
    }
    value.headers = stories(&record, pass, "headers", header)?;
    value.footers = stories(&record, pass, "footers", footer)?;
    Ok(value)
}

macro_rules! story {
    ($decode:ident, $ty:ident, $kind:literal, $resolve:ident) => {
        fn $decode(input: &Wire, pass: &Pass, path: &str) -> Result<$ty> {
            let record = declaration(input, path, $kind, &["kind", "children", "condition"])?;
            Ok($ty {
                children: program::required(&record, pass, "children", program::blocks)?,
                condition: guard(&record, pass)?,
            })
        }
        fn $resolve(
            input: Wire,
            pass: Pass,
        ) -> futures_util::future::BoxFuture<'static, Result<Option<program::Projection>>> {
            Box::pin(
                async move { program::resolve($decode(&input, &pass, $kind)?, story_handle).await },
            )
        }
    };
}
story!(header, Header, "header", resolve_header);
story!(footer, Footer, "footer", resolve_footer);

pub(crate) fn paragraph(input: &Wire, pass: &Pass, path: &str) -> Result<Paragraph> {
    let record = declaration(
        input,
        path,
        "paragraph",
        &[
            "kind",
            "text",
            "runs",
            "format",
            "numbering",
            "condition",
            "commentText",
            "commentAuthor",
        ],
    )?;
    let text = program::optional(&record, pass, "text", b::text)?;
    let runs = program::optional(&record, pass, "runs", program::inlines)?;
    if text.is_some() && runs.is_some() {
        return Err(Failure::invalid(
            path,
            "paragraph accepts either text or runs, not both",
        ));
    }
    let mut value = Paragraph::default().when(guard(&record, pass)?);
    if let Some(text) = text {
        value = value.with_text(text);
    }
    if let Some(runs) = runs {
        pass.alias(b::child_path(path, "children"), record.path("runs"));
        value = value.with_children(runs);
    }
    if let Some(format) = program::optional(&record, pass, "format", |value, _, path| {
        paragraph_format(value, path)
    })? {
        value.format = format;
    }
    if let Some(numbering) = program::optional(
        &record,
        pass,
        "numbering",
        styles::optional_numbering_reference,
    )? {
        value.numbering = numbering;
    }
    value.comment_text = program::optional(&record, pass, "commentText", b::text)?;
    value.comment_author = program::optional(&record, pass, "commentAuthor", b::text)?;
    Ok(value)
}

pub(crate) fn text_run(input: &Wire, pass: &Pass, path: &str) -> Result<TextRun> {
    let record = declaration(
        input,
        path,
        "text",
        &[
            "kind",
            "text",
            "format",
            "revision",
            "condition",
            "commentText",
            "commentAuthor",
        ],
    )?;
    let mut value = TextRun::new(program::required(&record, pass, "text", b::text)?)
        .when(guard(&record, pass)?);
    if let Some(format) = program::optional(&record, pass, "format", |value, _, path| {
        run_format(value, path)
    })? {
        value.format = format;
    }
    if let Some(revision) = program::optional(&record, pass, "revision", optional_revision)? {
        value.revision = revision;
    }
    value.comment_text = program::optional(&record, pass, "commentText", b::text)?;
    value.comment_author = program::optional(&record, pass, "commentAuthor", b::text)?;
    Ok(value)
}

fn optional_revision(input: &Wire, pass: &Pass, path: &str) -> Result<Option<TextRevision>> {
    (!input.js().is_undefined())
        .then(|| structured::text_revision(input, pass, path))
        .transpose()
}

fn resolve_paragraph(
    input: Wire,
    pass: Pass,
) -> futures_util::future::BoxFuture<'static, Result<Option<program::Projection>>> {
    Box::pin(async move {
        program::resolve(
            paragraph(&input, &pass, "paragraph")?,
            program::paragraph_handle,
        )
        .await
    })
}

fn resolve_text(
    input: Wire,
    pass: Pass,
) -> futures_util::future::BoxFuture<'static, Result<Option<program::Projection>>> {
    Box::pin(async move {
        program::resolve(text_run(&input, &pass, "textRun")?, program::inline_handle).await
    })
}

fn document_view(document: &ResolvedDocument, view: &JsValue) {
    b::set(view, "metadata", &dto::metadata_view(document.metadata()));
    if let Some(defaults) = document.defaults() {
        b::set(view, "defaults", &dto::document_defaults_view(defaults));
    }
}

fn story_handle(story: ResolvedHeaderFooter) -> JsValue {
    let blocks = story
        .blocks()
        .iter()
        .cloned()
        .map(program::block_handle)
        .collect::<Array>();
    b::handle(
        "headerFooter",
        story,
        b::record(&[("blocks", blocks.into())]),
    )
}

fn story_views(stories: &BTreeMap<HeaderFooterType, ResolvedHeaderFooter>) -> JsValue {
    let view = b::record(&[]);
    for (key, kind) in STORY_TYPES {
        if let Some(story) = stories.get(&kind) {
            b::set(&view, key, &story_handle(story.clone()));
        }
    }
    view
}

fn section_view(section: &ResolvedSection, view: &JsValue) {
    if let Some(properties) = section.properties() {
        b::set(
            view,
            "properties",
            &dto::section_properties_view(properties),
        );
    }
    b::set(view, "headers", &story_views(section.headers()));
    b::set(view, "footers", &story_views(section.footers()));
}

fn paragraph_view(paragraph: &ResolvedParagraph, view: &JsValue) {
    b::set(view, "format", &paragraph_format_view(paragraph.format()));
    if let Some(comment) = paragraph.comment() {
        b::set(view, "comment", &structured::plain_comment_view(comment));
    }
}

/// Common attributes for the leaf variants; container formatting belongs to children.
pub(crate) fn inline_attributes(inline: &ResolvedInline, view: &JsValue) {
    if let Some(format) = inline.format() {
        b::set(view, "format", &run_format_view(format));
    }
    if let Some(comment) = inline.comment() {
        b::set(view, "comment", &structured::plain_comment_view(comment));
    }
    if let Some(revision) = inline.revision() {
        b::set(view, "revision", &structured::text_revision_view(revision));
    }
}

macro_rules! validate {
    ($($validate:ident => $read:ident),+) => {
        $(fn $validate(input: &Wire) -> Result<JsValue> {
            let record = Record::new(input, "", &["kind", "value"])?;
            $read(&record.get("value")?, "value")?.validate()?;
            Ok(JsValue::UNDEFINED)
        })+
    };
}
validate!(
    validate_run_format => run_format,
    validate_paragraph_format => paragraph_format,
    validate_section_properties => section_properties,
    validate_document_defaults => document_defaults
);
