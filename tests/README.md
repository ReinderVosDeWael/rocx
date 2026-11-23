# Verification

Use uv for Python; there is no requirements.txt workflow. Building the optional
native extension requires a Rust toolchain and Python development libraries.
The `test` dependency group includes `lxml` for XML and schema checks and
`python-docx` for independent readback. No interop extra is needed for testing.

```sh
uv sync --locked --group test
cargo test --all-features --all-targets
cargo test --all-features --doc
cargo clippy --all-features --all-targets -- -D warnings
uv run --locked --group test pytest
```

On a relocated Python installation whose sysconfig reports a nonexistent library
directory, point `LIBRARY_PATH` and `LD_LIBRARY_PATH` at the installed libpython
for Cargo embedding tests. Wheel builds use maturin's extension-module linkage.
This is environment configuration, not a library dependency.

## Verification index

- **Contracts and compilation:** Rust domain, resolution, deferred-value,
  formatting, layout, style, numbering, template and package tests cover input
  validation, output semantics, preservation boundaries and deterministic plans.
- **Cross-language behavior:** Paired Rust/Python tests exercise bindings,
  independent readback, deferred values and negative controls.
- **Packed JavaScript consumer:** The [JavaScript verification guide](../javascript/README.md#verification)
  covers fresh Node and Chromium installs, TypeScript declarations, and exact
  DOCX byte parity with Rust and Python. This workflow does not publish to npm.
- **Standards and package structure:** `test_standards.py` and
  `tools/validate_ooxml.py` validate official schemas, OPC relationships and
  malformed-order/missing-target controls. See
  [Independent official-schema checks](#independent-official-schema-checks).
- **Catalogs:** Rust and Python author the visual examples independently; tests
  check expected content, API inventory, schema coverage and complete-container
  byte parity. See [Visual catalog parity](#visual-catalog-parity) and the
  [catalog inventory](../examples/ALL_FEATURES.md).

Content and serialization tests assert the documented native contract directly.
Feature-specific test builders live in `tests/support`, independent of example
implementations. The catalog tests intentionally run the actual example programs
so the documents used for visual inspection cannot drift from their checks.

## Visual catalog parity

See [the catalog inventory](../examples/ALL_FEATURES.md) for the canonical
exact-DOCX-byte parity command and its proof: the complete containers match
byte-for-byte (not just normalized XML). Fixed timestamps, fixture bytes and
equivalent declaration order make parity reproducible. The full Python suite
runs this check in a temporary directory; Rust's suite independently runs the
Rust generator. The catalog also has required-schema coverage, including
negative border/conditional-order mutations.

## Independent official-schema checks

Download the digest-pinned official ECMA-376 Part 4 (2016) Transitional schemas
and the W3C XML attribute schema once:

```sh
uv run --locked --group test python tools/validate_ooxml.py --download-schemas
```

The tool caches schemas under `/tmp/rocx-validation-schemas` by default.
`ROCX_SCHEMA_DIR` or `--schema-dir` selects another directory. Network access
is only needed for the explicit download; validation is local and does not fetch
external document references.

```sh
ROCX_SCHEMA_DIR=/tmp/rocx-validation-schemas ROCX_REQUIRE_SCHEMAS=1 \
  uv run --locked --group test pytest
cargo run --example feature_parity -- /tmp/features.docx
uv run --locked --group test python tools/validate_ooxml.py /tmp/features.docx
```

The source URLs/digests are recorded in the tool and downloaded `SOURCE.json`.
The XML namespace import is completed locally using W3C's xml.xsd; OOXML content
models are not changed. The validator checks WordprocessingML/DrawingML schemas
after supported MCE preprocessing, internal OPC targets and relationship uses.
For standard DrawingML charts, it checks the chart part's MIME type and
`c:chartSpace` root, validates its XML against the pinned chart schema, and
checks that each chart drawing in a validated part has an internal chart
relationship from its owner.
For custom document properties, it checks the root, schema, unique property
IDs, content type and the single internal package-level relationship, including
for parts at nonstandard paths. Extended document properties get the same
package and schema checks, without the custom-property ID rules.
It is independent of the native decoder/serializer. It is not full extension or
Word-layout validation. Tests skip with an explicit explanation if schemas have
not been downloaded; a release verification run must provide them.

`ROCX_REQUIRE_SCHEMAS=1` makes missing schemas an error instead of a skip.
The CI standards job uses this mode and validates Rust and Python output,
including negative controls.

## Wheel verification

```sh
uv run --no-sync python tools/check_wheel.py
```

This builds with `uv build --wheel`, installs only that wheel into a temporary
uv environment, and runs an isolated probe outside the source tree. It checks
native import locations, dependency metadata, exact Unicode and bold/italic XML,
bytes/save, repeatable snapshots without rerunning producers, and the missing
interop error. The optional `docx` package is not installed in this probe. Test editable
consumer objects separately with `interop` installed. Temporary environments and
output are cleaned up; set `TMPDIR` to choose their parent directory.

CI runs Rust 1.88/stable and Python 3.12–3.14 checks without publishing artifacts
or packages. [`.github/workflows/publish.yml`](../.github/workflows/publish.yml)
is the separate tag-gated release path: it reruns verification with required
schemas, builds wheels/sdist, and publishes only from `v*` tags. See
[releasing](../docs/RELEASING.md) for the one-time trusted-publisher setup and
release checklist. Dependency alert dispositions are in
[`docs/DEPENDENCY-AUDIT.md`](../docs/DEPENDENCY-AUDIT.md).
