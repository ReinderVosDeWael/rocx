# Migrating to 0.2

Version 0.2 implements the domain/functional reorganization. CMI remains a
historical source of fixtures, not a signature-compatibility requirement.

## Package rename to rocx

The package is now `rocx` in both ecosystems. Replace imports and distribution
references as follows:

| Former | Current |
| --- | --- |
| Rust crate `rv-docx-rs`, imported as `rv_docx_rs` | crate `rocx`, imported as `rocx` |
| Python distribution `rv-docx-rs` | distribution `rocx` |
| Python import `rv_docx` | import `rocx` |
| Native extension `rv_docx._native` | `rocx._native` |
| `RV_DOCX_SCHEMA_DIR`, `RV_DOCX_REQUIRE_SCHEMAS` | `ROCX_SCHEMA_DIR`, `ROCX_REQUIRE_SCHEMAS` |

Install the renamed distributions (`rocx[interop]` for the Python interop extra)
and update import paths. The GitHub repository remains
`ReinderVosDeWael/rv-docx-rs`; repository URLs, pinned historical source links and
trusted-publisher repository fields retain that name. The version remains 0.2.0.

## Rust package and authoring changes

R6v appends optional `TableProperties.alignment` / Python `Table.alignment`.
Rust uses `TableAlignment::{Left, Center, Right, Start, End}`; Python accepts
matching lowercase tokens. None retains old output and style/consumer defaults.
This does not set paragraph alignment or floating coordinates. Rust exhaustive
literals need the field or `..Default::default()`. See [TABLES.md](TABLES.md).

R6u appends optional `NumberingLevel.alignment`: Rust `NumberingAlignment` with
Left/Center/Right/Start/End, Python left/center/right/start/end tokens. This aligns
labels, not paragraph text. None preserves previous output and consumer defaults.
Rust exhaustive literals need the field or `..Default::default()`. See
[NUMBERING.md](NUMBERING.md).

R6t appends optional `NumberingLevel.restart`, a nonnegative 32-bit one-based
trigger request. Zero means never restart, not the first level. None preserves
previous output; consumer-ignored values are preserved. Rust exhaustive literals
need the field or `..Default::default()`. See [NUMBERING.md](NUMBERING.md).

R6s appends optional `NumberingLevel.suffix`: Rust `NumberingSuffix::{Tab, Space,
Nothing}`, Python `"tab"`, `"space"`, `"nothing"`. Omission retains previous
output (consumer default: tab). Rust exhaustive literals need the field or
`..Default::default()`. Separators are not inserted into paragraph text. See
[NUMBERING.md](NUMBERING.md).

R6r appends optional `NumberingLevel.is_legal` in both APIs, also used by concrete
level replacements. Omitted values preserve old output; explicit false and true
remain distinct requests. Rust exhaustive literals need the field or
`..Default::default()`. See [NUMBERING.md](NUMBERING.md).

R6q adds `TableLook` and optional `TableProperties.look` / Python `Table.look`.
Omitted fields retain existing library defaults and output bytes. Banding API
flags are positive; wire `noHBand`/`noVBand` attributes are inverted. Python
appends its argument; Rust exhaustive property literals need `look` or
`..Default::default()`. No effective conditional formatting is computed. See
[TABLES.md](TABLES.md).

R6p appends `NumberingDefinition.level_overrides` using existing `NumberingLevel`
values in both APIs. Replacement and start overrides can coexist at the same
defined level without mutating abstract levels. Python preserves existing
positional arguments; Rust exhaustive literals need the field or
`..Default::default()`. See [NUMBERING.md](NUMBERING.md) for the supported subset.

The all-features catalog exposed two property-order omissions. Generated
cell/table borders now follow the schema sequence regardless of input side order;
conditional table-style properties now put `pPr` before `rPr` when both are
present. This changes serialized bytes for previously invalid orderings, not
the requested formatting. Existing declaration inputs remain supported.

R6o appends optional `rtl_gutter` to `SectionProperties` in both APIs. It authors
a right-side gutter request independently of section/text direction; it does not
change gutter size, top-gutter settings, margins or width metrics. Omission
preserves previous/template values. Rust exhaustive literals need the field or
`..Default::default()`. See [FORMATTING.md](FORMATTING.md).

