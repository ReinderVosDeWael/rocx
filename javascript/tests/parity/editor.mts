import {
  openDocx, resolveComponent,
  type AlignmentChange, type EditBatch, type EditPreview, type HeaderImageChange,
  type HeaderImageEdit, type HeaderImagePreview, type ImportedDocument, type ImportedParagraph,
  type ResolvedImage, type ResolvedTable, type TableCellDeclaration, type TableDeclaration,
  type TableInsertionChange, type TableStructureEdit, type TableAnchor,
} from "rocx-docx";

export const editorSources = [
  "base", "unsafe", "ambiguous", "drawing", "relocated", "range-source", "control-source",
  "styles-source", "styles-edge", "styles-missing", "styles-invalid",
] as const;
export type Fixtures = Record<typeof editorSources[number] | "image", Uint8Array>;

function check(value: unknown, message: string): asserts value {
  if (!value) throw new Error(message);
}

function same(actual: unknown, expected: unknown, message: string) {
  const sorted = (value: unknown): unknown => {
    if (Array.isArray(value)) return value.map(sorted);
    if (value && typeof value === "object") {
      return Object.fromEntries(Object.entries(value).sort(([a], [b]) => a.localeCompare(b))
        .map(([key, item]) => [key, sorted(item)]));
    }
    return value;
  };
  const a = JSON.stringify(sorted(actual));
  const b = JSON.stringify(sorted(expected));
  check(a === b, `${message}: ${a} != ${b}`);
}

function bytesEqual(a: Uint8Array, b: Uint8Array, message: string) {
  check(a.length === b.length && a.every((byte, i) => byte === b[i]), message);
}

function rejects(action: () => unknown, message: RegExp, path?: string) {
  try {
    action();
  } catch (error) {
    check(error instanceof Error && message.test(error.message), `expected ${message}, got ${error}`);
    same(Reflect.get(error, "rule"), path ? undefined : "unsupported_edit", "error rule");
    if (path) same(Reflect.get(error, "path"), path, "adapter path");
    return;
  }
  throw new Error(`expected ${message}`);
}

function paragraph(doc: ImportedDocument, index: number): ImportedParagraph {
  const block = doc.body()[index];
  check(block?.kind === "paragraph", `body[${index}] is not a paragraph`);
  return block;
}

const cell = (children: TableCellDeclaration["children"], gridSpan?: number) => ({
  kind: "tableCell" as const, children, ...(gridSpan === undefined ? {} : { properties: { gridSpan } }),
});
const text = (value: string) => ({ kind: "paragraph" as const, text: value });
const table = (...cells: TableCellDeclaration[]): TableDeclaration =>
  ({ kind: "table", rows: [{ kind: "tableRow", children: cells }] });

async function payloads(png: Uint8Array) {
  const rich: ResolvedTable | undefined = await resolveComponent({
    ...table(cell([{
      kind: "paragraph",
      runs: [
        { kind: "text", text: "bold", format: { bold: true } },
        { kind: "fraction", numerator: "1", denominator: "2" },
        { kind: "hyperlink", target: { kind: "external", url: "https://example.invalid/inserted" },
          children: [{ kind: "text", text: "linked" }] },
      ],
    }, table(cell([text("nested A")]), cell([text("nested B")]))], 2), cell([text("tail")])),
    properties: { columnWidths: [1200, 2400, 3600] },
  });
  const auto = await resolveComponent(table(cell([text("next A")]), cell([text("next B")])));
  const unsupported = await resolveComponent({
    kind: "table",
    rows: [{
      kind: "tableRow", children: [cell([text("guarded")])],
      properties: { contentControl: { text: { kind: "rich" } } },
    }],
  });
  const inline = await resolveComponent({ kind: "image", data: png });
  const floatingRun = await resolveComponent({
    kind: "image", data: png,
    transformation: { anchor: { horizontalReference: "page", verticalReference: "page", wrap: "none" } },
  });
  const image: ResolvedImage | undefined = inline?.view.image;
  const floating = floatingRun?.view.image;
  check(rich && auto && unsupported && image && floating && inline, "editor payloads resolved");
  return { rich, auto, unsupported, image, floating, inline };
}

type Payloads = Awaited<ReturnType<typeof payloads>>;

