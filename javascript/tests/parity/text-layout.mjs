import {
  compileTemplate, createResolvedInline, future, inlineText, lazy, resolveComponent,
  resolveDocument, validateValue,
} from "rocx-docx";
import { optionalDeferredNoneFixture } from "./text-layout.mts";

const timestamp = "2021-04-03T02:01:00Z";
const document = (sections = [], fields = {}) => resolveDocument({ sections, timestamp, ...fields });
const section = (children, fields = {}) => ({ kind: "section", children, ...fields });
const paragraph = (text, fields = {}) => ({ kind: "paragraph", text, ...fields });
const text = (value, fields = {}) => ({ kind: "text", text: value, ...fields });
const pixel = Uint8Array.from(atob(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl2n9sAAAAASUVORK5CYII=",
), (ch) => ch.charCodeAt(0));
const read = (plan, name) => {
  const bytes = plan.part(name);
  if (!bytes) throw new Error(`missing ${name}`);
  return new TextDecoder().decode(bytes);
};
const equal = (actual, expected) => {
  if (!Object.is(actual, expected)) throw new Error(`expected ${String(expected)}, got ${String(actual)}`);
};
const match = (actual, expected) => {
  if (!expected.test(actual)) throw new Error(`missing ${expected} in ${actual}`);
};
const samePart = (original, edited, name) => {
  const before = original.part(name);
  const after = edited.part(name);
  if (!before || !after || before.length !== after.length || !before.every((byte, i) => byte === after[i])) {
    throw new Error(`untouched part changed: ${name}`);
  }
};
const rejects = async (call, message, rule, path) => {
  try {
    await call();
  } catch (error) {
    match(error.message, message);
    equal(error.rule, rule);
    if (path !== undefined) equal(error.path, path);
    return;
  }
  throw new Error(`expected rejection: ${message}`);
};
const pack = async (doc) => compileTemplate(await document(), { bytes: doc.toDocx() });

export async function documentFieldsAndCustomMetadata() {
  const metadata = {
    creator: "Creator", title: "Résumé", subject: "Subject", description: "Description",
    keywords: "Keywords", category: "Category", comments: "Comments", version: "3",
    company: "Company", manager: "Manager",
    customProperties: { Keep: "old", ["__proto__"]: "legal name", $rocxHandle: "plain metadata" },
    customBooleanProperties: { Visible: false, Other: true },
  };
  const empty = await document();
  equal(empty.view.sections.length, 0);
  equal(empty.view.defaults, undefined);
  equal(Object.keys(empty.view.metadata.customProperties).length, 0);
  const seed = await document([section([paragraph("seed")])], {
    metadata: future(async () => metadata),
    timestamp: lazy(() => timestamp),
    commentAuthor: lazy(() => "Default Author"),
  });
  equal(seed.view.timestamp, timestamp);
  equal(seed.view.commentAuthor, "Default Author");
  for (const key of Object.keys(metadata)) {
    if (!key.startsWith("custom")) equal(seed.view.metadata[key], metadata[key]);
  }
  equal(seed.view.metadata.customBooleanProperties.Visible, false);
  equal(seed.view.metadata.customProperties.__proto__, "legal name");
  equal(seed.view.metadata.customProperties.$rocxHandle, "plain metadata");
  const patch = await document([], { metadata: { title: "New title", customProperties: { Added: "new" } } });
  const merged = await compileTemplate(patch, { bytes: seed.toDocx() });
  match(read(merged, "docProps/core.xml"), /<dc:title>New title<\/dc:title>/);
  match(read(merged, "docProps/core.xml"), /<dc:creator>Creator<\/dc:creator>/);
  match(read(merged, "docProps/app.xml"), /<Company>Company<\/Company>/);
  const custom = read(merged, "docProps/custom.xml");
  for (const [name, value] of [["Keep", "old"], ["Added", "new"], ["Visible", "false"]]) {
    match(custom, new RegExp(`name="${name}"[^>]*><vt:[^>]+>${value}<`));
  }
  await rejects(
    () => document([], { metadata: { customProperties: { Same: "x" }, customBooleanProperties: { Same: false } } }),
    /overlap|both/i, "invalid_content",
  );
}

