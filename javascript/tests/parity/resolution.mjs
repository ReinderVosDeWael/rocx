import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";
import {
  createResolvedBlockControl, createResolvedInline, deferred, future, inlineText, lazy,
  property, ready, resolveComponent, resolveDocument, resolveValue,
  validateAuthoringId, validateValue,
} from "rocx-docx";

const timestamp = "2022-02-03T04:05:06Z";
const text = (value) => ({ kind: "text", text: value });
const paragraph = (value) => ({ kind: "paragraph", text: value });
const program = (...children) => ({ timestamp, sections: [{ kind: "section", children }] });

async function bounded(work) {
  let timeout;
  try {
    return await Promise.race([
      work,
      new Promise((_, reject) => {
        timeout = setTimeout(() => reject(new Error("Rust pass did not settle")), 3000);
      }),
    ]);
  } finally {
    clearTimeout(timeout);
  }
}

export async function recursiveValuesMapAndSingleUse() {
  const stages = [];
  const value = lazy(() => {
    stages.push("lazy");
    return future(async () => {
      stages.push("future");
      await Promise.resolve();
      return lazy(() => {
        stages.push("inner");
        return 6;
      });
    });
  }).map((number) => {
    stages.push("map");
    return future(() => ready(number * 7));
  });
  assert.deepEqual(stages, []);
  assert.equal(await resolveValue(value), 42);
  assert.deepEqual(stages, ["lazy", "future", "inner", "map"]);
  for (const token of [value, ready("ready"), lazy(() => "lazy"), future(() => "future")]) {
    if (token !== value) await resolveValue(token);
    await assert.rejects(resolveValue(token), (error) => /single-use/.test(error.cause?.message));
  }
  await assert.rejects(resolveValue(lazy(async () => "not lazy")),
    (error) => /use future/.test(error.cause?.message));

  const release = Promise.withResolvers();
  const started = Promise.withResolvers();
  const trace = [];
  const doc = resolveDocument(program(
    paragraph(future(async () => {
      trace.push("first starts");
      await release.promise;
      trace.push("first finishes");
      return "first";
    })),
    paragraph(future(async () => {
      trace.push("second finishes");
      started.resolve();
      return "second";
    })),
  ));
  try {
    await bounded(started.promise);
    assert.deepEqual(trace, ["first starts", "second finishes"]);
  } finally {
    release.resolve();
  }
  assert.deepEqual((await doc).view.sections[0].view.paragraphs.map((p) => p.view.text),
    ["first", "second"]);

  const cause = Symbol("opaque");
  await assert.rejects(resolveComponent(paragraph(ready("x").map(() => { throw cause; }))),
    (error) => error.rule === "producer_error" && error.path === "paragraph.text" && error.cause === cause);
}

