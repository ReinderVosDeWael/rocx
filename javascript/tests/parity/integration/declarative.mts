import {
  declarative as d, deferred, future, generateDocx, property, resolveDocument,
  type DocumentProgram, type InlineChildren, type ParagraphFormat, type RunFormat,
  type SectionDeclaration, type TableRowDeclaration,
} from "rocx-docx";

const timestamp = "2026-11-21T00:00:00Z";

export async function declarativeFixtures(png: Uint8Array) {
  checkFactories(png);
  // Independent raw program: include explicit falsy values, nested table and
  // rich inline content, so byte equality catches omitted/rewritten fields.
  const raw = {
    timestamp,
    metadata: { title: "" },
    sections: [{
      kind: "section",
      properties: { titlePage: false },
      children: [
        { kind: "paragraph", text: "Café 東京", format: { keepWithNext: false, spacingAfter: 0 } },
        { kind: "paragraph", runs: [
          { kind: "text", text: "", format: { bold: false } },
          { kind: "hyperlink", target: { kind: "external", url: "https://example.org/report" },
            children: [{ kind: "text", text: "Report", format: { italic: true } }] },
          { kind: "tab" },
          { kind: "image", data: png, altText: { description: "" }, transformation: { width: 36 } },
        ] },
        { kind: "table", properties: { layout: "fixed", columnWidths: [2400] }, rows: [
          { kind: "tableRow", properties: { repeatHeader: false }, children: [
            { kind: "tableCell", properties: { margins: { top: 0 } }, children: [
              { kind: "paragraph", text: "Cell" },
              { kind: "table", rows: [
                { kind: "tableRow", children: [
                  { kind: "tableCell", children: [{ kind: "paragraph", text: "Nested" }] },
                ] },
              ] },
            ] },
          ] },
        ] },
      ],
    }, { kind: "section", children: [{ kind: "paragraph", text: "Raw second section" }] }],
  } as const satisfies DocumentProgram;

  const calls = new Map<string, number>();
  function produce<T>(name: string, value: T) {
    return future(async () => {
      await Promise.resolve();
      calls.set(name, (calls.get(name) ?? 0) + 1);
      return value;
    });
  }

  const rich = raw.sections[0].children[1].runs;
  const mixedRuns: InlineChildren = [
    d.TextRun(property("empty text", produce("empty text", "")).reference(), {
      format: deferred(produce<RunFormat>("run format", { bold: false })),
    }),
    d.Hyperlink(rich[1].target, [d.TextRun("Report", { format: { italic: true } })]),
    rich[2],
    d.ImageRun(png, { altText: { description: "" }, transformation: { width: 36 } }),
  ];
  const row = raw.sections[0].children[2].rows[0];
  const cell = row.children[0];
  const factory = d.Document([
    d.Section([
      d.Paragraph.Text(deferred(produce("paragraph text", "Café 東京")), {
        format: produce<ParagraphFormat>("paragraph format", { keepWithNext: false, spacingAfter: 0 }),
      }),
      d.Paragraph.Runs(produce("inline children", mixedRuns)),
      d.Table(produce<readonly TableRowDeclaration[]>("table rows", [
        d.TableRow([d.TableCell([d.Paragraph("Cell"), cell.children[1]], {
          properties: { margins: { top: 0 } },
        })], { properties: { repeatHeader: false } }),
      ]), { properties: { layout: "fixed", columnWidths: [2400] } }),
    ], { properties: { titlePage: false } }),
    // A raw section can contain a factory-produced block, too.
    { kind: "section", children: [d.Paragraph("Raw second section")] },
  ], { timestamp: produce("timestamp", timestamp), metadata: { title: "" } });

  if (calls.size !== 0) throw new Error("declarative factories evaluated a producer eagerly");
  const resolved = await resolveDocument(factory);
  const section = resolved.view.sections[0].view;
  const [plain, paragraph] = section.paragraphs.map(({ view }) => view);
  const run = paragraph.runs[0].view;
  if (plain.text !== "Café 東京" || plain.format.keepWithNext !== false ||
      plain.format.spacingAfter !== 0 || run.kind !== "text" || run.text !== "" ||
      run.format.bold !== false || section.properties?.titlePage !== false ||
      resolved.view.metadata.title !== "" || resolved.view.timestamp !== timestamp) {
    throw new Error("declarative resolution changed deferred or falsy values");
  }
  const expectedCalls = ["empty text", "run format", "paragraph text", "paragraph format",
    "inline children", "table rows", "timestamp"];
  if (calls.size !== expectedCalls.length || expectedCalls.some((name) => calls.get(name) !== 1)) {
    throw new Error(`expected one call per producer, got ${JSON.stringify([...calls])}`);
  }
  const facade = resolved.toDocx();
  const record = await generateDocx(raw);
  if (facade.length === 0 || facade.length !== record.length ||
      !facade.every((byte, index) => byte === record[index])) {
    throw new Error("declarative and raw program DOCX bytes differ");
  }
  return { declarative: facade, "declarative-record": record, "legacy-workflow": await legacyWorkflow(png) };
}

