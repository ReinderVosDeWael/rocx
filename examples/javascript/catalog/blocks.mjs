// Small declaration helpers shared by the independently authored galleries.
export const STAMP = "2026-11-19T00:00:00Z";
export const EDIT_LABEL = "Imported editor: this paragraph changes from left to center in the edited catalog.";
export const INLINE_LABEL = "Imported editor inline structure: ";
export const INLINE_LINK_RUNS = ["jump to ", "CatalogBookmark"];

export const p = (text, format, fields) => ({ kind: "paragraph", text, format, ...fields });
export const para = (runs, format, fields) => ({ kind: "paragraph", runs, format, ...fields });
export const heading = (text) => p(text, { heading: 1, pageBreakBefore: true });
export const text = (value, format, fields) => ({ kind: "text", text: value, format, ...fields });
export const run = (value, format) => para([text(value, format)]);
export const cell = (value) => tableCell([p(value)]);
export const tableCell = (children, properties) => ({ kind: "tableCell", children, properties });
export const row = (children, properties) => ({ kind: "tableRow", children, properties });
export const table = (rows, properties) => ({ kind: "table", rows, properties });
export const lineHeight = (kind, value) => ({ kind, value });
export const tabStop = (position, alignment, leader) => ({ position, alignment, leader });
export const tab = { kind: "tab" };
export const br = (breakType = "line") => ({ kind: "break", breakType });
export const section = (children, properties, stories) => ({ kind: "section", children, properties, ...stories });
