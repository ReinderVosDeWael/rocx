# rocx-docx: JavaScript DOCX authoring, templates, and editing

`rocx-docx` is an ESM package that uses the Rust rocx engine via WebAssembly.
Use it to author a new DOCX, compose with a DOCX or DOTX template, or
inspect and preview supported edits to an existing DOCX. The APIs below
are implemented in this worktree and available in its local development
tarball; they have not been released. Older `0.1.0` artifacts can differ.
Start new documents with named factories from `declarative`. Existing
`generateDocx({ blocks }, { timestamp })` calls still work.

The package is not on the npm registry. Install the packed `.tgz`; installation
does not require Rust, a network connection, postinstall scripts, or runtime
dependencies.

## TypeScript declarations

The installed package exports [`index.d.ts`](index.d.ts) and its
[`types/`](types/) modules from `rocx-docx`. Import declaration types from
that entry point, not from internal paths. See the
[factory signatures](types/authoring.d.ts) for positional inputs and options.
Node filesystem helpers have
[separate declarations](types/file-io.d.ts) at `rocx-docx/node`. See the
[JavaScript API reference](https://reindervosdewael.github.io/rocx/api/javascript/)
for an overview.

## Build and install the tarball

Build the tarball from the checkout you want to use with Bash, `tar`, Cargo,
and Node. Alternatively, download and unpack a `rocx-docx-node-*` artifact
for the matching revision from a successful
[Packed JavaScript consumer workflow run](https://github.com/ReinderVosDeWael/rocx/actions/workflows/javascript.yml),
if available. The filename alone does not identify the API revision.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130 --locked
npm pack ./javascript --pack-destination /tmp
```

In your ESM or TypeScript project:

```sh
npm install --ignore-scripts --offline /tmp/rocx-docx-0.1.0.tgz
```

The tarball includes the generated Wasm and JS glue, declarations, the LGPL
license, and `rocx-source.tar.gz` with the corresponding rocx source, lockfiles,
and build scripts. To rebuild or modify the Wasm, extract that source archive
and run the same commands there.

## Supported runtimes

| Runtime | Setup |
| --- | --- |
| Node.js 22.12+ (22.x) and 24.x | Use ESM in `.mjs`, or set `"type": "module"`. Node loads the installed Wasm from disk. Use `rocx-docx/node` for local file helpers. CommonJS `require` is not a supported entry point. Later Node majors allowed by the manifest are not qualified here. |
| Chrome/Chromium with Vite 7 | Use the same ESM import in client code. Vite serves the Wasm as an asset. Include `optimizeDeps: { exclude: ["rocx-docx"] }` in `vite.config.mjs` to keep the generated glue next to the Wasm in development. Serve the production output over HTTP(S). |
| TypeScript 5.9 | Use `module: "NodeNext"` for Node, or `moduleResolution: "bundler"` for a browser client. The declarations ship with the package. |

No Node polyfills are needed in the browser. The client entry uses `fetch`
and WebAssembly; serve the `.wasm` asset alongside the app with
`Content-Type: application/wasm`. A restrictive CSP must permit fetching the
Wasm asset (`connect-src`) and compiling Wasm (`'wasm-unsafe-eval'` in
`script-src` on Chromium). `file://`, other browsers and bundlers, and SSR
integration are not part of this verified slice.

## Quickstart

Save this as `plain.mjs` in a Node project with the tarball installed, then
run `node plain.mjs`. The same declaration works in a browser; only file
output is Node-specific.

```javascript
import { writeFile } from "node:fs/promises";
import { declarative as d, generateDocx } from "rocx-docx";

const document = d.Document([d.Section([
  d.Paragraph("Hello, world!"),
  d.Paragraph("A second paragraph, with Unicode: café 🌍"),
])], { timestamp: "2026-11-21T00:00:00Z" });

await writeFile("plain.docx", await generateDocx(document));
```

See the runnable [JavaScript](../examples/javascript/plain.mjs),
[Rust](../examples/plain.rs), and [Python](../examples/plain.py) versions.

## Runnable report

Both examples use [the same named declaration](../examples/javascript/author.mjs):
a centered bold/italic Unicode paragraph, a two-column table with a rich cell
and nested table, and a sized inline picture with alternative text.
Copy the example files into a fresh consumer and
install the tarball:

```sh
mkdir /tmp/rocx-js-example
cp examples/javascript/{author,node,browser,vite.config}.mjs examples/javascript/index.html /tmp/rocx-js-example/
cp tests/fixtures/images/sample.png /tmp/rocx-js-example/
cd /tmp/rocx-js-example
npm init -y
npm install /tmp/rocx-docx-0.1.0.tgz
```

- **Node:** Run `node node.mjs sample.png report.docx`. The example reads the
  image with `node:fs` and writes the returned bytes to a local file.
- **Browser:** Run `npm install --save-dev vite@7.3.8` and `npx vite`. Open the
  displayed URL, choose `sample.png`, and click **Download DOCX**. The example
  obtains bytes with `File.arrayBuffer()` and saves them via a `Blob` download.
  It does not upload the image. Run `npx vite build` and `npx vite preview` to
  try the production bundle.

The [legacy report](../examples/javascript/legacy-author.mjs) is also
runnable: copy `legacy-author.mjs` and `legacy-node.mjs` to the consumer,
then run `node legacy-node.mjs sample.png legacy-report.docx`.

## Create and resolve declarations

The named factories return the same `DocumentProgram` and component records
that `generateDocx` and `resolveDocument` already accept. Use them together
with existing camelCase declarations: `format` holds run or paragraph
formatting, and `properties` holds section and table settings. Use
`generateDocx(program, options)` to render once or `resolveDocument(program)`
to inspect and render a snapshot more than once.
Use `d.Paragraph.Text(value)` or `d.Paragraph.Runs(value)` to specify
which input to resolve when passing a producer for the whole paragraph.

Set the program's `timestamp` for deterministic snapshot dates. A
`RenderOptions.timestamp` override changes just that output. Like Rust, both
preserve the supplied text without parsing or normalizing dates. Supply a
schema-valid date-time for DOCX core properties; fractional seconds and numeric
offsets are retained, e.g. `2026-11-21T00:00:00.1+00:00`.

```javascript
import { declarative as d, lazy, resolveDocument } from "rocx-docx";

const report = await resolveDocument(d.Document([d.Section([
  d.Paragraph([
    d.TextRun(lazy(() => "Quarterly summary"), { format: { bold: true } }),
  ]),
])], { timestamp: "2026-11-21T00:00:00Z" }));
console.log(report.view.sections[0].view.paragraphs[0].view.text);
const bytes = report.toDocx(); // Uint8Array, not a Promise
```

`lazy` takes a synchronous producer; `future` takes a producer that may
return a Promise. Explicit `Value` tokens are single-use. Use `deferred`
or `property` to share a result. Inside a producer, await dependencies with
`context.get(ref)` rather than starting another root resolution. On the
current Rust baseline, a cycle split between separate concurrent root
passes can leave neither Promise settled even though a cycle in one pass
can be diagnosed. Avoid cyclic dependency graphs across passes. An
`AbortSignal` can cancel a waiter but does not forcibly stop shared work or
undo external effects.

The legacy `DocumentDeclaration` takes `{ blocks }` for one default section
and does not support producers. Use a UTC ISO timestamp with seconds and
optional milliseconds for its `GenerateOptions.timestamp`. See the
[legacy field reference](https://reindervosdewael.github.io/rocx/api/javascript/#legacy-declaration-fields).

The adapter requires well-formed Unicode (no unpaired UTF-16 surrogates).
Pass encoded images, not pixels or URLs; a `Uint8Array` view uses only its
selected bytes, even across JavaScript realms. Fetch or read browser data
yourself. Do not mutate input records or bytes while an operation is in
progress. Async operations reject with an `Error`; synchronous handle
methods throw. Diagnostic `path`, `rule`, and producer `cause` are optional.

## Use an existing file

- **Compose:** `report.toDocxWithTemplate({ bytes, edits })` takes unsigned
  Transitional DOCX or DOTX bytes and emits DOCX. `compileTemplate` takes
  the same byte source and returns a `PackagePlan` for inspecting payloads
  before encoding. Edits apply to supported source stories before new
  declarations are inserted. This is not arbitrary OOXML editing; affected
  parts can be reserialized, while untouched part payloads stay unchanged.
- **Inspect:** `await openDocx(bytes)` opens an unsigned Transitional DOCX.
  Use `body()`, `sections()`, and `stories()` for typed readback, including
  direct table-cell paragraphs and all effective header/footer slots.
  `defaultHeaders()` also supplies section-scoped header edit targets.
  Unsupported children can appear as opaque markers, not editable nodes.
- **Preview edits:** `imported.preview(batch)` stages direct body paragraph
  alignment, table insertion after a body paragraph, and default-header
  image appends in one transaction. Take targets from that imported
  snapshot. Review `changes()`, `insertions()`, and `headerChanges()`,
  then call `toDocx()` for a new copy. To preview just one header-image edit,
  use `imported.previewHeaderImage(edit)`. See the
  [API reference](https://reindervosdewael.github.io/rocx/api/javascript/#preview-anchored-edits)
  for the supported targets and modes.
- **Read and save in Node:** `templateFile(path, edits)` reads on each render
  or save; `imageFile(path)` reads once when its `Value` is resolved. It
  carries the basename with the encoded bytes as `NamedImageData`, so the
  resolved image's name matches a native path source. `saveDocx` encodes
  before replacing the destination via a sibling temporary file.

The Promise supports loading Wasm, but does not move rendering off the calling
thread. If Wasm fails to load, a later call can retry initialization. Images
and the finished document cross the JS/Wasm boundary as
in-memory copies; large-document throughput and memory use are not qualified.

## Verification

The non-publishing packed consumer workflow rebuilds the bundled source
offline to verify the Wasm and glue, then installs the tarball into fresh
Node and Vite projects. It checks NodeNext and bundler type resolution and
compares the installed authoring fields and variants against Rustdoc JSON.
The inventory is a declaration check, not a runtime parity test. The workflow
also validates the legacy declaration fields and variants at runtime. The five
standalone report outputs (Rust, Python, Node, and Chromium via Vite
development and production) must match as complete ZIP bytes, open with
python-docx, and pass ECMA-376 schema and OPC relationship checks.

Both Chromium/Vite modes run the named report download and a portable suite with
all six [catalog](../examples/javascript/catalog.mjs) outputs, twelve focused
authoring/template outputs and seven editor outputs. These match the
equivalent Rust bytes and are checked separately for readback and untouched
payloads where applicable. Direct Chromium tests also exercise the browser
entry's resolution, inspection, timestamp text, and read budgets.
The complete Node contract tests and filesystem cases do not run in Chromium.
See the [parity inventory](../docs/JAVASCRIPT-PARITY.md) for evidence and limits.

To run locally after building the locked Python environment and installing
the pinned tools above:

```sh
uv sync --locked --group test
uv run --no-sync python tools/validate_ooxml.py --download-schemas
ROCX_INSTALL_BROWSER=1 bash javascript/tests/check-packed.sh
```

Set `ROCX_INSTALL_BROWSER=0` (the default) if the pinned Playwright Chromium
and system libraries are already available.
