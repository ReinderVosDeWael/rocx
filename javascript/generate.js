import { render_docx } from "./pkg/rocx.js";

export function createGenerator(initialize) {
  let initialized;
  return async function generateDocx(declaration, { timestamp = new Date().toISOString() } = {}) {
    if (typeof timestamp !== "string" ||
        !/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d{3})?Z$/.test(timestamp)) {
      throw new TypeError("timestamp must be a UTC ISO timestamp");
    }
    const canonical = timestamp.length === 20 ? timestamp.replace("Z", ".000Z") : timestamp;
    const date = new Date(timestamp);
    if (Number.isNaN(date.valueOf()) || date.toISOString() !== canonical || timestamp.startsWith("0000")) {
      throw new TypeError("timestamp is not a valid UTC date and time");
    }
    initialized ??= initialize();
    await initialized;
    return render_docx(declaration, timestamp);
  };
}