R6n appends `linked_style` to paragraph/character style definitions. Explicit
edges resolve opposite-kind names/IDs, including forward targets; no reciprocal
edge or effective formatting is inferred. Omission preserves template links.
Rust exhaustive literals need the field or `..Default::default()`; Python keeps
existing positional arguments. Direct paragraph references now reject character/
table targets instead of writing malformed `pStyle` references; linked character
styles must still be used as run styles. See [STYLES.md](STYLES.md).

R6m appends optional deferred `Document.defaults` and introduces `DocumentDefaults`.
Rust reuses run/paragraph format values; Python adds `RunDefaults` and
`ParagraphDefaults`. Paragraph-default indents use points, not paragraph-style
inches. Omitted properties retain templates; defaults do not materialize effective
formatting or accept style references/headings. Rust exhaustive `Document`
literals need the field or `..Default::default()`. See [STYLES.md](STYLES.md).

R6l appends optional language/script/font fields to `TableSectionFormat`, matching
the paragraph-style script vocabulary. Rust exhaustive literals need these
fields or `..Default::default()`. Existing Python positional arguments retain
their order. See [STYLES.md](STYLES.md) for conditional run semantics and limits.

R6k appends `line_height`, `tab_stops` and `bidi` to `TableSectionFormat` in both
APIs. Rust exhaustive literals need these fields or `..Default::default()`.
Conditional table paragraph properties use the same units/validation as direct
paragraphs, without computing effective conditional formatting. See [STYLES.md](STYLES.md).

R6j appends optional `bidi` to `SectionProperties` in both APIs. Rust exhaustive
literals need the new field or `..Default::default()`. This controls section page
direction only, independently of paragraph `bidi` and run `rtl`; omitted values
retain template/previous section settings. See [FORMATTING.md](FORMATTING.md).

R6i adds optional `bidi` on paragraph formats/styles (and Python paragraphs),
separate from run `rtl`. It requests layout direction without reversing text or
changing explicit indents/alignment. See [FORMATTING.md](FORMATTING.md).

R6h appends `start_overrides` to numbering definitions and adds
`NumberingStartOverride { level, start }`. Rust exhaustive definitions need an
empty vector or `..Default::default()` when no overrides are needed. Definitions
now reject duplicate/out-of-range abstract levels and duplicate/undefined
override levels. Concrete IDs/collision behavior remains unchanged. See
[NUMBERING.md](NUMBERING.md) for the restart subset and deferred inputs.

R6g adds `CharacterStyleDefinition` using Rust `RunFormat` and Python's flattened
run formatting fields, plus direct run `style` references. Rust custom matches on
`StyleDefinition` must handle its new `Character` variant. Character inheritance
and direct reference target kinds are checked; inherited properties are not
copied into direct formatting. See [STYLES.md](STYLES.md).

R6d adds `CellMargins` and checked `CellTextDirection` to cell properties, with
matching Python `margins`/`text_direction` fields. R6e adds `RunLanguage`, literal
East Asian/complex-script font names, complex-script bold/italic/size and run RTL/
script-selection fields to Rust run formats and paragraph styles and matching
Python declarations. Rust exhaustive literals need the new fields or
`..Default::default()`; Python optional fields are appended. See [TABLES.md](TABLES.md)
and [FORMATTING.md](FORMATTING.md) for units and inheritance limits.

R6f resolves forward style references and rejects authored/reachable based-on
cycles and incompatible reference kinds. Existing paragraph `base_style` remains
creation-only; table duplicates still fail. Self-next styles remain legal. See
[STYLES.md](STYLES.md) for imported graph boundaries and error behavior.

R6c adds deferred `TableRow.properties: Value<TableRowProperties>`, with optional
`RowHeight::{Exact,AtLeast}` point heights and `repeat_header`/`cant_split` flags.
Use `.with_properties(...)` or `..Default::default()` for Rust row literals.
Python appends equivalent optional row fields and exports `RowHeight` helpers.
No property is evaluated as layout; flag absence/false/true is preserved. See
[TABLES.md](TABLES.md) for header grouping and over-page row limitations.

