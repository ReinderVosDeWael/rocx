# JSON editor CLI

Run `cargo run --bin rocx --release`, supplying exactly one JSON request on
stdin. It writes one JSON response to stdout; success exits 0 with
`{"ok":true,"result":...}`, failure exits nonzero with `{"ok":false,"error":"..."}`.
Requests reject unknown fields. Paths are interpreted relative to the process
working directory.

1. Inspect: `{"command":"inspect","source":"input.docx"}` returns `snapshot`
   and direct body blocks in order. Paragraphs contain `target`, `text`,
   `alignment_xml` and `has_opaque_content`. Direct block controls report
   `type: "content-control"`, `index`, `tag`, `alias` and `lock`, without an
   edit target or contents. Tables and opaque blocks are identified but not
   expanded. Text is a projection, not rendered text.
2. Preview: `{"command":"preview","source":"input.docx","snapshot":"DIGEST","edits":[{"target":0,"alignment":"center"}]}`.
   Use the returned snapshot and paragraph target, not a guessed index. The
   result lists actual changes, with previous XML alignment and the new Rust
   alignment name; no-op edits are omitted.
3. Save: submit the same request with `"command":"save-copy"` and
   `"output":"copy.docx"`. The complete batch is validated again; output must
   not already exist. The source is never opened for writing.

Supported alignments: `left`, `center`, `right`, `justify`, `distribute`,
`justify-medium`, `justify-high`, `justify-low`, `thai-justify`. Duplicate
targets, invalid targets, unknown alignments, stale snapshots and editor-rejected
unsafe edits reject the whole batch before creating output. An I/O failure while
writing can leave an incomplete newly created file.

The snapshot is lowercase SHA-256 of the complete input DOCX bytes. The CLI
reopens the source on every request, checks the digest, and resolves body indices
to anchors in that newly opened core snapshot before previewing. Thus identical
bytes are intentionally interchangeable, even at another path; any byte change
invalidates all targets. This is a CLI binding, not serialization of the core's
process-local anchors. The digest provides change detection, not authentication.
Preview does not reserve source or output: save-copy independently revalidates
the bytes it reads and creates its destination exclusively.
