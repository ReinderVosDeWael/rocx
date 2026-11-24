import { bookmarkEnd, bookmarkStart, declarative as d } from "rocx-docx";
import { INLINE_LABEL, INLINE_LINK_RUNS, STAMP, cell, heading, p, para, row, table, tableCell, text } from "./blocks.mjs";

const external = (url) => ({ kind: "external", url });
const bookmark = (name) => ({ kind: "bookmark", name });
export const hyperlink = (target, children, properties) => d.Hyperlink(target, children, { properties });
const link = (label) => hyperlink(external("https://example.com/inspection?q=docx#sample"),
  [text(label, { color: { kind: "theme", name: "hyperlink" }, underline: "single" })],
  { tooltip: "External link; never fetched by the library", history: true });
const field = (instruction, children, properties) => d.Field(instruction, children, { properties });
const noteRef = (kind) => d.NoteReference({ kind, id: 0 });
export const inlineControl = (children, properties) => d.InlineContentControl(children, { properties });
export const revision = (kind, author, date, name) => ({ kind: { kind, ...(name === undefined ? {} : { name }) }, author, date });

export async function storyBlocks(image) {
  const blocks = [
    heading("06 Lists, hyperlinks, bookmarks, comments, notes and cached fields"),
    p("Concrete numbering-level replacements (numId=60): level 0 uses lowerLetter, text %1), replacement start=3 plus separate startOverride=7; level 1 uses upperRoman, text (%2), start=4. Abstract formats/starts are unchanged. The viewer computes the resulting labels."),
    p("Legal numbering requests: abstract level 2 and replacement level 1 enable decimal display of referenced levels; replacement level 0 explicitly disables it. Other levels omit the request. numFmt tokens remain unchanged; the viewer renders labels."),
    p("Numbering suffix requests: abstract level 2 explicitly uses tab; replacement level 0 uses space; replacement level 1 uses nothing. Other levels omit suff (consumer default: tab). Separators are not inserted into paragraph text by the library."),
    p("Level restart requests: abstract level 2 and replacement level 0 use 0 (never restart); replacement level 1 uses trigger 1 (one-based first level). Other levels omit lvlRestart (restart after the preceding level). This is independent of concrete startOverride; the viewer calculates counters."),
    p("Numbering-label alignment: abstract levels 0..4 request left, center, right, start, end; replacement levels 0/1 request end/start. Remaining levels omit lvlJc (direction-dependent consumer default). Paragraph text alignment is independent; the viewer positions generated labels."),
    p("Numbering label formatting: abstract level 0 requests bold; replacement level 0 requests bold off and italic on."),
    p("Numbering level paragraph properties: abstract level 0 requests 24 pt left and 12 pt hanging; replacement level 0 requests 36 pt left."),
    p("Numbering level style links: abstract level 0 links to Heading 1; replacement level 0 links to Heading 2."),
  ];
  const listed = (text, id, level = 0) => p(text, undefined, { numbering: { id, level } });
  for (let level = 0; level < 9; level++) {
    blocks.push(listed(`Numbered list: concrete numId=60 level=${level}; abstract start=1; concrete level 0 restart=7, level 8 restart=0. Labels are computed by your viewer.`, 60, level));
  }
  const catalogBookmark = "CatalogBookmark";
  const overlappingBookmark = "OverlappingBookmark";
  const comment = d.Comment([
    p("Structured comment: paragraph, table, image and hyperlink."), table([row([cell("Comment table")])]),
    para([image("png", { width: 18 }), link("Comment external link")]),
  ], { metadata: { author: "Structured reviewer", date: STAMP, initials: "SR" } });
  const editedRevision = (kind, date, name) => ({ revision: revision(kind, "Catalog editor", date, name) });
  blocks.push(
    listed("Bullet list: numId=61, literal bullet label.", 61),
    listed("Shared abstract numbering: numId=62 reuses the bullet levels of numId=61 with its own start override.", 62),
    p("Numbering style: Catalog Bullets links concrete numId=61 to abstract definition 100."),
    listed("Inherited numbering style: numId=63 gets its levels from Catalog Bullets.", 63),
    para([link("External hyperlink: example.com (tooltip and history=true)")]),
    para([hyperlink(bookmark(catalogBookmark), [text("Internal hyperlink: jump to CatalogBookmark")], { history: false })]),
    para([text(INLINE_LABEL), hyperlink(bookmark(catalogBookmark), INLINE_LINK_RUNS.map((value) => text(value)))]),
    para([
      await bookmarkStart(catalogBookmark), text("Bookmark CatalogBookmark starts here. "),
      await bookmarkStart(overlappingBookmark),
      d.RangeStart("cross-paragraph-review", { kind: "comment", text: "Cross-paragraph plain comment", author: "Range reviewer" }),
      text("Overlapping bookmark and comment begin."),
    ]),
    para([
      text("Cross-paragraph bookmark ends; overlapping bookmark continues. "),
      await bookmarkEnd(catalogBookmark), await bookmarkEnd(overlappingBookmark), d.RangeEnd("cross-paragraph-review"),
    ]),
    para([
      d.RangeStart("structured-review", { kind: "structuredComment", comment }),
      text("Structured comment anchor: inspect the comment pane."), d.RangeEnd("structured-review"),
    ]),
    para([text("Run comment convenience anchor", undefined, { commentText: "Run convenience comment", commentAuthor: "Run reviewer" })],
      undefined, { commentText: "Paragraph convenience comment", commentAuthor: "Paragraph reviewer" }),
    para([text("Footnote reference"), noteRef("footnote"), text("; endnote reference"), noteRef("endnote")]),
    p("Note numbering: lower-Roman footnotes start at 3 each page; uppercase-letter endnotes start at 7 each section. Footnotes are requested beneath text and endnotes at section end."),
    p("Tracked text revisions: inserted text and deleted text retain review metadata. Tracking future edits is explicitly off."),
    para([
      text("Current: "),
      text("inserted wording", undefined, editedRevision("insertion", "2026-11-20T00:00:00Z")),
      text("old wording", undefined, editedRevision("deletion")),
    ]),
    p("Inline content control: rich text, alias Review caption, tag review-caption, ID 5001, content locked."),
    para([
      text("Review: "),
      inlineControl([text("controlled caption", { bold: true })], { alias: "Review caption", tag: "review-caption", lock: "contentLocked", id: 5001 }),
    ]),
    p("Block content control: a controlled paragraph and table share an alias, tag and ID 5002."),
    d.BlockContentControl([p("Controlled block paragraph"), table([row([cell("Controlled table cell")])])], {
      properties: { alias: "Review block", tag: "review-block", lock: "controlLocked", id: 5002 },
    }),
    p("Plain-text control: one run, multiline enabled."),
    para([inlineControl([text("first line\nsecond line")], { tag: "plain-review", text: { kind: "plain", multiLine: true } })]),
    p("Drop-down control: Review status offers Pending and Accepted, with Accepted selected."),
    para([inlineControl([text("Accepted")], {
      alias: "Review status", tag: "review-status",
      text: { kind: "dropDown", items: [{ displayText: "Pending", value: "pending" }, { displayText: "Accepted", value: "accepted" }], selected: "accepted" },
    })]),
    p("Tracked text move: CatalogMove pairs a source and a destination."),
  );
  for (const [caption, kind] of [["Move source: ", "moveSource"], ["Move destination: ", "moveDestination"]]) {
    blocks.push(para([text(caption), text("relocated wording", undefined, editedRevision(kind, "2026-11-20T00:00:00Z", "CatalogMove"))]));
  }
  blocks.push(
    p("Table content controls: one row and its cell have separate tags."),
    table([
      row([cell("Uncontrolled table cell")]),
      row([tableCell([p("Controlled row and cell")], { contentControl: { tag: "review-cell", id: 5004 } })],
        { contentControl: { tag: "review-row", id: 5003 } }),
    ]),
  );
  for (const [instruction, label] of [
    [{ kind: "page" }, "PAGE"], [{ kind: "numPages" }, "NUMPAGES"],
    [{ kind: "reference", name: catalogBookmark }, "REF CatalogBookmark"],
    [{ kind: "tableOfContents" }, "TOC"], [{ kind: "raw", text: " DATE " }, "raw DATE"],
  ]) {
    for (const form of ["simple", "complex"]) {
      blocks.push(para([text(`Field ${form} ${label}: `),
        field(instruction, [text(`[${label} cached placeholder]`)], { form, dirty: true, locked: false })]));
    }
  }
  blocks.push(para([
    text("Nested cached field (outer complex locked=true): "),
    field({ kind: "raw", text: " IF 1 = 1 " }, [field({ kind: "page" }, [text("[nested PAGE cache]")])],
      { form: "complex", dirty: false, locked: true }),
  ]));
  return blocks;
}

