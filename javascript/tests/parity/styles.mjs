import {
  compileTemplate, future, resolveComponent, resolveDocument, validateAuthoringId, validateValue,
} from "rocx-docx";

const timestamp = "2022-02-03T04:05:06Z";
const program = (fields = {}) => ({ sections: [], timestamp, ...fields });
const empty = () => resolveDocument(program());
const part = (plan, path) => {
  const bytes = plan.part(path);
  if (!bytes) throw new Error(`missing package part ${path}`);
  return new TextDecoder().decode(bytes);
};
const equal = (actual, expected) => {
  if (actual !== expected) throw new Error(`expected ${expected}, got ${actual}`);
};
const equalBytes = (actual, expected) => {
  if (!actual || !expected || actual.length !== expected.length ||
      actual.some((byte, index) => byte !== expected[index])) {
    throw new Error("untouched part payload differs");
  }
};
const match = (actual, expected) => {
  if (!expected.test(actual)) throw new Error(`missing ${expected} in ${actual}`);
};
const node = (xml, tag, attribute, value) => {
  const candidates = xml.matchAll(new RegExp(`<w:${tag}\\b[^>]*>[\\s\\S]*?</w:${tag}>`, "g"));
  for (const [candidate] of candidates) {
    if (candidate.split(">", 1)[0].includes(`w:${attribute}="${value}"`)) return candidate;
  }
  throw new Error(`missing w:${tag} with w:${attribute}="${value}"`);
};
const rejects = async (call, message, path, rule) => {
  try {
    await call();
  } catch (error) {
    match(error.message, message);
    if (path !== undefined) equal(error.path, path);
    if (rule !== undefined) equal(error.rule, rule);
    return error;
  }
  throw new Error(`expected rejection: ${message}`);
};