function checkFactories(png: Uint8Array) {
  const run = { kind: "text", text: "sample" } as const;
  const block = { kind: "paragraph", text: "sample" } as const;
  const section = { kind: "section", children: [block] } as const;
  const cell = { kind: "tableCell", children: [block] } as const;
  const row = { kind: "tableRow", children: [cell] } as const;
  const target = { kind: "external", url: "https://example.org/" } as const;
  const note = { kind: "footnote", id: 1 } as const;
  const range = { kind: "bookmark", name: "Sample" } as const;
  const shape = { geometry: "rectangle" } as const;
  const chart = { width: 100, height: 50, categories: ["A"], values: [2] };
  const instruction = { kind: "page" } as const;
  // The mapped keys enforce coverage when the installed .d.ts adds a factory.
  // The runtime key comparison also catches export drift in either entry point.
  const cases = {
    Document: [d.Document([section], { timestamp }), { sections: [section], timestamp }],
    Section: [d.Section([block], { condition: false }), { ...section, condition: false }],
    Header: [d.Header([block], { condition: false }), { kind: "header", children: [block], condition: false }],
    Footer: [d.Footer([block]), { kind: "footer", children: [block] }],
    Paragraph: [d.Paragraph([run]), { kind: "paragraph", runs: [run] }],
    TextRun: [d.TextRun("sample"), run],
    Table: [d.Table([row]), { kind: "table", rows: [row] }],
    TableRow: [d.TableRow([cell]), row],
    TableCell: [d.TableCell([block]), cell],
    ImageRun: [d.ImageRun(png), { kind: "image", data: png }],
    ShapeRun: [d.ShapeRun(shape), { kind: "shape", properties: shape }],
    ColumnChartRun: [d.ColumnChartRun(chart), { kind: "columnChart", properties: chart }],
    InlineFraction: [d.InlineFraction("1", "2"), { kind: "fraction", numerator: "1", denominator: "2" }],
    Tab: [d.Tab(), { kind: "tab" }],
    Break: [d.Break({ breakType: "column" }), { kind: "break", breakType: "column" }],
    Hyperlink: [d.Hyperlink(target, [run], { properties: { history: false } }),
      { kind: "hyperlink", target, children: [run], properties: { history: false } }],
    Field: [d.Field(instruction, [run]), { kind: "field", instruction, children: [run] }],
    Note: [d.Note(note, [block]), { kind: "note", target: note, children: [block] }],
    NoteReference: [d.NoteReference(note), { kind: "noteReference", target: note }],
    Comment: [d.Comment([block], { metadata: { author: "Ada" } }),
      { kind: "comment", children: [block], metadata: { author: "Ada" } }],
    RangeStart: [d.RangeStart("sample", range), { kind: "rangeStart", id: "sample", range }],
    RangeEnd: [d.RangeEnd("sample"), { kind: "rangeEnd", id: "sample" }],
    InlineContentControl: [d.InlineContentControl([run]), { kind: "contentControl", children: [run] }],
    BlockContentControl: [d.BlockContentControl([block]), { kind: "contentControl", children: [block] }],
    ParagraphStyleDefinition: [d.ParagraphStyleDefinition("P", { keepTogether: false }),
      { kind: "paragraph", name: "P", keepTogether: false }],
    CharacterStyleDefinition: [d.CharacterStyleDefinition("C"), { kind: "character", name: "C" }],
    NumberingStyleDefinition: [d.NumberingStyleDefinition("N", 3), { kind: "numbering", name: "N", numberingId: 3 }],
    TableStyleDefinition: [d.TableStyleDefinition("T"), { kind: "table", name: "T" }],
  } satisfies { [K in keyof typeof d]: readonly [ReturnType<(typeof d)[K]>, ReturnType<(typeof d)[K]>] };

  function equal(actual: unknown, expected: unknown): boolean {
    if (Object.is(actual, expected)) return true;
    if (!actual || !expected || typeof actual !== "object" || typeof expected !== "object" ||
        Object.getPrototypeOf(actual) !== Object.getPrototypeOf(expected)) return false;
    return Object.keys(actual).length === Object.keys(expected).length &&
      Object.entries(expected).every(([key, value]) =>
        Object.hasOwn(actual, key) && equal((actual as Record<string, unknown>)[key], value));
  }
  if (!equal(Object.keys(d).sort(), Object.keys(cases).sort())) {
    throw new Error(`declarative namespace drift: ${Object.keys(d).sort().join(", ")}`);
  }
  for (const [name, [actual, expected]] of Object.entries(cases)) {
    if (!equal(actual, expected)) throw new Error(`declarative.${name} changed its declaration`);
  }
}

