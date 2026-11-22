# Python bindings

`rocx` exposes `rocx.declarative`. The optional PyO3 extension converts
resolved Python snapshots directly into the shared Rust document domain and
uses the native OOXML compiler. Native bytes/save has no Python package runtime
dependency; the optional `interop` extra provides python-docx's editable document
and enum conveniences.

```sh
uv sync --locked --group test
uv run --locked --group test pytest
uv build --wheel
```

## Authoring and resolution

The interface includes documents/templates, sections/properties/headers/footers,
paragraphs, text/images/tabs/breaks, nested tables/rows/cells/borders, styles, and
numbering definitions/references. Python uses familiar dictionaries, tuples and
keyword arguments. Constructors are guided by domain ergonomics and the
documented Python API.

`Hyperlink(target, children, tooltip=None, history=None)` separates an external
absolute URI/IRI from its display runs. All four fields support the same deferred
inputs as other declarations. TextRun/ImageRun/Tab/Break children are legal;
nested links and block children are unsupported. Formatting is explicit on the
child runs, not inherited from an implicit hyperlink style. Targets are preserved
verbatim, not fetched or normalized. Use `BookmarkTarget(name)` for an internal
link; missing/ambiguous names fail instead of creating dummy relationships. See
[`examples/hyperlinks.py`](examples/hyperlinks.py).

`RangeStart.bookmark(name)` / `RangeEnd.bookmark(name)` pair a bookmark.
`RangeStart.comment(id, text, author=None)` / `RangeEnd(id)` pair an explicit body
comment. Starts/ends can overlap and span paragraphs in one story. All fields can
be deferred when using the direct `RangeStart(id, kind, name=..., text=...,
author=...)` constructor. Complete-story validation occurs in the native domain
at render time, with structured locations. See [RANGES.md](docs/RANGES.md) for
identity, imported collision, template replacement and legal-context policies.

`Document(notes=[Note.footnote(0, children), Note.endnote(0, children)])` defines
ordinary notes; use `NoteReference.footnote(0)` or `.endnote(0)` in body content.
All fields support generic deferred inputs. Equal footnote/endnote IDs do not
collide, and normal ID zero is supported. The compiler owns special separator
entries; missing/duplicate/reserved-role targets fail explicitly. Each note is a
logical story whose media/links belong to the related physical part.

`RangeStart.structured_comment(id, Comment(children, author=None, date=None,
initials=None))` pairs with `RangeEnd(id)`. Blocks, images and links are supported;
date uses the checked UTC-seconds form and defaults to snapshot time. Threaded
comments, nested note references and non-body reference/anchor contexts remain
unsupported. See [NOTES.md](docs/NOTES.md) for exact policies and limitations.

`Field(FieldInstruction.page(), children, form="simple", dirty=None, locked=None)`
authors cached field results; `form="complex"` emits balanced markers. Helpers
also cover `num_pages()`, `reference(name)`, `table_of_contents()` and explicit
unparsed `raw(code)`. All properties accept generic deferred inputs. REF names
are checked against generated/imported bookmarks. No instruction is evaluated,
and no pages or TOC are calculated. See [FIELDS.md](docs/FIELDS.md) for nesting,
cached content, consumer-update flags and template-edit protection.

`Paragraph(..., line_height=d.LineHeight.exact(14.25))` sets point-based exact
line spacing; `at_least(points)` sets a minimum and `multiple(value)` sets a
multiplier. Paragraph styles accept the same property. Deferred inputs work
normally; old `line_spacing` remains a multiplier and cannot be combined with
`line_height`. See [formatting coverage](docs/FORMATTING.md).

`tab_stops=[d.TabStop(108, "right", "dot")]` defines a custom tab at 108 points
on a paragraph or paragraph style. Positions may be negative; they round to twips
and must be unique after rounding. Lists accept generic deferred inputs. Empty
lists leave inherited stops unchanged; `d.TabStop(72, "clear")` removes a known
inherited stop. Definitions do not insert tab characters or compute layout.

`TableRow(cells, height=d.RowHeight.at_least(18), repeat_header=True,
cant_split=True)` requests a minimum height and row pagination behavior. Use
`RowHeight.exact(points)` for fixed height. Each field supports generic deferred
inputs. Omitted flags differ from explicit false. Only leading contiguous header
groups repeat, and over-page rows may still split. See [row coverage](docs/TABLES.md).

Cells accept `margins=d.CellMargins(top=3, start=0, bottom=6, end=9)` in points
and checked `text_direction="tbRl"` (or the other documented directions).
Runs and paragraph styles accept `language=d.RunLanguage(bidi="ar-SA")`,
`font_east_asia`, `font_complex_script`, `bold_complex_script`,
`italic_complex_script`, `size_complex_script`, `rtl` and `complex_script`.
All use generic deferred resolution. These are serialized hints/overrides, not
font availability checks, language validation, shaping or layout computation.
Styles can reference later definitions; invalid target kinds and reachable
inheritance cycles fail. See [style references](docs/STYLES.md).

`CharacterStyleDefinition(name="Accent", italic=True)` adds a character style;
`TextRun("styled", style="Accent", bold=False)` references it while retaining
separate direct overrides. Character definitions share run formatting fields
(including `size`, not `font_size`) and support generic deferred inputs. Parent
and direct-reference kinds are checked; no effective style values are computed.

`NumberingDefinition(..., start_overrides=[NumberingStartOverride(0, 7)])` sets a
concrete list's level-zero start without rewriting its abstract starting value.
Levels/overrides must be unique and references must target defined levels.
Generic deferred nested fields work normally; list labels are not evaluated.
See [numbering coverage](docs/NUMBERING.md).

