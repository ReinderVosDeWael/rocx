# Installed stdio acceptance

Run from the repository root with Rust 1.88 or stable, Python 3.12, and `uv`.
Use a fresh install prefix (or remove your own previous prefix first):

```sh
export ROCX_MCP_BINARY="$PWD/target/mcp-acceptance-install/bin/rocx-mcp"
export ROCX_MCP_ORACLE="$PWD/target/mcp-acceptance/debug/examples/mcp_byte_oracle"
export ROCX_SCHEMA_DIR="$PWD/target/mcp-acceptance-schemas"
cargo install --locked --path crates/rocx-mcp --root "$PWD/target/mcp-acceptance-install"
cargo build --locked -p rocx-mcp --examples --target-dir "$PWD/target/mcp-acceptance"
uv run --no-project --with-requirements tests/mcp/requirements.txt \
  python tools/validate_ooxml.py --download-schemas
uv run --no-project --with-requirements tests/mcp/requirements.txt \
  pytest -q -p no:cacheprovider tests/mcp
```

The Python MCP SDK (`mcp==1.27.0`) negotiates `2025-11-25` and supplies
transport and client logic. Each server process runs in a clean temporary
directory. The mixed fixtures are generated with `python-docx`; the Rust
oracle uses fixed fixture coordinates and calls the public core APIs directly.
The MCP batch instead uses only anchors returned by inspection. Checks compare
full saved packages, then independently inspect DOCX semantics, protected part
payloads, header relationships, and Transitional schema/OPC validity.
For newly authored documents, the oracle receives the observed clock timestamp
as a render input; neither ZIP is normalized or rewritten before comparison.
The domain suites locate their independent native oracle binaries beside
`ROCX_MCP_ORACLE`.

Entry-story coverage (`test_stdio.py`):

- `test_entry_discovery` validates the entries selector, page bounds, exclusive
  preview request forms, and all seven edit payloads, including explicit null.
- `test_entry_paging_native_inspection_and_identity` reads 108 entries from a
  package with relocated notes. It compares paged results with direct Rust
  inspection and verifies opaque blocks, reserved entries, lexical IDs, stable
  anchors, and cursor ownership.
- `test_entry_scoped_native_bytes_and_preservation` runs each of the seven edits,
  both set and clear for optional comment fields, and a metadata no-op. Each
  saved preview matches native Rust bytes; python-docx, ZIP payload comparison,
  and Transitional schema/OPC validation check the result separately.
- `test_entry_rejection_retry_and_foreign_handles` rejects invalid payloads,
  mixed scopes, live-note deletion, destructive edits, and wrong-owner handles
  before retrying with the original anchor.

Additional scoped and import coverage:

- `test_notes.py` compares all six note-reference edits with Rust, including
  first-part creation, relocated and reserved-only parts, and rejected
  ineligible or foreign anchors.
- `test_revisions.py` compares all six proposal/review methods with Rust and
  independently checks Word markup, paired move IDs, both review dispositions
  and read-only nested evidence.
- `test_layout.py` checks every field name in discovery and compares a mixed
  paragraph/section transaction and a no-op with Rust. It tests clear, set,
  conflict, invalid target and unchanged package parts separately.
- `test_limits.py` compares the exact and just-under archive budgets with
  Rust and verifies native empty-batch preview bytes and retention.
- `test_stories.py`, `test_story_images.py` and `test_entry_runs.py` cover
  physical run edits, aliases, logical reference lifecycle, and shared/local
  image previews for every header/footer slot. They verify relationships
  and untouched parts separately from native byte comparisons.
- `test_resources.py` covers explicit set/clear, mixed resource/body previews,
  independent named queries, and rejection/retry before any anchor is issued.
- `test_blocks.py` checks every native body block edit against Rust, original
  section ranges, mixed conflict rejection, and block change records.
  `test_stories.py` checks reusable rich paragraph payloads in body/story
  insertion, section-local splices, eligible removals, and hyperlink
  relationship ownership.
- `test_fields.py` and `test_body_inlines.py` compare original-coordinate
  inline/field transactions and reports. `test_run_format.py` checks
  owner-scoped formatting, aliases and all clear slots.
- `test_physical_tables.py` distinguishes ordinary body table anchors from
  physical-owner anchors and covers native geometry and cell edits.
  `test_rich_table.py` checks recursive cell declarations and independent
  readback of table, row and cell properties.
