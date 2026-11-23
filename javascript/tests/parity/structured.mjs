import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { inflateRawSync } from "node:zlib";
import {
  bookmarkEnd, bookmarkStart, createResolvedBlockControl, createResolvedInline, future, inlineText,
  lazy, resolveComponent, resolveDocument, validateAuthoringId, validateValue,
} from "rocx-docx";

const timestamp = "2024-02-29T01:02:03Z";
const text = (text, extra) => ({ kind: "text", text, ...extra });
const paragraph = (...runs) => ({ kind: "paragraph", runs });
const section = (...children) => ({ kind: "section", children });
const program = (...children) => ({ timestamp, sections: [section(...children)] });
const reusable = (value) => ({ kind: "resolved", value });
const external = { kind: "external", url: "https://example.test/target?q=1&lang=ja#ここ" };
const bookmark = { kind: "bookmark", name: "Summary" };
const field = (instruction, children = [], properties) => ({ kind: "field", instruction, children, properties });
const link = (target, children = [], properties) => ({ kind: "hyperlink", target, children, properties });
const control = (children, properties) => ({ kind: "contentControl", children, properties });
const rangeStart = (id, range) => ({ kind: "rangeStart", id, range });
const rangeEnd = (id) => ({ kind: "rangeEnd", id });
const noteRef = (target) => ({ kind: "noteReference", target });
const inlines = (document) => document.view.sections[0].view.paragraphs[0].view.runs;

function countedProducers() {
  const calls = new Map();
  const record = (name) => {
    calls.set(name, (calls.get(name) ?? 0) + 1);
  };
  return {
    lazy: (name, value) => lazy(() => { record(name); return value; }),
    future: (name, value) => future(async () => { record(name); return value; }),
    assertOnce: (...names) => assert.deepEqual(calls, new Map(names.map((name) => [name, 1]))),
  };
}

// Inspect raw package relationships and markup separately from Rust's views.
function part(bytes, wanted) {
  const data = Buffer.from(bytes);
  assert.equal(data.readUInt32LE(data.length - 22), 0x06054b50);
  let entry = data.readUInt32LE(data.length - 6);
  while (data.readUInt32LE(entry) === 0x02014b50) {
    const nameLen = data.readUInt16LE(entry + 28);
    const name = data.toString("utf8", entry + 46, entry + 46 + nameLen);
    if (name === wanted) {
      const local = data.readUInt32LE(entry + 42);
      const start = local + 30 + data.readUInt16LE(local + 26) + data.readUInt16LE(local + 28);
      const payload = data.subarray(start, start + data.readUInt32LE(entry + 20));
      const method = data.readUInt16LE(entry + 10);
      assert.ok(method === 0 || method === 8, `unexpected zip method ${method}`);
      return (method === 8 ? inflateRawSync(payload) : payload).toString();
    }
    entry += 46 + nameLen + data.readUInt16LE(entry + 30) + data.readUInt16LE(entry + 32);
  }
  assert.fail(`missing package part ${wanted}`);
}