`Paragraph(..., bidi=True)` requests RTL paragraph layout; paragraph styles accept
the same flag. It is separate from `TextRun.rtl`, preserves stored text/explicit
alignment and indents, and supports generic deferred inputs.

`SectionProperties(bidi=True)` independently requests RTL page direction (such
as column order), not RTL paragraph/run text. Omission retains template/previous
section settings; false explicitly overrides true. No page layout is calculated.

Table-style regions accept `TableSectionFormat(line_height=LineHeight.exact(14),
tab_stops=[TabStop(36)], bidi=True)`. These use the shared paragraph codec and
generic nested deferred fields, without materializing effective formatting on
cell paragraphs or enabling table look flags. See [styles](docs/STYLES.md).

The same regions accept `language`, script-specific fonts, complex-script
bold/italic/size, `rtl` and `complex_script`. These remain style-level run requests;
no effective formatting, text shaping or font fallback is computed.

`Document(defaults=DocumentDefaults(run=RunDefaults(size=11),
paragraph=ParagraphDefaults(spacing_after=6)))` merges explicit document defaults
into the template styles part. Nested deferred fields/references work normally.
Omitted values retain templates; paragraph-default lengths use points. Defaults
are not copied into direct paragraph/run formatting. See [styles](docs/STYLES.md).

Paragraph/character style definitions accept `linked_style="Other Half"`.
Author both directions to request a reciprocal pair; the compiler writes only
explicit edges, without copying formatting or substituting direct style kinds.
Omission preserves existing links; names/IDs and deferred forward references work.

`SectionProperties(rtl_gutter=True)` requests right-side gutter placement,
separately from bidi settings. Omission preserves previous/template values; false
explicitly disables the request. Gutter size, top-gutter settings and margins
are not changed, and no binding/page layout is computed.

`NumberingDefinition(..., level_overrides=[NumberingLevel(0, 4, "upperRoman", "(%1)")])`
replaces the supported properties of a defined level for that concrete list.
It can coexist with `start_overrides` at the same level. Nested deferred values
work normally; abstract levels stay unchanged and counters are not calculated.

`NumberingLevel(..., is_legal=True)` requests decimal display of all referenced
levels in this level's label, including within concrete replacements. None omits
the request; false is explicit. Generic deferred fields work without changing
the stored format or calculating labels. See [numbering](docs/NUMBERING.md).

`NumberingLevel(..., suffix="space")` requests a label separator; `"tab"` and
`"nothing"` are also supported on abstract/replacement levels. None omits the
request (consumer default: tab). Nested deferred fields work; the library does
not insert separator text. See [numbering](docs/NUMBERING.md).

`Table(rows, alignment="center")` requests inline-table alignment. Supported
tokens are left/center/right/start/end; None keeps style/consumer defaults. It
does not set cell paragraph alignment, resize grids or add floating positioning.
Generic deferred fields/references work. See [tables](docs/TABLES.md).

`NumberingLevel(level=2, restart=1)` requests restarting after the first level;
`restart=0` means never restart, and None omits the request. The one-based value
is distinct from the zero-based level index and from concrete start overrides.
No counters are calculated. See [numbering](docs/NUMBERING.md).

`NumberingLevel(..., alignment="end")` aligns the generated label, independently
of paragraph text alignment. Supported tokens are left/center/right/start/end;
None omits the request and keeps consumer defaults. Nested deferred values and
concrete replacements work normally. See [numbering](docs/NUMBERING.md).

`Table(..., look=TableLook(last_row=True, last_column=True, vertical_banding=True))`
enables additional conditional regions without copying style properties to
cells. All six optional look fields support generic deferred values; omitted
ones retain existing library defaults. Banding API flags are positive, unlike
wire no-banding attributes. See [tables](docs/TABLES.md).

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
- `await ...to_docx()` requires `rocx[interop]` and returns a standard editable
  `docx.document.Document`. Missing interoperability support raises a clear import
  error; it is not required for native rendering.

Native rendering is a synchronous CPU step inside the async method, and releases
the GIL during compilation. Resolution uses asyncio; the Rust adapter does not
run a second executor. File reads remain explicit adapter work.

All three output methods accept keyword-only `options=` on both declarations and
snapshots. `rocx.RenderOptions(profile="transitional", timestamp=None,
read_limits=rocx.ReadLimits())` preserves defaults; an explicit timestamp
overrides output metadata without changing the snapshot. ReadLimits controls
part-count, per-part-byte and total-expanded-byte budgets while reading either
the default template or a supplied template. Strict is explicitly unsupported.
See [`examples/resolution_options.py`](examples/resolution_options.py).

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
The [migration guide](docs/MIGRATION.md) documents changed behavior and examples.

## Verification

The suite checks immutable resolution, property dependencies, cancellation,
conditions, error translation, the actual native backend, file/stream output,
formatting, images, comments, tables, sections, templates and numbering. Output is
checked by python-docx against expected content, with separate Rust-file readback
and deliberate semantic mutation controls. Independent official XSD checks and
invalid-order/relationship mutations are documented in [tests/README.md](tests/README.md).

maturin builds an ABI-stable extension for CPython 3.12+. Production wheel metadata
contains only optional interoperability dependencies; no renderer package or Rust
toolchain is needed to run the installed wheel. The project and bundled Word
defaults retain their respective LGPL and MIT licensing notices. Generated
binaries, virtual environments, caches and wheels stay out of Git.
