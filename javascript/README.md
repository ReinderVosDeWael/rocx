# rocx-docx: JavaScript DOCX authoring

`rocx-docx` is an ESM package that uses the Rust rocx engine via WebAssembly.
It exports one function with the same TypeScript declaration in Node and
the browser:

```ts
generateDocx(declaration: DocumentDeclaration, options?: GenerateOptions): Promise<Uint8Array<ArrayBuffer>>
```

The package is not on the npm registry. Install the packed `.tgz`; installation
does not require Rust, a network connection, postinstall scripts, or runtime
dependencies.

## TypeScript declarations

The package ships [`index.d.ts`](index.d.ts), including `DocumentDeclaration`,
its block and run types, `GenerateOptions`, and `GenerateDocxError` for the
optional declaration path and engine diagnostic code on errors.

## Build and install the tarball

Download and unpack a `rocx-docx-node-*` artifact from a successful
[Packed JavaScript consumer workflow run](https://github.com/ReinderVosDeWael/rocx/actions/workflows/javascript.yml),
or build the same tarball from a checkout with Bash, `tar`, Cargo, and Node:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130 --locked
npm pack ./javascript --pack-destination /tmp
```

In your ESM or TypeScript project:

```sh
npm install /tmp/rocx-docx-0.1.0.tgz
```

The tarball includes the generated Wasm and JS glue, declarations, the LGPL
license, and `rocx-source.tar.gz` with the corresponding rocx source, lockfiles,
and build scripts. To rebuild or modify the Wasm, extract that source archive
and run the same commands there.

## Supported runtimes

| Runtime | Setup |
| --- | --- |
| Node.js 22.12+ (22.x), or 24+ | Use `import { generateDocx } from "rocx-docx"` in `.mjs`, or set `"type": "module"`. Node loads the installed Wasm from disk. CommonJS `require` is not a supported entry point. |
| Chrome/Chromium with Vite 7 | Use the same ESM import in client code. Vite serves the Wasm as an asset. Include `optimizeDeps: { exclude: ["rocx-docx"] }` in `vite.config.mjs` to keep the generated glue next to the Wasm in development. Serve the production output over HTTP(S). |
| TypeScript 5.9 | Use `module: "NodeNext"` for Node, or `moduleResolution: "bundler"` for a browser client. The declarations ship with the package. |

No Node polyfills are needed in the browser. The client entry uses `fetch`
and WebAssembly; serve the `.wasm` asset alongside the app with
`Content-Type: application/wasm`. A restrictive CSP must permit fetching the
Wasm asset (`connect-src`) and compiling Wasm (`'wasm-unsafe-eval'` in
`script-src` on Chromium). `file://`, other browsers and bundlers, and SSR
integration are not part of this verified slice.

## Runnable report

Both examples use [the same declaration](../examples/javascript/author.mjs):
a bold/italic Unicode paragraph, a two-column table, and an inline picture
with alternative text. Copy the example files into a fresh consumer and
install the tarball:

```sh
mkdir /tmp/rocx-js-example
cp examples/javascript/* /tmp/rocx-js-example/
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

## Authoring scope

- One section with default layout; paragraphs hold text and inline image runs.
  Text accepts optional `bold` and `italic`; `false` is an explicit override.
- Tables accept a nonempty rectangular grid of plain-text cells.
- Image runs accept already-encoded `Uint8Array` bytes, not a file path, URL, or
  decoded pixels. PNG is exercised in the cross-runtime check; image format
  detection uses the same PNG, JPEG, GIF, BMP, and TIFF reader as native rocx.
  Images use their intrinsic dimensions. The optional `altText` becomes the
  picture description. Views, including views created in another JavaScript
  realm, use only their selected bytes.
- `generateDocx` rejects with an `Error` for invalid declarations or a rocx
  rendering error. Text, table cells, and image descriptions must contain
  well-formed Unicode (no unpaired UTF-16 surrogates). Await file reads/fetches
  before calling it; declarations do not accept callbacks or promises. Do not
  mutate them until the call settles.
- The optional `timestamp` is a UTC ISO string with second precision or three
  fractional digits (for example `2026-11-21T00:00:00Z`). It defaults to the
  host clock. Specify it for repeatable output.

### Rust/Python features not exposed

The [Rust/Python catalog](https://github.com/ReinderVosDeWael/rocx/blob/main/examples/ALL_FEATURES.md)
tracks the broader engine surface. The JavaScript binding does not expose
custom styles, lists, links, fields, notes, rich table cells, image transforms,
shapes or charts. It also omits multiple sections, headers and footers,
document metadata beyond the timestamp, deferred producers, templates, and
editing existing DOCX files. The quickstart is not a promise of further parity.

The Promise supports loading Wasm, but does not move rendering off the calling
thread. If Wasm fails to load, a later call can retry initialization. Images
and the finished document cross the JS/Wasm boundary as
in-memory copies; large-document throughput and memory use are not qualified.

## Verification

The non-publishing GitHub Actions workflow packs the package, installs it in
fresh Node and Vite projects, and type-checks both module resolutions. The
packed input contract requires sample values for every declared input field
and variant, including optional fields, and checks them against the running
adapter. The workflow downloads DOCX files in real Chromium from Vite
development and production servers. The five outputs (Rust, Python, Node, and
the two browser runs) must match exactly as ZIP bytes, open with python-docx,
and pass ECMA-376 schema and OPC relationship checks.

To run locally after building the locked Python environment and installing
the pinned tools above:

```sh
uv sync --locked --group test
uv run --no-sync python tools/validate_ooxml.py --download-schemas
ROCX_INSTALL_BROWSER=1 bash javascript/tests/check-packed.sh
```

Set `ROCX_INSTALL_BROWSER=0` (the default) if the pinned Playwright Chromium
and system libraries are already available.