R6b adds `tab_stops: Vec<TabStop>` to paragraph formats and paragraph style
definitions (`Vec::new()` in exhaustive Rust literals). Python appends the
optional `tab_stops` list to paragraphs/styles and exports `TabStop(position,
alignment="left", leader=None)`. Positions are signed points; duplicate rounded
twip positions fail. An empty list leaves inherited stops alone; use explicit
clear entries at known positions. See [FORMATTING.md](FORMATTING.md).

R6a adds optional `line_height: Option<LineHeight>` to `ParagraphFormat` and
`ParagraphStyleDefinition`. Add `line_height: None` to exhaustive Rust literals
or use `..Default::default()`. `Multiple`, `Exact` and `AtLeast` distinguish line
multipliers from point heights. Python adds `LineHeight.multiple/exact/at_least`
and an appended optional `line_height` argument on paragraphs/styles. Legacy
`line_spacing` retains its multiplier semantics; providing both properties is an
error. See [FORMATTING.md](FORMATTING.md) for units, rounding and limitations.

R5 adds `Field`, `FieldInstruction`, `FieldProperties` and `FieldForm`, plus
read-only `ResolvedField` cached content. Constructors/deferred inputs follow
existing inline patterns. Python adds `Field`/`FieldInstruction` and simple/complex
forms. Plain-text projections include cached display text, never instructions;
no result is evaluated. Imported complex-field edit protection now spans
paragraphs in the same story. See [FIELDS.md](FIELDS.md) for supported instructions,
nesting and the intentional paragraph-local authoring limit.

R4 adds `Document.notes`, `Note`/`NoteReference` and `NoteTarget` (`NoteKind` +
`NoteId`). Normal authoring IDs are nonnegative 32-bit values, with zero supported;
footnotes and endnotes have separate namespaces. Note targets and bodies use the
existing deferred machinery. `RangeStart::structured_comment(id, Comment)` adds
base-schema rich content and `CommentMetadata`, without changing the legacy text
comment fields. Python mirrors this with note helpers and `Comment` content.
See [NOTES.md](NOTES.md) for special entries, imported graph checks, exact date
forms and unsupported story contexts. Reference numbers are not plain-text
projection output; the consuming application evaluates their display numbering.

R3 adds `BookmarkName`, `RangeId`, `RangeKind`, `RangeStart` and `RangeEnd`.
Use `HyperlinkTarget::bookmark(name)` (Python: `BookmarkTarget(name)`) for internal
links; external strings keep their existing behavior. Complete documents now
validate marker pairing and unique identities/names. Internal target lookup and
imported name collisions require compilation/template context. See
[RANGES.md](RANGES.md) for overlap, story scope and template edit semantics.
`ooxml::StoryRegistry` replaces implicit part/kind identity: registering two
logical stories in one part does not alias their IDs. Existing comment
conveniences use the shared anchor codec without changing their public syntax.

R2 adds `Hyperlink::new(target, children)`, `HyperlinkTarget::external(...)`,
`HyperlinkProperties`, and `ResolvedInline::Hyperlink(ResolvedHyperlink)`.
Targets, children and properties use the existing `Value`/`PropertyRef` machinery.
Child formatting remains on runs. `ResolvedInline::format()` now returns
`Option<&RunFormat>`: containers have no run formatting. `ResolvedInline::text()`
now returns `Cow<'_, str>`: leaf text is borrowed, link display text is projected.
Use `.as_ref()` where an `&str` is needed, or `inline_text(&children)` for a whole
sequence. `ResolvedParagraph::text()` remains `&str`, including link display text.
The private snapshot projection cannot become stale because children are immutable.

Python adds `d.Hyperlink(target, children, tooltip=None, history=None)`; all fields
accept ready/lazy/async/reference inputs. Target strings convert directly to the
same typed Rust domain value at the native boundary. Relative/bookmark targets,
nested links and block children are rejected. No link is fetched and no default
blue/underline styling is imposed. See [hyperlink examples](../examples/hyperlinks.rs)
and the [Python counterpart](../examples/hyperlinks.py).

The Cargo package is now `rocx`, imported as `rocx`:

