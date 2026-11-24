import {
  compileTemplate, future, lazy, resolveComponent, resolveDocument, validateValue,
} from "rocx-docx";
import type {
  BlockChildren, DocumentDefaults, DocumentProgram, HeaderDeclaration, ParagraphDeclaration,
  ResolvedHeaderFooter, ResolvedTextRun, RunFormat, SectionProperties,
} from "rocx-docx";

// Imported by text-layout.mjs so both hosts execute this typechecked declaration.
export function optionalDeferredNoneFixture() {
  const calls = {
    "document.defaults": 0,
    "document.commentAuthor": 0,
    "document.timestamp": 0,
    "section.properties": 0,
    "paragraph.numbering": 0,
  };
  const none = (field: keyof typeof calls) => lazy(() => {
    calls[field]++;
    return undefined;
  });
  const program = {
    defaults: none("document.defaults"),
    commentAuthor: none("document.commentAuthor"),
    timestamp: none("document.timestamp"),
    sections: [{
      kind: "section",
      properties: none("section.properties"),
      children: [{
        kind: "paragraph",
        text: "Deferred optional fields",
        numbering: none("paragraph.numbering"),
      }],
    }],
  } satisfies DocumentProgram;
  return { program, calls };
}

const runFormat = {
  style: "Text", bold: false, italic: true, underline: "words", font: "Latin",
  size: 10, color: { kind: "theme", name: "accent2", tint: 0, shade: 10 },
  superscript: false, subscript: true, strike: false, allCaps: false, smallCaps: true,
  snapToGrid: false, language: { latin: "en-US", eastAsia: "ja-JP", bidi: "he-IL" },
  boldComplexScript: true, italicComplexScript: false, sizeComplexScript: 11, rtl: true,
  complexScript: false, fontEastAsia: "East", fontComplexScript: "Complex",
} satisfies RunFormat;
const heading: ParagraphDeclaration = {
  kind: "paragraph",
  format: lazy(() => ({
    bidi: false, heading: 1, alignment: "distribute", spacingBefore: 4, spacingAfter: 2,
    lineHeight: { kind: "exact", value: 14 },
    leftIndent: -2, rightIndent: 1, firstLineIndent: 0, keepTogether: false,
    keepWithNext: true, pageBreakBefore: false, widowControl: true, suppressLineNumbers: false,
    contextualSpacing: false, mirrorIndents: false, snapToGrid: true,
    tabStops: [{ position: -2, alignment: "clear" }],
  })),
  numbering: lazy(() => ({ id: 1, level: 0 })),
  commentText: lazy(() => "Body comment"),
  runs: future(async () => [{
    kind: "text", text: lazy(() => "Bonjour 👋"), format: runFormat,
    revision: lazy(() => ({ kind: { kind: "insertion" }, author: "Reviewer" })),
    commentText: "Run comment", commentAuthor: "Reader",
  }]),
};
const margins = { top: -63500, bottom: 914400, left: 914400, right: 914400, header: 127000, footer: 127000 };
const properties = {
  pageSize: { width: 10000000n, height: 9000000 },
  pageOrientation: "landscape", pageMargins: margins,
  columns: { count: 2, space: 480, separator: false },
  pageNumbering: { start: 3, format: "upperRoman" },
  lineNumbering: { countBy: 3, restart: "newPage", distance: 200, start: 0, clear: false },
  pageBorders: {
    offsetFrom: "page", display: "notFirstPage", zOrder: "front",
    top: { style: "single", size: 8, color: [0, 0, 0], space: 10 },
  },
  documentGrid: { mode: "linesAndChars", linePitch: 100, characterPitchDelta: -2 },
  verticalAlign: "center", type: "nextPage", titlePage: true, bidi: false, rtlGutter: true,
} satisfies SectionProperties;
const defaults = {
  run: { underline: true }, paragraph: { spacingAfter: 2, lineSpacing: 1.5 },
  trackRevisions: false, mirrorMargins: true, gutterAtTop: false,
  endnotePosition: "documentEnd", footnotePosition: "pageBottom",
  endnoteNumbering: { restart: "continuous", format: "upperRoman", start: 2 },
  footnoteNumbering: { restart: "eachPage", format: "decimal", start: 1 },
} satisfies DocumentDefaults;
const header: HeaderDeclaration = {
  kind: "header", children: future<BlockChildren>(async () => [lazy(() => ({ kind: "paragraph", text: "First" }))]),
};
const program = {
  timestamp: lazy(() => "2021-04-03T02:01:00Z"),
  metadata: {
    creator: "Author", title: "Title", subject: "Subject", description: "Description",
    keywords: "Keywords", category: "Category", comments: "Comments", version: "1",
    company: "Company", manager: "Manager",
    customProperties: { Language: "fr" }, customBooleanProperties: { Approved: false },
  },
  defaults: future(async () => defaults),
  commentAuthor: "Author",
  sections: [lazy(() => ({
    kind: "section", properties, children: [heading, { kind: "paragraph" }],
    headers: { first: header }, footers: { even: { kind: "footer", children: [] } },
  }))],
} satisfies DocumentProgram;

