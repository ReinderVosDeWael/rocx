import assert from "node:assert/strict";
import { generateDocx } from "rocx-docx";

const paragraph = (...runs) => ({ kind: "paragraph", runs });
const text = (text) => ({ kind: "text", text });
const table = (rows) => ({ kind: "table", rows });
const image = (bytes) => ({ kind: "image", bytes });
const render = (...blocks) => generateDocx({ blocks });

async function rejectsWith(promise, { message, path, rule }) {
  await assert.rejects(promise, (error) => {
    assert.ok(error instanceof Error);
    assert.match(error.message, message);
    for (const [key, expected] of Object.entries({ path, rule })) {
      if (expected === undefined) {
        assert.ok(!Object.hasOwn(error, key), `${key} should not be set`);
      } else {
        assert.equal(error[key], expected);
      }
    }
    return true;
  });
}

await rejectsWith(generateDocx({ blocks: 1 }), {
  message: /Invalid document/,
  path: "blocks",
});
await rejectsWith(generateDocx({ blocks: [], unsupported: true }), {
  message: /unknown field `unsupported`/,
});
await rejectsWith(
  render(paragraph(), paragraph(), paragraph(), table([["a"], ["b"], ["c", "d"]])),
  { message: /Table rows must form a non-empty rectangular grid/, path: "blocks[3].rows[2]" },
);
await rejectsWith(render(table([])), {
  message: /rectangular grid/,
  path: "blocks[0].rows",
});
await rejectsWith(render(table([[]])), {
  message: /rectangular grid/,
  path: "blocks[0].rows[0]",
});
await rejectsWith(render(paragraph(), { kind: "unknown" }), {
  message: /Invalid document/,
  path: "blocks[1]",
});
await rejectsWith(render(paragraph(), paragraph({ ...text("a"), unsupported: true })), {
  message: /unknown field `unsupported`/,
  path: "blocks[1].runs[0]",
});
await rejectsWith(render(paragraph(text("ok"), image([1]))), {
  message: /Image bytes must be a Uint8Array/,
  path: "blocks[0].runs[1]",
});
await rejectsWith(render(paragraph(image(new Uint8Array([1])))), {
  message: /invalid or unsupported image/,
  rule: "invalid_content",
});
await rejectsWith(render(paragraph(text("\0"))), {
  message: /invalid XML 1\.0 character/,
  rule: "invalid_xml",
});

const hyperlink = (target, ...runs) => ({ kind: "hyperlink", target, runs });
const richTable = (...blocks) => table([[{ blocks }]]);
await rejectsWith(render(richTable(paragraph(hyperlink("./relative", text("link"))))), {
  message: /absolute external URI\/IRI/,
  path: "blocks[0].rows[0][0].blocks[0].runs[0].target",
  rule: "invalid_relationship",
});
await rejectsWith(render(paragraph(hyperlink("https://example.org/", hyperlink("https://example.org/")))), {
  message: /no nested hyperlinks/,
  path: "blocks[0].runs[0].runs[0]",
});
await rejectsWith(render(richTable(paragraph(hyperlink("https://example.org/", text("\uD800"))))), {
  message: /malformed UTF-16/,
  path: "blocks[0].rows[0][0].blocks[0].runs[0].runs[0]",
});
const cyclicTable = richTable();
cyclicTable.rows[0][0].blocks.push(cyclicTable);
await rejectsWith(render(cyclicTable), {
  message: /at most 32 levels/,
  path: "blocks[0]" + ".rows[0][0].blocks[0]".repeat(32),
});
