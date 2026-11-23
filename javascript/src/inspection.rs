//! Typed views of the original package. Rust owns parsing and anchor validation.
mod view;

use crate::{
    boundary::{self as b, Failure, Operation, Paths, Record},
    host::Wire,
    program::{Hooks, read_limits},
};
use rocx::{
    Result,
    editor::{ImportedDocument, ImportedHeader, ImportedInlineNode, InlineAnchor},
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
        (
            "importedHeaderInlineStructure",
            (header_inline_structure, Paths::Package),
        ),
    ],
    ..Hooks::EMPTY
};

const DOCUMENT_METHODS: &[(&str, &str)] = &[
    ("body", "importedBody"),
    ("sections", "importedSections"),
    ("inlineAt", "importedInlineAt"),
    ("defaultHeaders", "importedDefaultHeaders"),
    // Editor operations borrow the same ImportedSource and original Rust anchors.
    ("preview", "importedPreview"),
    ("previewHeaderImage", "importedPreviewHeaderImage"),
];

#[derive(Default)]
struct Anchors {
    paragraphs: RefCell<HashMap<usize, JsValue>>,
    tables: RefCell<HashMap<usize, JsValue>>,
    headers: RefCell<HashMap<usize, JsValue>>,
    inlines: RefCell<HashMap<*const ImportedInlineNode, JsValue>>,
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

struct HeaderSource {
    header: ImportedHeader,
    anchors: Rc<Anchors>,
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
    let anchor = b::borrow::<InlineAnchor>(&argument(&call, "anchor")?, "anchor")?;
    let node = source.document().inline_at(&anchor)?;
    view::inline_node(node, &source.anchors)
}

fn default_headers(input: &Wire) -> Result<JsValue> {
    let source = b::borrow::<ImportedSource>(&call(input)?.get("target")?, "document")?;
    view::try_array(
        source
            .document()
            .default_headers()?
            .into_iter()
            .map(|header| {
                let view = view::header(&header, &source.anchors)?;
                Ok(b::handle_with_methods(
                    "importedHeader",
                    HeaderSource {
                        header,
                        anchors: source.anchors.clone(),
                    },
                    view,
                    &[("inlineStructure", "importedHeaderInlineStructure")],
                ))
            }),
    )
}

fn header_inline_structure(input: &Wire) -> Result<JsValue> {
    let call = call(input)?;
    let source = b::borrow::<HeaderSource>(&call.get("target")?, "header")?;
    let index = b::u64_number(&argument(&call, "blockIndex")?, "blockIndex")?;
    usize::try_from(index)
        .ok()
        .and_then(|index| source.header.inline_structure(index))
        .map_or(Ok(JsValue::UNDEFINED), |nodes| {
            view::inline_nodes(nodes, &source.anchors)
        })
}
