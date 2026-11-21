# rocx

Declarative DOCX authoring in Rust and Python, with asynchronous values,
conditions, and dependencies inside the authoring tree. A shared immutable
Rust document domain compiles directly to OOXML and OPC packages. CMI inspired
the original API; it does not constrain this project's interface.

The Rust package is **`rocx`**, imported as **`rocx`**. The Python
package is also `rocx`, imported as **`rocx`**. Version 0.2 reorganizes
the implementation and makes deliberate API changes; see the
[migration guide](docs/MIGRATION.md).

## Visual inspection catalog

Matching [Rust](examples/all_features.rs) and [Python](examples/all_features.py)
examples build a labeled catalog of the implemented functionality. Generate both
and verify exact DOCX byte equality with:

```sh
UV_PROJECT_ENVIRONMENT=.venv-opencode TMPDIR=/tmp/opencode uv run --no-sync python tools/check_examples.py
```

Open `examples/all_features-rust.docx` or `examples/all_features-python.docx`.
See the [coverage/inspection guide](examples/ALL_FEATURES.md) for sample locations,
viewer caveats and maintenance requirements. New features must update both
catalogs and their parity/coverage checks.

## Rust

```rust
use rocx::{Document, Error, Paragraph, RunFormat, Section, TextRun};

async fn report() -> Result<(), Error> {
    let run = TextRun::new("Hello, world!")
        .with_format(RunFormat::default().with_bold(true).with_size(14.));
    let declaration = Document::new(vec![
        Section::new(vec![Paragraph::from_children(vec![run.into()]).into()]).into(),
    ]);
    let snapshot = declaration.resolve().await?;
    snapshot.save("report.docx")
}
```

Use an executor of your choice to run `report`. The library does not require
Tokio or a Python runtime. Resolution consumes Rust declarations; snapshots can
be rendered repeatedly without running producers again.

## Python

Manage Python and the native extension through uv:

```sh
uv sync --locked --group test
uv run --locked examples/property_dependencies.py report.docx
```

```python
import asyncio
from rocx.declarative import Document, Paragraph, Section, TextRun

async def main():
    declaration = Document([Section(children=[Paragraph(children=[
        TextRun(text="Hello, world!", bold=True),
    ])])])
    snapshot = await declaration.resolve()
    await snapshot.save("report.docx")
    data = await snapshot.to_bytes()

asyncio.run(main())
```

Python resolution returns a deeply immutable snapshot and leaves declarations
unchanged. Use factories for repeatable async work; raw coroutine objects remain
single-use. `Document.save()` and `Document.to_bytes()` resolve implicitly.
`await snapshot.to_docx()` is an optional editable python-docx convenience:

```sh
uv sync --locked --extra interop
```

The production wheel's native bytes/save API has no Python package dependency.
`python-docx` is an optional `interop` extra and a development readback tool.

## Dependencies between properties

Python references resolve the selected field rather than the whole component:

```python
source = Paragraph(text=lambda: asyncio.sleep(0, result="42"))

async def summarize():
    return f"The result is {await source.deferred('text')}"

summary = Paragraph(text=summarize)
document = Document([Section(children=[summary, source])])
```

This works for formatting, comments, image data, children, metadata, and other
resolvable fields. Consumers share field work within a pass. Cycles and excluded
sources produce explicit errors; cancelled consumers do not cancel another
consumer's shared producer. `Deferred(factory)` is a separate, explicitly
memoized computation handle.

Rust offers typed named properties with guarded producers and references:

```rust
use rocx::{Document, Error, Paragraph, Property, Section, Value};

async fn dependent_report() -> Result<Vec<u8>, Error> {
    let source = Property::new("result", Value::future(async { "42".to_owned() }));
    let reference = source.reference();
    let summary = Paragraph::new(Value::try_future(async move {
        Ok::<_, Error>(format!("The result is {}", reference.await?))
    }));
    Document::new(vec![Section::new(vec![
        summary.into(), Paragraph::new(source).into(),
    ]).into()]).resolve().await?.to_docx()
}
```

`Property<T>` supports `Clone + Send + Sync + 'static` values. Its own `when`
guard controls requests to that property. Excluding a paragraph does not imply
excluding an independently declared property. Typed references detect dependency
cycles within a resolution pass; arbitrary opaque user futures are not inspected.
`Value::deferred()` remains available for explicitly shared computation results.

See the [deferred-field inventory and scenario matrix](docs/DEFERRED-FIELDS.md)
for supported fields, ownership differences and remaining cross-language gaps.
Image transformation and alt-text options use the same Rust deferred-field
machinery as image data, without rerunning producers when rendering snapshots.
Rust metadata, section and table/cell properties are deferred aggregates too.
Reuse immutable resolved child lists through `Value::map`, not cloned futures.
The [Rust](examples/resolution_options.rs) and
[Python](examples/resolution_options.py) examples show shared children and explicit
timestamp policy. Python exports `RenderOptions`/`ReadLimits`; native errors
retain normal exception categories with `rule`, `location` and `cause` attributes.

