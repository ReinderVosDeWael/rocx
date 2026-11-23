# JavaScript API reference

This reference describes APIs implemented in this worktree and available
in its unreleased development tarball. See
[installation and supported environments](../javascript.md). For full input
and result types, use the [declarations shipped in the package](../javascript.md#typescript-declarations).

## Generate and resolve

```typescript
generateDocx(declaration: DocumentDeclaration, options?: GenerateOptions): Promise<Uint8Array<ArrayBuffer>>
generateDocx(program: DocumentProgram, options?: RenderOptions): Promise<Uint8Array<ArrayBuffer>>
resolveDocument(program: DocumentProgram, options?: ResolveOptions): Promise<ResolvedDocument>
```

| API | Behavior |
| --- | --- |
| Legacy `generateDocx` | `{ blocks }` with text and byte images in paragraphs, or rectangular string tables; one default section. `GenerateOptions.timestamp` sets the snapshot timestamp. |
| Program `generateDocx` | `{ sections, ... }`, using `SectionDeclaration` objects and structured block/inline declarations. Resolves once, then renders. |
| `resolveDocument`, `resolveComponent` | Resolve a program or one typed component. An excluded standalone component returns `undefined`. Use `scope: "inline"` or `"block"` for standalone `contentControl` declarations. |
| `ResolvedDocument.toDocx(options?)` | Synchronously encode a fresh DOCX. Read the snapshot through `.view`; rendering with a timestamp override does not change the snapshot. |
| `ready`, `lazy`, `future`, `deferred`, `property`, `resolveValue` | Model literal, produced, and shared values at `V<T>` positions. `Value.map` is synchronous; use `future` to assimilate a Promise returned by a producer. `ResolveOptions.signal` cancels a waiter, not surviving shared work. |
| `inlineText`, `createResolvedInline`, `createResolvedBlockControl` | Compute the engine's display projection or construct reusable resolved content from ready input. Insert compatible block/inline handles with `{ kind: "resolved", value: handle }`. |
| `bookmarkStart`, `bookmarkEnd`, `validateAuthoringId`, `validateValue` | Create bookmark markers; check authoring IDs or a supported validation input. See the installed `AuthoringIdKind` and `ValidateValueInput` types for the accepted inputs. Contextual checks still run during resolution or rendering. |

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
import { compileTemplate, resolveDocument } from "rocx-docx";

// templateBytes comes from a file picker, HTTP request, or the caller's disk read.
const document = await resolveDocument({
  timestamp: "2026-11-21T00:00:00Z",
  sections: [{ kind: "section", children: [{ kind: "paragraph", text: "Summary" }] }],
});
const plan = await compileTemplate(document, {
  bytes: templateBytes,
  edits: { replacements: [["{{quarter}}", "Q4"]], scope: "body" },
});
const output = plan.toBytes();
```

Use unsigned Transitional DOCX or DOTX source packages. `RenderOptions`
supports `timestamp`, `profile`, and `readLimits`
(`maxParts`, `maxPartBytes`, `maxExpandedBytes`). `"strict"` is reserved
and rejected. Package guards can reject unsupported mandatory markup in
processed parts. Ordinary replacement cannot match through opaque
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
  cell paragraphs expose only flat inline items.
- `sections()` gives half-open body ranges and explicit section types,
  with raw type values where present.
- `defaultHeaders()` lazily inspects effective default headers, including
  inherited references. `header.inlineStructure(blockIndex)` returns a
  run/link projection for a paragraph, `[]` for an empty paragraph,
  or `undefined` for opaque or out-of-range blocks.
- `inlineAt(anchor)` looks up a run or hyperlink. Hyperlinks expose the
  source relationship ID or bookmark, not a resolved URL. Inline anchors
  are read-only: aliases of one physical header reuse the same inline
  addresses, but reopening identical bytes produces a distinct snapshot.

Opaque markers report names, not editable XML. The inspection view is not a
round-trip authoring declaration.

## Preview anchored edits

| Synchronous API | Behavior |
| --- | --- |
| `imported.preview(batch: EditBatch): EditPreview` | Stages optional `alignments`, `tableInsertions`, and `headerImages` in a single Rust transaction against the original snapshot. |
| `imported.previewHeaderImage(edit: HeaderImageEdit): HeaderImagePreview` | Stages one default-header image append. `change()` reports the effect. |
| `preview.toDocx(): Uint8Array` | Encodes a new copy. Repeatable; does not consume the preview or modify the imported document. |

Alignment edits take a `ParagraphAnchor` and `alignment`; table insertions
take a `ParagraphAnchor` and a `ResolvedTable` and insert immediately after
the original body paragraph. Header edits take a section-scoped
`HeaderAnchor` from `header.view.anchor`, a `ResolvedImage` for an inline
image, and `mode`. `"shared"` changes the physical header part for all its
consumers. `"sectionLocal"` forks a supported header for the selected
section, preserving other sections' effective headers. This is not
general story, table-cell, or inline editing. `TableAnchor` and `InlineAnchor`
remain read-only.

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
import { imageFile, resolveDocument, saveDocx, templateFile } from "rocx-docx/node";

const report = await resolveDocument({
  sections: [{
    kind: "section",
    children: [{
      kind: "paragraph",
      runs: [{ kind: "image", data: imageFile("logo.png"), altText: { description: "Logo" } }],
    }],
  }],
});
await saveDocx(report, "report.docx", { template: templateFile("layout.dotx") });
```

`imageFile` returns a single-use `Value<NamedImageData>`. Resolution reads
the file and passes `{ bytes, sourceName }` (the local basename, not the full
path) into Rust as `ImageData::NamedBytes { bytes, source_name }`.
`ResolvedImage.view.name` preserves that name; format detection still uses
the bytes. Use `Uint8Array` for an unnamed source. The new Rust enum variant
is additive for constructors, but downstream exhaustive matches on the
public `ImageData` enum must add an arm or a wildcard. Resolved images do
not reopen the source when rendered.

`saveDocx` encodes fully before writing and renaming a sibling temporary
file onto the destination. In the browser, pass bytes from the caller and
download the output with a `Blob`; the Node helpers are not available there.

## Legacy report declaration

This is the same declaration used by the Node and Chromium examples:

```javascript
{{snippet:examples/javascript/author.mjs:declaration}}
```

See the [runnable Node and browser examples](../javascript.md#runnable-report)
and the [complete TypeScript declarations](../javascript.md#typescript-declarations).