export async function stylesAndConditionalRegions() {
  const region = {
    bold: true, italic: false, underline: true, font: "Latin", fontSize: 10, color: [1, 2, 3],
    background: [4, 5, 6], alignment: "center", spacingBefore: 3, spacingAfter: 6,
    lineHeight: { kind: "exact", value: 14 }, tabStops: [{ alignment: "decimal", position: 18 }],
    bidi: true, language: { latin: "en-US", eastAsia: "ja-JP", bidi: "he-IL" },
    boldComplexScript: false, italicComplexScript: true, sizeComplexScript: 13, rtl: true,
    complexScript: true, fontEastAsia: "East", fontComplexScript: "Complex",
  };
  const regions = {
    wholeTable: null, firstRow: "firstRow", lastRow: "lastRow", firstColumn: "firstCol",
    lastColumn: "lastCol", banding1Row: "band1Horz", banding2Row: "band2Horz",
    banding1Column: "band1Vert", banding2Column: "band2Vert", topLeftCell: "nwCell",
    topRightCell: "neCell", bottomLeftCell: "swCell", bottomRightCell: "seCell",
  };
  const { background: _background, ...text } = region;
  const definitions = [
    {
      ...text, kind: "paragraph", name: "Pair Paragraph", baseStyle: "Forward Paragraph",
      nextParagraphStyle: "Forward Paragraph", linkedStyle: "Pair Character", leftIndent: 0.5,
      rightIndent: -0.1, firstLineIndent: 0.1, keepTogether: false, keepWithNext: true,
      pageBreakBefore: false, widowControl: true, suppressLineNumbers: false, contextualSpacing: true,
      mirrorIndents: false, snapToGrid: true,
    },
    { kind: "paragraph", name: "Forward Paragraph", lineSpacing: 1.5 },
    {
      kind: "character", name: "Pair Character", baseStyle: "Forward Character",
      linkedStyle: "Pair Paragraph", format: { underline: "double", size: 15 },
    },
    { kind: "character", name: "Forward Character" },
    {
      kind: "table", name: "Regional", baseStyle: "Table Base",
      ...Object.fromEntries(Object.keys(regions).map((key) => [key, region])),
    },
    { kind: "table", name: "Table Base" },
  ];
  let styleCalls = 0;
  const source = await resolveDocument(program({
    styles: future(async () => { styleCalls++; return definitions; }),
    sections: [{ kind: "section", children: [{
      kind: "table", properties: { style: "Regional", look: { firstRow: true } },
      rows: [{ kind: "tableRow", children: [{ kind: "tableCell", children: [
        { kind: "paragraph", text: "conditional formatting" },
      ] }] }],
    }] }],
  }));
  const [paragraph, next, character, fallback, table] = source.view.styles;
  equal(paragraph.keepTogether, false);
  equal(paragraph.leftIndent, 0.5);
  equal(paragraph.tabStops[0].alignment, "decimal");
  equal(paragraph.lineHeight.kind, "exact");
  equal(next.lineSpacing, 1.5);
  equal(character.format.underline, "double");
  equal(Object.keys(fallback.format).length, 0);
  for (const key of Object.keys(regions)) {
    equal(table[key].language.eastAsia, "ja-JP");
    equal(table[key].background[2], 6);
    equal(table[key].boldComplexScript, false);
    equal(table[key].lineHeight.value, 14);
  }
  equal(styleCalls, 1);
  const original = source.toDocx();
  const patch = await resolveDocument(program({
    styles: [{ kind: "paragraph", name: "Pair Paragraph", bold: false }],
  }));
  const plan = await compileTemplate(patch, { bytes: original });
  const unpatched = await compileTemplate(await empty(), { bytes: original });
  for (const path of ["word/document.xml", "word/numbering.xml", "word/fontTable.xml"]) {
    equalBytes(plan.part(path), unpatched.part(path));
  }
  const xml = part(plan, "word/styles.xml");
  const patched = node(xml, "style", "styleId", "PairParagraph");
  match(patched, /<w:basedOn w:val="ForwardParagraph"/);
  match(patched, /<w:next w:val="ForwardParagraph"/);
  match(patched, /<w:link w:val="PairCharacter"/);
  match(patched, /<w:b w:val="0"/);
  match(patched, /<w:ind\b[^>]*w:left="720"/);
  match(node(xml, "style", "styleId", "PairCharacter"), /<w:link w:val="PairParagraph"/);
  const styledTable = node(xml, "style", "styleId", "Regional");
  match(styledTable, /<w:basedOn w:val="TableBase"/);
  match(part(plan, "word/document.xml"), /<w:tblStyle w:val="Regional"/);
  for (const wire of Object.values(regions)) {
    const conditional = wire
      ? node(styledTable, "tblStylePr", "type", wire)
      : styledTable.split("<w:tblStylePr", 1)[0];
    match(conditional, /<w:spacing\b[^>]*w:line="280"/);
    match(conditional, /<w:tab\b[^>]*w:pos="360"/);
    match(conditional, /<w:lang\b[^>]*w:eastAsia="ja-JP"/);
    match(conditional, /<w:bCs w:val="0"/);
    match(conditional, /<w:shd\b[^>]*w:fill="040506"/);
  }
  equal(styleCalls, 1);
  for (const [style, message] of [
    [{ ...definitions[0], linkedStyle: "Forward Paragraph" }, /character/i],
    [{ ...definitions[0], nextParagraphStyle: "Missing" }, /Missing/],
    [{ ...definitions[0], baseStyle: "Pair Paragraph" }, /inheritance cycle/],
    [{ ...definitions[2], format: { style: "Forward Character" } }, /cannot contain a run style/],
  ]) {
    const doc = await resolveDocument(program({
      styles: definitions.map((entry) => entry.name === style.name ? style : entry),
    }));
    await rejects(() => doc.toDocx(), message);
  }
  const badRegion = await resolveDocument(program({
    styles: [{ kind: "table", name: "Invalid region", firstRow: { sizeComplexScript: -1 } }],
  }));
  await rejects(() => badRegion.toDocx(), /size_complex_script cannot be negative/);
  for (const fontSize of [NaN, Infinity, -Infinity]) {
    const badSize = await resolveDocument(program({
      styles: [{ kind: "paragraph", name: "Invalid size", fontSize }],
    }));
    await rejects(() => badSize.toDocx(), /must be finite|cannot be negative/);
  }
}