async function optionalDeferredNone() {
  const { program, calls } = optionalDeferredNoneFixture();
  const assertCalls = (expected) => {
    for (const [field, count] of Object.entries(calls)) {
      if (count !== expected) throw new Error(`${field}: expected ${expected} producer calls, got ${count}`);
    }
  };
  assertCalls(0);
  const before = Math.floor(Date.now() / 1000) * 1000;
  const snapshot = await resolveDocument(program);
  const after = Math.floor(Date.now() / 1000) * 1000;
  assertCalls(1);

  const sectionView = snapshot.view.sections[0].view;
  const paragraphView = sectionView.paragraphs[0].view;
  for (const [view, field] of [
    [snapshot.view, "defaults"], [snapshot.view, "commentAuthor"],
    [sectionView, "properties"], [paragraphView, "numbering"],
  ]) {
    if (field in view) throw new Error(`unexpected optional view field: ${field}`);
  }
  const clock = snapshot.view.timestamp;
  match(clock, /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/);
  const milliseconds = Date.parse(clock);
  if (!(before <= milliseconds && milliseconds <= after)) {
    throw new Error(`snapshot timestamp did not fall within resolution: ${clock}`);
  }

  const first = snapshot.toDocx();
  // Cross a UTC second so accidentally sampling the render-time clock is detectable.
  await new Promise((done) => setTimeout(done, 1000));
  const second = snapshot.toDocx();
  equal(snapshot.view.timestamp, clock);
  equal(first.length, second.length);
  equal(first.every((byte, i) => byte === second[i]), true);
  assertCalls(1);
}

export async function sectionAndStoryComposition() {
  const fail = () => { throw new Error("excluded producer was called"); };
  const defaultHeader = future(async () => ({
    kind: "header", children: lazy(() => [
      lazy(() => ({ kind: "paragraph", runs: [
        { kind: "hyperlink", target: { kind: "external", url: "https://example.org/header" }, children: [text("Link")] },
        { kind: "field", instruction: { kind: "page" }, children: [text("1")] },
        { kind: "image", data: lazy(() => pixel) },
      ] })),
    ]),
  }));
  const source = await document([
    lazy(() => section(future(async () => [lazy(() => paragraph("body"))]), {
      headers: {
        default: defaultHeader,
        first: { kind: "header", condition: false, children: lazy(fail) },
        even: { kind: "header", children: [paragraph("even")] },
      },
      footers: {
        default: { kind: "footer", children: [paragraph("footer")] },
        first: future(async () => ({ kind: "footer", children: [paragraph("first footer")] })),
        even: { kind: "footer", condition: false, children: lazy(fail) },
      },
    })),
    section(lazy(fail), { condition: false, headers: { default: lazy(fail) } }),
    section([paragraph("inherited")]),
    section([paragraph("new")], { headers: { default: { kind: "header", children: [] } } }),
  ]);
  equal(source.view.sections.length, 3);
  const [first, second, third] = source.view.sections.map((item) => item.view);
  equal(first.headers.first, undefined);
  equal(first.footers.even, undefined);
  equal(first.headers.default.view.blocks[0].view.text, "Link1");
  equal(first.headers.even.view.blocks[0].view.text, "even");
  equal(first.footers.default.view.blocks[0].view.text, "footer");
  equal(first.footers.first.view.blocks[0].view.text, "first footer");
  equal(second.headers.default, undefined); // Rust projects authored stories, not inferred inheritance.
  equal(third.headers.default.view.blocks.length, 0);
  for (const kind of ["header", "footer"]) {
    equal((await resolveComponent({ kind, children: [paragraph(kind)] })).view.blocks[0].view.text, kind);
    equal(await resolveComponent({ kind, condition: false, children: lazy(fail) }), undefined);
  }
  const plan = await pack(source);
  const headers = plan.parts().filter(([name]) => /^word\/header\d+\.xml$/.test(name));
  equal(headers.length, 3);
  const owned = headers.find(([, bytes]) => new TextDecoder().decode(bytes).includes("Link"))[0];
  match(read(plan, owned), /PAGE/);
  const relationships = owned.replace("word/", "word/_rels/") + ".rels";
  match(read(plan, relationships), /https:\/\/example\.org\/header/);
  match(read(plan, relationships), /relationships\/image/);
  const sections = [...read(plan, "word/document.xml").matchAll(/<w:sectPr\b[^>]*>([\s\S]*?)<\/w:sectPr>/g)];
  const references = sections.map(([, xml]) => xml.match(/<w:headerReference\b[^>]*>/g) ?? []);
  equal(references.map((list) => list.length).join(","), "2,0,1");
  const defaultId = (list) => list.find((ref) => ref.includes('w:type="default"'))
    ?.match(/\br:id="([^"]+)"/)?.[1];
  const inheritedId = defaultId(references[0]);
  const independentId = defaultId(references[2]);
  equal(Boolean(inheritedId && independentId && inheritedId !== independentId), true);
  const extension = await document([section([paragraph("appended")])]);
  const extended = await compileTemplate(extension, { bytes: source.toDocx() });
  samePart(plan, extended, owned);
  samePart(plan, extended, relationships);
  for (const [name] of plan.parts().filter(([name]) => name.startsWith("word/media/"))) {
    samePart(plan, extended, name);
  }
}

