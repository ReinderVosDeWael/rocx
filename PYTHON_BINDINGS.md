# Python bindings

`rv-docx-rs` exposes `rv_docx.declarative`. The optional PyO3 extension converts
resolved Python snapshots directly into the shared Rust document domain and
uses the native OOXML compiler. CMI is only a pinned historical test dependency.
Native bytes/save has no Python package runtime dependency. The optional
`interop` extra provides python-docx's editable document and enum conveniences.

```sh
uv sync --locked --group test
uv run --locked --group test pytest
uv build --wheel
```

## Authoring and resolution

The interface includes documents/templates, sections/properties/headers/footers,
paragraphs, text/images/tabs/breaks, nested tables/rows/cells/borders, styles, and
numbering definitions/references. Python uses familiar dictionaries, tuples and
keyword arguments. Constructors are guided by domain ergonomics rather than
exact CMI signature equality.

Fields may hold ready values, lazy zero-argument factories or awaitables.
Resolution recursively handles components, declaration dataclasses, dictionaries,
tuples/lists/generators, and components returned by producers. Independent fields
run concurrently. Conditions are synchronous and run before a component's fields;
async conditions are rejected.

`await declaration` / `await declaration.resolve()` return a new immutable
snapshot. Input declarations are not mutated. Source node identities remain
stable; snapshots contain tuples, read-only mappings and resolved scalar values.
Use factories for repeated passes; bare coroutines are single-use. Resolve a
Document once and render that snapshot repeatedly to reuse results and timestamp.

`component.deferred("field_name")` or `PropertyRef(owner, "field_name")` creates
a dependency on a particular declaration field. It waits for the field alone,
shares work within a pass, respects its source's own condition, and detects cycles
across sibling or same-component properties. References can request sources
outside the rendered tree. An excluded source raises `DependencyError` with
`rule="excluded_dependency"`; cycles use `rule="dependency_cycle"`.
`DependencyError` is a `ValueError` subclass. Unknown fields/conditions cannot be
referenced and raise `AttributeError` at construction.

`Deferred(factory_or_awaitable)` is a separate computation handle that memoizes
its result/error for its own lifetime. Cancelling one waiter does not poison
other waiters; its owner can call `cancel()`. Ordinary pass-owned tasks are
cancelled and drained when a pass ends or fails, preserving the original error.

## Output and interoperability

- `await Document.to_bytes()` or `await snapshot.to_bytes()` returns native bytes.
- `await ...save(path_or_binary_stream)` writes native output. File paths use a
  temporary sibling file and replacement after successful compilation.
- `await ...to_docx()` requires `rv-docx-rs[interop]` and returns a standard editable
  `docx.document.Document`. Missing interoperability support raises a clear import
  error; it is not required for native rendering.

Native rendering is a synchronous CPU step inside the async method, and releases
the GIL during compilation. Resolution uses asyncio; the Rust adapter does not
run a second executor. File reads remain explicit adapter work.

Runs accept RGB tuples, `"auto"`, or theme color mappings such as
`{"theme": "accent1", "tint": 128}`. Underline accepts booleans or supported
style names (`single`, `double`, `dotted`, `dash`, `wave`, `words`, `none`).
Numbering uses `NumberingDefinition`, `NumberingLevel` and `NumberingReference`.
The [migration guide](docs/MIGRATION.md) documents changed behavior and examples.

## Verification and provenance

The suite checks immutable resolution, property dependencies, cancellation,
conditions, error translation, the actual native backend, file/stream output,
formatting, images, comments, tables, sections, templates and numbering. Output is
checked by python-docx against expected content, with separate Rust-file readback
and deliberate semantic mutation controls. Independent official XSD checks and
invalid-order/relationship mutations are documented in [tests/README.md](tests/README.md).

Historical CMI test files retain original hashes in
`tests/python_bindings/upstream/ORIGIN.json`. Signature/dataclass-equality tests
were retired as requirements. One original test expecting CMI's trailing empty
section is explicitly skipped; native exclusion behavior has its own passing
test. The other applicable upstream content checks and selected differential
comparisons remain useful regression evidence, not a claim of full OOXML coverage.

maturin builds an ABI-stable extension for CPython 3.12+. Production wheel metadata
contains only optional interoperability dependencies; no CMI renderer or Rust
toolchain is needed to run the installed wheel. Adapted original field declarations
retain their LGPL provenance; bundled Word defaults retain their separate MIT
notice. Generated binaries, virtual environments, caches and wheels stay out of Git.
