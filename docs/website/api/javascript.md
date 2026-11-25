# JavaScript API reference

[Install `rocx-docx`](../javascript.md). For exact input and result types, see the [shipped declarations](../javascript.md#typescript-declarations).

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

- `body()` lists direct paragraphs, physical tables, content controls and opaque blocks in
  order. Tables show physical rows and cells without expanding merges;
  direct cell paragraphs expose flat inline items and `inlineStructure`
  with read-only inline anchors. Nested table blocks remain opaque.
  A direct block control (`kind: "contentControl"`) exposes optional literal
  `tag`, `alias` and `lock`; its contents are opaque and it has no edit anchor.
- `controls()` lists physical block, inline, row and cell controls in the main
  story. Each has an `anchor`, `context`, `bound`, and optional literal `id`,
  `tag`, `alias`, and `lock`. Pass `anchor` to `preview({controls: [{target:
  anchor, alias: "Caption"}]})` to patch an eligible control's alias, tag or
  single-run text. Tags and IDs are not unique addresses.
- `styles()` returns `undefined` when the styles relationship is absent.
  Otherwise it reports the physical part and direct style IDs, kinds, names
  and `basedOn` references in source order, without resolving inheritance.
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
  `inlineStructure(blockIndex)`: structured inline nodes for a paragraph,
  `[]` for an empty paragraph, or `undefined` for opaque or out-of-range blocks.
- `inlineAt(anchor)` looks up a run, hyperlink or inline revision. Insertion
  and deletion wrappers expose `revisionKind`, literal optional `id`, `author`
  and `date`, and ordered children. Move wrappers with authors expose
  `moveSource` or `moveDestination`; nested revisions remain opaque.
  `previewTrackedText` and `previewTrackedMove` track edits to supported direct
  body runs. `previewRevision` accepts or rejects one wrapper or both legs of
  a move; revision children remain inspection-only. Hyperlinks expose the
  source relationship ID or bookmark, not a resolved URL. Inline anchors
  are snapshot-local: aliases of one physical story reuse the same inline
  addresses, but reopening identical bytes produces a distinct snapshot.
- `previewTrackedBold` records a direct bold change on a plain or bold-only
  body run. Inspect `propertyRevision` on the reopened run and use its anchor
  with `previewRevision`. `previewTrackedParagraph` inserts a plain paragraph
  before an ordinary paragraph or deletes an eligible single-run paragraph.
  Inspect `markRevision` on the reopened paragraph, then pass its paragraph
  anchor and `"accept"` or `"reject"` to `previewParagraphRevision`.

Opaque markers report names, not editable XML. The inspection view is not a
round-trip authoring declaration.

## Preview anchored edits

| Synchronous API | Behavior |
| --- | --- |
| `imported.preview(batch: EditBatch): EditPreview` | Stages ordinary edits against original snapshot targets. All fields are optional: `alignments`, `tableInsertions`, `paragraphInsertions`, `cellAlignments`, `headerImages`, `textRanges`, `textInsertions`, `runText`, `runRemovals`, `blockEdits`, `inlineEdits`, `tableStructure`, `styleEdits`, `resources`, `controls`. |
| `imported.previewHeaderImage(edit: HeaderImageEdit): HeaderImagePreview` | Stages one image append to an effective header or footer slot. Obtain the target from `stories()`; `change()` reports the effect. |
| `imported.editStoryText({ownerPart, runText}): Uint8Array` | Returns DOCX bytes for a separate atomic batch in one physical part. Supply the inspected part name and anchors for eligible direct runs in that header/footer or ordinary footnote/endnote/comment entry. All aliases see the edit. |
| `imported.editEntryInlines({ownerPart, inlineEdits}): Uint8Array` | Insert after or replace eligible direct runs/hyperlinks, or patch direct run bold, in ordinary note/comment entries of one part. New external links belong to that part; reference marks and reserved notes are protected. |
| `imported.editNoteReference({ownerPart, target, kind, ...fields}): Uint8Array` | `create` takes an unused numeric ID and text; `attach`, `retarget`, `renumber` take an ID. `detach` retains the definition; `delete` also removes its simple definition. |
| `imported.editEntry({ownerPart, id, kind, ...fields}): Uint8Array` | Use the decimal string ID. Insert, replace or remove a simple paragraph; set base comment author/date/initials or clear date/initials; `deleteUnusedNote` removes a simple unreferenced note. |
| `preview.toDocx(): Uint8Array` | Encodes a new copy. Repeatable; does not consume the preview or modify the imported document. |

Alignment edits take a `ParagraphAnchor` and `alignment`; table insertions
take a `ParagraphAnchor` and a `ResolvedTable` and insert immediately after
the original body paragraph. Header/footer edits take a section-scoped
`HeaderAnchor` from `story.view.anchor`, a `ResolvedImage` for an inline
image, and `mode`. `"shared"` changes the physical part for all its
consumers. `"sectionLocal"` forks the story for the selected
section, kind and variant, preserving other slots' effective stories. To replace a direct body run containing
one text element outside fields and ranges, pass `runText: [{target: run.anchor,
text: "New text"}]`. `runTextChanges()` reports original `bodyIndex`, `inlineIndex`,
`before` and `after` for changed runs. Formatting and adjacent runs survive.

- `paragraphInsertions: [{target, paragraph}]` inserts a `ResolvedParagraph`
  after a direct body paragraph under the same inline and style restrictions
  as table insertion. `paragraphInsertions()` reports original anchor indices.
  Two insertions at the same anchor conflict, including a paragraph and a table.
- `cellAlignments: [{target, rowIndex, cellIndex, alignment}]` takes a direct
  body table anchor and physical indices (including opaque entries). Use
  `top`, `center`, `bottom` or `both`; `cellAlignmentChanges()` reports the
  physical cell, prior XML value and new alignment. Only the selected cell
  changes, including when it is a merge continuation. Ambiguous grids and
  unsupported wrappers reject the batch.
- `runRemovals: [anchor]` removes a whole single-text direct body run with
  recognized local formatting. Unknown markup, protected ranges, or replacing
  the same run in this batch reject the edit. `runRemovals()` reports original
  body and inline indices and the removed text.
- `textRanges: [{target, start, end, text}]` replaces a nonempty half-open range
  of Unicode scalar positions in a plain direct-body paragraph. Count with
  `Array.from(text)`, not `.length`. The replacement retains the formatting
  of the first selected scalar; `runTextChanges()` reports changed runs.
- `textInsertions: [{target, position, text}]` inserts nonempty text at an
  original scalar position. At a run boundary it takes the next character's
  formatting, or the last run's formatting at paragraph end.
- `blockEdits` accepts `replace`, `remove`, `replaceSection`, `keepWithNext`,
  `paragraphFlag`, `sectionFlag` and `sectionType` at eligible direct body
  paragraph anchors. Section edits target the section's first body block,
  which must be a paragraph. Read direct on/off properties from `flags`.
  A `null` value clears a flag or break type. `inlineEdits` accepts
  `insertAfter`, `replace`, `setBold`, `setToggle` and `patchFormat` at eligible direct inline anchors.
  `setToggle` takes `property` (`bold`, `italic`, `strike`, `hidden`) and `value`
  (`true`, `false`, or `null` to clear). Structured run inspection exposes the
  four direct values, a `format` view and `hasOpaqueFormatting`. Use `patchFormat`
  with a `RunFormat` value under `format` and optional field names under `clear`.
  Multiple format edits can share a run when their properties are disjoint.
  `editRunFormat({inlineEdits, ownerPart?})` returns new bytes and accepts
  run anchors in body cells and hyperlink children. For header/footer and
  ordinary note/comment runs, name their inspected physical `ownerPart`.
- `blockEdits` also accepts `splitSection` (`target: section.anchor`, `offset`
  in original blocks) and `mergeSection` (`target: section.anchor` for a
  nonfinal section). Merging retains the following section's layout.
- `previewLayout([{target, field, value}])` patches direct paragraph or section
  layout. For example, `{target: doc.sections()[0].anchor, field: "marginLeft",
  value: 1440}` sets a one-inch margin even if that section starts with a table.
  Use `null` to clear a field (clear `"margins"` as a group). Reopen the preview
  and inspect `paragraph.layout` or `section.layout`. See `DirectParagraphLayout`,
  `DirectSectionLayout` and `LayoutEdit` in the package declarations.
- `editStoryReference` creates, links, or removes an explicit header/footer
  reference by section index, story kind, and variant. `editStoryBlocks`
  splices physical blocks in `shared` or `sectionLocal` mode at a story anchor.
- `tableStructure` accepts `insertRow`, `removeRow`, `insertCell`, `removeCell`,
  `insertColumn`, `removeColumn`, `replaceCell` and `layout`. Row/cell positions
  are physical; column positions count logical grid columns. Pass `{target,
  kind: "layout", value: "fixed" | "autofit" | null}` to set or clear direct
  layout. Inspect a table's `layout`, `gridWidths` (twips), and each physical
  cell's `gridSpan` and `verticalMerge`.
- `patchProperty` copies one direct field from a resolved table, or clears it
  with `source: null`. For example, `{target, kind:"patchProperty", row:0,
  cell:1, field:"margin.end", source:table}` sets only that side. Table, row
  and cell `properties` expose field-to-attribute maps; `gridWidths` exposes
  columns. `replaceNode` replaces the addressed table, row or cell from a
  resolved table; `removeTable` removes an eligible direct body table.
- `partTables(ownerPart?)` lists physical tables, including tables nested in
  ordinary cells. Omit the part name for the main body, or use a physical
  header/footer or note/comment part. Pass the returned anchor to
  `editPartTables({ownerPart, tableStructure})` for geometry, property or
  cell-content edits; it returns new DOCX bytes. Shared aliases see the same
  edit. Hyperlinks inserted in a story cell belong to that part.
- `styleEdits` accepts `op: "add"`, `"patch"`, `"field"`, `"region"`, `"replace"`,
  `"remove"` or `"retarget"`. Obtain existing targets with `styleAnchor(id)`.
  Direct field patches retain unspecified properties. Retargeting migrates only
  proven references to a same-kind style and must be the sole style operation
  in its batch. Ordinary replace/remove require an unused custom style.
- `resources` accepts `kind: "coreTitle"`, `"coreSubject"`, `"defaultAlignment"` or `"abstractStart"`
  with `value` (`null` clears). Abstract starts also require `abstractId` and
  `level`. Inspect direct values with `coreTitle()`, `coreSubject()`, `defaultAlignment()` and
  `abstractStarts()`.
- Inspect direct run, paragraph and document-wide settings with
  `doc.documentDefaults()`. For example,
  `doc.preview({resources: [{kind: "documentDefault", field: "bold", value: false}]})`
  turns default bold off. `null` clears the field, restoring absence. `DefaultValues`
  lists keys and their types; tab positions and indent lengths use twips.

- `metadataField(field)` reads a core or extended string, and
  `customProperty(name)` reads a named string or boolean. Missing properties
  return `undefined`. Pass `{kind: "metadata", field, value}` or
  `{kind: "customProperty", name, value}` in `resources`; `null` removes the property.

- Numbering resource edits include `addAbstract`, `replaceAbstract`, `abstractLevel`,
  `removeAbstract`, `addInstance`, `replaceInstance`, `relinkInstance`,
  `removeInstance`, `instanceStart`, `instanceLevel`, `paragraphNumbering` and
  `storyParagraphNumbering`.
  `numberingInstances()` lists concrete IDs, abstract links, start overrides and
  replacement-level indices. `paragraphNumbering(anchor)` returns a direct
  `{id, level}` or `undefined`. `storyParagraphAnchor({ownerPart, blockIndex,
  entryId?})` and `cellParagraphAnchor({target, rowIndex, cellIndex, blockIndex})`
  locate direct physical paragraphs for `storyParagraphNumbering(anchor)` and
  the corresponding edit. Clear an override or direct reference with `value: null`.
  Live references and opaque content can block removal.
  `numberingCatalog()` reads modeled abstract and concrete levels, their
  label and paragraph formats, and exact stored style links. It does not
  synthesize missing properties or flatten inherited levels.

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
