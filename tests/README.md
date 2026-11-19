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
| `resolution_contract.rs` + `test_resolution_contract.py` | Matched R1 image-field, shared-lifetime, concurrent/nested pass and asset-snapshot scenarios; see `docs/DEFERRED-FIELDS.md` |
| `r1.rs` + `test_r1.py` | Deferred aggregate/metadata references, resolved-child reuse, shared pure validators, structured errors and explicit render policy |
| `hyperlinks.rs` + `test_hyperlinks.py` | Rich external links in each story/table context, deferred/reference fields, nesting/URI rejection, captured images, comments and template boundaries; independent Rust-to-Python readback with a target mutation control |
| `ranges.rs` + `test_ranges.py` | Story identities, overlapping/cross-paragraph bookmark/comment anchors, internal target validation, imported collisions/IDs and template marker positions; independent readback rejects a mutated endpoint |
| `notes.rs` + `test_notes.py` | Ordinary footnotes/endnotes, special-entry roles, structured comments, deferred fields, relocated owners, imported ID graphs, captured assets; independent OPC/XML readback and semantic mutations |
| `fields.rs` + `test_fields.py` | Both field encodings, cached nested results, flags/whitespace, typed targets, deferred producers, owned resources and multi-paragraph imported-field edit protection; independent readback and strict balance mutations |
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

Set `RV_DOCX_REQUIRE_SCHEMAS=1` to fail instead of skipping when schemas are
missing. `.github/workflows/ci.yml` uses this mode in its standards job, downloads
digest-checked schemas on a cache miss, and validates the Rust-generated
`feature_parity` document as well as Python output and negative controls.
After setup, validation remains offline. The intentional upstream skip is the
retired CMI trailing-section expectation (`upstream/conftest.py`); no comparison
or fixture normalization policy was changed for CI.

## Wheel verification

```sh
uv run --no-sync python tools/check_wheel.py
```

This builds with `uv build --wheel`, installs only that wheel into a temporary
uv environment, and runs an isolated probe outside the source tree. It checks
native import locations, dependency metadata, exact Unicode and bold/italic XML,
bytes/save, repeatable snapshots without rerunning producers, and the missing
interop error. Neither `docx` nor `cmi_docx` may be installed. Test editable
consumer objects separately with `interop` installed. Temporary environments and
output are cleaned up; set `TMPDIR` to choose their parent directory.

CI runs Rust 1.88/stable and Python 3.12–3.14 checks without publishing artifacts
or packages. Dependency alert dispositions are in
[`docs/DEPENDENCY-AUDIT.md`](../docs/DEPENDENCY-AUDIT.md).