export async function alignmentAndTablePreviewAndPreservation(input: Fixtures, { rich, auto }: Payloads) {
  const doc = await openDocx(input.base);
  const styles = doc.styles();
  check(styles?.partName === "word/styles.xml", "relationship-selected styles part");
  const normal = styles.styles.find((style) => style.id === "Normal");
  check(normal?.kind === "paragraph" && normal.kindXml === "paragraph"
    && normal.name === "Normal", "typed Normal style metadata");
  const edge = (await openDocx(input["styles-edge"])).styles();
  same(edge?.styles, [{ id: "X", kind: "unsupported", kindXml: "future" },
    { kind: "missing" }], "unknown and missing style kinds");
  check((await openDocx(input["styles-missing"])).styles() === undefined, "missing styles relationship");
  const invalidStyles = await openDocx(input["styles-invalid"]);
  rejects(() => invalidStyles.styles(), /invalid styles root/);
  const start = paragraph(doc, 0).anchor;
  const insertedParagraph = await resolveComponent(text("inserted paragraph"));
  check(insertedParagraph, "paragraph payload resolved");
  const paragraphEdit = { target: start, paragraph: insertedParagraph };
  const paragraphPreview = doc.preview({
    alignments: [{ target: start, alignment: "center" }],
    paragraphInsertions: [paragraphEdit],
  });
  same(paragraphPreview.paragraphInsertions(), [0], "paragraph insertion report");
  const paragraphOutput = await openDocx(paragraphPreview.toDocx());
  same(paragraph(paragraphOutput, 1).text, "inserted paragraph", "inserted paragraph readback");
  rejects(() => doc.preview({ paragraphInsertions: [paragraphEdit, paragraphEdit] }), /multiple paragraph/);
  const foreign = await openDocx(input.base);
  rejects(() => foreign.preview({ paragraphInsertions: [paragraphEdit] }), /different document snapshot/);
  const protectedSource = await openDocx(input.unsafe);
  rejects(() => protectedSource.preview({ paragraphInsertions: [
    { ...paragraphEdit, target: paragraph(protectedSource, 0).anchor },
  ] }), /range/);
  const firstRun = paragraph(doc, 0).inlineStructure[0];
  check(firstRun?.kind === "run", "first body inline is a run");
  const firstAnchor = firstRun.anchor;
  check(paragraph(doc, 0).inlineStructure[0]?.kind === "run", "repeated inspection");
  const repeated = paragraph(doc, 0).inlineStructure[0];
  check(repeated.kind === "run" && repeated.anchor === firstAnchor, "same inline has same handle");
  const otherRun = paragraph(doc, 1).inlineStructure[0];
  check(otherRun?.kind === "run" && otherRun.anchor !== firstAnchor, "different runs have different handles");
  const unset = paragraph(doc, 1).anchor;
  const sectionEnd = paragraph(doc, 3).anchor;
  const align = { target: start, alignment: "center" } as const;
  const insert = { target: start, table: rich };
  const preview: EditPreview = doc.preview({
    alignments: [{ target: unset, alignment: "right" }, align],
    tableInsertions: [{ target: sectionEnd, table: auto }, insert],
  });
  const changes: readonly AlignmentChange[] = preview.changes();
  const insertions: readonly TableInsertionChange[] = preview.insertions();
  same(changes, [{ bodyIndex: 0, before: "start", after: "center" },
    { bodyIndex: 1, after: "right" }], "raw and unset before values in source order");
  check(!Object.hasOwn(changes[1], "before"), "unset before is absent");
  check(Object.isFrozen(changes) && Object.isFrozen(changes[0]), "report is immutable");
  same(insertions, [{ afterBodyIndex: 0, rows: 1, columns: 3 },
    { afterBodyIndex: 3, rows: 1, columns: 2 }], "insertion dimensions and original indices");
  same(preview.headerChanges(), [], "no header changes");
  const bytes = preview.toDocx();
  const reopened = await openDocx(bytes);
  rejects(() => reopened.inlineAt(firstAnchor), /different document snapshot/);
  check(doc.inlineAt(firstAnchor).kind === "run", "preview and reopen leave original anchor valid");
  const fresh = paragraph(reopened, 0).inlineStructure[0];
  check(fresh?.kind === "run" && fresh.anchor !== firstAnchor, "reopen issues fresh handles");
  same(reopened.body().map((block) => block.kind), [
    "paragraph", "table", "paragraph", "paragraph", "paragraph", "table", "paragraph", "paragraph", "paragraph",
  ], "table placement including immediately after section terminator");
  same(paragraph(reopened, 0).alignment, "center", "reopened first alignment");
  same(paragraph(reopened, 2).alignment, "right", "reopened second alignment");
  const insertedTable = reopened.body()[5];
  check(insertedTable?.kind === "table", "inserted table is inspectable");
  const cellEdit = { target: insertedTable.anchor, rowIndex: 0, cellIndex: 0, alignment: "center" } as const;
  const cellPreview = reopened.preview({ cellAlignments: [cellEdit] });
  same(cellPreview.cellAlignmentChanges(), [{ bodyIndex: 5, rowIndex: 0, cellIndex: 0, after: "center" }],
    "physical cell patch report");
  rejects(() => reopened.preview({ cellAlignments: [cellEdit, cellEdit] }), /duplicate cell/);
  rejects(() => reopened.preview({ cellAlignments: [{ ...cellEdit, rowIndex: 8 }] }), /not found/);
  rejects(() => doc.preview({ cellAlignments: [cellEdit] }), /different document snapshot/);
  const cellAfter = await openDocx(cellPreview.toDocx());
  const patchedTable = cellAfter.body()[5];
  check(patchedTable?.kind === "table", "patched table remains inspectable");
  same(cellAfter.preview({ cellAlignments: [{ ...cellEdit, target: patchedTable.anchor }] }).cellAlignmentChanges(),
    [], "unchanged cell alignment omitted");
  const removed = paragraph(reopened, 2).inlineStructure[0];
  check(removed?.kind === "run", "expected a removable run after the insertion point");
  const combined = {
    paragraphInsertions: [{ target: paragraph(reopened, 0).anchor, paragraph: insertedParagraph }],
    cellAlignments: [cellEdit], runRemovals: [removed.anchor],
  };
  const mixed = reopened.preview(combined);
  same(mixed.paragraphInsertions(), [0], "original paragraph insertion site");
  same(mixed.cellAlignmentChanges(), cellPreview.cellAlignmentChanges(), "original table index");
  same(mixed.runRemovals(), [{ bodyIndex: 2, inlineIndex: 0, before: "unset alignment" }], "original run indices");
  const mixedBytes = mixed.toDocx();
  const combinedResult = await openDocx(mixedBytes);
  same(paragraph(combinedResult, 1).text, "inserted paragraph", "mixed paragraph insertion");
  same(paragraph(combinedResult, 3).text, "", "run removed from shifted paragraph");
  const shiftedTable = combinedResult.body()[6];
  check(shiftedTable?.kind === "table", "mixed table moved one slot");
  same(combinedResult.preview({ cellAlignments: [{ ...cellEdit, target: shiftedTable.anchor }] }).cellAlignmentChanges(),
    [], "shifted table cell already has the requested alignment");
  rejects(() => reopened.preview({ ...combined, runText: [{ target: removed.anchor, text: "conflict" }] }),
    /duplicate|conflicting/);
  bytesEqual(reopened.preview(combined).toDocx(), mixedBytes, "retry after conflict");
  same(paragraph(reopened, 2).text, "unset alignment", "failed batch leaves original run intact");
  same(paragraph(doc, 0).alignmentXml, "start", "original alignment is unchanged");
  const noop = doc.preview({ alignments: [{ target: paragraph(doc, 4).anchor, alignment: "right" }] });
  same(noop.changes(), [], "unchanged alignment omitted");
  bytesEqual(noop.toDocx(), doc.preview({}).toDocx(), "unchanged and empty batches encode identically");

  const other = await openDocx(input.base);
  rejects(() => other.inlineAt(firstAnchor), /different document snapshot/);
  rejects(() => doc.preview({ alignments: [{ ...align, target: paragraph(other, 0).anchor }] }), /different document snapshot/);
  rejects(() => doc.preview({ alignments: [align, align] }), /multiple alignment/);
  rejects(() => doc.preview({ tableInsertions: [insert, insert] }), /multiple table/);
  rejects(() => doc.preview({ tableInsertions: [{ ...insert, target: paragraph(other, 0).anchor }] }),
    /different document snapshot/);
  const unsafe = await openDocx(input.unsafe);
  rejects(() => unsafe.preview({ tableInsertions: [{ ...insert, target: paragraph(unsafe, 0).anchor }] }), /range/);
  const ambiguous = await openDocx(input.ambiguous);
  rejects(() => ambiguous.preview({ alignments: [{ ...align, target: paragraph(ambiguous, 0).anchor }] }), /ambiguous/);
  const blocks = doc.preview({ blockEdits: [
    { kind: "keepWithNext", target: start, value: false },
    { kind: "replace", target: unset, paragraph: insertedParagraph },
    { kind: "replaceSection", target: paragraph(doc, 4).anchor, blocks: [insertedParagraph, insertedParagraph, auto] },
  ] });
  same(blocks.blockChanges(), [0, 1, 4], "block edit report");
  const intermediate = await openDocx(blocks.toDocx());
  const remove = { kind: "remove", target: paragraph(intermediate, 5).anchor } as const;
  rejects(() => doc.preview({ blockEdits: [remove] }), /different document snapshot/);
  rejects(() => intermediate.preview({ blockEdits: [remove, remove] }), /conflicting/);
  const final = intermediate.preview({ blockEdits: [remove] });
  same(final.blockChanges(), [5], "retry after rejected batch");
  let tableStructure = bytes;
  const steps: ((target: TableAnchor) => TableStructureEdit)[] = [
    target => ({target, kind: "insertRow", index: 1}), target => ({target, kind: "removeRow", index: 1}),
    target => ({target, kind: "insertColumn", index: 1}), target => ({target, kind: "removeColumn", index: 1}),
    target => ({target, kind: "insertCell", row: 0, cell: 1}), target => ({target, kind: "removeCell", row: 0, cell: 1}),
    target => ({target, kind: "replaceCell", row: 0, cell: 1, blocks: [insertedParagraph, auto]}),
  ];
  for (const step of steps) {
    const source = await openDocx(tableStructure);
    const table = source.body()[5]; check(table?.kind === "table", "structure target");
    const edit = step(table.anchor);
    rejects(() => reopened.preview({tableStructure: [edit]}), /different document snapshot/);
    rejects(() => source.preview({tableStructure: [edit, edit]}), /exclusive|duplicate/);
    tableStructure = source.preview({tableStructure: [edit]}).toDocx();
  }
  return { tableStructure, body: bytes, noop: noop.toDocx(), cell: cellPreview.toDocx(), mixedBody: mixedBytes, blocks: final.toDocx() };
}

