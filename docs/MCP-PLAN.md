# Local MCP server — implementation plan for [#52](https://github.com/ReinderVosDeWael/rocx/issues/52)

**Goal:** A local MCP client can discover how to edit a DOCX, inspect its
targets, try competing edits, and save exactly the preview it selects. The
client needs neither CLI syntax nor knowledge of Word XML. The source install
and client workflow are in the [MCP guide](MCP.md). This milestone is
independent of the 0.4.0 release.

## Architecture decision

Add a separate Cargo workspace member at `crates/rocx-mcp/` that builds the
native `rocx-mcp` stdio binary. It depends on the root `rocx` library and owns
`rmcp`, Tokio, tool schemas, and session state. The dependency is one-way:
building the core or its bindings does not build the server. Keep document editing in
`rocx::editor::ImportedDocument`. The [JSON CLI](CLI.md) already has useful
in-process seams: `src/bin/rocx_protocol/` inspects an open document and
previews a batch as encoded bytes plus a change report. Use these as a reference
for mapping the seven edit kinds to public editor calls; do not
import private CLI modules across crates or publish the CLI wire format as a
core API. Leave file access, MCP discovery and handle
ownership in the server. Do not invoke `rocx` as a subprocess: `src/bin/rocx.rs`
reopens the source and replays the edits on save, so it cannot save a retained
preview. Nor should the MCP schema expose the CLI's much larger `Edit` enum.

## Client contract

One server process serves one stdio session. Publish six tools through
`tools/list`, with descriptions, required fields, typed inputs and result
examples. Paths refer to files on the server host; resolve relative paths
against the launch directory.

| Tool | Input | Successful result |
| --- | --- | --- |
| `create_document` | Optional `paragraphs`: array of plain-text strings | Use the core authoring API to build a DOCX with one section, then open its encoded bytes as a live editor snapshot. Omitted or empty `paragraphs` produces one empty paragraph that inspection can target. Return `document`, source SHA-256 and an initial `preview` for the unedited bytes. Write no file. |
| `open_document` | `path` | Fresh opaque `document` handle and SHA-256 of the bytes read. Open once, retain the immutable snapshot, and never open the source for writing. |
| `inspect_document` | `document` | Body order, paragraph/run text, physical table cells and default/first header slots with sharing information. Issue typed opaque anchors for candidate targets. Mark opaque content without issuing an editable anchor for it. |
| `preview_edits` | `document`, nonempty ordered `edits` | Fresh `preview` handle, output SHA-256 and core change report. Resolve all anchors against that document's original positions; validate and encode one batch before registering the result. |
| `save_copy` | `document`, `preview`, `output` | New file path and the selected preview's SHA-256. Write its retained bytes with exclusive creation. Do not accept edits on save. |
| `close_document` | `document` | Acknowledgment. Release the snapshot, anchor registry and all encoded previews owned by it. |

