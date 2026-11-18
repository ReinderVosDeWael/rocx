# rv-docx-rs

Declarative DOCX authoring in Rust and Python, with asynchronous values,
conditions, and dependencies inside the authoring tree. A shared immutable
Rust document domain compiles directly to OOXML and OPC packages. CMI inspired
the original API; it does not constrain this project's interface.

The Rust package is **`rv-docx-rs`**, imported as **`rv_docx_rs`**. The Python
package is also `rv-docx-rs`, imported as **`rv_docx`**. Version 0.2 reorganizes
the implementation and makes deliberate API changes; see the
[migration guide](docs/MIGRATION.md).

## Rust

```rust
use rv_docx_rs::{Document, Error, Paragraph, RunFormat, Section, TextRun};

async fn report() -> Result<(), Error> {
    let run = TextRun::new("Hello, world!").with_format(RunFormat {
        bold: Some(true), size: Some(14.), ..RunFormat::default()
    });
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
from rv_docx.declarative import Document, Paragraph, Section, TextRun

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
use rv_docx_rs::{Document, Error, Paragraph, Property, Section, Value};

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

Runnable examples: [Rust](examples/property_dependencies.rs),
[Python](examples/property_dependencies.py), and the
[feature document](examples/feature_parity.rs).

## Architecture and specifications

- [Module boundaries](docs/MODULES.md): functional core, interpreters, codecs,
  OPC, templates, adapters and effect ownership.
- [Reorganization plan and implementation status](docs/REORGANIZATION.md).
- [OOXML references and feature coverage](docs/OOXML.md): edition-pinned ECMA
  sources, clause mappings, current limitations and documentation policy.
- [Migration guide](docs/MIGRATION.md): Rust/Python changes in 0.2.
- [Python interface](PYTHON_BINDINGS.md) and [current support](COMPATIBILITY.md).
- [Testing](tests/README.md): native tests, independent schemas, consumer
  readback, mutation controls and clean wheel verification.
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
