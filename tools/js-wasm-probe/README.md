# JavaScript WebAssembly feasibility probe

Unsupported experiment for issue #9, not a package, supported binding, or release.
`paragraph_docx(text)` resolves a single Rust authoring paragraph and returns a
Promise of DOCX `Uint8Array` bytes. Errors reject with a string. No filesystem
is used by the API. The Node smoke script optionally writes bytes only for
inspection. The async boundary allows Rust resolution to yield; do not replace
it with a blocking browser executor.

## Reproduce

Requires Rust 1.88+, the `wasm32-unknown-unknown` target, `wasm-pack`, and
Node. From the repository root:

```sh
wasm-pack build tools/js-wasm-probe --target nodejs --out-dir pkg-node
node tools/js-wasm-probe/node-smoke.cjs /tmp/opencode/rocx-wasm-probe.docx
python3 -c 'from docx import Document; d = Document("/tmp/opencode/rocx-wasm-probe.docx"); assert [p.text for p in d.paragraphs] == ["WASM probe — hello"]'
wasm-pack build tools/js-wasm-probe --target web --out-dir pkg-web
```

The Node check tests the ZIP signature, not DOCX correctness; python-docx
independently checks paragraph content. The web build alone is not a browser
runtime test. Generated `pkg-*`, Cargo targets, and lockfile are local probe
artifacts, not publication-ready products.

## Boundaries and next checks

Only a single plain paragraph is exposed. This is not the existing full Rust or
Python declarative API, editor, templates, streaming API, or a compatibility
promise. Outstanding issue criteria:

- Define the supported slice, runtime versions, setup and npm package with
  TypeScript types. Reuse Rust; do not add a native Node adapter without a
  demonstrated need.
- Offer the same byte-oriented `Uint8Array` API in Node and browsers without
  browser-side Node dependencies or polyfills. Demonstrate the same formatted
  text, table and image workflow with a local Node write and a real browser
  Blob/download.
- Test packed artifacts in fresh consumers in CI. Verify exact bytes across
  equivalent Rust, Python, Node and browser outputs, plus independent DOCX and
  package relationship validation. Do not infer editor or full feature parity.
- Test Unicode and error propagation. Assess large document memory/copy
  overhead, async producer interop, browser loading, CSP and bundlers.
  Decide npm naming, WASM asset distribution, version pinning, licenses and
  provenance.

Neither browser support nor hosted build/release coverage follows from this
prototype.

Probe-only dependencies: `wasm-bindgen` provides JS/Wasm ABI and typed-array
conversion; `wasm-bindgen-futures` enables its async Promise bridge. Handwritten
Wasm ABI and manual polling would avoid them but add memory/lifetime and
executor complexity. `wasm-pack` is a build tool, not a runtime dependency.

## Local observation

With Rust/Cargo 1.97.1, the `wasm32-unknown-unknown` target, and
`wasm-bindgen` CLI 0.2.130, the probe cross-compiled and generated Node
bindings. The first Node run trapped when Rust attempted to read the system
clock. With a fixed timestamp supplied through the existing authoring API,
Node 26.3.1 returned a `Uint8Array` and python-docx read back the expected
paragraph. Browser execution and cross-runtime byte parity remain unverified.
