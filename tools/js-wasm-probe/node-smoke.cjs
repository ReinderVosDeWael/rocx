// Run after building the Node target; optional output is inspected separately.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const { paragraph_docx } = require("./pkg-node/rocx_js_wasm_probe.js");

(async () => {
  const bytes = await paragraph_docx("WASM probe — hello");
  assert(bytes instanceof Uint8Array);
  assert(bytes.length > 4);
  assert.deepEqual(Array.from(bytes.slice(0, 4)), [0x50, 0x4b, 0x03, 0x04]);
  if (process.argv[2]) fs.writeFileSync(process.argv[2], bytes);
  console.log(`DOCX ZIP signature OK (${bytes.length} bytes)`);
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