// Keep the original input form as a separate fixture after migrating the
// published workflow. This uses the same native oracles as the new example.
function legacyWorkflow(png: Uint8Array) {
  return generateDocx({
    blocks: [
      { kind: "paragraph", alignment: "center", runs: [
        { kind: "text", text: "JavaScript authoring: ", bold: true, italic: false },
        { kind: "text", text: "Café 東京", italic: true },
      ] },
      { kind: "table", rows: [
        ["Item", "Details"],
        ["Pages", { blocks: [
          { kind: "paragraph", runs: [
            { kind: "text", text: "2", bold: true },
            { kind: "tab" },
            { kind: "hyperlink", target: "https://example.org/report", runs: [
              { kind: "text", text: "Report details", italic: true },
            ] },
            { kind: "break", breakType: "line" },
            { kind: "text", text: "Verified" },
          ] },
          { kind: "table", rows: [["Nested detail"]] },
        ] }],
      ] },
      { kind: "paragraph", runs: [
        { kind: "image", bytes: png, altText: "JavaScript authoring sample image", width: 72, height: 36 },
      ] },
    ],
  }, { timestamp });
}

// This receives no calls: compiled in both installed TS module modes.
function invalidInputs(section: SectionDeclaration) {
  // @ts-expect-error a document needs a list of sections, not blocks
  d.Document([d.Paragraph("not a section")]);
  // @ts-expect-error options cannot override the positional sections
  d.Document([section], { sections: [] });
  // @ts-expect-error inline declarations are not section children
  d.Section([d.TextRun("not a block")]);
  // @ts-expect-error paragraph payload does not accept numbers
  d.Paragraph(12);
  // @ts-expect-error paragraph options cannot supply the other payload
  d.Paragraph("text", { runs: [] });
  // @ts-expect-error deferred payloads need Paragraph.Text or Paragraph.Runs
  d.Paragraph(future(async () => "select a payload"));
  // @ts-expect-error Paragraph.Text does not accept runs
  d.Paragraph.Text([d.TextRun("not text")]);
  // @ts-expect-error Paragraph.Runs does not accept text
  d.Paragraph.Runs("not runs");
  // @ts-expect-error a Promise is not a deferred token
  d.TextRun(Promise.resolve("use future instead"));
  // @ts-expect-error a text run doesn't accept paragraph formatting
  d.TextRun("text", { format: { keepWithNext: true } });
  // @ts-expect-error tables require rows, not cells
  d.Table([d.TableCell([])]);
  // @ts-expect-error rows require cells, not paragraphs
  d.TableRow([d.Paragraph("not a cell")]);
  // @ts-expect-error cells require blocks, not runs
  d.TableCell([d.TextRun("not a block")]);
  // @ts-expect-error hyperlinks accept inline children, not blocks
  d.Hyperlink({ kind: "external", url: "https://example.org/" }, [d.Paragraph("not inline")]);
  // @ts-expect-error image data is not a path
  d.ImageRun("sample.png");
}