export async function headerImageScopesAndEffects(input: Fixtures, { image, floating }: Payloads) {
  const doc = await openDocx(input.base);
  const headers = doc.defaultHeaders();
  const [first, inherited, later, distinct] = headers.map((header) => header.view);
  const firstInline = headers[0].inlineStructure(0)?.[0];
  const inheritedInline = headers[1].inlineStructure(0)?.[0];
  check(firstInline?.kind === "run" && inheritedInline?.kind === "run", "header runs");
  check(firstInline.anchor === inheritedInline.anchor, "header aliases share inline handle");
  const story = doc.stories().find((s) => s.view.partName === first.partName);
  const storyInline = story?.inlineStructure(0)?.[0];
  check(storyInline?.kind === "run" && storyInline.anchor === firstInline.anchor,
    "header and story views share inline handle");
  const distinctInline = headers[3].inlineStructure(0)?.[0];
  check(distinctInline?.kind === "run" && distinctInline.anchor !== firstInline.anchor,
    "distinct physical headers have distinct handles");
  check(first.anchor !== inherited.anchor && first.partName === inherited.partName, "aliases have section-scoped targets");
  const sharedEdit: HeaderImageEdit = { target: inherited.anchor, mode: "shared", image };
  const shared: HeaderImagePreview = doc.previewHeaderImage(sharedEdit);
  const change: HeaderImageChange = shared.change();
  same(change, { sourcePart: first.partName, outputPart: first.partName,
    affectedSections: [0, 1, 2, 3], mode: "shared" }, "default and first/even consumers");
  check(Object.isFrozen(change.affectedSections), "header report array is immutable");
  const sharedBatch = doc.preview({ headerImages: [sharedEdit] });
  same(sharedBatch.headerChanges(), [change], "batch header report matches dedicated report");
  bytesEqual(sharedBatch.toDocx(), shared.toDocx(), "batch and dedicated shared bytes");
  const sharedAfter = (await openDocx(shared.toDocx())).defaultHeaders().map((header) => header.view);
  same(sharedAfter.map((h) => h.partName), [first.partName, first.partName, first.partName, distinct.partName],
    "shared edit does not change defaults");
  same(sharedAfter.map((h) => h.blocks.length), [2, 2, 2, 1], "only the shared part gained a paragraph");

  const localEdit = { ...sharedEdit, mode: "sectionLocal" } as const;
  const local = doc.previewHeaderImage(localEdit);
  same(local.change(), { sourcePart: first.partName, outputPart: local.change().outputPart,
    affectedSections: [1], restoredSection: 2, mode: "sectionLocal" }, "local scope and redirected inheritor");
  check(local.change().outputPart !== first.partName, "local fork gets its own part");
  const localAfter = (await openDocx(local.toDocx())).defaultHeaders().map((header) => header.view);
  same(localAfter.map((h) => [h.partName, h.inherited, h.blocks.length]), [
    [first.partName, false, 1], [local.change().outputPart, false, 2],
    [later.partName, false, 1], [distinct.partName, false, 1],
  ], "only selected default is forked; later explicit restoration");
  const localBatch = doc.preview({ headerImages: [localEdit] });
  same(localBatch.headerChanges(), [local.change()], "batch local report");
  bytesEqual(localBatch.toDocx(), local.toDocx(), "batch and dedicated local bytes");

  const relocated = await openDocx(input.relocated);
  const relocatedTarget = relocated.defaultHeaders()[1].view.anchor;
  same(relocated.previewHeaderImage({ ...sharedEdit, target: relocatedTarget }).change().affectedSections,
    [0, 1, 2, 3], "even-only alias is counted despite a distinct default");
  const relocatedPreview = relocated.previewHeaderImage({
    ...localEdit, target: relocatedTarget,
  });
  same(relocatedPreview.change().sourcePart, "content/headers/source.xml", "relocated source part");
  check(relocatedPreview.change().outputPart.startsWith("content/headers/"), "local fork beside relocated source");
  const drawing = await openDocx(input.drawing);
  rejects(() => drawing.previewHeaderImage({ ...localEdit, target: drawing.defaultHeaders()[1].view.anchor }),
    /cannot be safely copied/);
  rejects(() => doc.previewHeaderImage({ ...sharedEdit, image: floating }), /requires an inline image/);
  rejects(() => relocated.previewHeaderImage(sharedEdit), /different document snapshot/);
  const firstPage = doc.stories().find(({ view }) => view.sectionIndex === 3 && view.kind === "header" && view.variant === "first")?.view;
  check(firstPage?.anchor, "first-page header exposes an edit anchor");
  const firstShared = doc.previewHeaderImage({target: firstPage.anchor, mode: "shared", image});
  const firstLocal = doc.previewHeaderImage({target: firstPage.anchor, mode: "sectionLocal", image});
  same(firstShared.change().affectedSections, [0, 1, 2, 3], "first-page shared aliases");
  same(firstLocal.change().affectedSections, [3], "first-page local scope");
  rejects(() => relocated.previewHeaderImage({target: firstPage.anchor!, mode: "shared", image}), /different document snapshot/);
  const reopenedFirst = (await openDocx(firstLocal.toDocx())).stories().find(
    ({view}) => view.sectionIndex === 3 && view.kind === "header" && view.variant === "first")?.view;
  same(reopenedFirst?.partName, firstLocal.change().outputPart, "first-page fork reopened");
  return {
    shared: shared.toDocx(), local: local.toDocx(), relocated: relocatedPreview.toDocx(),
    firstShared: firstShared.toDocx(), firstLocal: firstLocal.toDocx(),
    reports: { shared: change, local: local.change(), relocated: relocatedPreview.change() },
  };
}

