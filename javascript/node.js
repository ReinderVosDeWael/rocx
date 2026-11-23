import { readFile } from "node:fs/promises";
import init from "./pkg/rocx.js";
import { createGenerator } from "./generate.js";

export const generateDocx = createGenerator(async () => {
  const bytes = await readFile(new URL("./pkg/rocx_bg.wasm", import.meta.url));
  await init({ module_or_path: bytes });
});
