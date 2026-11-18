# Verification

Use uv for Python; there is no requirements.txt workflow. Building the optional
native extension requires a Rust toolchain and Python development libraries.

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

## Contracts

| Tests | Evidence |
| --- | --- |
| `domain.rs` | Physical units and standalone compilation of the domain with only std |
| `resolution.rs`, `deferred.rs`, `async_fields.rs` | Ordering, concurrency, conditions, typed property cycles, cancellation and error locations |
| `docx.rs`, `text_runs.rs`, `features.rs`, `compilation.rs` | Package/content semantics, explicit formatting and deterministic compilation plans |
| `preservation.rs`, `templates.rs` | Relocated parts/media, namespace shadowing, opaque content, replacements and structured edit boundaries |
| `python_docx.rs` + `python_docx/verify.py` | Rust writes files; python-docx checks exact expected content, formatting, images and relationships |
| Python binding tests | Native conversion, immutable snapshots, dependency behavior, output and selected independent comparisons |
| `test_standards.py` + `tools/validate_ooxml.py` | Independent official schema validation, invalid-order and missing-target controls |

Historical fixture comparisons normalize selected irrelevant differences, and one
known CMI omission (false small caps) is excluded from that legacy comparison.
Explicit-false behavior has a dedicated native assertion. Normalized comparisons
do not establish schema order; independent XSD mutation tests check that rule.
Fixture provenance remains in `fixtures/README.md` and upstream `ORIGIN.json`.
Test builders live in `tests/support`, independent of example implementations.

## Independent official-schema checks

Download the digest-pinned official ECMA-376 Part 4 (2016) Transitional schemas
and the W3C XML attribute schema once:

```sh
uv run --locked --group test python tools/validate_ooxml.py --download-schemas
```

The tool caches schemas under `/tmp/rv-docx-validation-schemas` by default.
`RV_DOCX_SCHEMA_DIR` or `--schema-dir` selects another directory. Network access
is only needed for the explicit download; validation is local and does not fetch
external document references.

```sh
RV_DOCX_SCHEMA_DIR=/tmp/rv-docx-validation-schemas uv run --locked --group test pytest
cargo run --example feature_parity -- /tmp/features.docx
uv run --locked --group test python tools/validate_ooxml.py /tmp/features.docx
```

The source URLs/digests are recorded in the tool and downloaded `SOURCE.json`.
The XML namespace import is completed locally using W3C's xml.xsd; OOXML content
models are not changed. The validator checks WordprocessingML/DrawingML schemas
after supported MCE preprocessing, internal OPC targets and relationship uses.
It is independent of the native decoder/serializer. It is not full extension or
Word-layout validation. Tests skip with an explicit explanation if schemas have
not been downloaded; a release verification run must provide them.

## Wheel verification

Build with `uv build --wheel`. Install it with `uv pip install --python <clean-env>
<dist-wheel>` into a fresh uv-created environment. Verify import and native bytes/
save with no `cmi_docx` or `docx` installed, inspect content with standard ZIP/XML
tools, and confirm `to_docx()` explains the optional extra. Test editable consumer
objects separately with `interop` installed. Artifacts belong outside the Git tree.
