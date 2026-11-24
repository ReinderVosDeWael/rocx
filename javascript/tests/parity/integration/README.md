# Installed-package parity gate

Run `bash javascript/tests/check-packed.sh` from the checkout with Node 22
or 24, the matching `wasm-bindgen` CLI, the project's `uv` test environment,
Playwright Chromium and the OOXML schema directory. CI runs both Node versions.
Set `ROCX_INSTALL_BROWSER=1` if the runner should install Chromium.

The gate writes a fresh tarball and creates isolated install directories.
No test imports the checkout's JavaScript adapter or a separately built Wasm.
The runtime fixtures import `rocx-docx` from that tarball. The browser runner
uses Chromium with the installed browser ES module entry and with a built
Vite bundle; Wasm is fetched from the same origin in both modes.

| Inventory | Installed evidence |
| --- | --- |
| B0, R01–R06 | Owner Node suites, B0 direct Chromium wait-graph/guard/waiter regression, strict declarations |
| A01–A10 | Owner layout/style/table cases; browser-safe fixtures shared between Node and both Chromium modes. Native focused layout, linked-style, replacement numbering and table oracles |
| A11–A12 | Owner Node runtime, including supported formats and path sources; focused image and drawing byte oracles in Node and both Chromium modes |
| A13–A15 | Owner Node runtime; focused native structured-content comparison in Node and both Chromium modes, plus catalog |
| Declarative factories (#29) | Typed/runtime namespace inventory, raw/factory composition, deferred and falsy values, strict invalid-input cases, and exact raw-program byte parity in all runtimes. Both the migrated workflow and original legacy input match Rust/Python exactly. The public Node plain quickstart matches both native quickstarts and passes independent python-docx readback |
| T01–T06 | Owner Node suite incl. Node-only file I/O; portable repeat render, validation, template plan and exact Rust bytes in both Chromium modes |
| E01–E03 | Owner Node and Chromium inspection suite, catalog readback and anchor identity checks |
| E04–E06 | Same typed editor fixtures and seven exact Rust outputs in Node and both Chromium modes, independent python-docx and package preservation checks |
| Six-artifact catalog | Each JS runtime compared against native Rust exact ZIP bytes. Rust/Python parity retained. Separate readback, schema, OPC and untouched-part assertions |

This is a runner inventory, not an independent review or a claim that every
field, variant, negative case or deferred path in those rows is covered.
See `docs/JAVASCRIPT-PARITY.md` for the governing inventory.

## Provenance evidence

Use disk-backed scratch with at least 2 GiB free per run. The runner defaults
to `/var/tmp`; set `TMPDIR` if another filesystem is appropriate. Keep the
checkout stable throughout the run. Do not use compiler wrappers, custom
Rust flags, or an external Cargo config for qualification.

`provenance.py` inventories files directly (including untracked inputs),
copies and verifies the frozen source for a clean `npm pack`, and checks
every corresponding-source archive file against that inventory. It then
replays the shipped archive offline, with a fresh target and at the same
absolute source path. The original build is retained as `packed-build/`;
the archive replay is `build/`. The runner requires exact raw Wasm,
published Wasm, and wasm-bindgen glue bytes. Both npm installs must match
**every** tarball file before and after the consumer tests. Checkout input,
Cargo registry source, and schema hashes must stay stable; build tool
versions and the locked npm consumer tool versions are recorded.

Retain the output directory and console log for each Node matrix entry.
The machine-readable `provenance.json` advances from `built-and-replayed`
to `qualified` only after the packed behavior gate passes. A terminal-only
reviewer can check retained evidence without npm, Cargo, or installing tools
(Python 3.11+ standard library only):

```sh
python3 javascript/tests/parity/integration/provenance.py verify /path/to/rocx-packed-js.XXXXXX --checkout "$PWD"
```

Omit `--checkout` to verify retained builds, tarball, installed files, and
recorded result artifacts without requiring the original checkout. The
manifest is a local evidence record, not a signed attestation; this reviewer
command checks bytes, not the underlying behavior or inventory coverage.
For a full replay use the qualification runner. It creates a new record
rather than overwriting an old one.
