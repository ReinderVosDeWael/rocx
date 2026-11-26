# Create and edit DOCX files from an MCP client

The local `rocx-mcp` server lets a client create a Word document or edit a
copy of an existing one. From a repository checkout, install it with:

```sh
cargo install --locked --path crates/rocx-mcp
```

Configure a **stdio** server in your client. For clients that use the
`mcpServers` configuration key:

```json
{
  "mcpServers": {
    "rocx": { "command": "rocx-mcp", "args": [] }
  }
}
```

If the client cannot find the executable, use its absolute installed path as
`command`. The client launches the process and exchanges MCP messages on stdin
and stdout. Document and output paths refer to the server's filesystem;
use absolute paths if the client controls the working directory.

## Create a document

After initialization, use `tools/list` to discover the tools and their input
schemas. The following are tool arguments; substitute the actual handles
returned by the server for the placeholders.

1. Call `create_document` with `{"paragraphs":["Quarterly report","Draft"]}`.
   It returns a `document` and an initial `preview`. Pass `{}` for a blank
   document with one editable paragraph. Neither call writes a file.
2. To save the document as created, call `save_copy` with
   `{"document":"<document>","preview":"<initial preview>","output":"report.docx"}`.
3. To edit before saving, call `inspect_document` with `{"document":"<document>"}`.
   Find the paragraph containing “Quarterly report” and copy its `anchor`.
4. Call `preview_edits`:

   ```json
   {
     "document": "<document>",
     "edits": [
       {"kind":"alignment","anchor":"<paragraph anchor>","alignment":"center"}
     ]
   }
   ```

5. Pass the **new preview** to `save_copy` with a fresh output path, then call
   `close_document` with `{"document":"<document>"}`.

## Edit an existing file

Start with `open_document` and `{"path":"input.docx"}`, then inspect, preview,
and save as above. The supported edits are paragraph alignment, paragraph or
table insertion after a paragraph, direct run text replacement or removal,
replacement/deletion of a text range or insertion at a text position,
physical table-cell vertical alignment, and image insertion in a default or
first-page header. For a header image, specify `shared` to edit its shared
part or `section-local` to limit the change to the selected section.

### Edit part of a paragraph

Use the **paragraph** anchor and count Unicode scalar values in its original
text. Inspection returns `scalar_length` for candidate paragraphs and `null`
where scalar positions are not advertised. A length is not a guarantee that
a protected paragraph can be edited. In `A😀BC`, the emoji occupies `[1,2)` and the end position
is `4`. This batch replaces the emoji and appends a period, producing `AxBC.`:

```json
{
  "document": "<document>",
  "edits": [
    {"kind":"text-range","anchor":"<paragraph anchor>","start":1,"end":2,"text":"x"},
    {"kind":"insert-text","anchor":"<paragraph anchor>","offset":4,"text":"."}
  ]
}
```

Use `text:""` in a nonempty `text-range` to delete. Use `insert-text` with
nonempty text for a point insertion. Offsets can cross consecutive plain
text runs but cannot cross a paragraph, field or hyperlink. A combining mark
counts separately from its base character. In JavaScript use `Array.from(text)`
instead of UTF-16 string indices. The core rejects conflicting edits and
paragraphs with unsupported markup; see [the native range contract](EDITOR-PARITY.md#character-ranges-in-plain-body-paragraphs)
for formatting and boundary rules.

### Inspect and edit a footnote, endnote, or comment

Call `inspect_document` with `{"document":"<document>","view":{"kind":"entries","limit":10}}`.
The `entries` list contains footnotes, then endnotes, then comments in source
order. Each entry identifies its physical part, original string ID, kind,
note type or comment metadata, and indexed blocks. An unsupported block is
marked opaque. Reserved note entries are visible but do not have edit anchors.
If `next_cursor` is non-null, pass it as `view.cursor` on the next call. The
default page size is 25 entries; the accepted range is 1–100.

Copy the `anchor` from the entry itself, not its ID. A footnote and an
endnote can share the same ID. For example, to change a base comment's author:

```json
{
  "document": "<document>",
  "operation": {
    "kind": "edit-entry",
    "anchor": "<comment entry anchor>",
    "edit": {"kind":"comment-author","value":"Editorial team"}
  }
}
```

Send this to `preview_edits` as **one operation**, without an `edits` array.
It returns a retained preview and digest; `changes` is `null` because the
native entry editor returns document bytes without a change report.

| Entry edit | Input and effect |
| --- | --- |
| `insert-paragraph` | `index`, `text`: insert before an original block index; use the block count to append. |
| `replace-paragraph` | `index`, `text`: replace an eligible simple paragraph at that index. |
| `remove-paragraph` | `index`: remove an eligible paragraph; cannot remove the last one. |
| `comment-author` | `value`: set a base comment's author string. |
| `comment-initials`, `comment-date` | `value`: set the string or pass explicit `null` to clear it. |
| `delete-unused-note` | Remove an unreferenced, discardable ordinary note definition. |

An anchor identifies a candidate. The editor still protects references,
reserved note markers, rich content and threaded comments. Entry cursors and
anchors are valid only for the document that issued them. After reopening a
saved copy, inspect it again.

### Save or continue editing

Every preview starts from the opened or created document. Put edits that
should apply together in one batch, using anchors from inspection of that
document. A rejected batch leaves earlier previews and anchors usable.
To continue editing a saved result, open that copy and inspect it for new
anchors. Changes to the source file on disk do not alter a live snapshot.

`save_copy` writes the selected preview's retained bytes and returns their
SHA-256 as `output_snapshot`. It refuses an existing destination. Choose
another name and retry with the same preview. An I/O failure after creating
the destination may leave a partial new copy.

One session can retain 16 documents and 64 previews (256 MiB of encoded
preview bytes in total). A created document's initial preview counts toward
those limits. Close documents to release capacity. Closing a document or
restarting the server invalidates its handles; open or create a document
again and inspect it before retrying.
