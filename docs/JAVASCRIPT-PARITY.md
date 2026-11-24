# JavaScript parity contract

Direct body controls expose source tag, alias, and lock as
`BodyBlock::ContentControl` (Rust), `BodyContentControl` (Python), and
`kind: "contentControl"` (JavaScript). Their contents remain opaque and have
no edit anchors. The catalog asserts all three metadata values; the installed
inspection fixture checks their JavaScript projection separately.

Current comparison baseline: `c1c730b` (including `1c17f03` table-cell inline
readback and `5f103c5` header/footer stories), plus the integrated Rust
changes in this worktree, including B1's `ImageData::NamedBytes`.
The original B0–B2 qualification used `8c3cc2e` plus its own changes; it
does not qualify this integration. This inventory defines what must be
reviewed, not what has passed.
Read the exports in [`src/lib.rs`](../src/lib.rs)
and their definitions; `declarative` and the root re-exports are aliases, not
additional features. Include public fields, enum cases, trait implementations
and macro-generated setters/IDs (a `pub fn` search alone misses some of these).
Rust integration tests below are semantic references, not JavaScript evidence.

The flat `generateDocx({ blocks }, { timestamp })` API remains supported:
a default section, paragraph alignment, text, byte images with optional
dimensions, tabs, breaks, external links and rectangular tables whose cells
can contain nested blocks. This is distinct from the declarative
`DocumentProgram` overload. B1 added authoring, resolution, composition,
and imported inspection. B2 adds anchored edit previews in the worktree,
not in a published release.
The current paragraph-insertion slice adds `paragraphInsertions` to
`ImportedDocument.preview`, with reports from `paragraphInsertions()`. Its
typed installed-consumer fixture is in `javascript/tests/parity/editor.mts`.
The same packed artifact passed focused Node 22, 24 and 26 and Chromium
development/production cases. Full packed Node 22 and 24 gates passed with
schema/OPC validation and checkout provenance checks.
Use the installed [`index.d.ts`](../javascript/index.d.ts) barrel and
[`types/`](../javascript/types/) modules for actual signatures. The tables
below retain Rust requirements and test case names; those are acceptance
targets, not blanket claims of equivalent behavior.
[`examples/support/mod.rs`](../examples/support/mod.rs) is a smaller feature
fixture (including a skipped table row), and [`examples/showcase/`](../examples/showcase/)
provides the six-artifact oracle, not the definition of completeness.

## Implementation and focused evidence

Inline insertion/deletion inspection is read-only: Rust
`ImportedInlineNode::Revision`, Python `ImportedRevision`, and JavaScript
`ImportedRevision` expose literal revision metadata, nested runs, and
snapshot-scoped anchors. Flat projections retain opaque revision boundaries.
Move revisions, nested revision wrappers, and revision editing are not covered.

The linked `.mts` and `.mjs` fixtures exercise the installed declarations
and adapter, respectively. Evidence is scoped to the assertions in the
fixtures and the runtimes run for each owner; optional native reference
arguments must actually be supplied to count as exact Rust byte evidence.