export async function sectionGeometryAndInheritedPatches() {
  const properties = {
    pageSize: { width: 9144000n, height: 6858000 },
    pageOrientation: "landscape",
    pageMargins: { top: -63500, bottom: 127000, left: 914400, right: 914400, header: 457200, footer: 317500 },
    pageBorders: {
      offsetFrom: "page", display: "firstPage", zOrder: "back",
      top: { style: "single", color: [1, 2, 3], size: 8, space: 7 },
      left: { style: "dashed" }, bottom: { style: "single" }, right: { style: "single" },
    },
    documentGrid: { mode: "linesAndChars", linePitch: 220, characterPitchDelta: -16 },
    pageNumbering: { format: "lowerRoman", start: 4 },
    lineNumbering: { clear: false, countBy: 2, start: 1, restart: "newSection", distance: 240 },
    columns: { count: 2, space: 100, separator: false },
    verticalAlign: "center", titlePage: false, type: "nextPage", bidi: false, rtlGutter: true,
  };
  const source = await document([section([paragraph("geometry")], { properties: lazy(() => properties) })]);
  const view = source.view.sections[0].view.properties;
  equal(view.pageSize.width, 9144000);
  equal(view.pageMargins.top, -63500);
  equal(view.pageBorders.top.color[2], 3);
  equal(view.lineNumbering.clear, false);
  equal(view.bidi, false);
  equal(view.rtlGutter, true);
  const xml = read(await pack(source), "word/document.xml");
  for (const expected of [
    /<w:pgSz\b[^>]*w:w="14400"/, /<w:pgMar\b[^>]*w:top="-100"/,
    /<w:lnNumType\b[^>]*w:distance="240"/, /<w:docGrid\b[^>]*w:linePitch="220"/,
    /<w:docGrid\b[^>]*w:charSpace="-16"/, /<w:pgBorders\b[^>]*w:display="firstPage"/,
    /<w:bidi w:val="0"/, /<w:rtlGutter\b/,
  ]) match(xml, expected);
  for (const value of [
    { type: "nextColumn", pageOrientation: "portrait", verticalAlign: "top", documentGrid: { mode: "default" } },
    { type: "continuous", verticalAlign: "bottom", documentGrid: { mode: "lines" }, lineNumbering: { restart: "newPage" } },
    { type: "evenPage", verticalAlign: "both", documentGrid: { mode: "snapToChars" }, lineNumbering: { restart: "continuous" },
      pageBorders: { offsetFrom: "text", display: "notFirstPage", zOrder: "front" } },
    { type: "oddPage", pageBorders: { display: "allPages" } },
  ]) {
    const resolved = await resolveComponent({ kind: "section", properties: value });
    equal(resolved.view.properties.type, value.type);
    equal(resolved.view.properties.documentGrid?.mode, value.documentGrid?.mode);
    equal(resolved.view.properties.pageBorders?.display, value.pageBorders?.display);
  }
  const patch = await document([section([], { properties: { pageMargins: { left: 635000 } } })]);
  const inherited = await compileTemplate(patch, { bytes: source.toDocx() });
  const inheritedXml = read(inherited, "word/document.xml");
  match(inheritedXml, /<w:pgMar\b[^>]*w:left="1000"/);
  match(inheritedXml, /<w:pgMar\b[^>]*w:top="-100"/);
  match(inheritedXml, /<w:lnNumType\b/);
  const clear = await document([section([], { properties: { lineNumbering: { clear: true } } })]);
  const clearedXml = read(await compileTemplate(clear, { bytes: source.toDocx() }), "word/document.xml");
  equal(/<w:lnNumType\b/.test(clearedXml), false);
  for (const [value, message, path] of [
    [{ documentGrid: { linePitch: 0 } }, /line pitch/i, "documentGrid.linePitch"],
    [{ pageBorders: { top: { style: "single", size: 1 } } }, /border size/i, "pageBorders.top.size"],
    [{ pageSize: { width: -635 } }, /page size/i, "pageSize.width"],
  ]) await rejects(() => validateValue({ kind: "sectionProperties", value }), message, "invalid_measurement", path);
}

