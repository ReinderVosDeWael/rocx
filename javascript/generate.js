import { render_docx } from "./pkg/rocx.js";
import { asError, createBoundary, validateTimestamp } from "./boundary.js";

export function createApi(initialize) {
  const boundary = createBoundary(initialize);
  async function generateDocx(declaration, options) {
    if (declaration != null && Object.hasOwn(declaration, "sections") &&
        !Object.hasOwn(declaration, "blocks")) {
      const document = await boundary.resolveDocument(declaration);
      return document.toDocx(options);
    }
    const { timestamp = new Date().toISOString() } = options === undefined ? {} : options;
    validateTimestamp(timestamp);
    await boundary.init();
    try {
      return await render_docx(declaration, timestamp);
    } catch (cause) {
      throw asError(cause);
    }
  }
  // Named root routes are reserved here, not in feature-owned JS entrypoints.
  return {
    ...boundary,
    generateDocx,
    openDocx: (bytes, options) => boundary.invoke("openDocx", { bytes, options }),
    compileTemplate: (document, source, options) => boundary.invoke("compileTemplate", { document, source, options }),
    inlineText: (handles) => boundary.invoke("inlineText", { handles }),
    createResolvedInline: (input) => boundary.invoke("createResolvedInline", input),
    createResolvedBlockControl: (properties, children) =>
      boundary.invoke("createResolvedBlockControl", { properties, children }),
    validateValue: (input) => boundary.invoke("validateValue", input),
    validateAuthoringId: (kind, value) => boundary.invoke("validateAuthoringId", { kind, value }),
    bookmarkStart: (bookmarkName) => boundary.invoke("bookmarkStart", { name: bookmarkName }),
    bookmarkEnd: (bookmarkName) => boundary.invoke("bookmarkEnd", { name: bookmarkName }),
  };
}
