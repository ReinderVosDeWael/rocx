import { compileTemplate, deferred, future, lazy, property, ready, resolveComponent, resolveDocument } from "rocx-docx";
import { STAMP, br, cell, heading, lineHeight, p, para, row, section, tab, table, tableCell, text } from "./blocks.mjs";
import { drawingBlocks, loadImages } from "./drawings.mjs";
import { formattingBlocks, formattingStyles } from "./formatting.mjs";
import { control, headerFooterStories, hyperlink, notes, numbering, revision, storyBlocks } from "./stories.mjs";
import { tableBlocks, tableStyles } from "./tables.mjs";

function templateProgram() {
  const templateRevision = (kind, date, name) => ({ revision: revision(kind, "Template editor", date, name) });
  return {
    timestamp: STAMP,
    sections: [section([
      para([text("Template {{re"), text("place}}: cross-run replacement.")]),
      p("Template retained paragraph: insertion appears before this."),
      para([
        text("Template plain-text control replacement: "),
        control([text("old controlled value", { bold: true })],
          { tag: "template-control", lock: "controlLocked", text: { kind: "plain" } }),
      ]),
      para([
        text("Template revision action: "),
        text("accepted template wording", undefined, templateRevision("insertion")),
        text("removed template wording", undefined, templateRevision("deletion")),
      ]),
      para([
        text("Template accepted move: "),
        text("moved template wording", undefined, templateRevision("moveSource", STAMP, "TemplateMove")),
        text("destination: "),
        text("moved template wording", undefined, templateRevision("moveDestination", STAMP, "TemplateMove")),
      ]),
    ])],
    metadata: {
      customProperties: { Project: "Atlas", "Review status": "Pending" },
      customBooleanProperties: { "Review complete": false },
    },
    styles: [
      { kind: "paragraph", name: "Catalog Existing", bold: true },
      { kind: "character", name: "Catalog Existing Character", format: { bold: true } },
    ],
  };
}

async function resolutionBlocks() {
  let calls = 0;
  const shared = property("catalog.shared", future(async () => {
    if (++calls !== 1) throw new Error("shared producer called more than once");
    return "Shared reference: computed once and used twice.";
  }));
  const paragraph = await resolveComponent(p("Resolved-child reuse: immutable paragraph used twice."));
  const inline = await resolveComponent(text("Resolved-inline reuse: immutable run used twice."));
  const resolvedBlock = { kind: "resolved", value: paragraph };
  const resolvedInline = { kind: "resolved", value: inline };
  return [
    heading("08 Authoring: ready, lazy, async, references, conditions and snapshot reuse"),
    p(ready("Ready value: this paragraph was supplied immediately.")),
    p(lazy(() => "Lazy value: this paragraph was produced by a factory.")),
    p(future(async () => "Async value: this paragraph was produced by an awaitable.")),
    p(shared.reference()), p(shared.reference()),
    p(deferred(lazy(() => "Explicit Deferred: shared computation."))),
    p("Condition true: this paragraph is visible.", undefined, { condition: true }),
    p(lazy(() => { throw new Error("excluded producer must not execute"); }), undefined, { condition: false }),
    p("Condition false: the excluded paragraph is absent and its producer did not execute."),
    resolvedBlock, resolvedBlock, para([resolvedInline, resolvedInline]),
  ];
}

