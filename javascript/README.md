# JavaScript: write a DOCX in Node or the browser

Install `rocx-docx` from a **development tarball**, not the npm registry. Use the artifact for your source revision: `0.1.0` alone does not identify the API. Installation needs no Rust toolchain, runtime dependencies or install scripts.

## Build and install the tarball

Download and unpack a `rocx-docx-node-*` artifact from the [package workflow](https://github.com/ReinderVosDeWael/rocx/actions/workflows/javascript.yml). From your application, install the extracted `.tgz`:

```sh
npm install ./path/to/rocx-docx-0.1.0.tgz
```

If no artifact is available for your revision, build one from the repository root with Bash, `tar`, Cargo and Node:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130 --locked
npm pack ./javascript --pack-destination /tmp
```

## Quickstart

Save as `plain.mjs` and run `node plain.mjs`:

```javascript
import { writeFile } from "node:fs/promises";
import { declarative as d, generateDocx } from "rocx-docx";

const document = d.Document([d.Section([d.Paragraph("Hello, world!")])]);
await writeFile("plain.docx", await generateDocx(document));
```

In a browser, use the same declaration and download the bytes as a `Blob`.

## Supported runtimes

- **Node:** 22.x (22.12+) or 24.x, ESM (`.mjs` or `"type": "module"`). Use `rocx-docx/node` for filesystem helpers. CommonJS is unsupported.
- **Chromium + Vite 7:** add `optimizeDeps: { exclude: ["rocx-docx"] }` to `vite.config.mjs`. Serve the Wasm over HTTP(S) as `application/wasm`. CSP must allow the asset fetch (`connect-src`) and Wasm compilation (`'wasm-unsafe-eval'` in `script-src`). No Node polyfills. Other browsers, bundlers, `file://` and SSR are unverified.
- **TypeScript 5.9:** use `module: "NodeNext"` in Node or `moduleResolution: "bundler"` for browser clients.

## TypeScript declarations

Import types from `rocx-docx` ([index.d.ts](index.d.ts), [factory signatures](types/authoring.d.ts)) or `rocx-docx/node` ([file helpers](types/file-io.d.ts)). The [API reference](https://reindervosdewael.github.io/rocx/api/javascript/) covers both the `DocumentProgram` factories and the single-section `{ blocks }` form.

## Runnable report

The Node and browser examples share [one declaration](../examples/javascript/author.mjs) with formatted Unicode text, a nested table and an image. From the repository root:

```sh
mkdir /tmp/rocx-js-example
cp examples/javascript/{author,node,browser,vite.config}.mjs examples/javascript/index.html /tmp/rocx-js-example/
cp tests/fixtures/images/sample.png /tmp/rocx-js-example/
cd /tmp/rocx-js-example
npm init -y
npm install /tmp/rocx-docx-0.1.0.tgz
node node.mjs sample.png report.docx
```

For the browser, run `npm install --save-dev vite@7.3.8` and `npx vite`. Open the displayed URL, select `sample.png`, then **Download DOCX**. Image bytes stay local. For a production preview, run `npx vite build` then `npx vite preview`.

## Create and resolve declarations

```javascript
import { declarative as d, lazy, resolveDocument } from "rocx-docx";

const report = await resolveDocument(d.Document([d.Section([
  d.Paragraph.Text(lazy(() => "Quarterly summary")),
])], { timestamp: "2026-11-21T00:00:00Z" }));
console.log(report.view.sections[0].view.paragraphs[0].view.text);
const bytes = report.toDocx(); // synchronous; snapshot can be rendered again
```

- `lazy` is synchronous; `future` accepts Promise-producing functions. Explicit `Value` tokens are single-use. Share with `deferred` or `property`; inside producers, `await context.get(ref)`. Use `Paragraph.Text` or `.Runs` for produced paragraph content.
- Avoid cycles between independent root resolutions: they may never settle. `AbortSignal` cancels a waiter, not shared work or its external effects.
- Set the program timestamp for deterministic dates. A render override changes one output. Use schema-valid date-time text; offsets and fractions are retained. Legacy `{ blocks }` accepts only UTC ISO timestamps with seconds and optional milliseconds, and does not support producers.
- Supply well-formed Unicode and encoded image bytes, not pixels or URLs. `Uint8Array` views use only the selected bytes, including across realms. Do not mutate inputs during an operation.
- Async calls reject with `Error`; synchronous methods throw. `path`, `rule` and producer `cause` are optional. Wasm initialization can be retried. Rendering runs on the calling thread; images and output are copied in memory.

## Use an existing file

| Task | API |
| --- | --- |
| Render against an unsigned Transitional DOCX or DOTX | `report.toDocxWithTemplate({ bytes, edits })`; `compileTemplate` returns an inspectable package plan. |
| Inspect an unsigned DOCX | `await openDocx(bytes)`, then `body()`, `sections()`, `stories()` or `defaultHeaders()`. Unsupported children may be opaque. |
| Edit a copy | `imported.preview(batch)` stages body alignment, table insertion and default-header images together. Inspect `changes()`, `insertions()`, `headerChanges()`, then `toDocx()`. |
| Read/write in Node | `templateFile` reads on each render; `imageFile` captures bytes and basename at resolution. `saveDocx` encodes before replacing the destination. |

Use anchors from the same snapshot. Untouched parts retain their payload bytes; edited XML and ZIP bytes may change. See [preview examples and shared-header modes](https://reindervosdewael.github.io/rocx/api/javascript/#preview-anchored-edits).
