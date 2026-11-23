import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { generateDocx } from "rocx-docx";

const bytes = await readFile(process.argv[2]);
const options = { timestamp: "2026-11-21T00:00:00Z" };
const paragraph = (...runs) => ({ kind: "paragraph", runs });
const fields = {
  "text run": (text) => paragraph({ kind: "text", text }),
  "table cell": (text) => ({ kind: "table", rows: [[text]] }),
  altText: (altText) => paragraph({ kind: "image", bytes, altText }),
};

for (const [field, block] of Object.entries(fields)) {
  const render = (text) => generateDocx({ blocks: [block(text)] }, options);
  for (const [kind, surrogate] of [["high", "\uD800"], ["low", "\uDC00"]]) {
    test(`${field} rejects a lone ${kind} surrogate`, async () => {
      await assert.rejects(render(`before${surrogate}after`), { message: /UTF-16/ });
    });
  }

  test(`${field} does not replace or drop a supplementary character`, async () => {
    const supplementary = await render("before\u{10437}after");
    assert.notDeepEqual(supplementary, await render("before\uFFFDafter"));
    assert.notDeepEqual(supplementary, await render("beforeafter"));
  });
}