export function headerFooterStories(image) {
  const names = ["default", "first", "even"];
  const headers = Object.fromEntries(names.map((name) => [name, d.Header([
    para([text(`Header ${name}: `), link("owner-local external hyperlink"), image("png", { width: 12 })]),
  ])]));
  const footers = Object.fromEntries(names.map((name) => [name, d.Footer([
    para([text(`Footer ${name}: PAGE `), field({ kind: "page" }, [text("1")]),
      text(" / NUMPAGES "), field({ kind: "numPages" }, [text("1")])]),
  ])]));
  return { headers, footers };
}

export function notes(image) {
  return [
    d.Note({ kind: "footnote", id: 0 }, [
      p("Footnote body: rich paragraph, table, image and external hyperlink."),
      table([row([cell("Footnote table")])]), para([image("png", { width: 18 }), link("Footnote external link")]),
    ]),
    d.Note({ kind: "endnote", id: 0 }, [p("Endnote body: this is an endnote, distinct from footnote id=0.")]),
  ];
}

export function numbering() {
  const bullets = [{ level: 0, start: 1, format: "bullet", text: "•" }];
  return [
    {
      id: 60,
      levels: Array.from({ length: 9 }, (_, level) => ({
        level, start: 1, format: level === 1 ? "lowerRoman" : "decimal", text: `%${level + 1}.`,
        alignment: ["left", "center", "right", "start", "end"][level],
        ...(level === 2 ? { isLegal: true, suffix: "tab", restart: 0 } : {}),
        ...(level === 0 ? { labelFormat: { bold: true }, paragraphFormat: { leftIndent: 24, firstLineIndent: -12 }, paragraphStyle: "Heading 1" } : {}),
      })),
      startOverrides: [{ level: 0, start: 7 }, { level: 8, start: 0 }],
      levelOverrides: [
        { level: 1, start: 4, format: "upperRoman", text: "(%2)", isLegal: true, suffix: "nothing", restart: 1, alignment: "start" },
        { level: 0, start: 3, format: "lowerLetter", text: "%1)", isLegal: false, suffix: "space", restart: 0, alignment: "end",
          labelFormat: { bold: false, italic: true }, paragraphFormat: { leftIndent: 36 }, paragraphStyle: "Heading 2" },
      ],
    },
    { id: 61, levels: bullets, abstractId: 100 },
    { id: 62, levels: bullets, abstractId: 100, startOverrides: [{ level: 0, start: 4 }] },
    { id: 63, abstractId: 101, inheritStyle: "Catalog Bullets" },
  ];
}