export async function sharedValuesAndNamedDependencyGraph() {
  let base;
  let baseStarts = 0;
  const branch = (name, suffix) => property(name, future(async (context) => {
    await Promise.resolve();
    return (await context.get(base.reference())) + suffix;
  }));
  const left = branch("left", "L");
  const right = branch("right", "R");
  base = property("base", lazy(() => {
    baseStarts++;
    return "base";
  }));
  assert.equal(left.reference().id, left.reference().id);
  assert.notEqual(left.reference().id, right.reference().id);
  const diamond = property("diamond", future(async (context) => {
    const join = await Promise.all([context.get(left.reference()), context.get(right.reference())]);
    return join.join("|");
  }));
  assert.deepEqual(await bounded(Promise.all([resolveValue(diamond), resolveValue(right.reference())])),
    ["baseL|baseR", "baseR"]);
  assert.equal(await resolveValue(left), "baseL");
  assert.equal(baseStarts, 1);

  let skipped = 0;
  const excluded = property("excluded", lazy(() => { skipped++; }), { condition: false });
  await assert.rejects(resolveValue(excluded), { rule: "excluded_dependency" });
  assert.equal(await resolveComponent({ ...paragraph(excluded), condition: false }), undefined);
  assert.equal(skipped, 0);

  let attempts = 0;
  const cause = { from: "shared producer" };
  const failure = deferred(future(() => {
    attempts++;
    return Promise.reject(cause);
  }));
  for (const target of [failure, future((context) => context.get(failure))]) {
    await assert.rejects(resolveValue(target), (error) =>
      error.cause === cause && error.rule === "producer_error");
  }
  assert.equal(attempts, 1);

  // Each cycle starts from one root. Native rocx does not guarantee detection
  // when independent passes start opposite nodes of the cycle.
  for (const mode of ["direct", "deferred", "indirect"]) {
    let self;
    const nested = deferred(future((context) => context.get(self.reference())));
    const other = property("other", future((context) => context.get(self.reference())));
    self = property(mode, future(async (context) => {
      await Promise.resolve();
      return context.get(mode === "direct" ? self.reference() :
        mode === "deferred" ? nested : other.reference());
    }));
    const controller = new AbortController();
    try {
      await assert.rejects(bounded(resolveValue(self, { signal: controller.signal })),
        { rule: "dependency_cycle" });
    } finally {
      controller.abort();
    }
  }
  assert.equal(await resolveValue(property("direct", ready("fresh graph"))), "fresh graph");
}

export async function standaloneAndConditionalComponents() {
  let excludedStarts = 0;
  let predicates = 0;
  const hidden = lazy(() => {
    excludedStarts++;
    throw new Error("excluded field started");
  });
  const condition = () => {
    predicates++;
    return false;
  };
  const specs = [
    [{ kind: "section", children: hidden }],
    [paragraph(hidden)],
    [text(hidden)],
    [{ kind: "hyperlink", target: hidden, children: hidden }],
    [{ kind: "contentControl", properties: hidden, children: hidden }, { scope: "inline" }],
    [{ kind: "contentControl", properties: hidden, children: hidden }, { scope: "block" }],
    [{ kind: "field", instruction: hidden, children: hidden }],
    [{ kind: "rangeStart", id: hidden, range: hidden }],
    [{ kind: "rangeEnd", id: hidden }],
    [{ kind: "note", target: hidden, children: hidden }],
    [{ kind: "noteReference", target: hidden }],
    [{ kind: "comment", metadata: hidden, children: hidden }],
    [{ kind: "image", data: hidden }],
    [{ kind: "shape", properties: hidden }],
    [{ kind: "columnChart", properties: hidden }],
    [{ kind: "fraction", numerator: hidden, denominator: hidden }],
    [{ kind: "tab" }],
    [{ kind: "break" }],
    [{ kind: "table", rows: hidden }],
    [{ kind: "tableRow", children: hidden }],
    [{ kind: "tableCell", children: hidden }],
    [{ kind: "header", children: hidden }],
    [{ kind: "footer", children: hidden }],
  ];
  for (const [declaration, options] of specs) {
    assert.equal(await resolveComponent({ ...declaration, condition }, options), undefined, declaration.kind);
  }
  assert.equal(predicates, specs.length);
  assert.equal(excludedStarts, 0);

  const cause = new Error("guard throws");
  await assert.rejects(resolveComponent({
    ...paragraph(hidden),
    condition() { throw cause; },
  }), (error) => error.path === "paragraph.condition" && error.cause === cause && !("rule" in error));
  assert.equal(excludedStarts, 0);

  let outerStarts = 0;
  let fieldStarts = 0;
  const shared = property("shared text", lazy(() => {
    fieldStarts++;
    return "forced by another component";
  }));
  const document = await resolveDocument(program(
    lazy(() => {
      outerStarts++;
      return { ...paragraph(shared), condition: false };
    }),
    paragraph(future((context) => context.get(shared.reference()))),
  ));
  assert.equal(outerStarts, 1);
  assert.equal(fieldStarts, 1);
  assert.deepEqual(document.view.sections[0].view.paragraphs.map((p) => p.view.text),
    ["forced by another component"]);
  assert.deepEqual((await resolveDocument({ sections: [{ kind: "section", condition: false }] })).view.sections, []);
}