export async function mixedTransactionAtomicityAndRepeatPreview(
  input: Fixtures, { rich, auto, unsupported, image, floating, inline }: Payloads,
) {
  // Return only previews and expected bytes: no source, inspection DTO or anchor is retained here.
  const retained = async () => {
    const doc = await openDocx(input.base);
    const [h0, h1, h2] = doc.defaultHeaders().map((h) => h.view);
    const p0 = paragraph(doc, 0).anchor;
    const alignments = [{ target: p0, alignment: "center" },
      { target: paragraph(doc, 1).anchor, alignment: "right" }] as const;
    const tableInsertions = [{ target: p0, table: rich }, { target: paragraph(doc, 3).anchor, table: auto }];
    const first = { target: h0.anchor, mode: "sectionLocal", image } as const;
    const second = { ...first, target: h1.anchor };
    const shared = { ...first, target: h2.anchor, mode: "shared" } as const;
    const batch: EditBatch = { alignments, tableInsertions, headerImages: [first, second] };
    const a = doc.preview(batch);
    const b = doc.preview({ alignments: [...alignments].reverse(), tableInsertions: [...tableInsertions].reverse(),
      headerImages: [second, first] });
    const expected = a.toDocx();
    bytesEqual(b.toDocx(), expected, "shuffling all inputs produces exact bytes");
    const headers = a.headerChanges();
    same(headers, b.headerChanges(), "shuffling preserves report order");
    same(headers.map((h) => [h.sourcePart, h.affectedSections, h.restoredSection]),
      [[h0.partName, [0], undefined], [h0.partName, [1], 2]], "adjacent locals use original part; last restores");
    check(!Object.hasOwn(headers[0], "restoredSection"), "absent restoration is omitted");
    const reopened = await openDocx(expected);
    same(reopened.defaultHeaders().map((h) => h.view.partName),
      [headers[0].outputPart, headers[1].outputPart, h0.partName, doc.defaultHeaders()[3].view.partName],
      "local forks preserve the original section slots despite table insertion");
    same([paragraph(reopened, 0).alignment, paragraph(reopened, 2).alignment], ["center", "right"], "mixed body edits");
    same([reopened.body()[1].kind, reopened.body()[5].kind], ["table", "table"], "mixed table locations");
    const c = doc.preview({ ...batch, headerImages: [shared] });
    check(c.headerChanges()[0].mode === "shared", "independent shared mixed preview");
    const cExpected = c.toDocx();
    const noEdits = doc.preview({}).toDocx();
    rejects(() => doc.preview({ ...batch, headerImages: [first, shared] }), /shared.+(conflict|excludes|cannot)/i);
    rejects(() => doc.preview({ ...batch, headerImages: [shared, { ...shared, target: h1.anchor }] }),
      /shared.+(conflict|excludes|cannot)/i);
    const relocated = await openDocx(input.relocated);
    const [relocatedFirst, , relocatedExplicit] = relocated.defaultHeaders().map((h) => h.view);
    check(!relocatedExplicit.inherited && relocatedFirst.partName === relocatedExplicit.partName,
      "same physical header has two explicit references");
    rejects(() => relocated.preview({
      headerImages: [{ ...shared, target: relocatedFirst.anchor }, { ...second, target: relocatedExplicit.anchor }],
    }), /same physical part/);
    rejects(() => doc.preview({ ...batch, headerImages: [first, first] }), /multiple header/);
    rejects(() => doc.preview({ ...batch, headerImages: [first, { ...second, image: floating }] }), /requires an inline image/);
    // The Rust transaction stages both headers before it discovers unsupported table content.
    rejects(() => doc.preview({ ...batch, tableInsertions: [{ target: p0, table: unsupported }] }), /row content controls/);
    const foreign = await openDocx(input.base);
    rejects(() => doc.preview({ ...batch, headerImages: [first, { ...second, target: foreign.defaultHeaders()[1].view.anchor }] }),
      /different document snapshot/);
    for (const forged of [{ ...p0 }, structuredClone(p0), Object.create(p0), h1.anchor,
      paragraph(doc, 0).inlineStructure[0]]) {
      rejects(() => doc.preview({ ...batch, alignments: [{ ...alignments[0], target: forged }] } as EditBatch),
        /right type/, "batch.alignments[0].target");
    }
    rejects(() => doc.preview({ tableInsertions: [{ target: p0, table: rich.view }] } as unknown as EditBatch),
      /right type/, "batch.tableInsertions[0].table");
    rejects(() => doc.previewHeaderImage({ ...first, image: inline } as unknown as HeaderImageEdit),
      /right type/, "edit.image");
    rejects(() => doc.previewHeaderImage({ ...first, mode: "local" } as unknown as HeaderImageEdit),
      /unknown HeaderEditMode/, "edit.mode");
    rejects(() => doc.preview({ alignments: [{ target: p0, alignment: "both" }] } as unknown as EditBatch),
      /unknown Alignment/, "batch.alignments[0].alignment");
    rejects(() => doc.preview({ headerImages: null } as unknown as EditBatch), /expected an array/, "batch.headerImages");
    bytesEqual(doc.preview({}).toDocx(), noEdits, "failed transactions expose no staged output");
    bytesEqual(doc.preview(batch).toDocx(), expected, "valid preview works after failures");
    return { a, b, c, expected, cExpected };
  };
  const { a, b, c, expected, cExpected } = await retained();
  a.toDocx().fill(0);
  c.toDocx().fill(255);
  bytesEqual(a.toDocx(), expected, "retained preview re-encodes after output mutation");
  bytesEqual(b.toDocx(), expected, "other retained preview unaffected");
  bytesEqual(c.toDocx(), cExpected, "shared retained preview independently re-encodes");
  check(expected.some((byte, i) => byte !== cExpected[i]), "mixed local and shared previews differ");
  return { mixed: expected, mixedShared: cExpected, mixedReports: [...a.headerChanges()] };
}