async function linksAndFieldCaches() {
  const entered = [];
  let release;
  const gate = new Promise((resolve) => { release = resolve; });
  const sibling = (name, value) => future(async () => {
    entered.push(name);
    if (entered.length === 3) release();
    await gate;
    return value;
  });
  const pending = resolveComponent(link(
    sibling("target", external),
    sibling("children", [text("outside")]),
    sibling("properties", { tooltip: "external", history: false }),
  ));
  const linked = await Promise.race([
    pending,
    new Promise((_, reject) => {
      const timeout = setTimeout(() => reject(new Error("link siblings did not start concurrently")), 3000);
      pending.then(() => clearTimeout(timeout), () => clearTimeout(timeout));
    }),
  ]);
  assert.deepEqual(new Set(entered), new Set(["target", "children", "properties"]));
  assert.deepEqual(linked.view.target, external);
  assert.deepEqual(linked.view.properties, { tooltip: "external", history: false });
  assert.equal(linked.view.children[0].view.text, "outside");

  const produced = countedProducers();
  const page = field(
    produced.lazy("page instruction", { kind: "page" }),
    produced.future("page cache", [text("42")]),
    produced.lazy("page properties", { dirty: false, locked: false }),
  );
  const nested = field({ kind: "numPages" }, [text("5")]);
  const referenceInstruction = { kind: "reference", name: "Summary" };
  const reference = field(
    produced.future("reference instruction", referenceInstruction),
    produced.lazy("reference cache", [nested]),
    produced.future("reference properties", { form: "complex", locked: true }),
  );
  const raw = field({ kind: "raw", text: " MERGEFIELD A & B " }, [text("raw cache")]);
  const toc = field({ kind: "tableOfContents" }, [text("cached TOC")]);
  const doc = await resolveDocument({
    timestamp,
    sections: [{
      ...section(paragraph(await bookmarkStart("Summary"), reusable(linked), await bookmarkEnd("Summary"),
        link(bookmark, [text("inside")]), page, reference, raw, toc)),
      headers: { default: { kind: "header", children: [paragraph(reusable(linked))] } },
    }],
  });
  const [start, , end, innerLink, simpleField, complexField, rawField, tocField] = inlines(doc);
  assert.equal(start.view.id, end.view.id);
  assert.equal(start.view.id, "bookmark:Summary");
  assert.deepEqual(innerLink.view.target, bookmark);
  assert.deepEqual(simpleField.view.instruction, { kind: "page" });
  assert.deepEqual(simpleField.view.properties, { form: "simple", dirty: false, locked: false });
  assert.equal(simpleField.view.children[0].view.displayText, "42");
  assert.equal(simpleField.view.displayText, "42");
  assert.deepEqual(complexField.view.instruction, referenceInstruction);
  assert.deepEqual(complexField.view.properties, { form: "complex", locked: true });
  assert.equal(complexField.view.children[0].view.instruction.kind, "numPages");
  assert.equal(complexField.view.displayText, "5");
  assert.equal(rawField.view.instructionText, " MERGEFIELD A & B ");
  assert.match(tocField.view.instructionText, /TOC/);
  const bytes = doc.toDocx();
  const body = part(bytes, "word/document.xml");
  assert.match(body, /<w:hyperlink w:anchor="Summary"/);
  assert.match(body, /<w:fldSimple[^>]+w:dirty="0"[^>]*><w:r><w:t[^>]*>42<\/w:t>/);
  assert.match(body, /<w:fldChar w:fldCharType="begin"/);
  assert.match(body, /<w:instrText[^>]*> REF Summary /);
  assert.match(body, /MERGEFIELD A &amp; B/);
  for (const rels of ["word/_rels/document.xml.rels", "word/_rels/header1.xml.rels"]) {
    assert.match(part(bytes, rels), /hyperlink"[^>]*Target="https:\/\/example\.test\/target\?q=1&amp;lang=ja#ここ"[^>]*TargetMode="External"/);
  }
  produced.assertOnce("page instruction", "page cache", "page properties",
    "reference instruction", "reference cache", "reference properties");
  await assert.rejects(resolveComponent(field({ kind: "raw", text: " \t " })), { rule: "invalid_content" });
  await assert.rejects(resolveComponent(field({ kind: "raw", text: "x\0" })), {
    rule: "invalid_content", path: "field.instruction.text",
  });
  await assert.rejects(resolveComponent(link({ kind: "external", url: "relative/path" })), {
    rule: "invalid_relationship", path: "hyperlink.target.url",
  });
  await assert.rejects(resolveComponent(link(undefined)), { path: "hyperlink.target.kind" });
  await assert.rejects(resolveComponent(link(external, [field({ kind: "page" })])), { rule: "invalid_content" });
  await assert.rejects(resolveComponent(field({ kind: "page" }, [rangeEnd("no")])), { rule: "invalid_content" });
  for (const unresolved of [link(bookmark), field(referenceInstruction)]) {
    const snapshot = await resolveDocument(program(paragraph(unresolved)));
    assert.throws(() => snapshot.toDocx(), /bookmark/i);
  }
}

async function noteAndRangeStoryComposition() {
  const produced = countedProducers();
  let excludedLoads = 0;
  const hidden = lazy(() => { excludedLoads++; throw new Error("must stay guarded"); });
  const foot = { kind: "footnote", id: 7 };
  const end = { kind: "endnote", id: 7 };
  const metadata = { author: "Ada", initials: "AL", date: timestamp };
  const comment = {
    kind: "comment", metadata: future(async () => metadata),
    children: [paragraph(link(external, [text("rich comment")]))],
  };
  const commentBody = await resolveComponent({ ...comment, metadata });
  const richComment = { ...comment, children: produced.future("rich comment children", comment.children) };
  const id = "rich-range";
  const reusedId = "reused-comment";
  const doc = await resolveDocument({
    ...program(
      paragraph(rangeStart(lazy(() => id), future(async () => ({ kind: "structuredComment", comment: richComment }))),
        noteRef(produced.future("footnote reference target", foot)),
        rangeStart("flat", { kind: "comment", text: "flat comment", author: "Grace" }), text("first")),
      paragraph(noteRef(produced.lazy("endnote reference target", end)),
        rangeEnd(produced.future("rich comment closing id", id)), text("second"), rangeEnd("flat"),
        rangeStart(reusedId, { kind: "structuredComment", comment: commentBody }),
        text("third"), rangeEnd(produced.future("reused comment closing id", reusedId)),
        { ...rangeStart(hidden, hidden), condition: false }),
    ),
    notes: lazy(() => [
      { kind: "note", target: produced.lazy("footnote target", foot),
        children: produced.future("footnote children", [paragraph(link(external, [text("foot body")]))]) },
      future(async () => ({ kind: "note", target: produced.future("endnote target", end),
        children: produced.lazy("endnote children", [paragraph(text("end body"))]) })),
      { kind: "note", target: hidden, children: hidden, condition: () => false },
    ]),
  });
  assert.equal(excludedLoads, 0);
  assert.deepEqual(doc.view.notes.map(({ view }) => view.target), [foot, end]);
  assert.deepEqual(doc.view.notes.map(({ view }) => view.blocks[0].view.text), ["foot body", "end body"]);
  const paragraphs = doc.view.sections[0].view.paragraphs;
  const [richStart, footReference] = paragraphs[0].view.runs;
  const [endReference, richEnd, , , reusedStart, , reusedEnd] = paragraphs[1].view.runs;
  const references = [footReference, endReference];
  assert.deepEqual(references.map(({ view }) => view.target), [foot, end]);
  assert.deepEqual(references.map(({ view }) => view.displayText), ["", ""]);
  assert.deepEqual([richStart, richEnd, reusedStart, reusedEnd].map(({ view }) => [view.kind, view.id]), [
    ["rangeStart", id], ["rangeEnd", id], ["rangeStart", reusedId], ["rangeEnd", reusedId],
  ]);
  assert.equal(richStart.view.range.kind, "structuredComment");
  const richBody = richStart.view.range.comment.view;
  assert.deepEqual(richBody.metadata, metadata);
  assert.deepEqual(richBody.blocks.map(({ view }) => view.text), ["rich comment"]);
  assert.deepEqual(richBody.blocks[0].view.runs[0].view.target, external);
  const bytes = doc.toDocx();
  const body = part(bytes, "word/document.xml");
  assert.match(body, /<w:footnoteReference w:id="\d+"\/>/);
  assert.match(body, /<w:endnoteReference w:id="\d+"\/>/);
  const comments = part(bytes, "word/comments.xml");
  assert.match(comments, /w:initials="AL"/);
  assert.match(comments, /flat comment/);
  const richCommentIds = [...comments.matchAll(/<w:comment\b[^>]*\bw:id="(\d+)"[^>]*>([\s\S]*?)<\/w:comment>/g)]
    .filter(([, , content]) => /<w:t[^>]*>rich comment<\/w:t>/.test(content))
    .map(([, id]) => id);
  assert.equal(richCommentIds.length, 2);
  assert.equal(new Set(richCommentIds).size, 2);
  for (const id of richCommentIds) {
    const start = body.indexOf(`<w:commentRangeStart w:id="${id}"/>`);
    const end = body.indexOf(`<w:commentRangeEnd w:id="${id}"/>`);
    const reference = body.indexOf(`<w:commentReference w:id="${id}"/>`);
    assert.ok(start >= 0 && start < end && end < reference, `missing or out-of-order range markers for comment ${id}`);
  }
  assert.match(part(bytes, "word/footnotes.xml"), /foot body/);
  assert.match(part(bytes, "word/endnotes.xml"), /end body/);
  assert.match(part(bytes, "word/_rels/footnotes.xml.rels"), /TargetMode="External"/);
  assert.match(part(bytes, "word/_rels/comments.xml.rels"), /TargetMode="External"/);
  produced.assertOnce("rich comment children", "rich comment closing id", "reused comment closing id",
    "footnote target", "footnote children", "footnote reference target",
    "endnote target", "endnote children", "endnote reference target");
  await assert.rejects(resolveComponent(rangeStart(id, {
    kind: "structuredComment", comment: { ...comment, children: hidden, condition: false },
  })), { rule: "excluded_dependency" });
  assert.equal(excludedLoads, 0);
  await assert.rejects(resolveComponent({ ...comment, metadata: { ...metadata, date: "2023-02-29T00:00:00Z" } }), /date/i);
  const unresolved = await resolveDocument(program(paragraph(noteRef(foot))));
  assert.throws(() => unresolved.toDocx(), /note/i);
  await assert.rejects(resolveDocument(program(paragraph(rangeEnd(id)))), /preceding start/i);
  await assert.rejects(resolveDocument(program(paragraph(rangeStart(id, { kind: "comment", text: "orphan" })))), /has no end/i);
  for (const invalid of [
    paragraph(rangeStart(id, { kind: "comment", text: "wrong story" }), text("range"), rangeEnd(id)),
    paragraph(noteRef(foot)),
  ]) {
    await assert.rejects(resolveDocument({
      ...program(),
      sections: [{ ...section(), headers: { default: { kind: "header", children: [invalid] } } }],
    }), /body story/i);
  }
}

const insertion = { kind: { kind: "insertion" }, author: "Ada" };
const deletion = { kind: { kind: "deletion" }, author: "Ada" };
const source = { kind: { kind: "moveSource", name: "Move" }, author: "Ada", date: timestamp };
const destination = { ...source, kind: { kind: "moveDestination", name: "Move" } };
const selected = {
  kind: "dropDown", selected: "b", items: [{ displayText: "Alpha", value: "a" }, { displayText: "Beta", value: "b" }],
};

async function controlsAndAuthoredRevisions() {
  const produced = countedProducers();
  const blockProperties = {
    id: 901, tag: "plain", alias: "Plain", lock: "controlLocked", text: { kind: "plain", multiLine: true },
  };
  const inlineProperties = { id: 902, lock: "contentLocked", text: selected };
  const doc = await resolveDocument(program(
    control(produced.future("block children", [paragraph(text("line\nbreak"))]),
      produced.lazy("block properties", blockProperties)),
    paragraph(
      control(produced.lazy("inline children", [text("Beta"), { ...text("hidden"), condition: false }]),
        produced.future("inline properties", inlineProperties)),
      control([text("rich")], { id: 903, lock: "controlAndContentLocked" }),
      control([text("open")], { id: 904, lock: "unlocked", text: { kind: "rich" } }),
      text("insert", { revision: produced.lazy("insertion revision", insertion) }),
      text("delete", { revision: produced.future("deletion revision", deletion) }),
      text("moved", { revision: source }), text("moved", { revision: destination }),
      text("plain lazy", { revision: produced.lazy("lazy undefined revision", undefined) }),
      text("plain future", { revision: produced.future("future undefined revision", undefined) }),
    ),
  ));
  const block = doc.view.sections[0].view.blocks[0].view;
  assert.deepEqual(block.properties, blockProperties);
  assert.deepEqual(block.children.map(({ view }) => view.text), ["line\nbreak"]);
  const [dropDown, rich, open, ins, del, from, to, plainLazy, plainFuture] = inlines(doc);
  assert.deepEqual(dropDown.view.properties, inlineProperties);
  assert.deepEqual(dropDown.view.children.map(({ view }) => view.text), ["Beta"]);
  assert.equal(dropDown.view.displayText, "Beta");
  assert.equal(rich.view.properties.text.kind, "rich");
  assert.equal(open.view.properties.lock, "unlocked");
  const trackedAndPlain = [ins, del, from, to, plainLazy, plainFuture];
  assert.deepEqual(trackedAndPlain.map((run) => run.view.displayText), ["insert", "", "", "moved", "plain lazy", "plain future"]);
  assert.deepEqual(trackedAndPlain.map((run) => run.view.revision), [insertion, deletion, source, destination, undefined, undefined]);
  const xml = part(doc.toDocx(), "word/document.xml");
  for (const tag of ["ins", "del", "moveFrom", "moveTo", "moveFromRangeStart", "moveToRangeEnd", "dropDownList"]) {
    assert.match(xml, new RegExp(`<w:${tag}[ >]`));
  }
  for (const token of ["sdtLocked", "contentLocked", "sdtContentLocked", "unlocked"]) {
    assert.match(xml, new RegExp(`<w:lock w:val="${token}"`));
  }
  assert.match(xml, /<w:delText[^>]*>delete</);
  assert.match(xml, /<w:text w:multiLine="1"/);
  assert.match(xml, /<w:r><w:t[^>]*>plain lazy<\/w:t><\/w:r><w:r><w:t[^>]*>plain future<\/w:t><\/w:r><\/w:p>/);
  produced.assertOnce("block children", "block properties", "inline children", "inline properties",
    "insertion revision", "deletion revision", "lazy undefined revision", "future undefined revision");
  const rejectedInline = (declaration, rule = "invalid_content") =>
    assert.rejects(resolveComponent(declaration, { scope: "inline" }), { rule });
  await rejectedInline(control([text("x")], { id: 0 }));
  await rejectedInline(control([text("x")], { id: 2 ** 31 }));
  await rejectedInline(control([text("wrong cache")], { text: selected }));
  await rejectedInline(control([text("Beta")], { text: { ...selected, selected: "absent" } }));
  await rejectedInline(control([text("Beta")], { text: { ...selected, items: [...selected.items, selected.items[0]] } }));
  await rejectedInline(control([text("x\ny")], { text: { kind: "plain", multiLine: false } }));
  await assert.rejects(resolveDocument(program(paragraph(
    text("moved", { revision: source }), text("moved", { revision: { ...destination, author: "Other" } }),
  ))), /identical text, author and date/);
  await assert.rejects(resolveDocument(program(paragraph(
    text("moved", { revision: source }), text("moved", { revision: { ...destination, date: "2024-03-01T00:00:00Z" } }),
  ))), /identical text, author and date/);
  await assert.rejects(resolveDocument(program(paragraph(
    text("moved", { revision: source }), text("moved", { revision: { ...destination, kind: { ...destination.kind, name: "Other" } } }),
  ))), /one source and one destination/);
}

async function standaloneAndConditionalComponents() {
  const skipped = lazy(() => assert.fail("excluded component started its fields"));
  const throwing = () => assert.fail("excluded field started its producer");
  const variants = [
    [link(external, [text("link")]), { target: skipped, children: skipped, properties: lazy(throwing) }],
    [field({ kind: "page" }), { instruction: skipped, children: skipped, properties: skipped }],
    [control([text("inline")]), { children: skipped, properties: skipped }, { scope: "inline" }],
    [control([paragraph(text("block"))]), { children: skipped, properties: skipped }, { scope: "block" }],
    [rangeStart("id", bookmark), { id: skipped, range: skipped }],
    [rangeEnd("id"), { id: skipped }],
    [{ kind: "note", target: { kind: "footnote", id: 2 }, children: [paragraph(text("note"))] }, { target: skipped, children: skipped }],
    [noteRef({ kind: "endnote", id: 2 }), { target: skipped }],
    [{ kind: "comment", children: [paragraph(text("comment"))] }, { children: skipped, metadata: skipped }],
  ];
  for (const [base, fields, options] of variants) {
    assert.equal(await resolveComponent({ ...base, ...fields, condition: () => false }, options), undefined);
    assert.ok(await resolveComponent({ ...base, condition: true }, options));
  }
  await assert.rejects(resolveComponent(control([])));
  await assert.rejects(resolveComponent(link(external), { scope: "inline" }), { path: "scope" });
}

async function resolvedBlockViewsAndReuse() {
  let runs = 0;
  const block = await resolveComponent(control([
    paragraph(text(lazy(() => { runs++; return "nested"; }))),
    { ...paragraph(text("excluded")), condition: false },
  ], { alias: "Reusable" }), { scope: "block" });
  const direct = await resolveComponent(paragraph(text("direct")));
  assert.equal(block.view.children.length, 1);
  assert.equal(block.view.children[0].view.text, "nested");
  assert.equal(block.view.properties.alias, "Reusable");
  const reconstructed = await createResolvedBlockControl({ alias: "Reusable" }, block.view.children);
  await assert.rejects(createResolvedBlockControl({}, [block]), { rule: "invalid_content", path: "children[0]" });
  await assert.rejects(createResolvedBlockControl({ text: selected }, [direct, direct]), { rule: "invalid_content" });
  const first = await resolveDocument(program(reusable(block), reusable(direct)));
  const second = await resolveDocument(program(reusable(reconstructed), reusable(direct)));
  assert.deepEqual(first.view.sections[0].view.paragraphs.map(({ view }) => view.text), ["direct"]);
  assert.equal(first.view.sections[0].view.blocks.length, 2);
  const bytes = first.toDocx();
  const secondBytes = second.toDocx();
  assert.deepEqual(bytes, secondBytes);
  bytes.fill(0);
  assert.deepEqual(first.toDocx(), secondBytes);
  assert.equal(runs, 1);
}

async function resolvedInlineViewsConstructorsAndReuse(imagePath) {
  let runs = 0;
  const leaf = await resolveComponent(text(lazy(() => { runs++; return "leaf"; })));
  const inserted = await createResolvedInline({ kind: "text", text: "ins", revision: insertion });
  const deleted = await createResolvedInline({ kind: "text", text: "del", revision: deletion });
  const attrs = { format: { bold: false }, comment: { text: "annotation", author: "Ada" } };
  const tab = await createResolvedInline({ kind: "tab", ...attrs });
  const line = await createResolvedInline({ kind: "break", breakType: "line", ...attrs });
  const page = await createResolvedInline({ kind: "break", breakType: "page" });
  const column = await createResolvedInline({ kind: "break", breakType: "column" });
  const wrapping = await createResolvedInline({ kind: "break", breakType: "textWrapping" });
  const hyperlink = await createResolvedInline(link(external, [leaf], { history: false }));
  const cache = await createResolvedInline(field({ kind: "raw", text: " RAW " }, [hyperlink]));
  const sdt = await createResolvedInline(control([leaf], { text: { kind: "plain" } }));
  const start = await createResolvedInline(rangeStart("manual", bookmark));
  const end = await createResolvedInline(rangeEnd("manual"));
  const ref = await createResolvedInline(noteRef({ kind: "endnote", id: 2 }));
  const fraction = await createResolvedInline({ kind: "fraction", numerator: "1", denominator: "3" });
  assert.equal(fraction.view.numerator, "1");
  const shape = await createResolvedInline({ kind: "shape", properties: { width: 72, height: 72 } });
  const chart = await createResolvedInline({
    kind: "columnChart", properties: { width: 72, height: 72, categories: ["A"], values: [1] },
  });
  const imageRun = await resolveComponent({ kind: "image", data: readFileSync(imagePath) });
  const image = await createResolvedInline({ kind: "image", image: imageRun.view.image, ...attrs });
  const all = [inserted, deleted, tab, line, page, column, wrapping, hyperlink, cache, sdt, start, end, ref, fraction, shape, chart, image];
  assert.deepEqual(new Set(all.map(({ view }) => view.kind)),
    new Set(["text", "tab", "break", "hyperlink", "field", "contentControl", "rangeStart", "rangeEnd",
      "noteReference", "fraction", "shape", "columnChart", "image"]));
  assert.equal(await inlineText(all.slice(0, 13)), "ins\t\n\nleafleafleaf");
  assert.equal(deleted.view.text, "del");
  for (const run of [tab, line, image]) {
    assert.equal(run.view.format.bold, false);
    assert.deepEqual(run.view.comment, attrs.comment);
  }
  assert.equal(image.view.image.view.bytes[0], imageRun.view.image.view.bytes[0]);
  await assert.rejects(createResolvedInline(link(external, [cache])), { rule: "invalid_content", path: "children[0]" });
  await assert.rejects(createResolvedInline(field({ kind: "page" }, [sdt])), { rule: "invalid_content", path: "children[0]" });
  await assert.rejects(createResolvedInline(control([hyperlink])), { rule: "invalid_content", path: "children[0]" });
  await assert.rejects(createResolvedInline(link(external, [leaf.view])), { path: "children[0]" });
  const make = () => resolveDocument(program(paragraph(...[hyperlink, cache, sdt].map(reusable))));
  assert.deepEqual((await make()).toDocx(), (await make()).toDocx());
  assert.equal(runs, 1);
}

async function standaloneValidationAndDiagnostics() {
  await validateAuthoringId("bookmarkName", "_Good_42");
  await validateAuthoringId("rangeId", "a range");
  await validateAuthoringId("styleId", "style");
  await assert.rejects(validateAuthoringId("bookmarkName", "1name"), { rule: "invalid_content", path: "value" });
  await assert.rejects(validateAuthoringId("rangeId", ""), { rule: "invalid_content", path: "value" });
  await assert.rejects(resolveComponent(rangeStart("valid-id", { kind: "bookmark", name: "1name" })), {
    rule: "invalid_content", path: "rangeStart.range.name",
  });
  await assert.rejects(resolveComponent(rangeEnd("")), { rule: "invalid_content", path: "rangeEnd.id" });
  await assert.rejects(bookmarkStart("\uD800"), (error) =>
    error.path === "name" && error.rule === undefined && /UTF-16/.test(error.message));
  await assert.rejects(validateValue({ kind: "textRevision", value: { ...source, date: undefined } }), {
    rule: "invalid_content", path: "date",
  });
  await assert.rejects(validateValue({ kind: "commentMetadata", value: { date: "2024-02-30T00:00:00Z" } }), {
    rule: "invalid_content", path: "date",
  });
  await assert.rejects(validateValue({ kind: "textRevision", value: { ...insertion, author: " " } }), {
    rule: "invalid_content", path: "author",
  });
  await validateValue({ kind: "textRevision", value: source });
  await validateValue({ kind: "commentMetadata", value: { initials: "A", date: timestamp } });
  const note = await resolveComponent({ kind: "note", target: { kind: "footnote", id: 9 }, children: [] });
  const comment = await resolveComponent({ kind: "comment", children: [] });
  await validateValue({ kind: "resolvedNote", value: note });
  await validateValue({ kind: "resolvedCommentBody", value: comment });
  await assert.rejects(validateValue({ kind: "resolvedCommentBody", value: note }), { path: "value" });
  const dangling = await resolveComponent(rangeEnd("missing"));
  await validateValue({ kind: "resolvedInline", value: dangling });
  await assert.rejects(resolveDocument(program(paragraph(reusable(dangling)))), /preceding start/i);
}

// Deterministic independent native fixture can be supplied by the packed runner.
async function structuredFixture() {
  return (await resolveDocument(program(
    control([paragraph(text("block"))], { id: 10, tag: "block" }),
    paragraph(
      await bookmarkStart("Summary"), text("marker"), await bookmarkEnd("Summary"),
      link(bookmark, [text("link")], { history: false }),
      field({ kind: "page" }, [text("3")], { form: "complex", dirty: false, locked: true }),
      control([text("Beta")], { id: 11, text: selected }),
      text("tracked", { revision: insertion }),
    ),
  ))).toDocx();
}

const [imagePath, nativeFixture, output] = process.argv.slice(2);
assert.ok(imagePath, "pass a PNG fixture path to structured.mjs");
await linksAndFieldCaches();
await noteAndRangeStoryComposition();
await controlsAndAuthoredRevisions();
await standaloneAndConditionalComponents();
await resolvedBlockViewsAndReuse();
await resolvedInlineViewsConstructorsAndReuse(imagePath);
await standaloneValidationAndDiagnostics();
const fixture = await structuredFixture();
if (nativeFixture) assert.deepEqual(Buffer.from(fixture), readFileSync(nativeFixture));
if (output) writeFileSync(output, fixture);
console.log(`structured parity: A13–A15/R03–R06 passed${nativeFixture ? " (including exact native DOCX bytes)" : ""}`);
