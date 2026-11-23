import init from "./pkg/rocx.js";
import { createApi } from "./generate.js";

export const {
  generateDocx, resolveDocument, resolveComponent, resolveValue,
  openDocx, compileTemplate, inlineText, createResolvedInline, createResolvedBlockControl,
  validateValue, validateAuthoringId, bookmarkStart, bookmarkEnd,
} = createApi(() => init());
export { ready, lazy, future, deferred, property } from "./boundary.js";
