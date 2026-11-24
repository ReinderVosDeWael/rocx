# JavaScript API reference

[Install the development tarball](../javascript.md#build-and-install-the-tarball). For exact input and result types, see the [shipped declarations](../javascript.md#typescript-declarations).

## Generate and resolve

For new code, import `declarative as d` from `rocx-docx`. Factories produce
ordinary authoring declarations: use them alongside existing record inputs
and pass the result to `generateDocx`, `resolveDocument`, or
`resolveComponent` as appropriate.

| Factory | Input |
| --- | --- |
| `Document(sections, options?)` | Sections; optional metadata, styles, timestamp and other document options |
| `Section(children, options?)` | Body blocks; optional page layout in `properties` and `headers` / `footers` by slot |
| `Paragraph(textOrRuns, options?)` | String or inline children; optional paragraph `format` |
| `Paragraph.Text(text, options?)`, `Paragraph.Runs(runs, options?)` | Explicit text or inline children, including produced values |
| `TextRun(text, options?)` | Text; optional run `format` |
| `Table(rows, options?)` | Table rows; optional table `properties` |
| `TableRow(cells, options?)` | Table cells; optional row `properties` |
| `TableCell(children, options?)` | Cell blocks; optional cell `properties` |
| `Header(children, options?)`, `Footer(children, options?)` | Story blocks, passed into a section's header or footer slots |

Other named factories cover `Tab`, `Break`, `Hyperlink`, `Field`, `ImageRun`,
`ShapeRun`, `ColumnChartRun`, `InlineFraction`, `Note`, `NoteReference`,
`Comment`, `RangeStart`, `RangeEnd`, `InlineContentControl` and
`BlockContentControl`. Style factories are `ParagraphStyleDefinition`,
`CharacterStyleDefinition`, `TableStyleDefinition` and
`NumberingStyleDefinition`. See the installed `declarative` declarations
for exact signatures.

```typescript
generateDocx(declaration: DocumentDeclaration, options?: GenerateOptions): Promise<Uint8Array<ArrayBuffer>>
generateDocx(program: DocumentProgram, options?: RenderOptions): Promise<Uint8Array<ArrayBuffer>>
resolveDocument(program: DocumentProgram, options?: ResolveOptions): Promise<ResolvedDocument>
```

| API | Behavior |
| --- | --- |
| Legacy `generateDocx` | Renders a `DocumentDeclaration` using the legacy blocks below. `GenerateOptions.timestamp` sets the snapshot timestamp. |
| Program `generateDocx` | Takes a `DocumentProgram`, built with `d.Document` or supplied as a `{ sections, ... }` record. Resolves once, then renders. |
| `resolveDocument`, `resolveComponent` | Resolve a program or one typed component. An excluded standalone component returns `undefined`. Use `scope: "inline"` or `"block"` for standalone `contentControl` declarations. |
| `ResolvedDocument.toDocx(options?)` | Synchronously encode a fresh DOCX. Read the snapshot through `.view`; rendering with a timestamp override does not change the snapshot. |
| `ready`, `lazy`, `future`, `deferred`, `property`, `resolveValue` | Model literal, produced, and shared values at `V<T>` positions. `Value.map` is synchronous; use `future` to assimilate a Promise returned by a producer. `ResolveOptions.signal` cancels a waiter, not surviving shared work. |
| `inlineText`, `createResolvedInline`, `createResolvedBlockControl` | Compute the engine's display projection or construct reusable resolved content from ready input. Insert compatible block/inline handles with `{ kind: "resolved", value: handle }`. |
| `bookmarkStart`, `bookmarkEnd`, `validateAuthoringId`, `validateValue` | Create bookmark markers; check authoring IDs or a supported validation input. See the installed `AuthoringIdKind` and `ValidateValueInput` types for the accepted inputs. Contextual checks still run during resolution or rendering. |

### Legacy declaration fields

| Input | Behavior |
| --- | --- |
| `DocumentDeclaration` | `blocks: readonly Block[]`; one section with default page layout |
| Paragraph block | `kind: "paragraph"`, `runs: readonly Inline[]`, optional `alignment` |
| Table block | `kind: "table"`, `rows: readonly (readonly (string \| TableCell)[])[]`; a nonempty rectangular grid |
| Rich table cell | `blocks: readonly Block[]` (including nested tables, up to 32 table levels); strings remain shorthand for a plain paragraph |
| Text run | `kind: "text"`, `text: string`, optional `bold` and `italic` booleans |
| Image run | `kind: "image"`, `bytes: Uint8Array` containing an encoded image, optional `altText: string` and positive `width` and `height` in points |
| Tab run | `kind: "tab"` |
| Break run | `kind: "break"`, optional `breakType`: `"line"` (default), `"page"`, `"column"`, or `"textWrapping"` |
| External hyperlink | `kind: "hyperlink"`, `target: string` (absolute URI/IRI), `runs: readonly Run[]` (text/image/tab/break, no links) |
| `GenerateOptions` | Optional `timestamp`: UTC ISO time with seconds and optional milliseconds; defaults to the current time |

An image with one dimension retains its intrinsic aspect ratio; with neither
it retains its intrinsic size.

Unlike legacy `GenerateOptions.timestamp`, a program snapshot timestamp
or `RenderOptions.timestamp` is not parsed, calendar-validated, or normalized.
Supply a schema-valid date-time for DOCX core properties. Fractional seconds
and numeric offsets, as in `2026-11-21T00:00:00.1+00:00`, are preserved.

Async calls reject with `Error`; synchronous methods throw. The
`GenerateDocxError` declaration describes optional `path`, `rule`, and
`cause` properties. Diagnostics can identify a declaration field or
package part, include a stable engine rule code, or preserve a producer's
original throw or rejection. The package does not fetch input images or
upload documents.

## Compose with a template

| API | Result |
| --- | --- |
| `document.toDocxWithTemplate(source: TemplateSource, options?)` | `Uint8Array<ArrayBuffer>` synchronously |
| `compileTemplate(document, source: TemplateSource, options?)` | `Promise<PackagePlan>` |
| `plan.parts()`, `plan.part(name)` | Sorted `[name, payload]` pairs or one exact, case-sensitive part lookup (leading slash optional). Bytes are copies; missing parts return `undefined` from `part`. |
| `plan.validate()`, `plan.toBytes()` | Synchronous package validation and encoding |

`TemplateSource` contains `bytes` and optional `edits`. `TemplateEdits`
accepts ordered `replacements` pairs (including repeated needles),
`contentControlReplacements` keyed by tag, and `textRevisionAction`
(`"accept"` or `"reject"`). These run in that order on source stories before
new declarations are composed. `scope` defaults to `"bodyHeadersFooters"`;
alternatives are `"body"` and `"supportedStories"` (also follows supported
related comments and notes). `paragraphIndex` is zero-based among original
direct body paragraphs, not nested paragraphs; omitting it or using an
index past the end appends.

```javascript
import { compileTemplate, declarative as d, resolveDocument } from "rocx-docx";

// templateBytes comes from a file picker, HTTP request, or the caller's disk read.
const document = await resolveDocument(d.Document([d.Section([d.Paragraph("Summary")])], {
  timestamp: "2026-11-21T00:00:00Z",
}));
const plan = await compileTemplate(document, {
  bytes: templateBytes,
  edits: { replacements: [["{{quarter}}", "Q4"]], scope: "body" },
});
const output = plan.toBytes();
```

Use unsigned Transitional DOCX or DOTX source packages. `RenderOptions`
supports `timestamp`, `profile`, and `readLimits`
(`maxParts`, `maxPartBytes`, `maxExpandedBytes`). `maxParts` is a
non-negative integer within the Wasm `usize` range. The byte limits
accept non-negative safe integers or `bigint` values up to `2^64 - 1`.
`"strict"` is reserved and rejected. Package guards can reject unsupported
mandatory markup in processed parts. Ordinary replacement cannot match through opaque
boundaries or edit protected field, revision, or control content. Targeted
control replacement supports only a restricted plain-text shape, and
revision actions apply only to supported text revision structures. Unchanged
part payloads are retained byte for byte; changed XML and the ZIP container
can be reserialized.

## Inspect a DOCX

`await openDocx(bytes, { readLimits })` opens an immutable, unsigned
Transitional DOCX snapshot. Its methods are synchronous:

- `body()` lists direct paragraphs, physical tables, and opaque blocks in
  order. Tables show physical rows and cells without expanding merges;
  direct cell paragraphs expose flat inline items and `inlineStructure`
  with read-only run and hyperlink anchors. Nested table blocks remain opaque.
- `sections()` gives half-open body ranges and explicit section types,
  with raw type values where present.
- `stories()` lazily inspects effective default, first-page, and even-page
  header/footer slots, including inherited references. Unset slots are
  omitted, regardless of page selection settings.
- `entryStories()` lists footnotes, endnotes and comments, including separator
  entries. Each includes `kind`, `partName`, its original `id` string and
  `blocks`. Paragraphs expose `inlineItems` and `inlineStructure`; other blocks
  are opaque. `noteType`, `author`, `date` and `initials` are optional.
- `defaultHeaders()` inspects effective default headers and supplies
  edit targets. Both story and header handles expose
  `inlineStructure(blockIndex)`: a run/link projection for a paragraph,
  `[]` for an empty paragraph, or `undefined` for opaque or out-of-range blocks.
- `inlineAt(anchor)` looks up a run or hyperlink. Hyperlinks expose the
  source relationship ID or bookmark, not a resolved URL. Inline anchors
  are snapshot-local: aliases of one physical story reuse the same inline
  addresses, but reopening identical bytes produces a distinct snapshot.

Opaque markers report names, not editable XML. The inspection view is not a
round-trip authoring declaration.

## Preview anchored edits

| Synchronous API | Behavior |
| --- | --- |
| `imported.preview(batch: EditBatch): EditPreview` | Stages optional `alignments`, `tableInsertions`, `headerImages` and `runText` in one transaction against the original snapshot. |
| `imported.previewHeaderImage(edit: HeaderImageEdit): HeaderImagePreview` | Stages one default-header image append. `change()` reports the effect. |
| `preview.toDocx(): Uint8Array` | Encodes a new copy. Repeatable; does not consume the preview or modify the imported document. |

Alignment edits take a `ParagraphAnchor` and `alignment`; table insertions
take a `ParagraphAnchor` and a `ResolvedTable` and insert immediately after
the original body paragraph. Header edits take a section-scoped
`HeaderAnchor` from `header.view.anchor`, a `ResolvedImage` for an inline
image, and `mode`. `"shared"` changes the physical header part for all its
consumers. `"sectionLocal"` forks a supported header for the selected
section, preserving other sections' effective headers. This is not
general story or table-cell editing. To replace a direct body run containing
one text element outside fields and ranges, pass `runText: [{target: run.anchor,
text: "New text"}]`. `runTextChanges()` reports original `bodyIndex`, `inlineIndex`,
`before` and `after` for changed runs. Formatting and adjacent runs survive.

```javascript
import { openDocx } from "rocx-docx";

const imported = await openDocx(sourceBytes);
const paragraph = imported.body().find((block) => block.kind === "paragraph");
if (!paragraph) throw new Error("Expected a direct body paragraph");
const preview = imported.preview({
  alignments: [{ target: paragraph.anchor, alignment: "center" }],
});
console.log(preview.changes());
const editedBytes = preview.toDocx();
```

`changes()` reports original body indices, previous XML alignment values
(when present), and the new alignment. It omits unchanged alignments.
`insertions()` reports original anchor indices and table dimensions.
Both follow original body order. `headerChanges()` follows target section
order and reports source/output parts, affected sections, and any restored
section. Snapshot mismatches, conflicting operations, and unsupported
edits throw without leaving a partial preview. Untouched part payloads
are preserved; edited XML and the ZIP container can be reserialized.

## Node filesystem helpers

Import from `rocx-docx/node` in Node.js. `templateFile(path, edits?)` reads
the local file on each render or save. Unlike a byte source, passing that
token to `document.toDocxWithTemplate` returns a Promise; `compileTemplate`
accepts byte sources only.

```javascript
import { declarative as d, imageFile, resolveDocument, saveDocx, templateFile } from "rocx-docx/node";

const report = await resolveDocument(d.Document([d.Section([
  d.Paragraph([d.ImageRun(imageFile("logo.png"), { altText: { description: "Logo" } })]),
])]));
await saveDocx(report, "report.docx", { template: templateFile("layout.dotx") });
```

`imageFile` reads the file once at resolution. `ResolvedImage.view.name` keeps the basename; the bytes determine the format. Pass a `Uint8Array` for an unnamed image.

`saveDocx` encodes fully before writing and renaming a sibling temporary
file onto the destination. In the browser, pass bytes from the caller and
download the output with a `Blob`; the Node helpers are not available there.

## Report declaration

The named factories build the same rich report in Node and Chromium:

```javascript
{{snippet:examples/javascript/author.mjs:declaration}}
```

See the [runnable Node and browser examples](../javascript.md#runnable-report).

## Legacy report declaration

The equivalent single-section record form remains supported:

```javascript
{{snippet:examples/javascript/legacy-author.mjs:declaration}}
```
