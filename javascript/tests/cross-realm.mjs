import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { runInNewContext } from "node:vm";
import { generateDocx } from "rocx-docx";

const image = await readFile(process.argv[2]);
const options = { timestamp: "2026-11-21T00:00:00Z" };
const render = (bytes) => generateDocx({
  blocks: [{ kind: "paragraph", runs: [{ kind: "image", bytes }] }],
}, options);

const foreignView = runInNewContext(`
  const padded = new Uint8Array(image.length + 8).fill(0xff);
  padded.set(image, 4);
  padded.subarray(4, 4 + image.length);
`, { image });
assert.ok(ArrayBuffer.isView(foreignView));
assert.ok(!(foreignView instanceof Uint8Array));
assert.equal(foreignView.byteOffset, 4);

const expected = await render(new Uint8Array(image));
assert.ok(expected instanceof Uint8Array);
assert.deepEqual(expected.slice(0, 4), Uint8Array.of(0x50, 0x4b, 3, 4));
assert.deepEqual(await render(foreignView), expected);
assert.deepEqual(await render(image), expected); // readFile returns a Node Buffer.

const clamped = new Uint8ClampedArray(image);
Object.defineProperty(clamped, Symbol.toStringTag, { value: "Uint8Array" });
const forged = Object.create(Uint8Array.prototype);
for (const bytes of [
  new Uint8ClampedArray(image),
  clamped,
  [...image],
  { [Symbol.toStringTag]: "Uint8Array" },
  forged,
]) {
  await assert.rejects(render(bytes), { name: "Error", message: /must be a Uint8Array/ });
}

const detached = new Uint8Array(image);
structuredClone(detached.buffer, { transfer: [detached.buffer] });
await assert.rejects(render(detached), { name: "Error", message: /Cannot copy image bytes/ });

console.log("Packed image tests passed: exact DOCX bytes for foreign subarray and Buffer; invalid brands rejected.");