// Feed a DOCX with its source w:pgMar absent (e.g. manufactured with python-docx).
export async function sectionGeometryWithoutInheritedMargins(bytes) {
  const empty = await document();
  const original = await compileTemplate(empty, { bytes });
  equal(/<w:pgMar\b/.test(read(original, "word/document.xml")), false);
  const patch = await document([section([], { properties: { pageMargins: { left: 635000 } } })]);
  const patched = await compileTemplate(patch, { bytes });
  const margin = read(patched, "word/document.xml").match(/<w:pgMar\b[^>]*>/)?.[0];
  match(margin, /w:left="1000"/);
  for (const side of ["top", "right", "bottom", "header", "footer", "gutter"]) {
    match(margin, new RegExp(`w:${side}="\\d+"`));
  }
  samePart(original, patched, "word/styles.xml");
}

export async function paragraphAndRunComposition() {
  const started = [];
  let release;
  const barrier = new Promise((done) => { release = done; });
  const job = (name, result) => future(async () => {
    started.push(name);
    if (started.length === 4) release();
    await barrier;
    return result;
  });
  const source = await document([section([
    paragraph(" shorthand\t\n😀 "),
    { kind: "paragraph" },
    {
      kind: "paragraph", commentText: job("paragraph comment", "P"),
      commentAuthor: lazy(() => "Paragraph Author"), runs: lazy(() => [
        text(job("text", " run\t\né "), {
          commentText: job("run comment", "R"), commentAuthor: job("run author", "Run Author"),
        }),
      ]),
    },
  ])]);
  equal(started.length, 4);
  const [short, empty, rich] = source.view.sections[0].view.paragraphs.map((item) => item.view);
  equal(short.text, " shorthand\t\n😀 ");
  equal(empty.text, "");
  equal(empty.runs.length, 0);
  equal(rich.text, " run\t\né ");
  equal(rich.comment.author, "Paragraph Author");
  equal(rich.runs[0].view.comment.author, "Run Author");
  const explicitInlineList = await resolveComponent({ kind: "paragraph", runs: lazy(() => []) });
  equal(explicitInlineList.view.text, "");
  equal(explicitInlineList.view.runs.length, 0);
  const xml = read(await pack(source), "word/document.xml");
  match(xml, /<w:tab\b/);
  match(xml, /<w:br\b/);
  match(xml, /xml:space="preserve"/);
  await rejects(
    () => resolveComponent({ kind: "paragraph", runs: [], text: "" }),
    /either text or runs/, undefined, "paragraph",
  );
}