export async function cancelledWaiterDoesNotPoisonSharedWork() {
  const entered = Promise.withResolvers();
  const getIssued = Promise.withResolvers();
  const release = Promise.withResolvers();
  const controller = new AbortController();
  const why = new Error("request cancelled");
  let starts = 0;
  const shared = property("surviving work", future(async ({ signal }) => {
    starts++;
    entered.resolve(signal);
    await release.promise;
    return "done";
  }));
  const first = resolveComponent(paragraph(shared.reference()), { signal: controller.signal });
  const second = resolveValue(future((context) => {
    const result = context.get(shared.reference());
    getIssued.resolve();
    return result;
  }));
  try {
    const [signal] = await bounded(Promise.all([entered.promise, getIssued.promise]));
    controller.abort(why);
    await assert.rejects(bounded(first), { name: "AbortError", cause: why });
    assert.equal(signal.aborted, false);
  } finally {
    release.resolve();
  }
  assert.equal(await bounded(second), "done");
  assert.equal(await resolveValue(shared), "done");
  assert.equal(starts, 1);
}

export async function resolvedBlockViewsAndReuse() {
  let starts = 0;
  const table = await resolveComponent({
    kind: "table",
    rows: [{
      kind: "tableRow",
      children: [
        { kind: "tableCell", properties: { gridSpan: 2 }, children: [
          paragraph(lazy(() => { starts++; return "nested"; })),
        ] },
        { kind: "tableCell", condition: false, children: lazy(() => { throw new Error("hidden cell"); }) },
      ],
    }],
  });
  assert.equal(table.view.columns, 2);
  assert.equal(table.view.rows[0].view.cells.length, 1);
  assert.equal(table.view.rows[0].view.cells[0].view.gridSpan, 2);
  const bodyParagraph = await resolveComponent(paragraph("direct"));
  const control = await createResolvedBlockControl({}, [bodyParagraph]);
  const doc = program({ kind: "resolved", value: control }, { kind: "resolved", value: table },
    { kind: "resolved", value: bodyParagraph });
  doc.defaults = { run: { bold: false } };
  doc.commentAuthor = "Example";
  doc.metadata = { title: "Resolved block reuse" };
  doc.sections[0].properties = { titlePage: false };
  doc.sections[0].headers = { default: { kind: "header", children: [paragraph("Header")] } };
  doc.sections[0].footers = { default: { kind: "footer", children: [paragraph("Footer")] } };
  const [first, second] = await Promise.all([resolveDocument(doc), resolveDocument(doc)]);
  const section = first.view.sections[0].view;
  assert.deepEqual(section.paragraphs.map((p) => p.view.text), ["direct"]);
  assert.equal(section.blocks.length, 3);
  assert.equal(section.properties.titlePage, false);
  assert.equal(section.headers.default.view.blocks[0].view.text, "Header");
  assert.equal(section.footers.default.view.blocks[0].view.text, "Footer");
  assert.equal(first.view.commentAuthor, "Example");
  assert.equal(first.view.defaults.run.bold, false);
  assert.equal(first.view.metadata.title, "Resolved block reuse");
  assert.equal(starts, 1);
  const expected = first.toDocx();
  assert.deepEqual(second.toDocx(), expected);
  expected.fill(0);
  assert.notDeepEqual(first.toDocx(), expected);
}