Rich external hyperlinks are supported in both languages, including deferred
targets, formatted display runs, images, tabs and breaks. Links belong to their
actual body/header/footer part and are never fetched. See
[`examples/hyperlinks.rs`](examples/hyperlinks.rs) and
[`examples/hyperlinks.py`](examples/hyperlinks.py). Internal links use an explicit
bookmark target; nested links are intentionally rejected. Paired bookmark/comment
markers support overlap and cross-paragraph ranges without nested containers;
see the [range contract](docs/RANGES.md) and [example](examples/ranges.py).

Footnotes/endnotes have deferred definitions and references, with images, tables
and links owned by their actual note part. Structured base comments support the
same block vocabulary and explicit metadata. See the [note/comment contract](docs/NOTES.md)
and runnable [`notes.rs`](examples/notes.rs)/[`notes.py`](examples/notes.py) examples.

Simple and complex fields support cached rich results and typed PAGE/NUMPAGES/
REF/TOC instructions, without evaluating fields or calculating layout. See the
[field contract](docs/FIELDS.md) and [`fields.rs`](examples/fields.rs)/
[`fields.py`](examples/fields.py) examples.

Paragraphs and paragraph styles support typed multiple, exact and minimum line
heights and custom tab stops with leaders and explicit clears. See
[formatting coverage](docs/FORMATTING.md) for units and examples.

Table rows support exact/minimum heights and optional header-repetition/splitting
requests without calculating pagination. See [row coverage](docs/TABLES.md).

Cell-local margins/text flow and run language/script-font controls use the same
checked native codecs. Styles support forward references and inheritance-cycle
checks; see [formatting](docs/FORMATTING.md), [tables](docs/TABLES.md) and
[style references](docs/STYLES.md) for supported behavior and limitations.

Character styles, concrete numbering start overrides and level replacements are
available in both APIs, without computing effective styles or list labels. See [styles](docs/STYLES.md)
and [numbering](docs/NUMBERING.md).

Section page direction is independent of paragraph/run direction. Conditional
table-style regions also support typed paragraph line heights, tabs and bidi;
these serialize requests without computing effective formatting or page layout.
Document run/paragraph defaults can be authored separately and merged into
template defaults without copying effective formatting into document content.
Paragraph/character styles also support explicit linked-style references, and
sections support a gutter-placement request independent of bidi direction.
Tables also expose `TableLook` activation flags for conditional style regions,
without materializing effective formatting into cells.
Numbering levels support optional legal-numbering display requests (`is_legal`),
including concrete replacements; consumers calculate the displayed labels.
Optional numbering suffixes request tab, space or no separator after labels,
without inserting separator text into paragraphs.
Level-restart requests preserve omitted, never-restart and one-based triggers
without computing counters.
Numbering-label alignment supports left/center/right/start/end independently of
paragraph text alignment.
Inline tables also accept left/center/right/start/end alignment requests without
changing grids or cell text alignment.

Runnable examples: [Rust](examples/property_dependencies.rs),
[Python](examples/property_dependencies.py), and the
[feature document](examples/feature_parity.rs).

## Architecture and specifications

- [Remaining plan and agent handoff](docs/PLAN.md): project decisions, ordered
  milestones, acceptance criteria and verification instructions.
- [Module boundaries](docs/MODULES.md): functional core, interpreters, codecs,
  OPC, templates, adapters and effect ownership.
- [Reorganization plan and implementation status](docs/REORGANIZATION.md).
- [OOXML references and feature coverage](docs/OOXML.md): edition-pinned ECMA
  sources, clause mappings, current limitations and documentation policy.
- [Migration guide](docs/MIGRATION.md): Rust/Python changes in 0.2.
- [Python interface](PYTHON_BINDINGS.md) and [current support](COMPATIBILITY.md).
- [Testing](tests/README.md): native tests, independent schemas, consumer
  readback, mutation controls and clean wheel verification.
- [Releasing](docs/RELEASING.md): tag-gated crates.io/PyPI publishing,
  trusted-publisher setup and version checks.
- [Historical architecture review](docs/ARCHITECTURE.md).

The initial output profile is Transitional. Strict is explicitly rejected until
implemented. The engine supports the documented surface; it does not yet author
every DOCX feature or implement Word's layout, pagination or field evaluation.
Templates preserve untouched part payloads and unknown namespace bindings;
structured text edits have explicit limits. The standards map distinguishes
implemented generation from safe editing and opaque retention.

## Build and verification

```sh
cargo test --all-features --all-targets
cargo test --all-features --doc
cargo clippy --all-features --all-targets -- -D warnings
uv run --locked --group test pytest
uv build --wheel
```

`python-tests` enables Rust-generated-file readback through python-docx.
`python-bindings` enables PyO3; default Cargo builds have no Python dependency.
Building bindings requires Python development libraries; maturin enables the
extension-module linkage mode for wheels. See [testing](tests/README.md) for
independent validation using official, digest-pinned schemas.

The code is LGPL-2.1-or-later. Bundled default Word assets retain their separate
[python-docx MIT notice](assets/PYTHON-DOCX-LICENSE); their provenance and minimal
schema correction are recorded in [assets/README.md](assets/README.md).
