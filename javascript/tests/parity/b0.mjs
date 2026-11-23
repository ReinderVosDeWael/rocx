import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createContext, runInContext } from "node:vm";
import { inflateRawSync } from "node:zlib";
import {
  deferred, future, generateDocx, lazy, property, ready,
  resolveComponent, resolveDocument, resolveValue,
} from "rocx-docx";

const timestamp = "2022-02-03T04:05:06Z";
const override = "2023-03-04T05:06:07Z";
const paragraph = (text, extra = {}) => ({ kind: "paragraph", text, ...extra });
const section = (...children) => ({ kind: "section", children });
const program = (...children) => ({ sections: [section(...children)], timestamp });
const text = ({ view }) => view.text;

function gate() {
  return Promise.withResolvers();
}

async function bounded(promise) {
  let timer;
  try {
    return await Promise.race([
      promise,
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error("Rust pass did not settle")), 3000);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

// Read one member from the central directory. This does not parse authoring
// DTOs or reserialize the generated artifact.
function part(docx, wanted) {
  const buffer = Buffer.from(docx);
  const end = buffer.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
  let entry = buffer.readUInt32LE(end + 16);
  while (entry < end && buffer.readUInt32LE(entry) === 0x02014b50) {
    const length = buffer.readUInt16LE(entry + 28);
    const name = buffer.toString("utf8", entry + 46, entry + 46 + length);
    if (name === wanted) {
      const offset = buffer.readUInt32LE(entry + 42);
      const start = offset + 30 + buffer.readUInt16LE(offset + 26) + buffer.readUInt16LE(offset + 28);
      const payload = buffer.subarray(start, start + buffer.readUInt32LE(entry + 20));
      return (buffer.readUInt16LE(entry + 10) === 8 ? inflateRawSync(payload) : payload).toString();
    }
    entry += 46 + length + buffer.readUInt16LE(entry + 30) + buffer.readUInt16LE(entry + 32);
  }
  assert.fail(`ZIP part not found: ${wanted}`);
}

async function legacyGenerateDocxCompatibility() {
  const legacy = {
    blocks: [{ kind: "paragraph", runs: [{ kind: "text", text: "same λ" }] }],
  };
  assert.deepEqual(await generateDocx(legacy, { timestamp }),
    await generateDocx(program(paragraph("same λ"))));
  await assert.rejects(generateDocx({ blocks: [], sections: [] }), /unknown field `sections`/);
  await assert.rejects(generateDocx({ blocks: [{ kind: "table", rows: [] }] }), /rectangular grid/);
  await assert.rejects(generateDocx({ blocks: [] }, { timestamp: "2025-02-29T00:00:00Z" }), TypeError);
}

async function boundaryErrorsAndByteOwnership() {
  const { view: { metadata } } = await resolveDocument({
    sections: [],
    metadata: {
      customProperties: { $rocxHandle: "ordinary data", ["__proto__"]: "another key" },
    },
  });
  assert.equal(metadata.customProperties.$rocxHandle, "ordinary data");
  assert.equal(metadata.customProperties.__proto__, "another key");
  const booleanMetadata = await resolveDocument({
    sections: [], metadata: { customBooleanProperties: { $rocxHandle: false } },
  });
  assert.equal(booleanMetadata.view.metadata.customBooleanProperties.$rocxHandle, false);
  const image = new Uint8Array(await readFile(process.argv[2]));
  const foreign = runInContext("Uint8Array.of(1, 2, ...image, 3).subarray(2, image.length + 2)",
    createContext({ image }));
  const run = await resolveComponent({ kind: "image", data: future(async () => foreign) });
  foreign.fill(0);
  const returned = run.view.image.view.bytes;
  assert.deepEqual(returned, image);
  returned.fill(0);
  assert.deepEqual(run.view.image.view.bytes, image);
  const sharedInput = image.slice();
  const sharedBytes = deferred(future(async () => sharedInput));
  const output1 = await resolveValue(sharedBytes);
  output1.fill(0);
  sharedInput.fill(0);
  assert.deepEqual(await resolveValue(sharedBytes), image);
  const runDeclaration = { kind: "resolved", value: run };
  const doc = await resolveDocument(program({ kind: "paragraph", runs: [runDeclaration] }));
  const rendered = doc.toDocx();
  const expected = rendered.slice();
  rendered.fill(0);
  assert.deepEqual(doc.toDocx(), expected);
  assert.deepEqual(await generateDocx(program({ kind: "paragraph", runs: [runDeclaration] })), expected);
  await assert.rejects(resolveComponent({ kind: "image", data: new Uint16Array([1, 2]) }),
    { path: "imageRun.data", message: /Uint8Array/ });
  await assert.rejects(resolveDocument(program({ kind: "paragraph", runs: [
    { kind: "text", text: future(async () => "\ud800") },
  ] })), { path: "sections[0].children[0].runs[0].text", message: /malformed UTF-16/ });
  const original = { producer: "user-defined cause" };
  await assert.rejects(resolveDocument(program(paragraph(future(async () => { throw original; })))), (error) =>
    error instanceof Error && error.rule === "producer_error" &&
    error.path === "sections[0].children[0].text" && error.cause === original);
  const sharedFailure = property("failing", future(async () => { throw original; }));
  for (let round = 0; round < 2; round++) {
    await assert.rejects(
      resolveComponent(paragraph(future((context) => context.get(sharedFailure.reference())))),
      (error) => error.rule === "producer_error" && error.path === "paragraph.text" && error.cause === original,
    );
  }
  await assert.rejects(resolveComponent({ kind: "image", data: Uint8Array.of(1) }),
    { path: "imageRun", rule: "invalid_content" });
  await assert.rejects(resolveComponent({ kind: "paragraph", runs: [{ kind: "text", text: "\0" }] }),
    { path: "paragraph.runs[0].text", rule: "invalid_xml" });
  await assert.rejects(resolveDocument(program({ kind: "resolved", value: { view: run.view } })),
    { path: "sections[0].children[0].value", message: /resolved handle/ });
  const withComment = await resolveDocument(program(paragraph("text", { commentText: "a comment" })));
  const changed = withComment.toDocx({ timestamp: override });
  assert.equal(withComment.view.timestamp, timestamp);
  assert.match(part(changed, "docProps/core.xml"), new RegExp(override));
  assert.match(part(changed, "word/comments.xml"), new RegExp(timestamp));
  assert.deepEqual(withComment.toDocx(), await generateDocx(program(paragraph("text", { commentText: "a comment" }))));
  assert.throws(() => withComment.toDocx({ profile: "strict" }), /strict/i);
  assert.throws(() => withComment.toDocx({ readLimits: { maxParts: -1 } }), { path: "options.readLimits.maxParts" });
  assert.match((await resolveDocument({ sections: [] })).view.timestamp, /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$/);
  let starts = 0;
  assert.deepEqual(await generateDocx(program(paragraph(future(async () => {
    starts++;
    return "once";
  }))), { timestamp: override }), await generateDocx(program(paragraph("once")), { timestamp: override }));
  assert.equal(starts, 1);
  await assert.rejects(resolveValue("x", { unknown: true }), /unknown option/);
}

async function rustWaitGraphAndGuards() {
  let a, b, c;
  const ids = [];
  const depends = (getRef) => future(async (context) => {
    await Promise.resolve(); // The dependency is not visible at callback start.
    const ref = getRef();
    ids.push(ref.id);
    return context.get(ref);
  });
  a = property("A", depends(() => b.reference()));
  b = property("B", depends(() => c.reference()));
  c = property("C", depends(() => a.reference()));
  const controller = new AbortController();
  try {
    await assert.rejects(bounded(resolveDocument(program(paragraph(a.reference())), {
      signal: controller.signal,
    })), { rule: "dependency_cycle", path: "sections[0].children[0].text" });
    assert.equal(new Set(ids).size, 3);
  } finally {
    controller.abort();
  }

  let fieldsStarted = 0;
  const invisible = () => lazy(() => { fieldsStarted++; return "invisible"; });
  assert.equal(await resolveComponent(paragraph(invisible(), { condition: false })), undefined);
  assert.equal(await resolveComponent(paragraph({ invalid: true }, { condition: false })), undefined);
  const predicateFailure = new Error("predicate failure");
  await assert.rejects(resolveDocument(program(paragraph(invisible(), {
    condition() { throw predicateFailure; },
  }))), (error) => error instanceof Error && error.cause === predicateFailure &&
    error.path === "sections[0].children[0].condition" && !Object.hasOwn(error, "rule"));
  assert.equal(fieldsStarted, 0);
  const excluded = property("excluded", invisible(), { condition: false });
  await assert.rejects(resolveValue(excluded.reference()), { rule: "excluded_dependency" });
  assert.equal(fieldsStarted, 0);
  const guarded = property("throwing", invisible(), { condition: () => { throw predicateFailure; } });
  for (let round = 0; round < 2; round++) {
    await assert.rejects(resolveValue(guarded.reference()),
      (error) => error.cause === predicateFailure && error.path === "condition" && !Object.hasOwn(error, "rule"));
  }
  assert.equal(fieldsStarted, 0);
}

async function concurrentWaitersAndCancellation() {
  const started = gate();
  const release = gate();
  const enteredSecond = gate();
  let calls = 0;
  let sharedSignal;
  const shared = property("shared", future(async ({ signal }) => {
    sharedSignal = signal;
    calls++;
    started.resolve();
    await release.promise;
    return "shared";
  }));
  const cancellation = new AbortController();
  const first = resolveComponent(paragraph(shared.reference()), { signal: cancellation.signal });
  const second = resolveComponent(paragraph(future(async (context) => {
    const get = context.get(shared.reference());
    enteredSecond.resolve();
    return get;
  })));
  await bounded(Promise.all([started.promise, enteredSecond.promise]));
  release.resolve();
  assert.deepEqual((await bounded(Promise.all([first, second]))).map(text), ["shared", "shared"]);
  assert.equal(calls, 1);
  assert.equal(sharedSignal.aborted, false);

  const begin = gate();
  const finish = gate();
  const sharedWork = deferred(future(async ({ signal }) => {
    begin.resolve(signal);
    await finish.promise;
    return "survived";
  }));
  const cancelled = resolveValue(sharedWork, { signal: cancellation.signal });
  const survived = resolveValue(sharedWork);
  const workSignal = await bounded(begin.promise);
  const abortCause = new Error("caller cancelled");
  cancellation.abort(abortCause);
  await assert.rejects(bounded(cancelled), { name: "AbortError", cause: abortCause });
  assert.equal(workSignal.aborted, false, "a cancelled waiter cannot abort a shared producer");
  finish.resolve();
  assert.equal(await bounded(survived), "survived");

  const controller = new AbortController();
  const ownedStart = gate();
  const late = gate();
  const owned = resolveValue(future(async ({ signal }) => {
    ownedStart.resolve(signal);
    await late.promise;
    return "too late";
  }), { signal: controller.signal });
  const ownedSignal = await bounded(ownedStart.promise);
  controller.abort();
  await assert.rejects(bounded(owned), { name: "AbortError" });
  assert.equal(ownedSignal.aborted, true);
  late.resolve();
  const stages = lazy(() => future(async () => lazy(() => "a"))).map((value) => ready(value + "b"));
  assert.equal(await resolveValue(stages), "ab");
  await assert.rejects(resolveValue(stages), (error) => error.cause instanceof TypeError && /single-use/.test(error.cause.message));
  assert.equal(text(await resolveComponent(paragraph("recovered"))), "recovered");
}

await legacyGenerateDocxCompatibility();
await boundaryErrorsAndByteOwnership();
await rustWaitGraphAndGuards();
await concurrentWaitersAndCancellation();
console.log("B0 packed: legacy compatibility, boundary errors/ownership, real Rust wait graph, guards and cancellation");
