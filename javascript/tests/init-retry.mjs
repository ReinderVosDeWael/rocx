import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const browserUrl = new URL("./browser.js", import.meta.resolve("rocx-docx"));
const wasmUrl = new URL("./pkg/rocx_bg.wasm", browserUrl);
const wasmBytes = await readFile(wasmUrl);
const transientError = new Error("transient Wasm fetch failure");
const originalFetch = globalThis.fetch;
let fetches = 0;

globalThis.fetch = async (url) => {
  assert.equal(String(url), wasmUrl.href);
  if (++fetches === 1) throw transientError;
  return new Response(wasmBytes, { headers: { "Content-Type": "application/wasm" } });
};

function assertDocx(bytes) {
  assert.ok(bytes instanceof Uint8Array);
  assert.deepEqual([...bytes.slice(0, 4)], [0x50, 0x4b, 3, 4]);
}

try {
  const { generateDocx } = await import(browserUrl.href);
  const empty = { blocks: [] };
  const isTransientError = (error) => error === transientError;

  await Promise.all([
    assert.rejects(generateDocx(empty), isTransientError),
    assert.rejects(generateDocx(empty), isTransientError),
  ]);
  assert.equal(fetches, 1, "concurrent callers share the failed load");

  const [first, second] = await Promise.all([
    generateDocx(empty),
    generateDocx(empty),
    assert.rejects(generateDocx({ ...empty, unsupported: true }), /unknown field/),
  ]);
  assertDocx(first);
  assertDocx(second);
  assert.equal(fetches, 2, "concurrent callers share one retry");

  assertDocx(await generateDocx(empty));
  assert.equal(fetches, 2, "render failure does not require another load");
} finally {
  globalThis.fetch = originalFetch;
}
