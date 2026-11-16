# cmi-docx (Rust)

A first milestone toward a Rust implementation of CMI Docx's declarative API.
It creates plain-text Word documents directly as Office Open XML, using the
`Document → Section → Paragraph` tree.

```rust
use cmi_docx::{Document, Paragraph, Section};

async fn example() -> cmi_docx::Result<()> {
let declaration = Document::new(vec![
    Section::new(vec![
        Paragraph::new("Hello, world!").into(),
        Paragraph::new("A second paragraph.").into(),
    ]).into(),
]);

let resolved = declaration.resolve().await?;
resolved.save("hello.docx")?;
let bytes = resolved.to_docx()?; // Render again without running producers again.
Ok(())
}
```

Resolution needs an executor; the library does not depend on a runtime. Saving
uses synchronous filesystem I/O. The runnable examples use the development-only
`futures` executor:

```sh
cargo run --example plain -- /tmp/plain.docx
cargo run --example async_text -- /tmp/async_text.docx
cargo run --example conditional -- /tmp/conditional.docx
```

## Deferred and conditional content

Public structs support struct literals as well as constructors and fluent setters:

```rust
use cmi_docx::{Condition, Document, Paragraph, Section, Value};

let paragraph = Paragraph {
    text: Value::future(async { "Fetched asynchronously".to_owned() }),
    condition: Condition::lazy(|| true),
};

let document = Document::new(vec![
    Section::default().with_children(Value::lazy(|| vec![
        paragraph.into(),
        Paragraph::new(Value::lazy(|| "Only computed if visible".into()))
            .when(false).into(),
    ])).into(),
]);
```

`Value<T>` supports immediate values, lazy producers, and futures. Its `lazy` and
`future` constructors accept infallible producers; `try_lazy` and `try_future`
preserve application errors. The enum variants can return another `Value<T>`
for multi-stage resolution. Entire sections, a section's child list, individual
blocks, and paragraph text can be deferred. Use `Value::future(async {
Block::from(paragraph) })` for an asynchronous paragraph block.

Conditions are `true` by default. `.when(false)` or
`.when(Condition::lazy(|| predicate()))` skips the component and its descendants
before their deferred fields run. A deferred component must first be produced
to inspect its condition; put the condition on its ancestor to skip producing
the component itself.

Resolution consumes the tree: producers and predicates are `FnOnce`, and futures
are single-use. Each condition is evaluated once. Sibling branches resolve
concurrently and their results retain declaration order. The first observed
failure drops remaining futures. Deferred captures must be `Send + 'static`;
capture owned values or an `Arc` rather than stack references. Resolved documents
are immutable, `Clone`, `Send`, and `Sync`.

Errors identify the failing field, for example
`sections[0].children[2].text`, and retain producer errors as sources. Invalid XML
1.0 characters are rejected during resolution, with their UTF-8 byte offset.
No output file is opened before resolution and packaging succeed. The final
filesystem write can still fail after creating or truncating a file.

## Current compatibility

Reference: [`childmindresearch/cmi-docx` revision
`3449b942034129ad74a28aeb253361035806e385`](https://github.com/childmindresearch/cmi-docx/tree/3449b942034129ad74a28aeb253361035806e385),
package version 0.6.16. This is a tested subset, not full parity yet.

Supported: empty documents and paragraphs, ordered paragraphs and sections,
Unicode, preserved spaces, tabs, line breaks, lazy and async content, and
conditional sections and paragraphs. Each CR and LF produces a line break, as
in `python-docx`, so CRLF becomes two line breaks. Sections use default next-page
boundaries, with no boundary after the final included section.

Formatting, `Paragraph.children`/inline components, metadata, templates,
images, tables, headers/footers, and configurable section properties are deferred.
The milestone emits no font, style, or page-layout settings; appearance uses the
reader's defaults. The pinned Python package's default template supplies styles
and page settings, so visual parity is not claimed in this milestone.

Two deliberate resolution differences follow the agreed Rust contract: predicates
run once, while the Python implementation can re-evaluate them; and excluded
sections are removed before placing section breaks. The pinned implementation can
leave a trailing section break when its final declared section is excluded. That
artifact is not reproduced. Strict feature parity remains the overall target;
these differences are recorded for the later compatibility review.

## Packaging and verification

The renderer writes three OPC parts: `[Content_Types].xml`, `_rels/.rels`, and
`word/document.xml`. ZIP entries are stored without compression, keeping the
dependency graph small. Fixed timestamps make repeated renders deterministic.
The two direct runtime dependencies are `zip` (all optional features disabled)
and `futures-util` (standard-library support only). XML parsing and a test/example
executor are development dependencies; serialization and image libraries are
not needed.

```sh
cargo test --all-targets
cargo test --doc
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Tests parse the DOCX package and compare paragraph text, section counts, and break
positions with checked-in XML generated by the pinned CMI source. Normal Rust
tests run without Python or network access after dependencies are downloaded.
To regenerate the reference fixtures, use Python 3.12+, `python-docx==1.2.0`,
`lxml`, and an existing checkout at the exact reference revision:

```sh
python tests/fixtures/generate_reference.py /path/to/cmi-docx
```

Licensed under LGPL-2.1-or-later; see [LICENSE](LICENSE). This crate is an
independent Rust implementation, inspired by the Child Mind Institute's package.
