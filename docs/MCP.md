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
physical table-cell vertical alignment, and image insertion in a default or
first-page header. For a header image, specify `shared` to edit its shared
part or `section-local` to limit the change to the selected section.

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
