# Maintaining the Rust-to-MCP parity inventory

This inventory tracks [#72](https://github.com/ReinderVosDeWael/rocx/issues/72).
It answers two different questions: *what does the compiled Rust editor
expose?* and *which parts have an MCP equivalent with evidence?*

From the repository root with Python 3.12, `uv` and Rust 1.88.0 installed:

```sh
python3 tools/mcp_parity_surface.py --check
python3 tools/check_mcp_parity.py
uv run --no-project --with-requirements tests/mcp/requirements.txt pytest -q tests/mcp_parity
```

`surface.json` is generated from rustdoc JSON using the pinned compiler
(bootstrap for the documentation command only). Rustdoc is not needed to
launch the server. `coverage.json` is reviewed: its operation rows and its
list of unverified reachable symbols are separate. When Rust changes:

1. Run `python3 tools/mcp_parity_surface.py --write` and review the diff for
   new methods, payload fields, variants, return types and compatibility
   aliases. Do not edit the generated snapshot to make a check pass.
2. Reconcile the ledger with each changed operation. Identify its owner and
   scope (ordinary mixed batch, separate physical owner, revision, etc.).
   A Rust capability without an MCP equivalent is useful inventory: record
   it as outstanding. Mapping a whole method does not prove that every
   reachable input or result is supported.
3. For newly supported behavior, link concrete tests for inspection, output
   and rejection. Run the [installed MCP tests](../mcp/README.md), which
   compare outputs with direct Rust calls independently of the MCP decoder.

The acceptance scope, sequencing and closure gate are in the
[parity plan](../../docs/MCP-EDITOR-PARITY.md). The inventory is development
metadata; live clients should use MCP tool discovery for supported inputs.
At closure, use `python3 tools/check_mcp_parity.py --require-complete`.
Without the flag the validator accepts explicitly recorded gaps so that
incremental changes can pass CI. An `excluded` row requires both a reason
and a link to a scope decision on #72; it is not the same as `covered`.

Use the validator's printed counts when reporting progress. Its reviewed rows
include convenience aliases; its unverified-symbol count includes data types
and helper methods. Neither is a denominator for a percentage of editor
features delivered.
