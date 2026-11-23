# Python bindings

`rocx` exposes `rocx.declarative`. The PyO3 extension is required for native
`to_bytes()` and `save()` output. Installed wheels include it; building from
source requires Rust. The optional `interop` extra provides python-docx's
editable-document and enum conveniences, but is not required for native output.
The extension converts resolved snapshots into the shared Rust document domain
and uses the native OOXML compiler.

## Python-specific behavior

Python uses keyword arguments and familiar dictionaries, tuples and lists.
Feature-specific contracts, paired examples and unsupported contexts are
documented in the [formatting index](docs/FORMATTING.md), the
[feature catalog](https://reindervosdewael.github.io/rocx/catalog/), and the
generated [Python API reference](https://reindervosdewael.github.io/rocx/api/python/).

### Async ownership and deferred values

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

### Output and interoperability

- `await Document.to_bytes()` or `await snapshot.to_bytes()` returns native bytes.
- `await ...save(path_or_binary_stream)` writes native output. File paths use a
  temporary sibling file and replacement after successful compilation.
- `await ...to_docx()` requires `rocx[interop]` and returns a standard editable
  `docx.document.Document`. Missing support raises a clear import error.

Native rendering is a synchronous CPU step inside the async method and releases
the GIL during compilation. Resolution uses asyncio; the Rust adapter does not
run a second executor. File reads remain explicit adapter work.

All output methods accept keyword-only `options=` on declarations and snapshots.
`RenderOptions` controls profile, timestamp and read limits for part count,
per-part bytes and expanded bytes. Strict is unsupported. See
[`examples/resolution_options.py`](examples/resolution_options.py).

### Errors and compatibility

Native conversion/domain errors expose `rule`, `location`, and optional string
`cause` attributes while retaining their normal ValueError, KeyError or OSError
categories. Known declaration locations use structural paths; package part
locations can be absolute OPC names such as `/word/document.xml`. Locations may
be absent where the compiler has no specific context. Source-producer errors
retain their identity and original `__cause__`, with component/field labels such
as `Paragraph.text`; image/output I/O errors identify their adapter boundary.
Legacy compiler checks can still report broad `invalid_content` codes rather than
a distinct rule per historical message. Never classify failures by message prefixes.

Runs accept RGB tuples, `"auto"`, or theme color mappings such as
`{"theme": "accent1", "tint": 128}`. Underline accepts booleans or supported
style names (`single`, `double`, `dotted`, `dash`, `wave`, `words`, `none`).
Numbering uses `NumberingDefinition`, `NumberingLevel` and `NumberingReference`.
The [migration guide](docs/MIGRATION.md) documents changed behavior.

## Verification

See [the verification runbook](tests/README.md) for source setup, schema
validation, and the isolated CPython wheel check.
