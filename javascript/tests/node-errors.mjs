import assert from "node:assert/strict";
import { generateDocx } from "rocx-docx";

const empty = { blocks: [] };
const image = (bytes) => ({ blocks: [{ kind: "paragraph", runs: [{ kind: "image", bytes }] }] });

// Start concurrent calls before the module is initialized. A failed render must
// not poison initialization or prevent other callers from getting bytes.
const [first, second] = await Promise.all([
  generateDocx(empty),
  generateDocx(empty, { timestamp: "2026-11-21T00:00:00.123Z" }),
  assert.rejects(generateDocx(image([1])), { name: "Error", message: /Uint8Array/ }),
  assert.rejects(generateDocx(image(new Uint8Array([1]))), { name: "Error", message: /invalid or unsupported image/ }),
  assert.rejects(generateDocx({ ...empty, unsupported: true }), { name: "Error", message: /unknown field/ }),
  assert.rejects(generateDocx({ blocks: [{ kind: "table", rows: [["a"], ["b", "c"]] }] }),
    { name: "Error", message: /rectangular grid/ }),
  assert.rejects(generateDocx(empty, { timestamp: "2026-02-30T00:00:00Z" }),
    { name: "TypeError", message: /valid UTC date/ }),
]);
for (const result of [first, second]) {
  assert.ok(result instanceof Uint8Array);
  assert.deepEqual([...result.slice(0, 4)], [0x50, 0x4b, 3, 4]);
}