function catalogSections(blocks, image) {
  const horizontal = { style: "single", size: 12, space: 6, color: [32, 48, 96] };
  const vertical = { style: "dashed", size: 8, space: 4, color: [96, 96, 96] };
  const first = section(blocks, {
    pageSize: { width: 7772400, height: 10058400 }, pageOrientation: "portrait",
    pageMargins: { top: 914400, bottom: 914400, left: 914400, right: 914400, header: 457200, footer: 228600 },
    pageBorders: { top: horizontal, left: vertical, bottom: horizontal, right: vertical, offsetFrom: "page", display: "firstPage", zOrder: "back" },
    titlePage: true, bidi: false, rtlGutter: false,
  }, headerFooterStories(image));
  const following = ["nextPage", "nextColumn", "continuous", "evenPage", "oddPage"].map((type, i) => {
    const verticalAlign = ["top", "center", "bottom", "both", "top"][i];
    const flag = i % 2 === 1;
    const restart = ["newPage", "newSection", "continuous"][i];
    const grid = ["default", "lines", "linesAndChars", "snapToChars"][i];
    const numberingLabel = restart
      ? `Line numbering: every 5 lines, start=0, distance=240 twips, restart=${restart}.`
      : `Line numbering: ${i === 4 ? "clear" : "preserve"} previous section settings.`;
    const gridLabel = grid
      ? `Document grid: mode=${grid}, line pitch=360 twips, character pitch delta=-4096.`
      : "Document grid: preserve previous section settings.";
    return section([
      p(`09 Section: break=${type}; landscape; 2 columns with separator; page numbering lowerRoman starts at ${i + 1}; vertical alignment=${verticalAlign}; section bidi=${flag}; rtl_gutter=${flag}.`),
      p(numberingLabel),
      p("Paragraph layout: contextual spacing enabled, mirrored indents explicitly off.", { contextualSpacing: true, mirrorIndents: false }),
      p(gridLabel),
      para([text("Grid snapping: paragraph enabled, run explicitly off.", { snapToGrid: false })], { snapToGrid: true }),
      p("Paragraph line numbers: suppressed.", { suppressLineNumbers: true }),
      p("Paragraph line numbers: suppression explicitly off.", { suppressLineNumbers: false }),
      p("Section layout request: physical margins remain unchanged. Gutter placement does not create a gutter amount. This is the first column."),
      para([br("column"), text("This follows a column break in the section sample.")]),
    ], {
      type, pageOrientation: "landscape", columns: { count: 2, space: 360, separator: true },
      pageNumbering: { start: i + 1, format: "lowerRoman" },
      documentGrid: grid ? { mode: grid, linePitch: 360, characterPitchDelta: -4096 } : undefined,
      lineNumbering: restart ? { countBy: 5, start: 0, distance: 240, restart } : i === 4 ? { clear: true } : undefined,
      verticalAlign, titlePage: false, bidi: flag, rtlGutter: flag,
    });
  });
  return [first, ...following];
}

export const HEADER_INLINE_LABEL = "Default-header inline structure: ";
export const HEADER_BATCH_LABEL = "Combined editor batch: this paragraph is centered and followed by a table in both image previews.";
export const HEADER_LINK_RUNS = ["shared ", "link"];
export const STORY_VARIANTS = ["default", "first", "even"];

export function storyLabel(kind, variant) {
  return `Story inspection: ${variant} ${kind}`;
}

function headerProgram() {
  const labeledStory = (kind, variant) => ({ kind, children: [p(storyLabel(kind, variant))] });
  return {
    timestamp: STAMP,
    sections: [
      section([p(HEADER_BATCH_LABEL)], undefined, {
        headers: {
          default: { kind: "header", children: [
            para([text(HEADER_INLINE_LABEL), hyperlink({ kind: "external", url: "https://example.invalid/catalog-header" },
              HEADER_LINK_RUNS.map((value) => text(value)))]),
          ] },
          first: labeledStory("header", "first"),
          even: labeledStory("header", "even"),
        },
        footers: Object.fromEntries(STORY_VARIANTS.map((variant) => [variant, labeledStory("footer", variant)])),
      }),
      section([p("Header image scope: second section shares the same header.")]),
    ],
  };
}

export function assert(condition, message) {
  if (!condition) throw new Error(`Catalog: ${message}`);
}

export function assertBytes(first, second, message) {
  assert(first.length === second.length && first.every((value, i) => second[i] === value), message);
}