async function editControls(fixtures: Fixtures) {
  const controlled = await openDocx(fixtures["control-source"]);
  const controls = controlled.controls();
  same(controls.map(c => c.context), ["block", "inline", "row", "cell", "block", "block"], "contexts");
  check(controls.every(c => c.tag === "repeat" && c.id === undefined) && controls[4].bound, "source metadata");
  const [block, inline, row, cell, bound, unsafe] = controls.map(c => c.anchor);
  const edits = [{target: block, text: " block "}, {target: inline, alias: "caption"},
    {target: row, text: "new row"}, {target: cell, tag: "cell", text: "new cell"}];
  rejects(() => controlled.preview({ controls: [edits[0], {target: inline, text: "no"}] }), /locked/);
  rejects(() => controlled.preview({ controls: [{target: block, tag: "no"}] }), /locked/);
  rejects(() => controlled.preview({ controls: [edits[0], {target: bound, alias: "no"}] }), /XML-bound/);
  rejects(() => controlled.preview({ controls: [edits[0], {target: unsafe, text: "no"}] }), /structure/);
  rejects(() => controlled.preview({ controls: [edits[0], edits[0]] }), /duplicate/);
  const stale = await openDocx(fixtures["control-source"]);
  rejects(() => stale.preview({ controls: edits }), /snapshot/);
  return controlled.preview({ controls: edits }).toDocx();
}