// Compile-only examples include deliberately invalid input; do not run on import.
async function checkTypes() {
  const doc = await resolveDocument(program);
  const section = doc.view.sections[0];
  const resolvedStory: ResolvedHeaderFooter | undefined = await resolveComponent(header);
  const count: number | undefined = resolvedStory?.view.blocks.length;
  const footer = await resolveComponent({ kind: "footer", children: [] });
  const approved: boolean = doc.view.metadata.customBooleanProperties.Approved;
  const flag: boolean | undefined = doc.view.defaults?.trackRevisions;
  const margin: number | bigint | undefined = section.view.properties?.pageMargins?.top;
  const tab: number = section.view.paragraphs[0].view.format.tabStops[0].position;
  const run = section.view.paragraphs[0].view.runs[0];
  if (run.view.kind === "text") {
    const raw: string = run.view.text;
    const author: string | undefined = run.view.comment?.author;
    const variant: string | undefined = run.view.revision?.kind.kind;
    void [raw, author, variant];
  }
  const standalone: ResolvedTextRun | undefined = await resolveComponent({
    kind: "text", text: "Standalone", revision: lazy(() => undefined),
  });
  void [count, footer, approved, flag, margin, tab, standalone];
  await validateValue({ kind: "runFormat", value: runFormat });
  await validateValue({ kind: "paragraphFormat", value: { tabStops: [{ position: -3, alignment: "clear" }] } });
  await validateValue({ kind: "sectionProperties", value: properties });
  await validateValue({ kind: "documentDefaults", value: defaults });
  await validateValue({ kind: "resolvedParagraph", value: section.view.paragraphs[0] });
  await validateValue({ kind: "resolvedDocument", value: doc });
  await compileTemplate(doc, { bytes: new Uint8Array() });

  // @ts-expect-error paragraphs cannot declare both content arms
  const mixed: ParagraphDeclaration = { kind: "paragraph", text: "x", runs: [] };
  // @ts-expect-error section direction is not implicit run direction
  const wrongProperties: SectionProperties = { rtl: true };
  // @ts-expect-error custom Boolean metadata does not accept string values
  const wrongMetadata: DocumentProgram = { sections: [], metadata: { customBooleanProperties: { a: "false" } } };
  // @ts-expect-error unknown token
  const badHeight: ParagraphDeclaration = { kind: "paragraph", format: { lineHeight: { kind: "auto", value: 1 } } };
  // @ts-expect-error standalone validation takes a value, not a producer
  validateValue({ kind: "runFormat", value: lazy(() => runFormat) });
  void [mixed, wrongProperties, wrongMetadata, badHeight];
}
