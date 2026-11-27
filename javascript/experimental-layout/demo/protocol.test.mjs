// Run the real main module with a minimal DOM and deterministic worker delivery.
import assert from "node:assert/strict";
import { test } from "node:test";

function element() {
  const listeners = new Map();
  return {
    dataset: {},
    addEventListener(type, listener) { listeners.set(type, listener); },
    emit(type, event) { listeners.get(type)?.(event); },
    click() { if (!this.disabled) this.emit("click"); },
    append() {},
    remove() {},
    replaceChildren() {},
    setAttribute() {},
    hasPointerCapture() { return false; },
    setPointerCapture() {},
  };
}

async function setup(t) {
  const controls = Object.fromEntries([
    "pages", "change", "open", "editor", "replacement", "apply", "download", "status", "diagnostic",
  ].map((id) => [id, element()]));
  const root = {
    dataset: { page: "0" },
    closest() { return this; },
    getScreenCTM() { return { inverse() {} }; },
  };
  controls.pages.contains = (node) => node === root;
  const requests = [];
  const downloads = [];
  const worker = element();
  worker.postMessage = (message) => requests.push(message);
  worker.terminate = () => assert.fail(controls.status.textContent);
  const fakeGlobals = {
    document: {
      body: element(),
      querySelector: (selector) => controls[selector.slice(1)],
      elementFromPoint: () => root,
      createElementNS: element,
      createElement: () => ({
        click() { downloads.push(this.href); },
        remove() {},
      }),
    },
    Worker: class { constructor() { return worker; } },
    DOMPoint: class { matrixTransform() { return { x: 643, y: 640 }; } },
  };
  for (const [key, value] of Object.entries(fakeGlobals)) globalThis[key] = value;
  t.after(() => {
    for (const key of Object.keys(fakeGlobals)) delete globalThis[key];
  });
  const blobs = [];
  t.mock.method(URL, "createObjectURL", (blob) => {
    blobs.push(blob);
    return `blob:test-${blobs.length}`;
  });
  t.mock.method(globalThis, "setTimeout", () => {});

  await import(`./main.mjs?case=${encodeURIComponent(t.name)}`);
  const reply = (data) => worker.emit("message", { data });
  const layout = (request, docx, editId) => reply({
    type: "layout", id: request.id, revision: request.revision ?? request.id, editId,
    pages: [{ index: 0, lines: [], glyphs: [], rows: [] }], spec: { width: 3203, height: 4608 },
    source: docx ? "sample.docx" : undefined, edition: editId ? 1 : 0,
  });
  reply({ type: "ready", atlas: { unitsPerEm: 2048 }, fontSize: 832 });
  layout(requests.at(-1));
  controls.open.click();
  const opened = requests.at(-1);
  assert.equal(opened.docx, true);
  layout(opened, true);

  const pick = () => {
    const pointer = { button: 0, pointerId: 1 };
    controls.pages.emit("pointerdown", pointer);
    controls.pages.emit("pointerup", pointer);
    const point = requests.at(-1);
    assert.equal(point.type, "point");
    reply({ ...point, focus: { paragraph: 0, offset: 0 } });
    assert.equal(controls.apply.disabled, false);
  };
  return { controls, downloads, requests, blobs, reply, layout, pick, opened };
}

test("a failed reset paints the latest worker commit skipped during fast switching", async (t) => {
  const { controls, requests, reply, layout, opened } = await setup(t);
  controls.change.click();
  const fixed = requests.at(-1);
  controls.open.click();
  const committed = requests.at(-1);
  controls.open.click();
  const newest = requests.at(-1);
  layout(fixed);
  layout(committed, true);
  assert.equal(controls.pages.dataset.revision, opened.id, "do not paint while newest is pending");
  reply({
    type: "error", id: newest.id, request: "layout", source: "sample.docx", message: "load failed",
  });
  assert.match(controls.diagnostic.textContent, /sample\.docx.*load failed/);
  assert.equal(controls.pages.dataset.revision, committed.id);
  controls.download.click();
  assert.deepEqual(requests.at(-1), {
    type: "download", id: newest.id + 1, layoutId: committed.id, revision: committed.id,
  });
});

test("a failed reset without a new commit keeps the current DOCX and caret", async (t) => {
  const { controls, requests, reply, pick, opened } = await setup(t);
  pick();
  controls.open.click();
  const failed = requests.at(-1);
  reply({ type: "error", id: failed.id, request: "layout", message: "load failed" });
  assert.equal(controls.pages.dataset.revision, opened.id);
  controls.apply.click();
  assert.equal(requests.at(-1).type, "edit");
  assert.equal(requests.at(-1).revision, opened.id);
});

for (const afterEditReply of [false, true]) {
  const phase = afterEditReply ? "after the new revision paints" : "while the edit is pending";
  test(`download honors its original revision ${phase}`, async (t) => {
    const { controls, requests, downloads, blobs, reply, layout, pick, opened } = await setup(t);
    pick();
    controls.download.click();
    const requested = requests.at(-1);
    assert.equal(requested.type, "download");
    controls.replacement.value = "A";
    controls.apply.click();
    const edit = requests.at(-1);
    assert.equal(edit.type, "edit");
    const finishEdit = () => layout({ id: opened.id, revision: opened.id + 1 }, true, edit.id);
    if (afterEditReply) finishEdit();

    controls.download.click();
    assert.equal(requests.at(-1), edit, "do not send a second in-flight download");
    const bytes = Uint8Array.of(7, 11, 42);
    reply({ ...requested, revision: requested.revision + 1, bytes });
    reply({ ...requested, layoutId: requested.layoutId + 1, bytes });
    assert.deepEqual(downloads, [], "wrong-revision and wrong-layout replies cannot download");
    reply({ ...requested, bytes });
    reply({ ...requested, bytes });
    assert.deepEqual(downloads, ["blob:test-1"], "the matching response downloads exactly once");
    assert.deepEqual(new Uint8Array(await blobs[0].arrayBuffer()), bytes);
    if (!afterEditReply) finishEdit();
    assert.equal(controls.download.disabled, false);
  });
}