export async function paragraphAndRunFormatVariants() {
  const tabStops = [
    { position: -12, alignment: "clear" },
    ...["left", "center", "right", "decimal", "bar", "start", "end"].map((alignment, i) => ({
      alignment, position: 8 * i,
      leader: ["none", "dot", "hyphen", "underscore", "heavy", "middleDot"][i],
    })),
  ];
  const paragraphFormat = {
    bidi: false, heading: 2, alignment: "justifyHigh", spacingBefore: 1, spacingAfter: 2,
    leftIndent: -3, rightIndent: 4, firstLineIndent: -5, keepTogether: false, keepWithNext: true,
    pageBreakBefore: false, widowControl: true, suppressLineNumbers: false,
    contextualSpacing: true, mirrorIndents: false, snapToGrid: false, tabStops,
    lineHeight: { kind: "minimum", value: 10 },
  };
  const runFormat = {
    bold: false, italic: true, underline: true, font: "Latin", size: 13,
    color: { kind: "theme", name: "accent5", tint: 32, shade: 144 },
    superscript: true, subscript: false, strike: false, allCaps: false, smallCaps: true,
    snapToGrid: false, language: { latin: "en-US", eastAsia: "ja-JP", bidi: "he-IL" },
    boldComplexScript: true, italicComplexScript: false, sizeComplexScript: 14, rtl: true,
    complexScript: false, fontEastAsia: "East", fontComplexScript: "Complex",
  };
  const source = await document([section([{
    kind: "paragraph", format: future(async () => paragraphFormat), runs: [
      text("formatted", { format: lazy(() => runFormat) }),
      text("automatic", { format: { underline: false, color: "auto" } }),
      text("literal", { format: { underline: "wave", color: [16, 32, 64] } }),
    ],
  }])]);
  const view = source.view.sections[0].view.paragraphs[0].view;
  equal(view.format.tabStops[0].position, -12);
  equal(view.format.tabStops[0].alignment, "clear");
  equal(view.format.lineHeight.kind, "minimum");
  equal(view.runs[0].view.format.underline, "single");
  equal(view.runs[0].view.format.language.eastAsia, "ja-JP");
  equal(view.runs[0].view.format.complexScript, false);
  equal(view.runs[1].view.format.underline, "none");
  equal(view.runs[2].view.format.color[2], 64);
  const xml = read(await pack(source), "word/document.xml");
  for (const expected of [
    /<w:tab\b[^>]*w:pos="-240"/, /<w:spacing\b[^>]*w:lineRule="atLeast"/,
    /<w:b w:val="0"/, /<w:bCs\b/, /<w:iCs w:val="0"/, /<w:szCs w:val="28"/,
    /w:themeColor="accent5"/, /w:themeTint="20"/, /w:themeShade="90"/,
    /<w:color w:val="auto"/, /<w:color w:val="102040"/,
  ]) match(xml, expected);
  await rejects(
    () => validateValue({ kind: "paragraphFormat", value: { lineHeight: { kind: "exact", value: 10 }, lineSpacing: 1 } }),
    /mutually exclusive/i, "invalid_content", "lineHeight",
  );
  await rejects(
    () => validateValue({ kind: "runFormat", value: { sizeComplexScript: -1 } }),
    /negative/i, "invalid_measurement", "sizeComplexScript",
  );
  await rejects(
    () => validateValue({ kind: "runFormat", value: { language: { eastAsia: "\ud800" } } }),
    /malformed UTF-16/i, undefined, "value.language.eastAsia",
  );
  const wrongStyle = await document([section([paragraph("wrong", { format: { style: "OnlyChar" } })])], {
    styles: [{ kind: "character", name: "OnlyChar" }],
  });
  await rejects(() => wrongStyle.toDocx(), /style/i, "invalid_content");
  for (const alignment of ["left", "center", "right", "justify", "distribute", "justifyMedium", "justifyLow", "thaiJustify"]) {
    equal((await resolveComponent(paragraph("align", { format: { alignment } }))).view.format.alignment, alignment);
  }
  for (const underline of ["none", "single", "double", "dotted", "dash", "wave", "words"]) {
    equal((await resolveComponent(text("underline", { format: { underline } }))).view.format.underline, underline);
  }
  for (const name of [
    "dark1", "light1", "dark2", "light2", "accent1", "accent2", "accent3", "accent4",
    "accent5", "accent6", "hyperlink", "followedHyperlink", "background1", "text1", "background2", "text2",
  ]) {
    const run = await resolveComponent(text("theme", { format: { color: { kind: "theme", name } } }));
    equal(run.view.format.color.name, name);
  }
}

