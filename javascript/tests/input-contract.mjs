import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { generateDocx } from "rocx-docx";
import { contractInput } from "./contract-input.mjs";

const [declaration, options] = contractInput(new Uint8Array(await readFile(process.argv[2])));
const result = await generateDocx(declaration, options);
assert.deepEqual([...result.slice(0, 4)], [0x50, 0x4b, 3, 4]);

function* inputObjects(value, path = "") {
  if (Array.isArray(value)) {
    for (const [index, child] of value.entries()) {
      yield* inputObjects(child, `${path}[${index}]`);
    }
  } else if (value && typeof value === "object" && !ArrayBuffer.isView(value)) {
    yield [value, path];
    for (const [key, child] of Object.entries(value)) {
      yield* inputObjects(child, path ? `${path}.${key}` : key);
    }
  }
}

// Each field must reach validation, not just be accepted and silently ignored.
// The native parity/readback check separately verifies what valid fields render.
let checked = 0;
for (const [object, path] of inputObjects(declaration)) {
  for (const [key, valid] of Object.entries(object)) {
    object[key] = {};
    await assert.rejects(generateDocx(declaration, options), {
      name: "Error",
      path: path || key,
    }, `invalid ${path ? `${path}.` : ""}${key} should be rejected`);
    object[key] = valid;
    checked++;
  }

  object.unexpectedContractField = true;
  await assert.rejects(generateDocx(declaration, options), {
    name: "Error",
    message: /unknown field `unexpectedContractField`/,
  }, `${path || "declaration"} should reject unknown fields`);
  delete object.unexpectedContractField;
}

for (const key of Object.keys(options)) {
  await assert.rejects(generateDocx(declaration, { ...options, [key]: {} }), {
    name: "TypeError",
  }, `invalid options.${key} should be rejected`);
  checked++;
}

console.log(`Packed input contract: ${checked} fields accepted with valid values and validated with invalid values.`);