export async function resolvedInlineViewsConstructorsAndReuse() {
  const literal = await createResolvedInline({ kind: "text", text: "A" });
  const tab = await createResolvedInline({ kind: "tab", format: { bold: true }, comment: { text: "tab comment" } });
  const line = await createResolvedInline({ kind: "break", breakType: "line" });
  const page = await createResolvedInline({ kind: "break", breakType: "page" });
  assert.equal(literal.view.text, "A");
  assert.equal(tab.view.displayText, "\t");
  assert.equal(tab.view.format.bold, true);
  assert.equal(tab.view.comment.text, "tab comment");
  assert.equal(line.view.breakType, "line");
  assert.equal(page.view.displayText, "");
  assert.equal(await inlineText([literal, tab, line, page, literal]), "A\t\nA");

  for (const [kind, displayText] of [
    [{ kind: "insertion" }, "revision"],
    [{ kind: "deletion" }, ""],
    [{ kind: "moveSource", name: "_move" }, ""],
    [{ kind: "moveDestination", name: "_move" }, "revision"],
  ]) {
    const revision = { author: "Reviewer", kind, date: timestamp };
    const run = await resolveComponent({ ...text("revision"), revision });
    assert.deepEqual(run.view.revision, revision);
    assert.equal(run.view.text, "revision");
    assert.equal(run.view.displayText, displayText);
    assert.equal(await inlineText([run]), displayText);
  }

  const data = Uint8Array.from(atob(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl2n9sAAAAASUVORK5CYII=",
  ), (byte) => byte.charCodeAt(0));
  const sourceImage = await resolveComponent({ kind: "image", data });
  const image = await createResolvedInline({ kind: "image", image: sourceImage.view.image,
    format: { italic: true }, comment: { text: "image comment" } });
  assert.equal(image.view.format.italic, true);
  assert.equal(image.view.comment.text, "image comment");
  assert.deepEqual(image.view.image.view.bytes, data);
  image.view.image.view.bytes.fill(0);
  assert.deepEqual(image.view.image.view.bytes, data);

  const noteTarget = { kind: "footnote", id: 12 };
  const comment = await resolveComponent({
    kind: "comment", metadata: { author: "Reviewer" }, children: [paragraph("Comment")],
  });
  const shape = await createResolvedInline({ kind: "shape", properties: { geometry: "ellipse" } });
  const chart = await createResolvedInline({
    kind: "columnChart", properties: { width: 100, height: 50, categories: ["a"], values: [1] },
  });
  const fraction = await createResolvedInline({ kind: "fraction", numerator: "1", denominator: "4" });
  const hyperlink = await createResolvedInline({
    kind: "hyperlink", target: { kind: "external", url: "https://example.com/" }, children: [literal],
  });
  const field = await createResolvedInline({ kind: "field", instruction: { kind: "page" }, children: [literal] });
  const control = await createResolvedInline({ kind: "contentControl", properties: { tag: "control" }, children: [literal] });
  const start = await createResolvedInline({
    kind: "rangeStart", id: "comment", range: { kind: "structuredComment", comment },
  });
  const end = await createResolvedInline({ kind: "rangeEnd", id: "comment" });
  const note = await createResolvedInline({ kind: "noteReference", target: noteTarget });
  assert.equal(shape.view.properties.geometry, "ellipse");
  assert.deepEqual(chart.view.properties.values, [1]);
  assert.equal(fraction.view.denominator, "4");
  assert.equal(hyperlink.view.target.url, "https://example.com/");
  assert.equal(field.view.instruction.kind, "page");
  assert.equal(control.view.properties.tag, "control");
  assert.equal(start.view.range.comment.view.metadata.author, "Reviewer");
  assert.equal(end.view.id, start.view.id);
  assert.deepEqual(note.view.target, noteTarget);
  assert.equal(await inlineText([image, shape, chart, fraction, hyperlink, field, control, start, end, note]), "AAA");

  const reused = await resolveComponent({
    kind: "paragraph",
    runs: [literal, tab, line, page].map((value) => ({ kind: "resolved", value })),
  });
  assert.equal(reused.view.text, "A\t\n");
  await assert.rejects(createResolvedInline({ kind: "contentControl", children: [page], properties: {
    text: { kind: "plain" },
  } }), { rule: "invalid_content", path: "children" });
  await assert.rejects(createResolvedBlockControl({ text: { kind: "plain" } }, []),
    { rule: "invalid_content", path: "children" });
  await assert.rejects(inlineText([{ view: literal.view }]), { path: "handles[0]" });
}