export async function documentDefaultSettings() {
  const defaults = {
    run: { bold: true, font: "Defaults Font", size: 11 },
    paragraph: { alignment: "right", keepWithNext: false, lineHeight: { kind: "multiplier", value: 1.2 } },
    trackRevisions: true, mirrorMargins: true, gutterAtTop: false,
    footnotePosition: "beneathText", endnotePosition: "sectionEnd",
    footnoteNumbering: { format: "upperLetter", start: 2, restart: "eachPage" },
    endnoteNumbering: { format: "lowerRoman", start: 3, restart: "eachSection" },
  };
  const source = await document([section([paragraph("not direct formatting")])], { defaults: lazy(() => defaults) });
  equal(source.view.defaults.run.font, "Defaults Font");
  equal(source.view.defaults.run.size, 11);
  equal(source.view.defaults.paragraph.alignment, "right");
  equal(source.view.defaults.paragraph.keepWithNext, false);
  equal(source.view.defaults.paragraph.lineHeight.value, 1.2);
  equal(source.view.defaults.trackRevisions, true);
  equal(source.view.defaults.mirrorMargins, true);
  equal(source.view.defaults.footnoteNumbering.start, 2);
  equal(source.view.defaults.footnoteNumbering.restart, "eachPage");
  equal(source.view.defaults.endnoteNumbering.start, 3);
  equal(source.view.defaults.endnoteNumbering.restart, "eachSection");
  equal(source.view.defaults.gutterAtTop, false);
  equal(source.view.sections[0].view.paragraphs[0].view.runs[0].view.format.bold, undefined);
  const patch = await document([], { defaults: { run: { bold: false }, trackRevisions: false } });
  const merged = await compileTemplate(patch, { bytes: source.toDocx() });
  const body = read(merged, "word/document.xml");
  equal(/<w:(?:pPr|rPr)\b/.test(body), false);
  const styles = read(merged, "word/styles.xml");
  match(styles, /<w:docDefaults>/);
  match(styles, /<w:rFonts\b[^>]*Defaults Font/);
  match(styles, /<w:b w:val="0"/);
  const settings = read(merged, "word/settings.xml");
  for (const expected of [
    /<w:trackRevisions w:val="0"/, /<w:mirrorMargins\b/, /<w:gutterAtTop w:val="0"/,
    /<w:pos w:val="beneathText"/, /<w:pos w:val="sectEnd"/,
    /<w:numRestart w:val="eachPage"/, /<w:numRestart w:val="eachSect"/,
  ]) match(settings, expected);
  await rejects(
    () => validateValue({ kind: "documentDefaults", value: { paragraph: { heading: 1 } } }),
    /heading/i, "invalid_content",
  );
  for (const format of ["decimal", "lowerRoman", "upperRoman", "lowerLetter", "upperLetter"]) {
    const doc = await document([], { defaults: {
      footnotePosition: "pageBottom", endnotePosition: "documentEnd",
      endnoteNumbering: { format, restart: "continuous" },
    } });
    equal(doc.view.defaults.endnoteNumbering.format, format);
    equal(doc.view.defaults.endnotePosition, "documentEnd");
    equal(doc.view.defaults.footnotePosition, "pageBottom");
  }
}

export async function resolvedBlockViewsAndReuse() {
  let calls = 0;
  const child = await resolveComponent(paragraph(lazy(() => { calls++; return "reused"; }), {
    format: { alignment: "center" }, numbering: { id: 88, level: 0 },
  }));
  for (let i = 0; i < 2; i++) {
    const doc = await document([section([
      { kind: "resolved", value: child },
      paragraph("hidden", { condition: false }),
      { kind: "table", rows: [{ kind: "tableRow", children: [{ kind: "tableCell", children: [paragraph("nested")] }] }] },
    ])], { numbering: [{ id: 88, levels: [{ level: 0, start: 1, format: "decimal", text: "%1" }] }] });
    const view = doc.view.sections[0].view;
    equal(view.blocks.length, 2);
    equal(view.paragraphs.length, 1);
    equal(view.paragraphs[0].view.text, "reused");
    equal(view.paragraphs[0].view.numbering.id, 88);
    equal(view.paragraphs[0].view.format.alignment, "center");
    const copy = doc.toDocx();
    copy[0] = 0;
    equal(doc.toDocx()[0], 0x50);
    await validateValue({ kind: "resolvedDocument", value: doc });
  }
  equal(calls, 1);
  await validateValue({ kind: "resolvedParagraph", value: child });
}

