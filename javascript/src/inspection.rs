//! Typed views of the original package. Rust owns parsing and anchor validation.
mod view;

use crate::{
    boundary::{self as b, Failure, Operation, Paths, Record},
    host::Wire,
    program::{Hooks, read_limits},
};
use rocx::{
    Result,
    editor::{ImportedDocument, ImportedHeader, ImportedInlineNode, ImportedStory, InlineAnchor},
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::JsValue;

pub(crate) const HOOKS: Hooks = Hooks {
    operations: &[
        ("openDocx", (open, Paths::Package)),
        ("importedBody", (body, Paths::Package)),
        ("importedSections", (sections, Paths::Package)),
        ("importedInlineAt", (inline_at, Paths::Package)),
        ("importedDefaultHeaders", (default_headers, Paths::Package)),
        ("importedStories", (stories, Paths::Package)),
        ("importedEntryStories", (entry_stories, Paths::Package)),
        ("importedStyles", (styles, Paths::Package)),
        (
            "importedStoryInlineStructure",
            (story_inline_structure, Paths::Package),
        ),
    ],
    ..Hooks::EMPTY
};

const DOCUMENT_METHODS: &[(&str, &str)] = &[
    ("body", "importedBody"),
    ("sections", "importedSections"),
    ("inlineAt", "importedInlineAt"),
    ("defaultHeaders", "importedDefaultHeaders"),
    ("stories", "importedStories"),
    ("entryStories", "importedEntryStories"),
    ("styles", "importedStyles"),
    ("styleAnchor", "importedStyleAnchor"),
    // Editor operations borrow the same ImportedSource and original Rust anchors.
    ("editStoryText", "importedEditStoryText"),
    ("preview", "importedPreview"),
    ("previewHeaderImage", "importedPreviewHeaderImage"),
];

#[derive(Default)]
struct Anchors {
    paragraphs: RefCell<HashMap<usize, JsValue>>,
    tables: RefCell<HashMap<usize, JsValue>>,
    headers: RefCell<HashMap<(usize, rocx::editor::SectionStoryKind, usize), JsValue>>,
    inlines: RefCell<HashMap<InlineKey, JsValue>>,
}

#[derive(Eq, Hash, PartialEq)]
enum InlineKey {
    Body(*const ImportedInlineNode),
    Story(StoryPosition),
}

// Rust's default-header and full story inspections have separate allocations
// for the same physical part. The part and source indices identify both copies.
#[derive(Clone, Eq, Hash, PartialEq)]
struct StoryPosition {
    part: Rc<str>,
    block: usize,
    path: Vec<usize>,
}

struct InlineSource {
    anchor: InlineAnchor,
    story: Option<StoryPosition>,
}

pub(crate) fn inline_anchor(value: &Wire, path: &str) -> Result<InlineAnchor> {
    Ok(b::borrow::<InlineSource>(value, path)?.anchor.clone())
}

fn entry_stories(input: &Wire) -> Result<JsValue> {
    let source = b::borrow::<ImportedSource>(&call(input)?.get("target")?, "document")?;
    view::try_array(
        source
            .document()
            .entry_stories()?
            .iter()
            .map(|entry| view::entry(entry, &source.anchors)),
    )
}

fn styles(input: &Wire) -> Result<JsValue> {
    let source = b::borrow::<ImportedSource>(&call(input)?.get("target")?, "document")?;
    Ok(source
        .document()
        .styles()?
        .map_or(JsValue::UNDEFINED, |styles| {
            let items = styles.styles.iter().map(|style| {
                let value = b::record(&[("kind", style.kind.as_str().into())]);
                for (key, item) in [
                    ("id", style.id.as_deref()),
                    ("name", style.name.as_deref()),
                    ("basedOn", style.based_on.as_deref()),
                    (
                        "kindXml",
                        match &style.kind {
                            rocx::editor::StyleKind::Unsupported(raw) => Some(raw.as_str()),
                            rocx::editor::StyleKind::Missing => None,
                            kind => Some(kind.as_str()),
                        },
                    ),
                ] {
                    if let Some(item) = item {
                        b::set(&value, key, &item.into());
                    }
                }
                value
            });
            b::record(&[
                ("partName", styles.part_name.as_str().into()),
                ("styles", view::array(items)),
            ])
        }))
}

pub(crate) struct ImportedSource {
    document: ImportedDocument,
    anchors: Rc<Anchors>,
}

impl ImportedSource {
    /// Borrow the unmodified Rust snapshot for preview and package operations.
    pub(crate) fn document(&self) -> &ImportedDocument {
        &self.document
    }
}

struct StorySource {
    slot: StorySlot,
    part: Rc<str>,
    anchors: Rc<Anchors>,
}

enum StorySlot {
    DefaultHeader(ImportedHeader),
    Story(ImportedStory),
}

impl StorySource {
    fn handle(slot: StorySlot, anchors: &Rc<Anchors>) -> Result<JsValue> {
        let (kind, part, view) = match &slot {
            StorySlot::DefaultHeader(header) => (
                "importedHeader",
                header.part_name.as_str(),
                view::header(header, anchors)?,
            ),
            StorySlot::Story(story) => (
                "importedStory",
                story.part_name.as_str(),
                view::story(story, anchors)?,
            ),
        };
        let part = Rc::from(part);
        Ok(b::handle_with_methods(
            kind,
            Self {
                slot,
                part,
                anchors: anchors.clone(),
            },
            view,
            &[("inlineStructure", "importedStoryInlineStructure")],
        ))
    }
}

fn open(input: &Wire) -> Result<JsValue> {
    let input = Record::new(input, "", &["bytes", "options"])?;
    let options = input.get("options")?;
    let limits = if options.js().is_undefined() {
        Default::default()
    } else {
        let options = Record::new(&options, "options", &["readLimits"])?;
        read_limits(&options.get("readLimits")?, "options.readLimits")?
    };
    let bytes = b::bytes(&input.get("bytes")?, &Operation::new().pass(), "bytes")?;
    let document = ImportedDocument::open_with_limits(&bytes, limits)?;
    Ok(b::handle_with_methods(
        "importedDocument",
        ImportedSource {
            document,
            anchors: Rc::default(),
        },
        JsValue::UNDEFINED,
        DOCUMENT_METHODS,
    ))
}

fn call(input: &Wire) -> Result<Record> {
    Record::new(input, "", &["target", "args"])
}

fn argument(call: &Record, path: &str) -> Result<Wire> {
    b::list(&call.get("args")?, "args")?
        .into_iter()
        .next()
        .ok_or_else(|| Failure::invalid(path, "missing argument"))
}

fn body(input: &Wire) -> Result<JsValue> {
    let source = b::borrow::<ImportedSource>(&call(input)?.get("target")?, "document")?;
    view::try_array(
        source
            .document()
            .body()
            .iter()
            .enumerate()
            .map(|(i, block)| view::body_block(block, i, &source.anchors)),
    )
}

fn sections(input: &Wire) -> Result<JsValue> {
    let source = b::borrow::<ImportedSource>(&call(input)?.get("target")?, "document")?;
    Ok(view::array(
        source.document().sections()?.iter().map(view::section),
    ))
}

fn inline_at(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = b::borrow::<ImportedSource>(&call.get("target")?, "document")?;
    let target = b::borrow::<InlineSource>(&argument(&call, "anchor")?, "anchor")?;
    let node = source.document().inline_at(&target.anchor)?;
    view::inline_node(node, &source.anchors, target.story.as_ref())
}

fn default_headers(input: &Wire) -> Result<JsValue> {
    let source = b::borrow::<ImportedSource>(&call(input)?.get("target")?, "document")?;
    view::try_array(
        source
            .document()
            .default_headers()?
            .into_iter()
            .map(|header| StorySource::handle(StorySlot::DefaultHeader(header), &source.anchors)),
    )
}

fn stories(input: &Wire) -> Result<JsValue> {
    let source = b::borrow::<ImportedSource>(&call(input)?.get("target")?, "document")?;
    view::try_array(
        source
            .document()
            .stories()?
            .into_iter()
            .map(|story| StorySource::handle(StorySlot::Story(story), &source.anchors)),
    )
}

fn story_inline_structure(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = b::borrow::<StorySource>(&call.get("target")?, "story")?;
    let index = b::u64_number(&argument(&call, "blockIndex")?, "blockIndex")?;
    let Ok(block) = usize::try_from(index) else {
        return Ok(JsValue::UNDEFINED);
    };
    let nodes = match &source.slot {
        StorySlot::DefaultHeader(header) => header.inline_structure(block),
        StorySlot::Story(story) => story.inline_structure(block),
    };
    nodes.map_or(Ok(JsValue::UNDEFINED), |nodes| {
        view::inline_nodes(
            nodes,
            &source.anchors,
            Some(&StoryPosition {
                part: source.part.clone(),
                block,
                path: Vec::new(),
            }),
        )
    })
}
