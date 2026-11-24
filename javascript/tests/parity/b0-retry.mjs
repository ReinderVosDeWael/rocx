import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const browserUrl = new URL("./browser.js", import.meta.resolve("rocx-docx"));
const wasm = await readFile(new URL("./pkg/rocx_bg.wasm", browserUrl));
const transient = new Error("transient init");
const nativeFetch = globalThis.fetch;
let fetches = 0;
globalThis.fetch = async () => {
  if (++fetches === 1) throw transient;
  return new Response(wasm, { headers: { "Content-Type": "application/wasm" } });
};

try {
  const { resolveDocument, resolveComponent, resolveValue } = await import(browserUrl);
  const paragraph = { kind: "paragraph", text: "retry" };
  await Promise.all([
    assert.rejects(resolveDocument({ sections: [] }), (error) => error === transient),
    assert.rejects(resolveComponent(paragraph), (error) => error === transient),
    assert.rejects(resolveValue("value"), (error) => error === transient),
  ]);
  assert.equal(fetches, 1);
  const [doc, p, value] = await Promise.all([
    resolveDocument({ sections: [] }), resolveComponent(paragraph), resolveValue("value"),
  ]);
  assert.equal(fetches, 2);
  assert.deepEqual(Array.from(doc.toDocx().slice(0, 4)), [0x50, 0x4b, 3, 4]);
  assert.equal(p.view.text, "retry");
  assert.equal(value, "value");
} finally {
  globalThis.fetch = nativeFetch;
}
console.log("B0 packed: new entry points share failed initialization and one retry");