export async function resolvedInlineViewsConstructorsAndReuse() {
  const deleted = await resolveComponent(text("removed", { revision: { kind: { kind: "deletion" }, author: "Reviewer" } }));
  const inserted = await resolveComponent(text("inserted", { revision: { kind: { kind: "insertion" }, author: "Reviewer" } }));
  const movedFrom = await resolveComponent(text("before", {
    revision: { kind: { kind: "moveSource", name: "MoveA" }, author: "Reviewer", date: timestamp },
  }));
  const movedTo = await resolveComponent(text("after", {
    revision: { kind: { kind: "moveDestination", name: "MoveA" }, author: "Reviewer", date: timestamp },
  }));
  const tab = await createResolvedInline({ kind: "tab", format: { bold: false }, comment: { text: "Tab comment" } });
  const line = await createResolvedInline({ kind: "break", breakType: "line" });
  const page = await createResolvedInline({ kind: "break", breakType: "page" });
  equal(deleted.view.text, "removed");
  equal(deleted.view.displayText, "");
  equal(inserted.view.displayText, "inserted");
  equal(inserted.view.revision.author, "Reviewer");
  equal(movedFrom.view.displayText, "");
  equal(movedTo.view.displayText, "after");
  equal(tab.view.format.bold, false);
  equal(tab.view.comment.text, "Tab comment");
  equal(page.view.breakType, "page");
  const handles = [deleted, inserted, tab, line, page];
  equal(await inlineText(handles), "inserted\t\n");
  const reuse = await resolveComponent({ kind: "paragraph", runs: handles.map((value) => ({ kind: "resolved", value })) });
  equal(reuse.view.text, "inserted\t\n");
  await validateValue({ kind: "resolvedInline", value: tab });
  // Rust permits direct construction before standalone validation.
  const invalid = await createResolvedInline({ kind: "text", text: "raw", format: { size: -1 } });
  await rejects(() => validateValue({ kind: "resolvedInline", value: invalid }),
    /negative/i, "invalid_measurement", "size");
}

export async function standaloneValidationAndDiagnostics() {
  await validateValue({ kind: "runFormat", value: { style: "OnlyCheckedInContext" } });
  await rejects(
    () => validateValue({ kind: "paragraphFormat", value: { tabStops: [{ position: -2 }, { position: -2.001 }] } }),
    /collide/i, "invalid_content", "tabStops[1].position",
  );
  await rejects(
    () => resolveComponent({ kind: "paragraph", runs: [text("\u0000")] }),
    /XML/i, "invalid_xml", "paragraph.runs[0].text",
  );
  const invalidRun = await createResolvedInline({ kind: "text", text: "raw", format: { size: -1 } });
  const rawParagraph = { kind: "paragraph", runs: [{ kind: "resolved", value: invalidRun }] };
  const resolved = await resolveComponent(rawParagraph);
  await rejects(() => validateValue({ kind: "resolvedParagraph", value: resolved }), /negative/i, "invalid_measurement");
  await rejects(() => document([section([rawParagraph])]),
    /negative/i, "invalid_measurement", "sections[0].children[0].runs[0].format.size");
  await rejects(() => resolveDocument({ sections: [] }, { unknownOption: true }), /unknown/i, undefined);
  await validateValue({ kind: "paragraphFormat", value: { spacingBefore: 2 } });
  equal((await document([section([paragraph("after invalid inputs")])])).view.sections.length, 1);
}

export async function layoutParityFixture() {
  const doc = await document([section([{
    kind: "paragraph", format: { alignment: "center", spacingAfter: 6, bidi: false },
    runs: [text("Deterministic\tlayout\n😀", { format: { bold: false, color: [16, 32, 48] } })],
  }], { properties: {
    pageMargins: { left: 914400 }, lineNumbering: { countBy: 2, restart: "newSection" },
  } })], {
    metadata: { title: "Text layout", customBooleanProperties: { Reviewed: false } },
    defaults: { run: { size: 11 }, mirrorMargins: true },
  });
  return doc.toDocx();
}

await documentFieldsAndCustomMetadata();
await optionalDeferredNone();
await sectionAndStoryComposition();
await sectionGeometryAndInheritedPatches();
await paragraphAndRunComposition();
await paragraphAndRunFormatVariants();
await documentDefaultSettings();
await resolvedBlockViewsAndReuse();
await resolvedInlineViewsConstructorsAndReuse();
await standaloneValidationAndDiagnostics();
console.log("text-layout: owned authoring, resolved-view, and validation cases passed");
