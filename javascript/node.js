import { readFile } from "node:fs/promises";
import init from "./pkg/rocx.js";
import { createApi } from "./generate.js";

export const {
  generateDocx, resolveDocument, resolveComponent, resolveValue,
  openDocx, compileTemplate, inlineText, createResolvedInline, createResolvedBlockControl,
  validateValue, validateAuthoringId, bookmarkStart, bookmarkEnd,
} = createApi(async () => {
  const bytes = await readFile(new URL("./pkg/rocx_bg.wasm", import.meta.url));
  await init({ module_or_path: bytes });
});
export { ready, lazy, future, deferred, property } from "./boundary.js";