/** Produce all source DOCX data with public authoring APIs; no editor needed here. */
export async function authorCatalog(readFixture, { namedPng } = {}) {
  const images = await loadImages(readFixture, namedPng);
  const { image } = images;
  const template = (await resolveDocument(templateProgram())).toDocx();
  const blocks = [
    p("ALL FEATURES — visual inspection catalog"),
    p("Each sample names its requested feature. Field caches are placeholders; update fields in your viewer. Font fallback, pagination and conditional table-style activation depend on the viewer. Inspect comments, footnotes, endnotes and document properties too."),
    p("Document defaults: Calibri 11 pt, after spacing 4 pt, 1.1 line height. See template replacement at the start and the retained template paragraph after topic 08."),
    p("Template safety: this sample uses an unsigned template. Templates with an OPC signature origin are rejected before any output is written."),
    p("Custom document properties: Project (Atlas) is retained; Review status changes to Draft & review."),
    p("Boolean custom properties: Review complete changes from false to true; Published is explicitly false."),
    p("Template compatibility: unsupported mandatory namespaces in edited WordprocessingML are rejected."),
    p("Extended document properties: Company is Atlas & Co.; Manager is Taylor."),
    ...formattingBlocks(), ...tableBlocks(), ...await storyBlocks(image), ...drawingBlocks(images),
    para([text("JavaScript authoring: ", { bold: true, italic: false }), text("Café 東京", { italic: true })],
      { alignment: "center" }),
    table([
      row([cell("Item"), cell("Details")]),
      row([cell("Pages"), tableCell([
        para([text("2", { bold: true }), tab,
          hyperlink({ kind: "external", url: "https://example.org/report" }, [text("Report details", { italic: true })]),
          br(), text("Verified")]),
        table([row([cell("Nested detail")])]),
      ])]),
    ]),
    para([{ ...image("png", { width: 72, height: 36 }), altText: { description: "JavaScript authoring sample image" } }]),
    ...await resolutionBlocks(),
    p("Header/footer distances: 720 and 360 twips from the page edges."),
    p("Page borders: navy single top/bottom and gray dashed left/right; first page, offset from page, behind text."),
    p("Document page settings: mirrored margins enabled, top gutter explicitly off."),
  ];
  const snapshot = await resolveDocument({
    sections: catalogSections(blocks, image), timestamp: STAMP, commentAuthor: "Catalog reviewer",
    metadata: {
      creator: "DOCX inspection catalog", title: "All features visual inspection", subject: "Rust/Python/JavaScript byte parity",
      description: "Labeled functionality samples; not a rendering guarantee", keywords: "docx,visual,parity",
      category: "Examples", comments: "Core metadata comments", version: "1.0",
      customProperties: { "Review status": "Draft & review" },
      customBooleanProperties: { "Review complete": true, Published: false },
      company: "Atlas & Co.", manager: "Taylor",
    },
    defaults: {
      run: { font: "Calibri", size: 11 }, paragraph: { spacingAfter: 4, lineHeight: lineHeight("multiplier", 1.1) },
      mirrorMargins: true, gutterAtTop: false, trackRevisions: false,
      footnotePosition: "beneathText", endnotePosition: "sectionEnd",
      footnoteNumbering: { start: 3, restart: "eachPage", format: "lowerRoman" },
      endnoteNumbering: { start: 7, restart: "eachSection", format: "upperLetter" },
    },
    styles: [...formattingStyles(), ...tableStyles(), { kind: "numbering", name: "Catalog Bullets", numberingId: 61 }],
    numbering: numbering(), notes: notes(image),
  });
  const source = { bytes: template, edits: {
    replacements: [["{{replace}}", "REPLACED"]], paragraphIndex: 1, scope: "supportedStories",
    contentControlReplacements: { "template-control": "updated controlled value" }, textRevisionAction: "accept",
  } };
  const options = {
    timestamp: STAMP, profile: "transitional",
    readLimits: { maxParts: 10000, maxPartBytes: 67108864, maxExpandedBytes: 268435456 },
  };
  const final = snapshot.toDocxWithTemplate(source, options);
  assertBytes(final, snapshot.toDocxWithTemplate(source, options), "reused snapshot must render identical bytes");
  const plan = await compileTemplate(snapshot, source, options);
  plan.validate();
  assert(plan.parts().length > 10 && plan.part("/word/document.xml"), "compiled template includes document part");
  assertBytes(final, plan.toBytes(), "compiled template must encode identical bytes");

  const headerSource = (await resolveDocument(headerProgram())).toDocx();
  return { template, final, headerSource, image };
}
