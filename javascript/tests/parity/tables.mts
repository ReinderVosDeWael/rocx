import {
  deferred, future, lazy, resolveComponent, resolveDocument, validateValue,
  type BlockDeclaration, type BorderSide, type CellBorderSide, type CellTextDirection,
  type ContentControlProperties, type DocumentProgram, type ResolvedTable,
  type TableAlignment, type TableCellDeclaration, type TableCellProperties,
  type TableDeclaration, type TableProperties, type TableRowDeclaration,
  type TableRowProperties, type V, type VerticalAlignment,
} from "rocx-docx";

const timestamp = "2022-02-03T04:05:06Z";
const paragraph = (text: V<string>) => ({ kind: "paragraph", text } as const);
const cell = (children: TableCellDeclaration["children"], properties?: TableCellDeclaration["properties"]) =>
  ({ kind: "tableCell", children, properties } as const);
const row = (children: TableRowDeclaration["children"], properties?: TableRowDeclaration["properties"]) =>
  ({ kind: "tableRow", children, properties } as const);
const table = (rows: TableDeclaration["rows"], properties?: TableDeclaration["properties"]) =>
  ({ kind: "table", rows, properties } as const);
const program = (...children: V<BlockDeclaration>[]): DocumentProgram =>
  ({ timestamp, sections: [{ kind: "section", children }] });

function check(value: unknown, message: string): asserts value {
  if (!value) throw new Error(message);
}

function same(actual: unknown, expected: unknown, message: string) {
  // Object key order is not part of the DTO contract.
  const ordered = (value: unknown): unknown => {
    if (Array.isArray(value)) return value.map(ordered);
    if (value && typeof value === "object") {
      const props = value as Record<string, unknown>;
      return Object.fromEntries(Object.keys(props).sort().map((key) => [key, ordered(props[key])]));
    }
    return value;
  };
  check(JSON.stringify(ordered(actual)) === JSON.stringify(ordered(expected)),
    `${message}: got ${JSON.stringify(actual)}, expected ${JSON.stringify(expected)}`);
}

async function rejects(action: () => unknown, rule: string | undefined, path?: string) {
  try {
    await action();
  } catch (error) {
    check(error instanceof Error, "failure must expose an Error");
    same(Reflect.get(error, "rule"), rule, "diagnostic rule");
    if (path !== undefined) same(Reflect.get(error, "path"), path, "diagnostic path");
    return;
  }
  throw new Error(`expected rejection (${rule ?? "adapter"}, ${path ?? "no path assertion"})`);
}

const tableSides = ["top", "left", "bottom", "right", "insideH", "insideV"] as const satisfies readonly BorderSide[];
const cellSides = ["top", "bottom", "start", "end"] as const satisfies readonly CellBorderSide[];
const directions = [
  "leftToRight", "topToBottomRightToLeft", "bottomToTopLeftToRight",
  "leftToRightVertical", "topToBottomRightToLeftVertical", "topToBottomLeftToRightVertical",
] as const satisfies readonly CellTextDirection[];
const alignments = ["left", "center", "right", "start", "end"] as const satisfies readonly TableAlignment[];
const verticalAlignments = ["top", "center", "bottom", "both"] as const satisfies readonly VerticalAlignment[];
const borders = <S extends string>(sides: readonly S[]) =>
  sides.map((side, index) => ({
    side, sz: 4 + index, color: [12, 34, 56] as const, val: index % 2 === 0 ? "single" : "dashed",
  } as const));
const control = {
  text: { kind: "rich" }, alias: "Row wrapper", tag: "row-control", lock: "controlLocked", id: 101,
} as const satisfies ContentControlProperties;
const cellControl = { ...control, alias: "Cell wrapper", tag: "cell-control", id: 102 };
const properties = {
  layout: "fixed", columnWidths: [1200, 2400, 3600], alignment: "end", style: "Data grid",
  look: {
    firstRow: true, lastRow: false, firstColumn: false, lastColumn: true,
    horizontalBanding: false, verticalBanding: true,
  },
  borders: borders(tableSides),
} as const satisfies TableProperties;