export async function numberingInstancesAndOverrides() {
  const alignments = ["left", "center", "right", "start", "end"];
  const suffixes = ["tab", "space", "nothing"];
  const levels = alignments.map((alignment, level) => ({
    level, start: 1, format: "lowerRoman", text: "%1.", isLegal: level !== 0, restart: 0,
    alignment, suffix: suffixes[level % suffixes.length], labelFormat: { bold: true },
    paragraphFormat: { leftIndent: 18 }, paragraphStyle: "Outline Paragraph",
  }));
  const replacement = { ...levels[0], start: 4, format: "bullet", text: "•" };
  const numbering = [
    { id: 602, abstractId: 901, inheritStyle: "Outline" },
    { id: 600, abstractId: 900, levels },
    { id: 601, abstractId: 900, levels, startOverrides: [{ level: 0, start: 0 }],
      levelOverrides: [replacement] },
  ];
  let referenceCalls = 0;
  const paragraph = await resolveComponent({
    kind: "paragraph", text: "reference, not a calculated label",
    numbering: future(async () => { referenceCalls++; return { id: 602, level: 1 }; }),
  });
  equal(paragraph.view.numbering.id, 602);
  equal(paragraph.view.numbering.level, 1);
  equal(paragraph.view.text, "reference, not a calculated label");
  const doc = await resolveDocument(program({
    styles: [{ kind: "numbering", name: "Outline", numberingId: 600 },
      { kind: "paragraph", name: "Outline Paragraph" }],
    numbering: future(async () => numbering),
    sections: [{ kind: "section", children: [{ kind: "resolved", value: paragraph }] }],
  }));
  equal(doc.view.sections[0].view.paragraphs[0].view.numbering.level, 1);
  equal(doc.view.numbering[0].inheritStyle, "Outline");
  equal(doc.view.numbering[0].levels.length, 0);
  equal(doc.view.numbering[2].startOverrides[0].start, 0);
  equal(doc.view.numbering[2].levelOverrides[0].text, "•");
  equal(doc.view.numbering[1].levels[0].isLegal, false);
  equal(doc.view.styles[0].numberingId, 600);
  const blank = await empty();
  const plan = await compileTemplate(doc, { bytes: blank.toDocx() });
  const xml = part(plan, "word/numbering.xml");
  const abstract = node(xml, "abstractNum", "abstractNumId", 900);
  match(abstract, /<w:styleLink w:val="Outline"/);
  match(abstract, /<w:pStyle w:val="OutlineParagraph"/);
  match(abstract, /<w:ind\b[^>]*w:left="360"/);
  match(abstract, /<w:b(?:\/>| w:val="1")/);
  match(abstract, /<w:isLgl w:val="0"/);
  match(abstract, /<w:lvlRestart w:val="0"/);
  equal((abstract.match(/<w:start w:val="1"/g) || []).length, 5);
  equal((abstract.match(/<w:numFmt w:val="lowerRoman"/g) || []).length, 5);
  for (const token of alignments) match(abstract, new RegExp(`<w:lvlJc w:val="${token}"`));
  for (const token of suffixes) match(abstract, new RegExp(`<w:suff w:val="${token}"`));
  const concrete = node(xml, "num", "numId", 601);
  match(concrete, /<w:abstractNumId w:val="900"/);
  match(concrete, /<w:lvlOverride w:ilvl="0"><w:startOverride w:val="0"\/><w:lvl w:ilvl="0">/);
  match(concrete, /<w:numFmt w:val="bullet"/);
  match(node(xml, "abstractNum", "abstractNumId", 901), /<w:numStyleLink w:val="Outline"/);
  match(part(plan, "word/document.xml"), /<w:numPr><w:ilvl w:val="1"\/><w:numId w:val="602"/);
  equal(referenceCalls, 1);

  const existing = await resolveDocument(program({
    numbering: [{ ...numbering[1], levels: [{ level: 0, start: 1, format: "decimal", text: "%1." }] }],
  }));
  const bytes = existing.toDocx();
  await rejects(() => compileTemplate(existing, { bytes }), /numbering ID already exists/);
  const abstractCollision = await resolveDocument(program({
    numbering: [{ id: 603, abstractId: 900, levels: [] }],
  }));
  await rejects(() => compileTemplate(abstractCollision, { bytes }), /abstract numbering ID already exists/);
  const duplicate = await resolveDocument(program({
    numbering: [{ id: 600 }, { id: 600 }],
  }));
  await rejects(() => duplicate.toDocx(), /numbering ID already exists/);
  await rejects(() => resolveDocument(program({
    numbering: [numbering[1], { ...numbering[2], levels: [replacement] }],
  })), /require identical levels/, "numbering[1].levels", "invalid_content");
  for (const target of [0, 604, 602]) {
    const invalid = await resolveDocument(program({
      styles: [{ kind: "numbering", name: "Outline", numberingId: target }], numbering,
    }));
    await rejects(() => invalid.toDocx(), /with local levels/);
  }
}

