# Migrating to 0.2

Version 0.2 implements the domain/functional reorganization. CMI remains a
historical source of fixtures, not a signature-compatibility requirement.

## Rust package and authoring changes

The Cargo package is now `rv-docx-rs`, imported as `rv_docx_rs`:

```rust
use rv_docx_rs::{Document, Paragraph, Section};
```

Paragraphs have one `ParagraphContent` value instead of independent optional
`text`/`children` fields. `Paragraph::new`, `from_children`, `with_text` and
`with_children` remain convenient constructors/setters; invalid combinations
cannot be constructed through those fields anymore.

Run/paragraph formatting is a deferred field containing the domain format:

```rust
use rv_docx_rs::{Paragraph, ParagraphFormat, RunFormat, TextRun};

let run = TextRun::new("bold").with_format(RunFormat {
    bold: Some(true), ..RunFormat::default()
});
let paragraph = Paragraph::new("heading").with_format(ParagraphFormat {
    heading: Some(1), ..ParagraphFormat::default()
});
```

The format can also be a `Value::future`, lazy value, `Property` or `PropertyRef`.
Use `Color::Rgb(rgb)` (or `rgb.into()`) and `Underline::Single`/`bool.into()` in
`RunFormat`; theme/automatic colors and additional underline styles are now
expressible. Explicit false caps/small-caps/strike overrides are serialized.
Other point-valued numeric conveniences retain their documented units.

Resolved runs are `ResolvedInline::{Text,Tab,Break,Image}` variants. Common
`text()`, `format()`, `comment()` and `image()` accessors remain available.
Snapshots live in `document`; rendering/save methods are implemented by the
application layer, not by effectful domain code.

`Value::deferred()` still shares an explicitly memoized computation. For typed
named dependency nodes, use `Property::new(name, value)` or `value.property(name)`
and pass `property.reference()` to consumers. The property owns its own condition
through `.when(...)`; a separate paragraph condition does not implicitly become
that property's guard. Typed reference cycles are rejected. Opaque arbitrary
futures cannot be inspected for dependencies they do not express as references.

Rust declarations/property sources are single-use owned programs; references may
observe their cached result after resolution. Use new source factories/programs
for fresh passes. Rendering a resolved snapshot never reruns producers.

R1's first slice makes Rust `ImageRun.transformation` and `ImageRun.alt_text`
`Value<Option<...>>` fields. Assign ready options with `Some(value).into()` or
`None.into()`; lazy/async values and typed property references use those same
fields. Image data, transformation and alt text resolve concurrently after the
image condition passes. Producer errors identify the failing field. Image header
inspection and domain output are unchanged. See the
[field inventory and scenario matrix](DEFERRED-FIELDS.md) for remaining gaps and
intentional Rust/Python ownership differences.

## Python snapshots and optional interoperability

`rv_docx` remains the import. Resolution no longer mutates declarations or returns
the same object:

```python
from rv_docx.declarative import Document, Paragraph, Section

source = Paragraph(text=lambda: fetch_text())
declaration = Document([Section(children=[source])])
snapshot = await declaration.resolve()
await snapshot.save("first.docx")
await snapshot.save("second.docx")  # no second fetch
```

Use factories for work intended to run in each new resolution pass. Raw coroutine
objects are single-use. A property reference shares field work inside its current
pass; `Deferred(factory)` deliberately memoizes a computation for the handle's
lifetime. `source.deferred("field")` / `PropertyRef(source, "field")` work for any
resolvable field and do not wait for the entire source component. Snapshot
containers are immutable tuples/read-only mappings, with no `_active` flags.
Image paths are read during resolution and retained as immutable bytes, so a
snapshot does not depend on the source file remaining available.
Cancelling one consumer does not cancel a shared producer another consumer needs;
ending a pass cancels and drains its owned work.

Native `to_bytes()` and `save()` have no Python package runtime dependency.
`to_docx()` requires the optional `interop` extra:

```sh
uv sync --locked --extra interop
```

The return value remains a normal editable python-docx document. Heading input
errors are now `ValueError`; missing templates are `FileNotFoundError`; malformed
template packages produce a native validation error. These no longer mimic CMI
exceptions. Excluded final sections no longer leave an extra empty section.

`Document(timestamp=...)` accepts an explicit snapshot timestamp. Otherwise the
I/O adapter selects UTC once during resolution. Use the resolved snapshot when
repeatable output is needed.

## Templates and numbering

Rust's file template separates its source path from pure edit requests:

```rust
use rv_docx_rs::{DocumentTemplate, EditScope};
let mut template = DocumentTemplate::new("source.docx");
template.edits.replacements.push(("{{NAME}}".into(), "Ada".into()));
template.edits.scope = EditScope::BodyHeadersFooters;
```

Python retains the convenient path/replacements/index constructor and adds
`scope="body"`, `"body_headers_footers"` (default), or `"supported_stories"`.
Supported story scope also follows related comments/footnotes/endnotes for text
edits; this does not add typed generation for all those stories.

Replacement preserves untouched text/control nodes and permits a match within a
hyperlink, but rejects matches across structured boundaries or inside fields,
content controls and revisions. Comment/bookmark markers stay in place; text
inserted across their positions belongs to the first matched run, preserving the
existing replacement convention. This is an explicit edit policy, not a universal
Word range-editing model. Unknown extension subtrees stay opaque.

Python numbering now uses actual native declarations rather than an ignored
arbitrary dictionary:

```python
from rv_docx.declarative import NumberingDefinition, NumberingLevel, NumberingReference

numbering = [NumberingDefinition(42, [NumberingLevel(start=3)])]
paragraph = Paragraph(text="item", numbering=NumberingReference(42))
document = Document([Section(children=[paragraph])], numbering=numbering)
```

See [the standards map](OOXML.md), [module boundaries](MODULES.md), and
[testing guide](../tests/README.md) for implemented scope and validation limits.