export async function structuredTableComposition() {
  let excludedRows = 0;
  let excludedCells = 0;
  let hiddenFields = 0;
  const hidden = () => lazy(() => { hiddenFields++; throw new Error("guarded field ran"); });
  const nested = table([row([cell([paragraph("nested A")]), cell([paragraph("nested B")])])]);
  const declaration = table(future(async () => [
    future(async () => {
      excludedRows++;
      return { ...row(hidden(), hidden()), condition: false };
    }),
    future(async () => row(deferred(future(async () => [
      future(async () => {
        excludedCells++;
        return { ...cell(hidden(), hidden()), condition: () => false };
      }),
      cell(future(async () => [nested]), future<TableCellProperties>(async () => ({
        gridSpan: 2, vmerge: "restart",
      }))),
      cell([paragraph("right")]),
    ])))),
    row([cell([], { gridSpan: 2, vmerge: "continue" })]),
  ]), lazy(() => properties));
  const resolved: ResolvedTable | undefined = await resolveComponent(declaration);
  check(resolved, "table resolved");
  same([excludedRows, excludedCells, hiddenFields], [1, 1, 0], "only outer producers of guarded items start");
  same(resolved.view.columns, 3, "column count includes spans");
  same(resolved.view.properties, properties, "table properties");
  same(resolved.view.rows.map((r) => r.view.cells.length), [2, 1], "no padding in physical view");
  const [first, second] = resolved.view.rows;
  same(first.view.cells.map((c) => c.view.gridSpan), [2, 1], "explicit and default spans");
  same(first.view.cells[0].view.vmerge, "restart", "merge restart");
  same(second.view.cells[0].view.vmerge, "continue", "merge continuation");
  same(second.view.cells[0].view.blocks, [], "no synthetic paragraphs in physical view");

  const withStyle = {
    ...program({ kind: "resolved", value: resolved }),
    styles: [{ kind: "table", name: "Data grid" }],
  } satisfies DocumentProgram;
  const docx = (await resolveDocument(withStyle)).toDocx();
  same([excludedRows, excludedCells, hiddenFields], [1, 1, 0], "block reuse did not rerun producers");
  const invalid = (widths: readonly number[]) => table([row([cell([]), cell([])])], { columnWidths: widths });
  await rejects(() => resolveComponent(invalid([1000])), "invalid_content", "table.columnWidths");
  await rejects(() => resolveComponent(invalid([1000, 0])), "invalid_content", "table.columnWidths");
  const withSpans = (...spans: number[]) => table([row(spans.map((gridSpan) => cell([], { gridSpan })))]);
  same((await resolveComponent(withSpans(8192, 8192)))?.view.columns, 16384, "column limit is inclusive");
  await rejects(() => resolveComponent(withSpans(16385)), "invalid_content", "table");
  const overflowing = withSpans(0xffff_ffff, 2);
  await rejects(() => resolveComponent(overflowing), "invalid_content", "table");
  await rejects(() => resolveDocument(program(overflowing)), "invalid_content", "sections[0].children[0]");
  const autofit = await resolveComponent(table([row([cell([]), cell([nested], { gridSpan: 2 })])],
    { columnWidths: [0, 0], layout: "autofit" }));
  check(autofit && autofit.view.columns === 3, "autofit accepts unused fixed widths");
  same(autofit.view.properties.columnWidths, [0, 0], "autofit preserves authored widths");
  const autoDocx = (await resolveDocument(program({ kind: "resolved", value: autofit }))).toDocx();
  return { docx, autoDocx };
}

export async function rowCellLayoutAndControls() {
  const margins = { top: 0, start: 1.25, bottom: 2, end: 3 };
  const exact = {
    height: { kind: "exact", points: 18 }, repeatHeader: true, cantSplit: false, contentControl: control,
  } as const satisfies TableRowProperties;
  const atLeast = {
    height: { kind: "atLeast", points: 11.25 }, repeatHeader: false, cantSplit: true,
  } as const satisfies TableRowProperties;
  const declarations = directions.map((textDirection, index) => ({
    textDirection,
    verticalAlignment: verticalAlignments[index % verticalAlignments.length],
    ...(index === 0 ? {
      margins, backgroundColor: [255, 0, 12], borders: borders(cellSides), contentControl: cellControl,
    } as const : {}),
  } satisfies TableCellProperties));
  const layout = table([
    row(declarations.map((prop, index) => cell([paragraph(`direction ${index}`)], lazy(() => prop))), future(async () => exact)),
    row([cell([])], atLeast),
    row([cell([])]),
  ]);
  const snapshot = await resolveComponent(layout);
  check(snapshot, "row and cell layout resolved");
  const rows = snapshot.view.rows;
  same(rows.map((r) => r.view.properties), [exact, atLeast, {}], "row flags, height, wrapper and omission");
  for (const [index, prop] of declarations.entries()) {
    const { blocks, gridSpan, ...view } = rows[0].view.cells[index].view;
    same(view, { borders: [], ...prop }, `cell layout ${index}`);
    same(gridSpan, 1, "implicit span");
    same(blocks.length, 1, "one resolved cell paragraph");
  }
  const docx = (await resolveDocument(program({ kind: "resolved", value: snapshot },
    ...alignments.map((alignment) => table([row([cell([])])], { alignment })),
  ))).toDocx();
  for (const alignment of alignments) {
    const aligned = await resolveComponent(table([row([cell([])])], { alignment }));
    same(aligned?.view.properties.alignment, alignment, "table alignment variant");
  }
  const nonRich = { text: { kind: "plain", multiLine: false } } as const satisfies ContentControlProperties;
  await rejects(() => resolveComponent(row([], { contentControl: nonRich })),
    "invalid_content", "tableRow.properties.contentControl.text");
  await rejects(() => resolveComponent(cell([], { contentControl: nonRich })),
    "invalid_content", "tableCell.properties.contentControl.text");
  const blockControl = { kind: "contentControl", children: [paragraph("wrong context")] } as const;
  // @ts-expect-error a block declaration cannot stand in for wrapper metadata
  const wrongContext = row([], { contentControl: blockControl });
  await rejects(() => resolveComponent(wrongContext), undefined, "tableRow.properties.contentControl");
  const duplicate = table([row([cell([], { contentControl: control })], { contentControl: control })]);
  const document = await resolveDocument(program(duplicate));
  await rejects(() => document.toDocx(), "invalid_content");
  return docx;
}

