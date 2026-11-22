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
| `line_height.rs` + `test_line_height.py` | Multiple/exact/minimum spacing on paragraphs/styles, units/rounding, legacy conflicts, deferred values and invalid measurements; schema and independent consumer negative controls |
| `tab_stops.rs` + `test_tab_stops.py` | Signed/sorted custom tab stops, alignments/leaders/clears, rounded collisions, deferred fields and style-update preservation; schema and independent readback mutations |
| `row_properties.rs` + `test_row_properties.py` | Exact/minimum row heights, optional pagination flags, deferred/hidden properties and concurrent row resolution; schema and independent readback mutations |
| `cell_layout.rs` + `test_cell_layout.py` | Cell margin units/zero/omission, checked text direction, deferred properties and schema/readback mutations |
| `script_formatting.rs` + `test_script_formatting.py` | Language/font slots, independent complex-script formatting, deferred values, template-style preservation and schema/readback mutations |
| `style_references.rs` + `test_style_references.py` | Forward references, self-next links, kind/cycle checks and imported graph boundaries; retargeted-link readback controls |
| `character_styles.rs` + `test_character_styles.py` | Character resources, forward bases, run reference kinds, direct overrides, deferred style fields and native/schema/readback negative controls |
| `numbering_overrides.rs` + `test_numbering_overrides.py` | Concrete start overrides, abstract-start preservation, checked levels, zero/deferred starts and schema/readback controls |
| `numbering_replacements.rs` + `test_numbering_replacements.py` | Concrete level replacements, mixed override union/order, unchanged abstracts, checked/deferred targets, schema/readback controls and catalog parity |
| `table_look.rs` + `test_table_look.py` | Conditional-region activation, all 64 flag combinations/masks, default byte compatibility, deferred/template boundaries and schema/readback mutations |
| `table_alignment.rs` + `test_table_alignment.py` | Five inline alignment tokens/omission, widths and paragraph alignment unchanged, source getters, deferred/hidden/nested/template boundaries, schema/readback/catalog mutations |
| `legal_numbering.rs` + `test_legal_numbering.py` | Omitted/false/true legal display requests, shared abstract/replacement codec, nested deferred values, unchanged formats, schema and catalog mutation checks |
| `numbering_suffix.rs` + `test_numbering_suffix.py` | Omitted/tab/space/nothing separators, shared level codec, deferred inputs, invalid tokens, independent loaded readback/schema/catalog mutation controls |
| `numbering_restart.rs` + `test_numbering_restart.py` | Omitted/zero/one-based restart requests, ignored values and bounds, separate start overrides, deferred fields, independent readback/schema/catalog mutation controls |
| `numbering_alignment.rs` + `test_numbering_alignment.py` | Five label-alignment tokens and omission in both level locations; paragraph alignment independence, deferred fields, schema/readback/catalog mutation controls |
| `section_direction.rs` + `test_section_direction.py` | Independent section/page direction, inherited/explicit/template values, deferred/hidden inputs and schema/readback mutations |
| `conditional_paragraph_formatting.rs` + `test_conditional_paragraph_formatting.py` | Table-style region line heights/tabs/bidi, checked/deferred values, unchanged direct paragraphs and schema/readback mutations |
| `conditional_script_formatting.rs` + `test_conditional_script_formatting.py` | Table-style run script requests, checked sizes, omission/zero/false, deferred values and schema/readback mutations |
| `document_defaults.rs` + `test_document_defaults.py` | Document defaults, template attribute merging, point units, deferred snapshots, style-reference rejection and schema/readback mutations |
| `linked_styles.rs` + `test_linked_styles.py` | Forward opposite-kind links, reciprocal/one-sided references, imported/update boundaries, direct kind checks and schema/readback controls |
| `section_gutter.rs` + `test_section_gutter.py` | Independent gutter placement, inherited/template/deferred values, unchanged geometry/width metrics and schema/readback mutations |
| `showcase.rs` + `test_all_features.py` | Independently authored visual catalogs, exact ZIP-byte equality, API inventory drift guard, expected-content assertions and container-only/missing-feature controls |
| `docx.rs`, `text_runs.rs`, `text_contracts.rs`, `features.rs`, `compilation.rs` | Package/content semantics, explicit formatting, paragraph and section structure, and deterministic compilation plans |
| `preservation.rs`, `templates.rs` | Relocated parts/media, namespace shadowing, opaque content, replacements and structured edit boundaries |
| `python_docx.rs` + `python_docx/verify.py` | Rust writes files; python-docx checks exact expected content, formatting, images and relationships |
| Python binding tests | Native conversion, immutable snapshots, dependency behavior, output and API contracts |
| `test_standards.py` + `tools/validate_ooxml.py` | Independent official schema validation, invalid-order and missing-target controls |

Content and serialization tests assert the documented native contract directly.
Feature-specific test builders live in `tests/support`, independent of example
implementations. The catalog tests intentionally run the actual example programs
so the documents used for visual inspection cannot drift from their checks.

## Visual catalog parity

`UV_PROJECT_ENVIRONMENT=.venv-opencode TMPDIR=/tmp/opencode uv run --no-sync python tools/check_examples.py`
generates both DOCX files under `examples/`, compares complete container bytes and
reports a SHA-256. It does not normalize XML or repack before comparing. Fixed
timestamps, fixture bytes and equivalent declaration order make parity
reproducible. The full Python suite runs this check in a temporary directory;
Rust's suite independently runs the Rust generator. The whole catalog also has
required-schema coverage, including negative border/conditional-order mutations.
See [the catalog inventory](../examples/ALL_FEATURES.md).

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
ROCX_SCHEMA_DIR=/tmp/rocx-validation-schemas uv run --locked --group test pytest
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

Set `ROCX_REQUIRE_SCHEMAS=1` to fail instead of skipping when schemas are
missing. `.github/workflows/ci.yml` uses this mode in its standards job, downloads
digest-checked schemas on a cache miss, and validates the Rust-generated
`feature_parity` document as well as Python output and negative controls.
After setup, validation remains offline.

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
## Rust authoring builders

`run_format_builders.rs` verifies every run setter, three-state bold, overwrite
and unrelated-field preservation, and exact DOCX byte equality with equivalent
literals using a fixed timestamp. `authoring_builders.rs` compares every other
public authoring value field to literal assignments, both separately and chained,
and checks collection replacement, existing validation and deferred component
inputs. The catalog inventory guard checks field/setter/test coverage; paired
catalog samples independently assert omitted/true/false bold, including an
explicit-false-removal negative control.
## Shape text styling

`shape_text_format.rs` checks DrawingML property placement, blank lines, explicit
false, font-size bounds and byte equality for omitted/empty styles. Python
`test_shape_text_format.py` independently checks readback, all supported alignment
tokens, font-size rounding/bounds, nested deferred input, snapshot repeatability
and invalid tokens. Both catalogs carry matching labeled samples. Their schema
checks reject invalid font sizes and paragraph-property order; an independent
catalog mutation rejects changed shape bold requests.
R8m extends those checks to omitted/false/true italic and Latin typeface requests,
including Unicode/empty strings, XML escaping/rejection, deferred font values,
styling on blank lines and schema child order. Catalog mutations reject altered
italic and font requests independently of bold.
R8n adds `test_text_boxes.py` and Rust acceptance for direct canvas text shapes,
independent transforms, plain text/style preservation, empty text, unique mixed
drawing IDs, and rejected geometric properties/unsupported stories. The shared
codec refactor preserved the previous geometric catalog bytes before the new
sample was added. Schema and content mutations reject missing transforms and
changed extents; final and intermediate template ZIP bytes are compared exactly.