export async function run(fixtures: Fixtures) {
  const catalog = await openDocx(fixtures["styles-source"]);
  const active = catalog.styleAnchor("MemoActive");
  const batch: EditBatch = { styleEdits: [
    { op: "add", definition: { kind: "paragraph", name: "Memo Added", baseStyle: "Memo Active" } },
    { op: "replace", target: catalog.styleAnchor("MemoReplace"), definition: {kind: "paragraph", name: "Memo Replace", bold: true} },
    { op: "remove", target: catalog.styleAnchor("MemoDrop") },
    { op: "patch", target: active, italic: true },
  ] };
  rejects(() => catalog.preview({styleEdits: [...batch.styleEdits!, {op: "remove", target: active}]}), /duplicate/);
  rejects(() => catalog.preview({styleEdits: [{op: "remove", target: active}]}), /references/);
  const styleEdits = catalog.preview(batch).toDocx();
  const reopened = await openDocx(styleEdits);
  rejects(() => reopened.preview({styleEdits: [{op: "patch", target: active, bold: true}]}), /different document snapshot/);
  check(reopened.styles()?.styles.find(s => s.id === "MemoAdded")?.basedOn === "MemoActive", "new style inheritance");
  const resolved = await payloads(fixtures.image);
  const doc = await openDocx(fixtures.base);
  const first = paragraph(doc, 0).inlineStructure[0];
  check(first?.kind === "run", "expected removable run");
  const removal = doc.preview({ runRemovals: [first.anchor] });
  same(removal.runRemovals(), [{ bodyIndex: 0, inlineIndex: 0, before: "Source bold" }], "removal report");
  rejects(() => doc.preview({ runRemovals: [first.anchor, first.anchor] }), /duplicate|conflicting/);
  rejects(() => doc.preview({ runRemovals: [first.anchor], runText: [{ target: first.anchor, text: "x" }] }), /duplicate|conflicting/);
  const other = await openDocx(fixtures.base);
  rejects(() => other.preview({ runRemovals: [first.anchor] }), /different document snapshot/);
  const ranged = await openDocx(fixtures["range-source"]);
  const selection = {target: paragraph(ranged, 1).anchor, start: 1, end: 6, text: " 🐈 "};
  rejects(() => ranged.preview({textRanges: [selection, selection]}), /overlapping/);
  rejects(() => doc.preview({textRanges: [selection]}), /different document snapshot/);
  rejects(() => doc.preview({textRanges: [{...selection, target: paragraph(doc, 0).anchor}]}), /plain direct body/);
  same(ranged.preview({textRanges: [selection]}).runTextChanges().map(c => c.after), ["a 🐈 ", "", "d"], "scalar ranges retain run containers");
  const ordinary: EditBatch = {
    textRanges: [{target: paragraph(doc, 1).anchor, start: 0, end: 1, text: "X"}],
    blockEdits: [{kind: "keepWithNext", target: paragraph(doc, 4).anchor, value: true}],
    resources: [{kind: "coreTitle", value: "Combined editor"}],
  };
  const combined = doc.preview(ordinary).toDocx();
  rejects(() => doc.preview({...ordinary, textRanges: [selection]}),
    /different document snapshot/);
  bytesEqual(doc.preview(ordinary).toDocx(), combined, "mixed retry after stale anchor");
  check(paragraph(doc, 0).text.startsWith("Source bold"), "source unchanged");
  const second = paragraph(doc, 1).inlineStructure[0];
  const fourth = paragraph(doc, 4).inlineStructure[0];
  check(second.kind === "run" && fourth.kind === "run", "inline targets");
  const payload = await resolveComponent({kind: "paragraph", runs: [
    {kind: "break", breakType: "line"}, {kind: "hyperlink", target: {kind: "external", url: "https://example.invalid/inline"}, children: [{kind: "text", text: "editable link"}]},
  ]});
  check(payload, "inline declarations resolved");
  const inlineEdits: NonNullable<EditBatch["inlineEdits"]> = [
    { kind: "insertAfter", target: fourth.anchor, inline: payload.view.runs[0] },
    { kind: "setBold", target: first.anchor, bold: false },
    { kind: "replace", target: second.anchor, inline: payload.view.runs[1] },
  ];
  const inline = doc.preview({inlineEdits});
  same(inline.inlineChanges(), [[0,0],[1,0],[4,0]], "inline change sites");
  rejects(() => other.preview({inlineEdits}), /different document snapshot/);
  rejects(() => doc.preview({inlineEdits, runRemovals: [first.anchor]}), /conflicting/);
  bytesEqual(doc.preview({inlineEdits}).toDocx(), inline.toDocx(), "inline retry");
  const meta = {author: "A & B", date: "2024-02-29T12:34:56Z"};
  const blockRevisions: Record<string,Uint8Array> = {};
  let current = doc;
  for (const label of ["Insert","Delete"]) {
    const edit = label === "Insert" ? {kind:"insertBefore" as const, text:" proposed "} : {kind:"delete" as const};
    const tracked = current.previewTrackedParagraph({target:paragraph(current,1).anchor,...edit,...meta}).toDocx();
    blockRevisions["para"+label] = tracked;
    const reopened = await openDocx(tracked), p = paragraph(reopened,1);
    same([p.markRevision?.kind,p.markRevision?.author,p.markRevision?.date,p.text],
      [label==="Insert"?"insertion":"deletion",meta.author,meta.date,""],"paragraph mark and projection");
    rejects(()=>current.previewParagraphRevision({target:p.anchor,action:"reject"}), /different document snapshot/);
    for(const action of ["accept","reject"] as const){
      const bytes=reopened.previewParagraphRevision({target:p.anchor,action}).toDocx();
      blockRevisions["para"+label+(action==="accept"?"Accepted":"Rejected")] = bytes;
      if(action==="accept") current=await openDocx(bytes);
    }
  }
  const boldChange = doc.previewTrackedBold({target:first.anchor, bold:false, ...meta}).toDocx();
  const formatted = await openDocx(boldChange);
  const formattedRun = paragraph(formatted, 0).inlineStructure[0];
  check(formattedRun.kind === "run" && !!formattedRun.propertyRevision?.id, "run property revision");
  same([formattedRun.propertyRevision.author, formattedRun.propertyRevision.date], [meta.author, meta.date], "property revision metadata");
  rejects(() => formatted.previewTrackedBold({target:first.anchor, bold:true, ...meta}), /different document snapshot/);
  rejects(() => formatted.previewTrackedBold({target:formattedRun.anchor, bold:true, ...meta}), /supported formatting/);
  const boldAccepted = formatted.previewRevision({target:formattedRun.anchor, action:"accept"}).toDocx();
  const boldRejected = formatted.previewRevision({target:formattedRun.anchor, action:"reject"}).toDocx();
  const tracked = doc.previewTrackedText({target:first.anchor, kind:"replace", text:" Revised ", ...meta}).toDocx();
  const changed = await openDocx(tracked);
  const [deleted, added] = paragraph(changed,0).inlineStructure;
  check(deleted.kind === "revision" && deleted.revisionKind === "deletion" && added.kind === "revision" && added.revisionKind === "insertion", "tracked replacement wrappers");
  same([added.author,added.date, paragraph(changed,0).text], [meta.author,meta.date,""], "revision metadata and flat projection");
  rejects(() => doc.previewRevision({target:added.anchor, action:"reject"}), /different document snapshot/);
  const reviewed = changed.previewRevision({target:added.anchor, action:"accept"}).toDocx();
  same(paragraph(await openDocx(reviewed),0).text," Revised ","accepted insertion");
  const moved = doc.previewTrackedMove({target:first.anchor, after:second.anchor, name:"Move1", ...meta}).toDocx();
  const pair = await openDocx(moved);
  const source = paragraph(pair,0).inlineStructure[1];
  const dest = paragraph(pair,1).inlineStructure[2];
  check(source.kind === "revision" && dest.kind === "revision" && source.revisionKind === "moveSource" && dest.revisionKind === "moveDestination", "paired move anchors");
  rejects(() => pair.previewRevision({target:source.anchor,action:"accept"}), /range|both move wrappers/);
  const moveAccepted = pair.previewRevision({target:source.anchor,paired:dest.anchor,action:"accept"}).toDocx();
  return {
    boldChange, boldAccepted, boldRejected,
    ...blockRevisions,
    tracked, reviewed, moved, moveAccepted, combined, styleEdits, inline: inline.toDocx(),
    range: ranged.preview({textRanges: [selection]}).toDocx(),
    removal: removal.toDocx(),
    controlEdits: await editControls(fixtures),
    ...await alignmentAndTablePreviewAndPreservation(fixtures, resolved),
    ...await headerImageScopesAndEffects(fixtures, resolved),
    ...await mixedTransactionAtomicityAndRepeatPreview(fixtures, resolved),
  };
}