```rust
use rocx::{Document, Paragraph, Section};
```

Paragraphs have one `ParagraphContent` value instead of independent optional
`text`/`children` fields. `Paragraph::new`, `from_children`, `with_text` and
`with_children` remain convenient constructors/setters; invalid combinations
cannot be constructed through those fields anymore.

Run/paragraph formatting is a deferred field containing the domain format:

```rust
use rocx::{Paragraph, ParagraphFormat, RunFormat, TextRun};

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

R1 also moves Rust document metadata into one deferred `metadata: Value<Metadata>`
aggregate. For example:

```rust
use rocx::{Document, Metadata, Value};
let mut document = Document::default();
document.metadata = Metadata { title: Some("Report".into()), ..Default::default() }.into();
document.timestamp = Some("2026-11-18T00:00:00Z".into()).into();
document.styles = Vec::new().into();
document.numbering = Value::future(async { Vec::new() });
```

`comment_author`, `timestamp`, `styles` and `numbering` are deferred fields too.
`Section.properties` is `Value<Option<SectionProperties>>`; paragraph numbering
is `Value<Option<NumberingReference>>`. Ready values need `.into()`.
Table/cell options now live in deferred `Table.properties: Value<TableProperties>`
and `TableCell.properties: Value<TableCellProperties>` aggregates, rather than
separate fields. Child content stays separate. There are no optional/async duplicate
fields. Use `Value::map` and resolved block/inline variants to share immutable
child lists; authoring futures remain single-use and are never cloned.

Pure domain checks now reject invalid formatting during Rust resolution as well
as compilation. Python's direct domain conversion uses those same checks at
render time. Callers should not rely on invalid snapshots surviving until ZIP
serialization. Undefined imported styles still require template context and may
fail only during compilation.

## Python snapshots and optional interoperability

`rocx` remains the import. Resolution no longer mutates declarations or returns
the same object:

```python
from rocx.declarative import Document, Paragraph, Section

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

Explicit rendering policy is now available in both frontends:

```python
from rocx import ReadLimits, RenderOptions
options = RenderOptions(timestamp="2026-01-01T00:00:00Z",
                        read_limits=ReadLimits(max_parts=500))
data = await snapshot.to_bytes(options=options)
await snapshot.save("report.docx", options=options)
```

The optional `to_docx()` method also accepts `options=`. Declarations accept the
same keyword. Omitting options preserves previous defaults. A render timestamp
override does not change `snapshot.timestamp`; Strict is explicitly rejected.
Rust provides `to_docx_with_options`, `to_docx_with_template_and_options`,
`save_with_options`, and `save_with_template_and_options`, using
`application::RenderOptions`. Pure `compile`/`compile_with_edits` still take
caller-provided template bytes; path loading remains an application effect.

Python native errors retain ValueError/KeyError/OSError categories and now expose
`rule`, `location` and `cause`. Inspect rule codes instead of message prefixes.
Producer exceptions retain identity and their original `__cause__`; dependency
errors also expose a source-field location. See the inventory for the distinction
between source-field labels and native structural declaration paths.

## Templates and numbering

Rust's file template separates its source path from pure edit requests:

```rust
use rocx::{DocumentTemplate, EditScope};
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
from rocx.declarative import NumberingDefinition, NumberingLevel, NumberingReference

numbering = [NumberingDefinition(42, [NumberingLevel(start=3)])]
paragraph = Paragraph(text="item", numbering=NumberingReference(42))
document = Document([Section(children=[paragraph])], numbering=numbering)
```

See [the standards map](OOXML.md), [module boundaries](MODULES.md), and
[testing guide](../tests/README.md) for implemented scope and validation limits.
## Shape text style requests

`ShapeProperties` adds optional `text_format` with adjacent
`with_text_format(ShapeTextFormat)`; exhaustive Rust literals need the new field
or `..Default::default()`. Python appends `ShapeRun.text_format` and exports
`ShapeTextFormat`. Existing omitted styling produces unchanged bytes. Shape text
uses dedicated DrawingML alignment enums/tokens rather than paragraph formatting;
see [the contract](FORMATTING.md#shape-text-styling-r8l).