export async function standaloneValidationAndDiagnostics() {
  const base = { id: 600, levels: [{ level: 0, start: 0, format: "openFormat", text: "%1." }] };
  await validateValue({ kind: "numberingDefinition", value: base });
  const max = 2 ** 32 - 1;
  const limits = await resolveDocument(program({
    numbering: [{
      id: max, abstractId: max, levels: [{ ...base.levels[0], start: max }],
      startOverrides: [{ level: 0, start: max }],
    }],
  }));
  const [numbering] = limits.view.numbering;
  equal(numbering.id, max);
  equal(numbering.abstractId, max);
  equal(numbering.levels[0].start, max);
  equal(numbering.startOverrides[0].start, max);
  // An independent definition need not have its numbering style available.
  await validateValue({ kind: "numberingDefinition", value: { id: 601, inheritStyle: "Outside" } });
  for (const [value, message, path] of [
    [{ ...base, levels: [{ ...base.levels[0], level: 9 }] }, /unique and in/, "levels[0].level"],
    [{ ...base, startOverrides: [{ level: 1, start: 0 }] }, /unique defined level/, "startOverrides[0].level"],
    [{ ...base, levelOverrides: [{ ...base.levels[0], level: 1 }] }, /unique defined level/, "levelOverrides[0].level"],
    [{ ...base, inheritStyle: "Conflict" }, /cannot contain levels/, "inheritStyle"],
    [{ ...base, levels: [{ ...base.levels[0], labelFormat: { style: "Missing" } }] },
      /cannot contain run style/, "levels[0].labelFormat.style"],
  ]) {
    await rejects(() => validateValue({ kind: "numberingDefinition", value }), message, path, "invalid_content");
  }
  for (const [id, message] of [[-1, /safe integer/], [0.5, /safe integer/],
    [max + 1, /outside field bounds/], [Number.MAX_SAFE_INTEGER + 1, /safe integer/]]) {
    const shape = await rejects(() => validateValue({
      kind: "numberingDefinition", value: { ...base, id },
    }), message, "value.id");
    equal(shape.rule, undefined);
  }
  const malformed = await rejects(() => resolveDocument(program({
    numbering: [{ ...base, levels: [{ ...base.levels[0], text: "\ud800" }] }],
  })), /malformed UTF-16/, "numbering[0].levels[0].text");
  equal(malformed.rule, undefined);
  await validateAuthoringId("styleId", "AnId");
  await rejects(() => validateAuthoringId("styleId", ""), /cannot be empty/, undefined, "invalid_content");
}

/** Equivalent to tests/support/{linked_styles,numbering_replacements}.rs with a fixed timestamp. */
export async function styleByteFixtures() {
  const linkedStyles = await resolveDocument(program({
    sections: [{ kind: "section", children: [{
      kind: "paragraph",
      runs: [{ kind: "text", text: "linked", format: { style: "Pair Character", italic: false } }],
      format: { style: "Pair Paragraph" },
    }] }],
    styles: [{ kind: "paragraph", name: "Pair Paragraph", linkedStyle: "Pair Character", bold: true },
      { kind: "character", name: "Pair Character", linkedStyle: "Pair Paragraph", format: { italic: true } }],
  }));
  const numberingReplacements = await resolveDocument(program({
    sections: [{ kind: "section", children: [{
      kind: "paragraph", text: "replacement request, not a calculated label", numbering: { id: 42, level: 0 },
    }] }],
    numbering: [{
      id: 42,
      levels: [0, 1, 2].map((level) => ({ level, start: 1, format: "decimal", text: `%${level + 1}.` })),
      startOverrides: [{ level: 2, start: 0 }, { level: 0, start: 7 }],
      levelOverrides: [
        { level: 1, start: 0, format: "bullet", text: "•" },
        { level: 0, start: 4, format: "upperRoman", text: "Replacement-%1" },
      ],
    }],
  }));
  return { linkedStyles: linkedStyles.toDocx(), numberingReplacements: numberingReplacements.toDocx() };
}

await stylesAndConditionalRegions();
await numberingInstancesAndOverrides();
await standaloneValidationAndDiagnostics();
console.log("styles parity: stylesAndConditionalRegions, numberingInstancesAndOverrides, standaloneValidationAndDiagnostics");