// Compile only. Check public inputs and output types without letting invalid calls execute.
export function rejectedTypes(doc: ImportedDocument, p: ImportedParagraph, header: HeaderImageEdit, table: ResolvedTable) {
  // @ts-expect-error source tags and IDs are not physical control anchors
  doc.preview({ controls: [{ target: "repeat", tag: "new" }] });
  for (const {abstractId, level} of doc.abstractStarts()) {
    doc.preview({resources: [{kind: "abstractStart", abstractId, level, value: null}]});
  }
  // @ts-expect-error omitting the value is not a request to clear a title
  doc.preview({resources: [{kind: "coreTitle"}]});
  for (const e of doc.entryStories()) {
    const id: string = e.id;
    for (const block of e.blocks) {
      if (block.kind !== "paragraph") continue;
      for (const node of block.inlineStructure) {
        if (node.kind === "run") doc.inlineAt(node.anchor);
      }
    }
  }
  const run = p.inlineStructure[0];
  if (run?.kind === "run") {
    // @ts-expect-error tracked deletion has no insertion text
    doc.previewTrackedText({target: run.anchor, kind: "delete", author: "Ada", text: "lost"});
    // @ts-expect-error review requires an inline anchor from inspection
    doc.previewRevision({target: p.anchor, action: "reject"});
    const text: string = doc.preview({runText: [{target: run.anchor, text: "New"}]}).runTextChanges()[0].after;
    const before: string = doc.preview({runRemovals: [run.anchor]}).runRemovals()[0].before;
  }
  // @ts-expect-error text edits target runs, not whole paragraphs
  doc.preview({runText: [{target: p.anchor, text: "New"}]});
  // @ts-expect-error header anchors are not body paragraph anchors
  doc.preview({ tableInsertions: [{ target: header.target, table }] });
  // @ts-expect-error cloned records cannot stand in for header anchors
  doc.previewHeaderImage({ ...header, target: {} });
  // @ts-expect-error resolved views are not owning handles
  doc.preview({ tableInsertions: [{ target: p.anchor, table: table.view }] });
  // @ts-expect-error Word XML values are not JS alignment variants
  doc.preview({ alignments: [{ target: p.anchor, alignment: "both" }] });
  // @ts-expect-error edit report collections are readonly
  doc.preview({}).changes().push({ bodyIndex: 0, after: "left" });
  // @ts-expect-error dedicated preview is not a batch preview
  doc.previewHeaderImage(header).headerChanges();
}
