// Small declaration helpers shared by the independently authored galleries.
import { declarative as d } from "rocx-docx";

export const STAMP = "2026-11-19T00:00:00Z";
export const EDIT_LABEL = "Imported editor: this paragraph changes from left to center in the edited catalog.";
export const INLINE_LABEL = "Imported editor inline structure: ";
export const INLINE_LINK_RUNS = ["jump to ", "CatalogBookmark"];

export const p = (text, format, fields) => d.Paragraph.Text(text, { format, ...fields });
export const para = (runs, format, fields) => d.Paragraph.Runs(runs, { format, ...fields });
export const heading = (text) => p(text, { heading: 1, pageBreakBefore: true });
export const text = (value, format, fields) => d.TextRun(value, { format, ...fields });
export const run = (value, format) => para([text(value, format)]);
export const cell = (value) => tableCell([p(value)]);
export const tableCell = (children, properties) => d.TableCell(children, { properties });
export const row = (children, properties) => d.TableRow(children, { properties });
export const table = (rows, properties) => d.Table(rows, { properties });
export const lineHeight = (kind, value) => ({ kind, value });
export const tabStop = (position, alignment, leader) => ({ position, alignment, leader });
export const tab = d.Tab();
export const br = (breakType = "line") => d.Break({ breakType });