export async function standaloneValidationAndDiagnostics() {
  for (const [kind, value] of [["bookmarkName", "_Valid1"], ["rangeId", "range:one"], ["styleId", "style"]]) {
    assert.equal(await validateAuthoringId(kind, value), undefined);
    await assert.rejects(validateAuthoringId(kind, ""), { rule: "invalid_content" });
    await assert.rejects(validateAuthoringId(kind, "\ud800"), (error) =>
      error.path === "value" && /malformed UTF-16/.test(error.message) && !("rule" in error));
  }
  await assert.rejects(validateValue({ kind: "imageTransformation", value: { rotation: 361 } }),
    { rule: "invalid_measurement", path: "rotation" });
  await assert.rejects(validateValue({ kind: "tableCellProperties", value: { gridSpan: 0 } }),
    { rule: "invalid_content", path: "gridSpan" });
  await assert.rejects(validateValue({ kind: "cellMargins", value: { top: -1 } }),
    { rule: "invalid_measurement", path: "top" });
  await assert.rejects(validateValue({ kind: "tableRowProperties", value: { height: { kind: "exact", points: -1 } } }),
    { rule: "invalid_measurement", path: "height" });
  await assert.rejects(validateValue({ kind: "documentDefaults", value: { run: { style: "character-style" } } }),
    { rule: "invalid_content", path: "run.style" });
  const readyInputs = [
    ["runFormat", {}], ["paragraphFormat", {}], ["sectionProperties", {}],
    ["numberingDefinition", { id: 9, levels: [{ level: 0, start: 1, format: "decimal", text: "%1" }] }],
    ["tableRowProperties", {}], ["cellMargins", {}], ["documentDefaults", { run: { bold: false } }],
    ["textRevision", { kind: { kind: "insertion" }, author: "Reviewer" }], ["commentMetadata", {}],
    ["resolvedNote", await resolveComponent({ kind: "note", target: { kind: "footnote", id: 1 }, children: [paragraph("Note")] })],
    ["resolvedCommentBody", await resolveComponent({ kind: "comment", children: [paragraph("Comment")] })],
  ];
  for (const [kind, value] of readyInputs) {
    assert.equal(await validateValue({ kind, value }), undefined, kind);
  }
  const invalid = await createResolvedInline({ kind: "fraction", numerator: "", denominator: "4" });
  await assert.rejects(validateValue({ kind: "resolvedInline", value: invalid }),
    { path: "numerator", rule: "invalid_content" });

  const end = await resolveComponent({ kind: "rangeEnd", id: "unmatched" });
  assert.equal(await validateValue({ kind: "resolvedInline", value: end }), undefined);
  const p = await resolveComponent(paragraph("recover"));
  await assert.rejects(validateValue({ kind: "resolvedInline", value: p }), (error) =>
    error.path === "value" && !("rule" in error));
  assert.equal(await validateValue({ kind: "resolvedParagraph", value: p }), undefined);
  await assert.rejects(resolveDocument(program({
    kind: "paragraph", runs: [{ kind: "resolved", value: end }],
  })), { rule: "invalid_content", path: "sections[0].children[0].runs[0]", message: /no preceding start/ });
  assert.equal(await validateValue({ kind: "resolvedDocument", value: await resolveDocument(program(paragraph("safe"))) }),
    undefined);
}

export async function runResolutionCases() {
  await recursiveValuesMapAndSingleUse();
  await sharedValuesAndNamedDependencyGraph();
  await standaloneAndConditionalComponents();
  await cancelledWaiterDoesNotPoisonSharedWork();
  await resolvedBlockViewsAndReuse();
  await resolvedInlineViewsConstructorsAndReuse();
  await standaloneValidationAndDiagnostics();
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
  await runResolutionCases();
  console.log("Resolution packed: seven R01–R06 shared cases passed.");
}