Make `edits.items` a discriminated union. Every variant requires `kind` and
`anchor`; reject unknown kinds and extra fields. Enumerate allowed values in
the schema, using the [CLI's seven-operation table](CLI.md#preview-and-save)
as the semantic reference:

| Kind | Anchor | Additional input |
| --- | --- | --- |
| `alignment` | Paragraph | `alignment`: paragraph alignment enum |
| `insert-paragraph` | Paragraph | `text`: plain text to insert after it |
| `insert-table` | Paragraph | `rows`: nonempty rectangular matrix of strings to insert after it |
| `run-text` | Direct run | `text`: replacement single-run text |
| `run-remove` | Direct run | None |
| `cell-alignment` | Physical cell | `alignment`: `top`, `center`, `bottom` or `both` |
| `header-image` | Header slot | `mode`: `shared` or `section-local`; `bytes`: integer array, 0–255 |

Clients pass back the anchor they received; they never reconstruct a run
index, expand a merged cell or guess a header part. An anchor identifies a
candidate, not permission to discard opaque content. Let core rules decide
whether a candidate and the other edits in its batch are compatible.

### Lifetime and failure rules

- Created and opened documents have the same inspection, edit and close
  semantics. A created document has no source path. Its initial preview can
  be saved immediately with `save_copy`, even after later previews are made.
- Repeated inspection of a live document returns reusable anchors. Preview
  and save do not advance the snapshot. Two opens of identical bytes have
  different owners. Disk changes after open do not change a live snapshot;
  reopen to see the new source and inspect again for new anchors.
- Previews A and B can coexist. Saving A after creating B must write A. A
  successful save does not consume the preview; an occupied destination can be
  retried under a new name. Close invalidates all descendants; EOF or restart
  invalidates the entire session. Never recycle an identifier into a live alias.
- Report tool failures with `isError`, a stable category, and a corrective
  action (for example, inspect this document again, change edit 2, or choose
  a new path). Distinguish invalid/expired handle, wrong owner/anchor type,
  rejected batch, destination exists, I/O and capacity failures. Use protocol
  errors for malformed protocol requests. No failed preview registers a handle
  or changes the source, earlier previews or still-live anchors.
- Guard the original source path even if it has since been removed. Use
  exclusive creation rather than a check-then-write for every destination.
  Distinguish failure to create from a later write failure that may leave an
  incomplete new copy; never report either as a successful save.

## Delivery sequence and gates

1. **Prove the transport and install path.** Verify and pin `rmcp`, the official Rust MCP SDK,
   and negotiated protocol revision after checking Rust 1.88 and the existing
   native/Wasm build matrix. From the checkout, run
   `cargo install --locked --path crates/rocx-mcp --root <prefix>` with a clean
   prefix. Launch the binary from a pinned, independent MCP SDK client;
   complete `initialize`, initialized notification and `tools/list`. Assert the
   six tool names and seven edit schema variants. Reserve stdout for protocol
   frames and stderr for diagnostics. Record tested versions and the working
   install command before building out the handlers.
2. **Create, keep and inspect snapshots.** Add the session registry, then
   create, open, inspect and close. Register a created document and its initial
   preview together only after authoring and opening succeed. Map each returned anchor to its original core address
   and owner; retain physical indices and distinguish aliased header slots.
   Test repeated inspection, simultaneous opens, source replacement, invalid
   DOCX, close/reopen and process restart. Set documented limits for open
   documents and retained preview bytes/count; reject excess without silently
   evicting advertised handles, and verify close releases capacity.
3. **Preview through the existing core.** Map the seven schema variants to
   existing operations; reuse the current declaration/image conversion and
   change reporting. Exercise every variant through `tools/call`, including
   both header modes. Mix an insertion and a later original-coordinate edit.
   Reject malformed input, wrong-kind and foreign anchors, duplicate/conflicting
   edits and an opaque target. After failure, retry a valid batch using the
   same inspected anchors. Keep existing CLI process regressions passing.
4. **Save the selected bytes.** Register encoded bytes and digest together.
   Test A/B/A selection, foreign preview, source path, existing destination,
   and retry. Change the input file between preview and save to prove no
   replay or source read occurs. Verify an occupied destination is unchanged
   byte for byte, including when two attempts target the same name.
5. **Qualify and document the installed workflow.** Drive the installed
   executable over real stdio from a clean working directory. Generate an
   independent `python-docx` fixture with multiple paragraphs/runs, a table
   and a header. From inspection alone, form a deterministic mixed batch;
   independently apply it to the same input using the native Rust editor API.
   Compare the **complete saved DOCX bytes**, not just reported digests. Reopen
   with `python-docx` to check text, insertion order and cell alignment.
   Independently check ZIP part payloads outside the expected change set,
   header/media relationship ownership for the image case, and schema/OPC
   validity. Check original input bytes after both success and failure.
   Reuse the fixture and assertion patterns in
   `tests/python_bindings/test_cli_mixed.py`; the oracle must not call the MCP
   handler. Also drive `create_document` with no arguments and with two
   paragraphs. Verify no file appears until save, inspect the blank paragraph
   and insert after it, and save both initial and edited previews. Compare
   the initial bytes to direct Rust authoring for the same input and reopen
   the saved copies with `python-docx` to verify paragraph text and order.
   Run the client tests in CI against the installed binary.

Publish a user guide with install/launch instructions, a working client
`command`/`args` configuration, a discovery-to-close example, path semantics,
handle recovery and the supported edit list. Keep qualification details here.
If a missing **core** capability is discovered, track it separately with the
repository's Rust/Python/JavaScript parity and catalog requirements; #52
itself transports existing functionality. Broader CLI operation coverage,
remote transport and cross-process persistence are follow-ups.

**Close #52** when the installed-client workflow, schema discovery, negative
lifecycle/save cases and native byte/preservation checks pass on `main`.
Link the fixing commit, CI run and user guide on the issue.