export async function standaloneAndConditionalComponents() {
  const bomb = () => future(() => { throw new Error("excluded producer started"); });
  same(await resolveComponent({ ...table(bomb(), bomb()), condition: false }), undefined, "excluded table");
  same(await resolveComponent({ ...row(bomb(), bomb()), condition: false }), undefined, "excluded row");
  same(await resolveComponent({ ...cell(bomb(), bomb()), condition: false }), undefined, "excluded cell");
  same(await resolveComponent(table([ { ...row([]), condition: false } ])), undefined, "all rows excluded");
  same((await resolveComponent(row([ { ...cell([]), condition: false } ])))?.view.cells, [], "standalone row");
  const emptyCell = await resolveComponent(cell([{ ...paragraph("invisible"), condition: false }]));
  same(emptyCell?.view.blocks, [], "standalone cell");
  same(emptyCell?.view.borders, [], "omitted cell borders");
  same(emptyCell?.view.gridSpan, 1, "standalone default span");
}

export async function resolvedBlockViewsAndReuse() {
  let starts = 0;
  const authored = paragraph(future(async () => { starts++; return "deep reused text"; }));
  const leaf = await resolveComponent(authored);
  check(leaf, "paragraph handle resolved");
  const source = await resolveComponent(table([row([cell([{ kind: "resolved", value: leaf }])])]));
  check(source, "standalone table handle resolved");
  const reusable = { kind: "resolved", value: source } as const;
  const first = await resolveDocument(program(paragraph("direct"), reusable));
  const second = await resolveDocument(program(paragraph("direct"), reusable));
  const section = first.view.sections[0].view;
  same(section.paragraphs.map((p) => p.view.text), ["direct"], "nested paragraphs not in direct projection");
  const grid = section.blocks[1].view;
  check("rows" in grid, "block hook projected table view");
  const nested = grid.rows[0].view.cells[0].view.blocks[0].view;
  check("runs" in nested, "nested paragraph preserved");
  same(nested.text, "deep reused text", "nested paragraph text");
  same(starts, 1, "repeated reuse does not rerun producers");
  const a = first.toDocx();
  const b = second.toDocx();
  check(a !== b && a.length === b.length && a.every((byte, i) => byte === b[i]), "deterministic bytes");
  a.fill(0);
  check(b[0] === 0x50 && first.toDocx()[0] === 0x50, "output buffers are independent of engine and each other");
}

export async function standaloneValidationAndDiagnostics() {
  await validateValue({ kind: "cellMargins", value: { start: 0 } });
  await validateValue({ kind: "tableCellProperties", value: { gridSpan: 2 } });
  // A standalone cell does not have a table grid. This same span is rejected by table resolution.
  await validateValue({ kind: "tableCellProperties", value: { gridSpan: 16385 } });
  await validateValue({ kind: "tableRowProperties", value: { height: { kind: "exact", points: 0 } } });
  await rejects(() => validateValue({ kind: "cellMargins", value: { start: -1 } }),
    "invalid_measurement", "start");
  await rejects(() => validateValue({ kind: "tableRowProperties", value: { height: { kind: "atLeast", points: NaN } } }),
    "invalid_measurement", "height");
  await rejects(() => validateValue({ kind: "tableCellProperties", value: { gridSpan: 0 } }),
    "invalid_content", "gridSpan");
  await rejects(() => resolveComponent(cell([], { margins: { end: Infinity } })),
    "invalid_measurement", "tableCell.properties.margins.end");
  await rejects(() => resolveComponent(table([], { style: "\ud800" })), undefined, "table.properties.style");
  await rejects(() => resolveComponent(table([], { columnWidths: [2 ** 32] })), undefined,
    "table.properties.columnWidths[0]");
  await rejects(() => resolveComponent(cell([], { backgroundColor: [0, 0, 256] })), undefined,
    "tableCell.properties.backgroundColor[2]");
  await rejects(() => resolveComponent(cell([], { backgroundColor: [0, 0] as unknown as [0, 0, 0] })), undefined,
    "tableCell.properties.backgroundColor");
  const document = await resolveDocument(program(table([row([cell([])])], { style: "Not an authored style" })));
  await rejects(() => document.toDocx(), "unknown_style");
}

// Compile-time checks only; the matching runtime cases are above.
function typeRejections() {
  // @ts-expect-error standalone rows are not block declarations
  program(row([]));
  // @ts-expect-error legacy plain strings are not structured cell declarations
  resolveComponent(table([row(["legacy"])]));
  // @ts-expect-error semantic direction tags, not Word wire codes
  cell([], { textDirection: "tbRlV" });
  // @ts-expect-error validation kinds determine the required DTO
  validateValue({ kind: "cellMargins", value: { gridSpan: 1 } });
}
void typeRejections;