| Rows | Installed API and focused evidence |
| --- | --- |
| Legacy / B0 | The [packed consumer](../javascript/tests/check-packed.sh) checks the legacy report and invalid-input, Unicode, cross-realm, diagnostics, and initialization-retry cases. [B0 tests](../javascript/tests/parity/b0.mjs) exercise the expanded boundary, byte ownership, one-pass cycle errors, guards and cancellation; [direct Chromium test](../javascript/tests/parity/b0-browser.mjs). |
| A01–A06 | [`text-layout.d.ts`](../javascript/types/text-layout.d.ts): `DocumentProgram`, sections, stories, text and formatting, metadata and defaults; [typed fixture](../javascript/tests/parity/text-layout.mts), [runtime cases](../javascript/tests/parity/text-layout.mjs), including inherited margins, explicit flags and resolved views. |
| A07–A08 | [`styles.d.ts`](../javascript/types/styles.d.ts): style definitions and numbering; [typed fixture](../javascript/tests/parity/styles.mts), [runtime cases](../javascript/tests/parity/styles.mjs) check conditional regions, override values and validation. |
| A09–A10 | [`tables.d.ts`](../javascript/types/tables.d.ts): declarative tables, rows, cells and resolved views; [typed fixture](../javascript/tests/parity/tables.mts), [runtime cases](../javascript/tests/parity/tables.mjs) exercise lazy children, merged/nested sizing, table controls and invalid geometry. |
| A11–A12 | [`drawings.d.ts`](../javascript/types/drawings.d.ts): images, drawings, fractions and breaks; [typed fixture](../javascript/tests/parity/drawings.mts), [runtime cases](../javascript/tests/parity/drawings.mjs) check resolved metadata and bytes separately from rendering. The focused Node path check compares the name, data, dimensions and content type as well as exact DOCX bytes against a native `ImageData::Path` run. |
| A13–A15 | [`structured.d.ts`](../javascript/types/structured.d.ts): links, fields, notes, markers, controls and revisions; [typed fixture](../javascript/tests/parity/structured.mts), [runtime cases](../javascript/tests/parity/structured.mjs) exercise typed views, cached content and relationships. |
| R01–R06 | [`resolution.d.ts`](../javascript/types/resolution.d.ts) and [`text-layout.d.ts`](../javascript/types/text-layout.d.ts): resolution, views, constructors and validation; [typed fixture](../javascript/tests/parity/resolution.mts), [runtime cases](../javascript/tests/parity/resolution.mjs) check shared work, one-pass cycles, typed reuse, and diagnostics. Owner modules extend `ValidateValueInput`. |
| T01–T06 | [`template.d.ts`](../javascript/types/template.d.ts) and [`file-io.d.ts`](../javascript/types/file-io.d.ts): rendering, package plans, source edits, Node path sources/save; [typed fixture](../javascript/tests/parity/template.mts), [runtime cases](../javascript/tests/parity/template.mjs) check repeat output, guards, budgets, untouched opaque payloads and failed saves. One focused timestamp/limit case also runs the direct browser entry in Chromium; the detailed template suite is Node-only. |
| E01–E03 | [`inspection.d.ts`](../javascript/types/inspection.d.ts): `openDocx` and synchronous inspection; [typed fixture](../javascript/tests/parity/inspection.mts), [runtime cases](../javascript/tests/parity/inspection.mjs) check physical order, opaque markers, lazy inspection and snapshot/alias identity, including the current-Rust additions for structured table-cell paragraphs and all six header/footer variants. |
| E04–E06 | [`editor.d.ts`](../javascript/types/editor.d.ts) exposes `preview(batch)` and `previewHeaderImage(edit)`; the [adapter](../javascript/src/editor.rs#L100-L139) delegates to the Rust editor. The [typed cases](../javascript/tests/parity/editor.mts) check reports, invalid targets and conflicts, and failure atomicity separately. The integrated gate compares seven focused editor outputs to Rust and runs [independent readback and payload checks](../javascript/tests/parity/editor.py). |

### Qualification and independent review

The [JavaScript catalog](../examples/javascript/catalog.mjs) builds the final,
template, edited, header-source, header-shared, and header-local DOCX files.
The [installed gate](../javascript/tests/check-packed.sh) and
[comparator](../tools/check_javascript.py) test exact ZIP bytes against the Rust
catalog and retain the Rust/Python comparison. The earlier B0–B2 branch
recorded one tarball (SHA-256
`8d5a9f2bbbcbfbb3d82d0d1b27b352766a14d5f603546640f384038dd7517dfc`)
passed on Node 22.23.5, Node 24.22.3 and Chromium 159.0.8154.6 in Vite
development and production. All six catalogs, twelve focused authoring and
template outputs, and seven editor outputs matched native Rust byte for byte.
The gate separately checked schema/OPC, typed readback, previews, invalid
inputs and untouched parts. This is historical evidence. The integrated
source, installed tarball, and current inventory must be qualified separately.

#### Reproducing the current authoring declaration audit

The [machine-readable mappings](../javascript/tests/parity/authoring-inventory.json)
and [checker](../javascript/tests/parity/check-authoring-inventory.mjs) compare
the public Rust declarations with the TypeScript compiler's view of the
installed package. They use compiled Rustdoc rather than source-line regexes,
and include inherited TypeScript fields, literal/tagged variants and their
named payload fields, resolved-inline constructor arms, public Rust
`validate()` coverage, projected fields, and nested `Value` sites in records
(including `Value<Option<T>>`). Explicit exceptions distinguish primitive
IDs, host APIs, and lower-level APIs from document declarations.

From the root, with the package already installed in `consumer/node_modules`
and TypeScript installed in the same consumer:

```bash
CARGO_TARGET_DIR="$TMPDIR/rocx-audit" \
  cargo +nightly rustdoc --locked --lib -- -Z unstable-options --output-format json
node javascript/tests/parity/check-authoring-inventory.mjs \
  "$TMPDIR/rocx-audit/doc/rocx.json" \
  consumer/node_modules/rocx-docx/index.d.ts \
  consumer/node_modules/typescript/lib/typescript.js
```

For a local audit only, `RUSTC_BOOTSTRAP=1 cargo rustdoc` with the same
Rustdoc arguments also worked on the stable toolchain. The check against the
local inspection consumer's installed authoring declarations found 75
records, 536 field entries, 74 deferred-value sites, 55 enums, 243 variant
entries, 15 view/DTO projections, 16 validators and 25 explicitly mapped
exceptions. This is a structural check, not proof of Wasm decoding, correct
units/defaults, every method or builder, or positive and negative coverage.
Editor, resolution operations, and the flat legacy API have separate
behavior contracts.

#### Current audit findings

| ID | Finding and reproduction | Acceptance needed |
| --- | --- | --- |
| E01–E02 | The incoming B2 adapter exposed only `inlineItems` for direct table-cell paragraphs. Current Rust's `ImportedCellParagraph.inline_structure`, `ImportedTableCell::inline_structure`, and `ImportedDocument::inline_at` recognize cell run/link anchors. | The JavaScript repair exposes `ImportedCellParagraph.inlineStructure` and uses the Rust anchors for `.inlineAt`. Focused verification reported below. |
| E03 | The incoming B2 adapter exposed only `defaultHeaders()`, omitting current Rust's `ImportedDocument::stories()`. | The JavaScript repair adds `.stories()` for all six slots with Rust ordering, inheritance, physical identity, read-only anchors and lazy inspection. Focused verification reported below. |
| A01 / T01 | The adapter rejected fractional seconds and numeric offsets that Rust preserved. Native Rust does not parse or normalize snapshot or override timestamp strings; it writes them as XML text. | Nonlegacy JavaScript now checks only type and UTF-16 before passing the string to Rust. Native byte fixtures cover fractional `Z`, `+00:00`, nonzero offsets and XML-escaped non-date text. Clients must supply valid dates for schema-valid output. Flat legacy date validation is unchanged. |
| T01 / T02 / E01 | Before repair, Wasm rejected all `bigint` byte budgets even though Rust's `max_part_bytes` and `max_expanded_bytes` are `u64`. | The shared converter and installed declarations now accept non-negative safe `number`s or `bigint`s through `u64::MAX` for both fields. Focused cases check overflow, negatives, fractions and unsafe numbers, and enforce exact and one-byte-short BigInt limits in template compilation. `maxParts` remains a number bounded by Wasm's `usize`. |
| A01–A15 / R04–R06 | Expanded `.mts` fixtures are not exhaustive by construction. Add `readonly auditOnly?: boolean` to `RunFormat` in a scratch installed declaration tree and run the expanded suite's strict NodeNext type command: it still passes. The Rust-source checker reports `RunFormat: unmapped TypeScript field auditOnly`; Wasm rejects `{kind:"text", text:"x", format:{auditOnly:true}}` at `textRun.format`. The legacy `Variants` fixture guards flat declarations only. | Run the structural comparison and retain linked positive and negative Wasm cases. A type-only pass or a green six-file catalog does not establish every input field and variant's behavior. |
| A11–A15 / R01–R06 | The packed browser fixture executes a portable subset, not every detailed Node case. For example, `runResolutionCases()` is called from the Node-only `resolution.mjs` entry, while the direct browser B0 test checks one-pass cycles, guards and concurrent waiters separately. Chromium compares drawing/structured output bytes but does not run the complete Node negative suites for those owners. | Attribute the browser results to the cases actually invoked. Byte comparisons are valuable but are not evidence of browser errors, cancellation, or the full standalone constructor/validation contract. |

The inspection developer reported focused Node 22/24 and Chromium runtime
checks, strict NodeNext and bundler declarations, and seven byte-exact Rust
editor output comparisons after the E01–E03 repair. On the integrated
tree the final tarball (SHA-256
`8c8cc6475c36d102c6227b049f24bcdac89e58155ddb7a73dde6c91ce90be221`)
passed the installed gate on Node 22.23.5 and 24.22.3, including the portable
catalog in Chromium 159 under Vite development and production. All six
catalogs matched Rust and Python byte for byte; the offline rebuild matched
the packaged Wasm and glue.

The seven focused [template tests](../javascript/tests/parity/template.mjs)
compare the [native oracles](../tests/javascript_parity/template.rs)
on Node and through the unbundled Chromium entry served by Vite. The shared
regression verifies snapshot and override preservation, exact fresh/override/template bytes,
matching `generateDocx` and package-plan encodings, imported readback, and
rejections at the original declaration or options path. The Node tests also
checked the file-template renderer, save behavior, and exact BigInt budget
thresholds. The installed template declarations passed strict NodeNext and
bundler checks with the Node condition; the three native files passed
schema/OPC checks (9, 9, and 10 covered parts for snapshot, override, and
template, respectively). The focused regressions ran through the direct
browser entry; the production bundle was checked using the portable catalog.

### Known boundary limits

- Rust detects property cycles within a single resolution pass. Separate
  concurrent root passes can start different properties and split the wait
  edges across graphs, leaving both calls pending. See the
  [B0 note](../javascript/tests/parity/B0-INTEGRATION.md). The acyclic waiter
  tests and bounded one-pass cycle tests do not close this gap.
- `imageFile(path)` is a Node-only, single-use `Value<NamedImageData>`.
  Node reads the file on resolution and carries `{ bytes, sourceName }`
  with the basename. Rust receives [`ImageData::NamedBytes`](../src/authoring/components.rs)
  and uses the name for resolved metadata, not for file lookup or format
  detection. Existing `Bytes` and `Path` inputs remain; downstream Rust
  exhaustive matches must account for the new public variant.
- `openDocx` opens unsigned Transitional DOCX only. Composition also accepts
  unsigned Transitional DOTX. Strict output and unsupported mandatory
  markup in processed parts are rejected. Source edits do not cross opaque
  boundaries or target arbitrary OOXML. Changed XML and the ZIP container
  may be reserialized; untouched payloads are checked separately.

## Shared declaration and Wasm boundary (B0)

- Keep `generateDocx`, its current TypeScript types (`DocumentDeclaration`,
  `Block`, `Run`, `Paragraph`, `TextRun`, `ImageRun`, `Table`, `GenerateOptions`,
  `GenerateDocxError`), existing accepted values and timestamp validation.
  Add an overload for `DocumentProgram` with `RenderOptions`; keep the
  legacy flat branch strict (including rejecting unknown keys, empty or
  nonrectangular legacy tables). `resolveDocument(program, {signal?})`
  returns `Promise<ResolvedDocument>`; `generateDocx(program, options)`
  resolves once then renders. An explicitly authored snapshot timestamp is
  separate from a render override; legacy `timestamp` still sets the snapshot
  timestamp. The program and override paths preserve date text without
  date parsing, matching Rust; malformed JavaScript UTF-16 is rejected.
  Supply the host clock once per snapshot at Rust's seconds
  precision for the new API (leave legacy defaults unchanged), never call the
  Wasm target's unavailable wall clock, and do not use render overrides to
  mutate a snapshot or its default comment dates.
- Use readonly record literals, camelCase Rust field names and lower
  camelCase variant tags, e.g. `{kind: "exact", value: 12}`. `kind` tags
  component/union identity; `breakType` encodes `Break.r#type`, `type` encodes
  `ImageRun.r#type` and `SectionProperties.r#type`. Preserve units, numeric
  bounds, order, absent vs explicit `false`/zero/empty string, and Rust defaults.
  Scalars in i64 fields accept `bigint` or safe integral `number` (reject
  precision loss); RGB is a 3-byte tuple. Validate known keys, UTF-16, numeric
  shape, and byte views at the adapter; leave document/OPC rules to Rust.
  `.when`, `with_*`, `From` and struct defaults map to guards, record members,
  tagged alternatives and omitted fields; no duplicate builder hierarchy.
- Write `V<T>` for `T | Value<T> | Deferred<T> | Property<T> | PropertyRef<T>`.
  Wrap **each** Rust `Value<T>` position, including containers and individual
  items separately where Rust does both, and do not promote every member of
  an already-ready format/metadata DTO to its own deferred field. Optional
  `Value<Option<T>>` fields distinguish omitted producer from a producer
  returning `undefined`. Use `condition?: boolean | (() => boolean)` where
  Rust has `Condition`; exclusion precedes resolution of that node's fields,
  not the outer producer of the node. Do not recursively normalize or
  validate deferred payloads before their Rust resolution sites need them.
- `resolveComponent(declaration, {signal?})` has typed overloads returning the
  specific `Resolved*` or `undefined`, using Rust `Component::resolve`.
  Resolved objects have readonly `.view` projections and
  opaque, instance-bound engine identity; pass resolved blocks/inlines as
  `{kind: "resolved", value: handle}` (or borrow dedicated table/image handles
  in edits). Keep borrowed input and output bytes separate from engine storage.
  Reusable handles are not implicitly consumed by insertion or encoding.
- Recommended boundary spike: store JS functions/promises in a host registry;
  Rust `Send` closures/futures carry only owned wire values and scoped tickets,
  not `JsValue` or `JsFuture` across await. Start/poll host producers on demand,
  wake on completion, decode callback results at the field site, and bridge
  callback errors through a cause ticket. Route `context.get(ref)` requests
  through the **waiting Rust producer's** future (also after JS `await`),
  so the Rust `PropertyRef` wait graph sees the real caller. Do not evaluate
  the graph in JS or call Rust in unrelated top-level passes for dependency
  requests. Prove a real indirect cycle, a false guard and two concurrent
  waiters before declaring this boundary usable. If the bridge cannot preserve
  these semantics without core changes, stop and escalate, rather than
  substituting eager JS traversal or unsafe `Send`. Also spike predicate
  exceptions: Rust `Condition` returns only `bool`, so latch a host predicate
  error and reject the pass rather than treating it as a successful exclusion
  or relying on Rust unwinding across Wasm. Check that the error does not start
  guarded field producers.
- Expose `Error` rejections, never raw thrown strings from Wasm. Preserve
  Rust `rule` (`Rule::code`) and `path` when available (map declaration
  segments to JS names; package part paths stay lossless), and original
  callback throw/rejection as `cause`. Do not turn unrelated failures into
  `producer_error` or invent a rule when Rust supplies none.
  Preserve initialization retry, cross-realm/subarray semantics, and
  no partial writes on failure. Release host tickets, wait edges and owned
  pending work on cancellation. An arbitrary external JS Promise cannot be
  forcibly stopped; supply a cooperative signal to owned producers and
  discard late settlement. Shared producer lifetime is independent of a
  cancelled waiter; non-cooperative external effects are not rolled back.
- B0 packed cases: `legacyGenerateDocxCompatibility`, retaining existing
  `contract-input.mts`/`input-contract.mjs`; `boundaryErrorsAndByteOwnership`,
  extending `diagnostics.mjs`, `unicode.mjs`, `cross-realm.mjs` and
  `init-retry.mjs` with the new endpoints, subarrays and failed producer paths.

The inventory names **required cases**. For declaration groups, each named
case must include positive typed fixtures for every public field/variant in its
Rust group (including omission and explicit false where applicable), assert
runtime acceptance/readback or matching bytes, and exercise Rust-specific
negative branches indicated in the row. Merely type-checking a fixture, or
only counting covered type names, does not qualify. Keep the type and runtime
fixtures linked so an ignored declaration member cannot pass.

## Authoring

| ID / owner | Rust baseline and capability | Proposed JavaScript API | Planned packed test case; Rust reference |
| --- | --- | --- | --- |
| A01 / text-layout | `authoring/mod.rs`: `Document` sections of individually deferred `Section`s, deferred metadata, timestamp, comment author, styles, numbering, defaults, and deferred note list with deferred elements. `Metadata` includes independent string/boolean custom maps, standard core and extended fields. | `DocumentProgram {sections, notes?, metadata?, timestamp?, commentAuthor?, styles?, numbering?, defaults?}`; `Metadata.customProperties` and `.customBooleanProperties`. | `documentFieldsAndCustomMetadata`: overlap error, explicit false, retained template values, empty program; `deferred.rs`, `docx.rs`, `templates.rs`. |
| A02 / text-layout | `Section`, `BlockChildren`, `Header`/`Footer`, default/first/even `HeaderFooterType`; deferred children and map entries; conditional stories; explicit fresh parts vs inheritance and template extension. | `SectionDeclaration {kind:"section", properties?, children?, headers?, footers?, condition?}`; `HeaderDeclaration` and `FooterDeclaration` accept `children: V<BlockChildren>`; `BlockChildren` consists of `V<BlockDeclaration>`; story maps use `Partial<Record<HeaderFooterType, V<...>>>`. | `sectionAndStoryComposition`: delayed children, skipped headers, inherited/explicit sources, part-local link/image/field; `async_fields.rs`, `features.rs`, `templates.rs`. |
| A03 / text-layout | `SectionProperties` and associated `PageSize`, `PageMargins`, `Orientation`, `SectionType`, `VerticalAlignment`, `Columns`, `PageNumbering`, `LineNumbering`/`LineNumberRestart`, `PageBorder(s)`/offset/display/z-order, `DocumentGrid`/mode. | Same-named DTOs; `Section.properties`, including r#type as `type`, `bidi`, `rtlGutter`; preserve physical EMU dimensions/margins vs twip-valued settings and explicit line numbering `clear`. | `sectionGeometryAndInheritedPatches`: all variant tokens, partial margins with/without template margins, direction flags, clear vs omit, invalid grid/borders/geometry; `features.rs`, `section_direction.rs`, `section_gutter.rs`. |
| A04 / text-layout | `ParagraphContent::Text/Inlines`; `Paragraph` format, numbering, inline list and simple comment values; `TextRun` text, format, comments, revision. `InlineChildren` is a list of deferred `Inline` values. | `ParagraphDeclaration` with mutually exclusive `text: V<string>` / `runs: V<InlineChildren>` (neither means empty text); `format`, `numbering`, `commentText`, `commentAuthor`; `TextRunDeclaration` with `text`, `format`, `commentText`, `commentAuthor`, `revision`; `InlineChildren` of `V<InlineDeclaration>`. | `paragraphAndRunComposition`: both content arms, defaults, concurrent text/comments and distinct paragraph/run authors, tabs, newlines, whitespace and Unicode; `text_runs.rs`, `text_contracts.rs`, `async_fields.rs`, `authoring_builders.rs`. |
| A05 / text-layout | `ParagraphFormat`, `Alignment`, `LineHeight`, `TabStop`/alignment/leader; `RunFormat`, `RunLanguage`, `Color`, `ThemeColor`, `Underline` including Rust Boolean shorthand; style refs, scripts, grid snapping, independent format flags, legacy line spacing and validation. | Same-named DTOs under `.format`; tagged exact/minimum/multiplier height; literal RGB, automatic and theme+modifiers; underline Boolean or named variant. | `paragraphAndRunFormatVariants`: independent script toggles, signed tab/clear, conflicting height/spacing, invalid scripts/measurement, explicit false, style-kind mismatch; `run_format_builders.rs`, `script_formatting.rs`, `line_height.rs`, `tab_stops.rs`, `style_references.rs`. |
| A06 / text-layout | `DocumentDefaults`, `NoteNumbering`, `FootnotePosition`, `EndnotePosition`, `NoteNumberRestart`, `NoteNumberFormat`. | `DocumentProgram.defaults` with run/paragraph defaults, tracking, mirror/top-gutter and independently authored footnote/endnote placement/numbering. | `documentDefaultSettings`: no implicit direct formatting, preservation on merge, invalid settings; `document_defaults.rs`, `notes.rs`, `section_gutter.rs`. |
| A07 / styles | Four `StyleDefinition` variants, `ParagraphStyleDefinition`, `CharacterStyleDefinition`, `NumberingStyleDefinition`, `TableStyleDefinition`, all 13 `TableSectionFormat` regions, style links/bases/next, template patches. | `DocumentProgram.styles` tagged `paragraph`, `character`, `numbering`, `table` records; same-named DTOs with all formatting fields (legacy style inch indents differ from paragraph point indents). | `stylesAndConditionalRegions`: forward, reciprocal and invalid refs, partial template update, all regions with paragraph/script/cell style rules; `features.rs`, `linked_styles.rs`, `character_styles.rs`, `conditional_{paragraph,script}_formatting.rs`. |
| A08 / styles | `NumberingDefinition`, `NumberingLevel`, `NumberingStartOverride`, `NumberingReference`, `NumberingAlignment`, `NumberingSuffix`; abstract vs concrete, shared/inherited definitions, start/replacement overrides, legal/restart/label formatting, numbering styles. | `DocumentProgram.numbering`, paragraph `.numbering`, same-named DTOs with open format/text strings. | `numberingInstancesAndOverrides`: overrides including zero, invalid levels/IDs/cycles, unchanged abstracts, template collisions, reference/style resolution; `numbering_{alignment,overrides,replacements,restart,suffix}.rs`, `legal_numbering.rs`. |
| A09 / tables | `Table`, `TableRow`, `TableCell`: delayed properties/lists/items, conditions at all three levels, nested block content, automatic width and padding, fixed/auto grids, span/vertical merge. `TableProperties`, `TableLayout`, `TableAlignment`, `TableLook`, table/cell borders and side/line tags. | `TableDeclaration {kind:"table", rows, properties?, condition?}` with `V<readonly V<TableRowDeclaration>[]>`, `row.children: V<readonly V<TableCellDeclaration>[]>`, `cell.children: V<BlockChildren>`; `TableBorder`/`CellBorder` retain `side`, `sz`, `color`, `val`. Keep legacy `Table` on the legacy input branch. | `structuredTableComposition`: rows/cells start lazily and disappear correctly, padding, width across spans incl. nested tables, merge and invalid geometry; `features.rs`, `table_{alignment,look}.rs`, `cell_layout.rs`, `async_fields.rs`. |
| A10 / tables | `TableRowProperties`, `RowHeight`, `TableCellProperties`, `CellMargins`, `CellTextDirection`, vertical alignment/shading; row/cell control wrappers. | `TableRowDeclaration.properties`, `TableCellDeclaration.properties`, same-named DTOs; `contentControl` uses A15's shared DTO. | `rowCellLayoutAndControls`: height variants, repeat/cantSplit, all directions, zero margins, wrappers and structural rejection; `row_properties.rs`, `cell_layout.rs`, `tests/python_bindings/test_table_content_controls.py` (see also Rust `document/content_control.rs`). |
| A11 / drawings | `ImageData::Bytes/Path` at baseline; B1 adds `NamedBytes`. `ImageRun` deferred data/transformation/alt text, ignored type hint; `ImageTransformation`, `ImageCrop`, `ImageAnchor`, `ImageAnchorReference`, alignments, `ImageWrap`/side, contour, flags, distances, `AltText`; `ResolvedImage` accessors. | `ImageRunDeclaration {kind:"image", data: V<ImageData>, transformation?, altText?, type?, condition?}`; `ImageData` is `Uint8Array` or `NamedImageData`; Node `imageFile(path)` returns `Value<NamedImageData>`. The image inline view exposes `image: ResolvedImage`, whose `.view` has name, alt text, owned bytes, dimensionsEmu and contentType. | `imageSourcesTransformsAndLifetime` and `nativeImagePathParity`: supported formats and density heuristics, dedup, crop/rotation/flip/wrap, invalid image/geometry, source mutation/deletion after resolve and exact native path equivalence; `features.rs`, `resolution_contract.rs`, `examples/image_dependencies.rs`. |
| A12 / drawings | `ShapeRun`/`ShapeProperties` incl. geometry, outline, text formatting/alignments and anchor; `ColumnChartRun`/`ColumnChartProperties`; `InlineFraction` deferred numerator/denominator; `Tab`, `Break`, `BreakType`. | `kind:"shape"`, `"columnChart"`, `"fraction"`, `"tab"`, `"break"`; deferred `properties` on shape/chart; `V<string>` fraction sides; optional `breakType` with Rust line default (not OOXML `br` default). | `shapeChartFractionAndBreakVariants`: validate positions, dimensions, chart lengths, text/XML constraints and fraction content; `shape_text_format.rs`, `column_charts.rs`, `inline_fractions.rs`, `text_runs.rs`. |
| A13 / structured | `Hyperlink`, both `HyperlinkTarget` constructors, `HyperlinkProperties`, display children, `ResolvedHyperlink::new`; `Field`, `FieldInstruction` (PAGE/NUMPAGES/REF/TOC/raw), checked `RawFieldInstruction`, `FieldForm`, `FieldProperties`, nested cached children and `ResolvedField::new`. | `kind:"hyperlink"` with `target`, `children`, `properties`; target `{kind:"external", url}` or `{kind:"bookmark", name}`. `kind:"field"` with `instruction`, `children`, `properties`; tags `page`, `numPages`, `reference` (name), `tableOfContents`, `raw` (text); no field evaluation. | `linksAndFieldCaches`: async siblings, relationship ownership, safe raw inputs, missing target/bookmark, child restrictions, simple/complex caches and flag false; `hyperlinks.rs`, `fields.rs`, `async_fields.rs`. |
| A14 / structured | `Note`, `NoteReference`, `NoteKind`, `NoteTarget`, `NoteId`, note constructors; `Comment`, `CommentMetadata`, plain `ResolvedComment` and `ResolvedCommentBody`; `RangeStart`/`RangeEnd`, `RangeKind`, `RangeId`, `BookmarkName`, `structured_comment`. Conditions affect markers and structured payload independently. | `kind:"note"` / `"noteReference"` with `{kind:"footnote"\|"endnote", id}` target; `kind:"comment"` with children/metadata; `"rangeStart"` with `id` and `range: V<RangeKind>`, `"rangeEnd"` with `id`. Range kinds `bookmark`, `comment` (text/author), `structuredComment` (comment declaration or resolved body handle). `bookmarkStart(name)`/`bookmarkEnd(name)` delegate bookmark ID derivation to Rust. | `noteAndRangeStoryComposition`: separate note ID spaces, rich comment/notes, overlap and cross-paragraph pairs, deferred IDs and excluded comment/markers, invalid metadata/ref/pairs/story; `notes.rs`, `ranges.rs`. |
| A15 / structured | `InlineContentControl`, `BlockContentControl`, `ContentControlProperties`, `ContentControlText` rich/plain (multiline)/dropdown (items/selection), `ContentControlListItem`, lock variants, checked resolved constructors. `TextRevision`, `RevisionKind` insertion/deletion/move source/destination, matching names/authors/dates. | Separate inline/block `kind:"contentControl"` at typed positions, deferred children/properties; `resolveComponent` takes `scope: "inline" \| "block"` for this tag to disambiguate empty children. Same-named DTOs also used by row/cell wrappers; text-run `revision` with tagged move including `name`. | `controlsAndAuthoredRevisions`: valid IDs/choices, conditional children, nonmatching move pairs/date, revision display and package markup independently; `text_revisions.rs`, `document/content_control.rs`, `tests/python_bindings/test_inline_content_control.py`, `tests/python_bindings/test_block_content_control.py`. |

Nested declarations assigned to the same row/fixture as their parent:

- A03: `PageBorders`, `PageBorderDisplay`, `PageBorderOffset`,
  `PageBorderZOrder`, `DocumentGridMode`.
- A05: `TabAlignment`, `TabLeader`.
- A09: `BorderLine`, `BorderSide`, `CellBorderSide`, `VerticalMerge`.
- A11: `ImageAnchorHorizontalAlignment`, `ImageAnchorVerticalAlignment`,
  `ImageWrapSide`.
- A12: `ShapeGeometry`, `ShapeOutline`, `ShapeTextAlignment`,
  `ShapeTextFormat`, `ShapeTextVerticalAlignment`.
- A15: `ContentControlLock`.

## Resolution and typed resolved views

| ID / owner | Rust baseline and capability | Proposed JavaScript API | Planned packed test case; Rust reference |
| --- | --- | --- | --- |
| R01 / resolution | `Value::{Ready,Lazy,Async}`, `ready/lazy/try_lazy/future/try_future`, recursive stage flattening, `map`, standalone `resolve`; producers single-use with Rust ordering/concurrency. | `ready(value)`, `lazy(producer)`, `future(asyncProducer)`, `Value.map(transform)`, `resolveValue(value, {signal?})`; callback may return another `V<T>` or, for async, a Promise thereof. Throw/reject is the fallible form. A Promise-returning factory, not an eager Promise, models a polled Rust future. Explicit `Value` tokens are single-use; fresh declarations/factories make new work. | `recursiveValuesMapAndSingleUse`: lazy→async→lazy+map before/on resolution; opaque errors with cause/path; concurrent sibling starts without reordering output; `async_fields.rs`, `deferred.rs`. |
| R02 / resolution | `Deferred::new/resolve`/`Value::deferred`, `Property::new/when/reference`, `Value::property`, `PropertyRef::id/resolve`, implicit await/field conversions. Shared success/failure, graph cycle diagnostics, named guard distinct from outer component. | `deferred(value)`, `property(name, value, {condition?})`, `.reference()`, `ref.id`, `resolveValue(deferredOrPropertyOrRef)`, `context.get(refOrDeferred)` from producers (including async continuations). `PropertyId` opaque; explicit shared tokens cache success or failure across passes. | `sharedValuesAndNamedDependencyGraph`: forward refs, diamond, concurrent roots, guards, cached failure and original cause, nested/direct/indirect cycle and fresh graph; `deferred.rs`, `resolution_contract.rs`, `compilation.rs`. |
| R03 / resolution | `Condition`, `Component`, and `Document::resolve`/`IntoFuture`; excluded component has no resolved object and skipped fields are not started; implicit dependencies may still start unguarded shared fields. Cancellation drops waiters, not surviving shared work. | `condition` on **every** Rust conditional component; `resolveComponent` and `resolveDocument`; producer context receives `signal`. `resolveDocument` returns a snapshot, not `undefined`. | `standaloneAndConditionalComponents` fixtures for section, paragraph, text, link, both controls, field, both range markers, note/ref/comment, image, shape/chart/fraction, tab/break, table/row/cell, header/footer; `cancelledWaiterDoesNotPoisonSharedWork`; `resolution/components.rs`, `async_fields.rs`, `resolution_contract.rs`. |
| R04 / boundary + feature owners | `ResolvedDocument` defaults/notes/metadata/timestamp/comment_author/styles/numbering/sections; `ResolvedSection` props/headers/footers/blocks/paragraphs; `ResolvedHeaderFooter.blocks`, `ResolvedParagraph` format, numbering, text, comment, runs. `ResolvedBlock` variants and physical `ResolvedTable` rows, properties/column count; `ResolvedTableRow` properties and cells, `ResolvedTableCell` blocks, span, merge, border/fill/direction/margins/control. | `.view` projects those fields (including direct `paragraphs` and Rust text projection) on corresponding `Resolved*` types. Reuse table, paragraph and control block handles and inline handles across programs, without re-running producers. | `resolvedBlockViewsAndReuse`: deep views, direct vs nested paragraphs, conditional members missing, width/count/properties, reuse and independent returned buffers; `docx.rs`, `features.rs`, `showcase.rs`. |
| R05 / boundary + feature owners | `ResolvedInline` all variants and accessors `text/format/comment/image/revision`; `ResolvedHyperlink`, `ResolvedField`, `ResolvedInlineContentControl` and `ResolvedBlockContentControl` projections and checked `new`; note target/blocks, comment body metadata/blocks, image readback; `inline_text`. Directly constructible `ResolvedInline` arms also allow formatted/commented tab/break/image. | Discriminated `ResolvedInlineView` (`breakType` for break case) with `displayText` separate from raw text, and nested resolved handles as applicable. `inlineText(handles)` calls Rust; `createResolvedInline(input)` and `createResolvedBlockControl(properties, children)` bind Rust enum arms/checked constructors (e.g. from child handles; image from A11). | `resolvedInlineViewsConstructorsAndReuse`: inspect every variant; authored insertion projects display text, deletion/move source do not, tabs/line breaks distinct from page breaks, check checked-constructor rejection and reuse without executing producers; `text_revisions.rs`, `fields.rs`, `hyperlinks.rs`, `showcase.rs`. |
| R06 / boundary + feature owners | Public `validate` on `RunFormat`, `NumberingDefinition`, `ParagraphFormat`, `SectionProperties`, `ImageTransformation`, `TableCellProperties`, `CellMargins`, `TableRowProperties`, `ResolvedInline`, `ResolvedParagraph`, `ResolvedDocument`, `DocumentDefaults`, `ResolvedNote`, `ResolvedCommentBody`, `TextRevision`, `CommentMetadata`. Other checks occur via constructors, resolution or compile. | `validateValue({kind, value})` discriminated over those public `validate()` inputs (including typed resolved handles); delegate to Rust. Constructors such as external URL, bookmark, raw field, ranges and style IDs delegate at their adapter use sites; `validateAuthoringId(kind, value)` exposes independent checks for `bookmarkName`, `rangeId`, `styleId`. | `standaloneValidationAndDiagnostics`: direct checks vs document context, exact available `rule`/`path`, malformed strings, options and recoverability, no fabricated rules; `domain.rs`, `text_contracts.rs`, `style_references.rs`, `diagnostics.mjs`. |

## Rendering, compilation and templating

| ID / owner | Rust baseline and capability | Proposed JavaScript API | Planned packed test case; Rust reference |
| --- | --- | --- | --- |
| T01 / template | `application.rs`: all `ResolvedDocument::to_docx*` forms with explicit/default `RenderOptions` (`OutputProfile`, timestamp, `ReadLimits`). `Strict` is reserved and rejected. Repeated fresh/template output from the same snapshot, with optional different output timestamp. | `resolved.toDocx(options?: RenderOptions)` and `resolved.toDocxWithTemplate(template: TemplateSource, options?)`; `OutputProfile = "transitional" \| "strict"`; `readLimits {maxParts,maxPartBytes,maxExpandedBytes}`. Do not emulate a fresh render with the public template entry point. | `freshAndTemplateRepeatRendering`: repeat exact bytes, switch timestamp or template and render original again, stable producers and snapshot timestamp, strict/profile and each read-budget boundary; `docx.rs`, `compilation.rs`, `opc/mod.rs`. |
| T02 / template | `DocumentTemplate`, `compile`/`compile_with_edits` with bytes, `PackagePlan::parts/part/validate/to_bytes`. DOCX/DOTX template input through relationship discovery; no implicit file I/O in compile. | `TemplateSource {bytes: Uint8Array, edits?: TemplateEdits}`; `compileTemplate(resolved, source, options?) → Promise<PackagePlan>`; `parts()`, `part(name)`, `validate()`, `toBytes()`; copy payloads on handoff; retain sorted ZIP entry names and Rust exact-name lookup. | `compileTemplatePlanAndBudgets`: inspect names/payloads/missing part, repeat encoding and mutate returned buffers without corrupting plan, accept valid DOTX conversion, reject invalid packages/limits; `compilation.rs`, `preservation.rs`, `opc/mod.rs`. |
| T03 / template | `TemplateEdits.replacements`, `EditScope` (`Body`, default `BodyHeadersFooters`, `SupportedStories` incl. related comments and notes), ordered text replacement, safe boundaries, and formatting preservation. | `edits.replacements: readonly (readonly [needle: string, replacement: string])[]`, `edits.scope`. No object/dictionary collapse of repeat needles. | `templateOrderedReplacementScopesAndBoundaries`: split runs, chained/duplicate needles, deletion, empty needle rejected, literal tab vs tab stop, comment marker vs opaque hyphen, reject protected/cross-container matches, no edit to appended content; `templates.rs`, `template_unmatched_stories.rs`, `ranges.rs`. |
| T04 / template | `content_control_replacements` by tag and `TextRevisionAction::{Accept,Reject}`. Edits run in Rust order: ordinary replacements, targeted control edits, then revision action per source story, before adding declarations. | `edits.contentControlReplacements: Readonly<Record<string,string>>`, `edits.textRevisionAction`. | `templateControlAndRevisionComposition`: preserve styled plain cache, unknown/ambiguous/unsupported targets, accept and reject including paired moves, guard malformed revisions, distinguish source from new declarations; `template/{content_control,revisions}.rs`, `templates.rs`, `showcase.rs`. |
| T05 / template | `paragraph_index` insertion among direct original body paragraphs, interleaved tables/section boundaries, append fallback; template section/header/style/metadata merge; signed/unsupported markup guards and untouched payloads. | `edits.paragraphIndex` zero-based; bytes render/compile as in T01–T02. | `indexedTemplateCompositionAndPreservation`: nested content not counted, multiple sections, beyond-end fallback, retained opaque bytes/relationships, reject signature/mandatory markup on affected parts; `indexed_template_sections.rs`, `preservation.rs`, `templates.rs`. |
| T06 / template + boundary | Native `DocumentTemplate::new(path)`, image path, and all `save*` forms. Atomic path replacement after successful compile; no file APIs in browser/Wasm. | `rocx-docx/node` host helpers `templateFile(path, edits?)` (read each time `toDocxWithTemplate`/`saveDocx` uses it), `imageFile(path)` (read on resolution, carries the basename), `saveDocx(resolved, path, {template?, ...renderOptions}?)` (sibling-temp write/rename after encoding). A file template returns a Promise from `toDocxWithTemplate`; its byte overload is synchronous. `compileTemplate` stays byte-only. Browser consumes returned bytes (caller chooses File/Blob/download); no implicit fetch or ambient browser path access. | `nodeFileSourcesAndAtomicSave`: file vs byte equivalence, deferred file errors, hidden image never read, image source deletion after resolve vs template re-read on render, same input/output path, build/write failure leaves existing output untouched; `docx.rs`, `templates.rs`, `resolution_contract.rs`. |

## Imported editor

These are package operations, not replacements for a resolved authoring
snapshot. Never reconstruct source ZIP from the inspection DTO. Anchor tokens
are opaque and not transferable across imports, Wasm instances or workers;
only identical inline addresses in one physical part of one snapshot compare
equal. Use a per-snapshot identity table and the Rust anchor, not caller-chosen
part names/indices. `InlineAnchor` and `TableAnchor` are read-only.

| ID / owner | Rust baseline and capability | Proposed JavaScript API | Planned packed test case; Rust reference (`tests/imported_editor.rs` unless stated) |
| --- | --- | --- | --- |
| E01 / inspection | `ImportedDocument::open/open_with_limits`, immutable snapshot; `body`, physical `ImportedTable`/rows/cells with ordered opaque children; `sections` half-open body bounds and explicit raw/recognized types. | `openDocx(bytes, {readLimits?}) → Promise<ImportedDocument>`; `.body()`, `.sections()`; `BodyBlock`, `ImportedSection`, `ImportedTable`, `ImportedTableChild`, `ImportedTableRow`, `ImportedRowChild`, `ImportedTableCell`, `ImportedCellBlock` tagged DTOs with readonly anchors as applicable. Once initialized, read/preview/encode handle methods are synchronous. | `importedBodySectionsAndPhysicalCells`: implicit/explicit/raw boundaries, opaque content/order, no merge expansion, unknown alignment XML/raw type, open limits/unsupported input; `reads_direct_body_blocks_and_paragraph_properties`, `table_inspection_preserves_unknown_children_in_order`, `inspects_explicit_and_implicit_sections_and_refuses_ambiguity`. |
| E02 / inspection | `ImportedParagraph` text, recognized/raw alignment, opaque flag, `inline_items`, `inline_structure`; `ImportedCellParagraph` exposes the flat and structured projections but has no paragraph edit anchor. `ImportedInline`, `ImportedInlineNode`, `ImportedRun`, `ImportedHyperlink` and `inline_at`: retain run/link boundaries, raw `relationship_id`/bookmark (not resolved URLs), explicit opaque names and literal tab/line/wrapping/page/column distinctions. | Same-named DTOs in camelCase, `.inlineAt(anchor)`; direct paragraph has `ParagraphAnchor`, run/link has `InlineAnchor` (including in direct table cells); typed foreign or forged tokens rejected. | Body and cell fixtures check two adjacent runs are not merged, cache vs opaque nodes, stale even on reopening identical bytes, actual node lookup, body independent of malformed stories, empty vs opaque cell blocks, and absence of cell-paragraph edit anchors. |
| E03 / inspection | `default_headers` and `stories` are lazy. `ImportedStory` identifies each effective header/footer slot (`SectionStoryKind` × `HeaderFooterType`), part name, section, provenance and ordered blocks. `ImportedHeader` retains a section-scoped edit anchor. `inline_structure(block_index)` returns an empty slice for an empty paragraph, absent for opaque/out of range. | `.defaultHeaders()` and `.stories()`, each with `inlineStructure(index)`; `sectionIndex`, `partName`, `inherited`, ordered blocks, story `kind` and `variant`; stable inline anchors on physical aliases through both views. Only `.defaultHeaders()` exposes `HeaderAnchor`. | Header and story fixtures check all six slots, order, alias equality, separate physical parts unequal, inherited flags, empty/opaque/out-of-range distinction, foreign and forged anchors, and lazy failure for malformed story parts. |
| E04 / editor | `ParagraphAlignmentEdit`, `TableInsertion` (resolved table), `preview` / `preview_batch`; preserve unknown properties, correct insertion width/location and range guards; one alignment and one insertion allowed on same anchor. | `imported.preview({alignments?, tableInsertions?, headerImages?})` with `target`, `alignment` or `table: ResolvedTable`; `EditPreview.changes()` and `.insertions()` ordered by original source, skip unchanged alignment, report `before` raw XML, original indices, rows/columns; `.toDocx()`. | `alignmentAndTablePreviewAndPreservation`: typed preview and reopened body, no-op all payloads, nested format/fraction/link and relationships/span sizing, ambiguous/unsafe range and stale/duplicate targets; `patch_preserves_other_markup_and_parts_and_preview_is_non_mutating`, `inserts_exactly_after_anchor_in_mixed_batch_and_preserves_untouched_parts`, `table_after_section_terminator_uses_next_section_width`. |
| E05 / editor | `HeaderImageEdit`, `HeaderEditMode::{Shared,SectionLocal}`, `preview_header_image`/`HeaderImagePreview`, image from Rust resolution. Default and first-page story anchors are editable. Shared mode reports consumers incl. first/even aliases; local copies only supported headers, restores later inheritor for the selected slot if necessary. | `imported.previewHeaderImage({target, mode:"shared"\|"sectionLocal", image: ResolvedImage})`; obtain first-page target from `stories().view.anchor`. `.change()` and `.toDocx()`; `HeaderImageChange {sourcePart,outputPart,affectedSections,restoredSection?,mode}`. Same operation through batch. | `headerImageScopesAndEffects`: shared consumer indices vs section-scoped target; local fork keeps original payload/relationships and redirects inheritor; relocated header; reject unsupported existing/floating image or stale anchor; `first_header_image_local_preserves_aliases_and_inheritance`. |
| E06 / editor | `preview_transaction` with all three edit types; duplicates and physical shared conflicts, local forks relative to original part, all-or-nothing; retained `EditPreview` survives source drop. | Same `.preview` as E04; `.headerChanges()` in target section order. Preview rendering repeatable and independent of source and other previews. JS checks handle type only; Rust judges anchor validity/conflicts and stages the package. | `mixedTransactionAtomicityAndRepeatPreview`: shuffled input deterministic, shared/local collisions, adjacent local precedence, stale/forged target or late failure leaks no staged output, next preview still works, repeated independent outputs, exact untouched payloads; `mixed_transaction_preserves_source_and_uses_original_slots`, `adjacent_local_forks_are_order_independent_and_failure_leaks_no_staged_work`, `retained_previews_write_independent_edits_after_source_drop`. |

`AlignmentChange {bodyIndex,before?,after}` and
`TableInsertionChange {afterBodyIndex,rows,columns}` are the element types of
E04's report arrays; `before` is absent for a previously unset alignment.

## Scope edges and remaining gaps

- `src/lib.rs` also exports domain/support primitives (`Points`, `Twips`, `Emu`,
  `HalfPoints`; `StyleId`, `AssetId`, `NoteId`, `StoryId`, `CommentId`,
  `DrawingId`, `StoryKind`; `DeclarationPath`, `Diagnostic`, `Rule`,
  `Error`/`Result`). Authoring uses numbers/strings, checked ID/measure inputs,
  typed story/anchor metadata and Error rejections. If independent primitive
  conversion is desired, expose `convertMeasurement({value, from, to})`
  (and Rust checked scaling) backed by Rust; test `checkedMeasurementUnits`
  against `domain.rs`. Rust identity allocation, `Display`, `From`,
  diagnostic construction and XML-token formatting are not separate document
  operations. Do not claim standalone JS parity for these helpers.
- `opc` and `ooxml` separately expose lower-level `PartName`, `ContentType`,
  `RelationshipId`, `RelationshipType`, `Target`, `StoryContent`,
  `StoryDescriptor`, `StoryRegistry`. `PackagePlan`, read limits and profiles
  are in scope above; independent registry/OPC name utilities are not
  authoring/editor APIs. `compileTemplatePlanAndBudgets` should cover
  observable package validation rather than promise a public OPC toolkit.
- Rust itself does not expose general imported OOXML editing. Direct body
  table-cell paragraphs expose both inline projections, but nested tables and
  row/cell wrappers may remain opaque. All six header/footer variants can be
  inspected; only default headers accept the current image edit. Alignment is
  direct only, and no general inline/row/section/resource edit targets exist.
  Authoring footers, notes, controls, shapes and revisions does
  not imply inspecting/editing their existing typed nodes. See
  [EDITOR-PARITY.md](EDITOR-PARITY.md) for separate Rust gaps.
- Cross-thread execution/transfer of Rust `Send`/`Sync` values and Rust
  executor/drop mechanics have no direct shared-object JS analogue. Same-instance
  concurrent promises, cancellation of a waiter and preservation of shared
  work remain in scope; no claim that arbitrary Promise effects are cancellable.
  Browser path read/save has no portable equivalent beyond host-supplied bytes.

## Recommended implementation batches and verification

1. **B0, single owner first:** settle declaration/type exports and Wasm
   dispatcher before feature workers. Own `javascript/index.d.ts` (public
   barrel), `javascript/src/lib.rs`, proposed `javascript/src/boundary.rs`,
   entrypoints/loader and package file list. Freeze handle and wire schemas,
   initialization/error rules, path conversion, typed decode and encode hooks,
   cancellation protocol and the host-producer bridge spike. Reserve typed
   module declarations and register all module hooks in this batch; do not
   have parallel workers edit the barrel, entrypoints, lockfiles or pack list.
2. **B1, parallel after B0:** GPT-6.2 Sol owners `text-layout`, `styles`,
   `tables`, `drawings`, `structured`, `resolution`, `template` and
   `inspection`. Propose one exclusive `javascript/src/<owner>.rs` and
   `javascript/types/<owner>.d.ts` pair per owner (replace hyphens in Rust
   module names with underscores), plus `javascript/tests/parity/<owner>.mts`
   holding that owner's named cases. `resolution` also owns
   `javascript/resolution.js`; `template` owns a Node-only
   `javascript/file-io.js` behind B0's Node export. B0 owns shared DTO/handle
   dispatch; feature owners supply Rust conversion and view hooks for their
   domain, not new Word/ZIP codecs. Tables use the structured control DTO,
   styles use text-layout format DTOs, template uses a resolved document
   handle, and imported inspection needs only B0's package/anchor boundary.
   Use agreed hooks/types while implementing independently.
3. **B2 integration (implemented locally):** `editor` owns its own module/type/test trio after inspection anchors
   and resolved table/image handles are agreed and usable. Use Rust
   `preview_transaction`, not three renders or a JS transaction. One
   integration owner sequences `javascript/tests/check-packed.sh`,
   `tools/check_javascript.py`, the JavaScript catalog example, existing paired
   examples and their coverage/docs; no worker owns them implicitly. Paths
   listed here are proposed allocations, not authorization to write them.
4. **Gate from the installed tarball (local results above):** use `bash javascript/tests/check-packed.sh`
   for Node 22 and 24 and Chromium (bundled and direct browser module modes).
   Run strict TypeScript with NodeNext and bundler resolution and execute
   the same new behavior fixtures against the installed package. Keep the
   existing legacy workflow and negative tests. The catalog comparison runs
   the Rust catalog and a JS authoring program with
   aligned inputs/assets/timestamps. Compare complete ZIP bytes for final,
   `.template.docx`, `.edited.docx`, `.header-source.docx`,
   `.header-shared.docx` and `.header-local.docx` on each runtime; no XML
   normalization. Keep Rust/Python catalog/schema/OPC checks. Use ZIP part
   diffs only to diagnose byte mismatches.
5. **Independent assertions:** besides bytes, check typed resolved/imported
   views, snapshot anchors and ordered preview reports, rule/path/cause,
   bounded rejection of cycles, no-op/nonmutation, and exact payloads of
   untouched parts (incl. relocated relationships, media and opaque parts).
   Reuse `tests/fixtures/imported-editor.docx` as independently generated
   input; reopen output and inspect basic features with python-docx without
   resaving, and perform package-level checks for lossless retention. Include
   focused Rust-equivalent fixtures for rows not exercised by the catalog.
6. **Independent review:** reviewer walks every A/R/T/E row from Rust source
   to implemented public declaration, adapter and passing installed-package
   evidence, including fields emitted by macros and composed/deferred paths.
   Fix confirmed findings, rerun gates and repeat review; missing evidence is
   a gap. No publishing implied by this contract.

## Editor additions for #3

`entryStories()` exposes Rust `entry_stories()`; `runText` and `runTextChanges()`
expose anchored run-text replacements and reports. The portable catalog checks
entry metadata, inline lookup, duplicate-target rejection and change details;
the `.run-text.docx` artifact must match Rust/Python in Node and Chromium.
# Anchored body-run removal

`preview({runRemovals: [anchor]})` and `runRemovals()` correspond to Rust
`preview_with_run_removals` / `run_removals` and Python
`preview([], run_removals=[anchor])` / `run_removals`. Installed editor parity
fixtures cover the removal report, stale and conflicting targets, independent
readback, unchanged parts and exact Rust/Python/JavaScript bytes. Packed Node
22.23.5 and 24.22.3 and Chromium 159 development and production checks passed,
including schema/OPC validation and reproducible package provenance.
