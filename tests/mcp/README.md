# Installed stdio acceptance

Run from the repository root with Rust 1.88 or stable, Python 3.12, and `uv`.
Use a fresh install prefix (or remove your own previous prefix first):

```sh
export ROCX_MCP_BINARY="$PWD/target/mcp-acceptance-install/bin/rocx-mcp"
export ROCX_MCP_ORACLE="$PWD/target/mcp-acceptance/debug/examples/mcp_byte_oracle"
export ROCX_SCHEMA_DIR="$PWD/target/mcp-acceptance-schemas"
cargo install --locked --path crates/rocx-mcp --root "$PWD/target/mcp-acceptance-install"
cargo build --locked -p rocx-mcp --example mcp_byte_oracle --target-dir "$PWD/target/mcp-acceptance"
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
