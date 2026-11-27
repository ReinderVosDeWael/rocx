import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { test } from "node:test";

const [glue, fontPath, fixturePath] = process.argv.slice(2);
const glueUrl = pathToFileURL(resolve(glue));
const wasm = await import(glueUrl);
const font = await readFile(fontPath);
const fixture = JSON.parse(await readFile(fixturePath, "utf8"));
await wasm.default({ module_or_path: await readFile(new URL("./rocx_bg.wasm", glueUrl)) });

function unpack(page) {
  assert.ok(page.lines instanceof Uint32Array);
  assert.ok(page.glyphs instanceof Uint32Array);
  assert.ok(page.carets instanceof Uint32Array);
  let expectedGlyph = 0;
  let expectedCaret = 0;
  const rows = [];
  for (let i = 0; i < page.lines.length; i += 9) {
    const [paragraph, line, start, end, width, glyphStart, glyphCount, caretStart, caretCount] =
      page.lines.subarray(i, i + 9);
    assert.equal(glyphStart, expectedGlyph);
    assert.equal(caretStart, expectedCaret);
    expectedGlyph += glyphCount;
    expectedCaret += caretCount;
    const glyphs = [];
    for (let index = glyphStart; index < expectedGlyph; index++) {
      const [id, x, y, advance, cluster] = page.glyphs.subarray(index * 5, index * 5 + 5);
      glyphs.push({ id, x: x | 0, y: y | 0, advance: advance | 0, cluster });
    }
    const carets = [];
    for (let index = caretStart; index < expectedCaret; index++) {
      const [offset, x] = page.carets.subarray(index * 2, index * 2 + 2);
      carets.push({ offset, x: x | 0 });
    }
    rows.push({ paragraph, line, geometry: { start, end, width: width | 0, glyphs, carets } });
  }
  assert.equal(page.glyphs.length, expectedGlyph * 5);
  assert.equal(page.carets.length, expectedCaret * 2);
  return rows;
}

test("native parity for pages, scalar hits, and reuse across revisions", () => {
  const session = new wasm.ExperimentalLayoutSession(font, fixture.fontSize);
  const snapshots = fixture.revisions.map(({ input }) =>
    session.layout(input, fixture.width, fixture.linesPerPage));
  session.free(); // Snapshots need neither the live session nor its cache.
  try {
    for (const [i, snapshot] of snapshots.entries()) {
      const { snapshot: native, hits } = fixture.revisions[i];
      const revision = i + 1;
      assert.equal(snapshot.revision, revision);
      assert.equal(snapshot.pageCount, native.pages.length);
      assert.ok(snapshot.pageCount > 1);
      assert.deepEqual(snapshot.reuse, {
        shapedHits: native.reuse.shaped_hits,
        lineHits: native.reuse.line_hits,
        pageHits: native.reuse.page_hits,
      });
      for (const [pageIndex, expected] of native.pages.entries()) {
        const page = snapshot.page(pageIndex);
        assert.equal(page.revision, revision);
        assert.equal(page.index, pageIndex);
        assert.equal("rows" in page, false);
        assert.deepEqual(unpack(page), expected.lines.map(({ paragraph, line }) =>
          ({ paragraph, line, geometry: native.paragraphs[paragraph][line] })));
      }
      for (const [page, row, x, expected] of hits) {
        assert.deepEqual(snapshot.hitRow(page, row, x), expected === null
          ? undefined : { revision, paragraph: expected[0], offset: expected[1] });
      }
    }
    assert.ok(snapshots[1].reuse.lineHits > 0);
    assert.ok(snapshots[1].reuse.pageHits > 0);
  } finally {
    for (const snapshot of snapshots) snapshot.free();
  }
});

function checkPhysical(snapshot, native, revision) {
  assert.equal(snapshot.revision, revision);
  assert.equal(snapshot.pageCount, native.rows.length);
  assert.deepEqual(snapshot.reuse, {
    shapedHits: native.layout.reuse.shaped_hits,
    lineHits: native.layout.reuse.line_hits,
    pageHits: native.layout.reuse.page_hits,
  });
  for (const [index, rows] of native.rows.entries()) {
    const page = snapshot.page(index);
    assert.equal(page.revision, revision);
    assert.equal(page.index, index);
    assert.ok(page.rows instanceof Int32Array);
    assert.deepEqual([...page.rows], rows.flatMap(({ paragraph, line, x, top, baseline, bottom }) =>
      [paragraph, line, x, top, baseline, bottom]));
    assert.deepEqual(unpack(page), rows.map(({ paragraph, line }) =>
      ({ paragraph, line, geometry: native.layout.paragraphs[paragraph][line] })));
  }
  for (const [page, x, y, expected] of native.hits) {
    assert.deepEqual(snapshot.hitTest(page, x, y), expected === null
      ? undefined : { revision, paragraph: expected[0], offset: expected[1] });
  }
  for (const [point, affinity, expected] of native.carets) {
    assert.deepEqual(snapshot.caretBox(point, affinity), expected ?? undefined);
  }
  for (const [anchor, focus, expected] of native.selections) {
    assert.deepEqual(snapshot.selectionRects(anchor, focus), expected ?? undefined);
  }
}

function checkNavigation(snapshot, native) {
  let position = snapshot.hitPosition(...native.at);
  const id = position.snapshotId;
  assert.ok(Number.isInteger(id) && id > 0);
  const check = (actual, expected) => assert.deepEqual(actual, expected === null
    ? undefined : { ...expected, revision: snapshot.revision, snapshotId: id });
  check(position, native.initial);
  const positions = [position];
  for (const [direction, expected] of native.moves) {
    position = snapshot.moveCaret(position, direction);
    check(position, expected);
    positions.push(position);
  }
  return positions;
}

test("physical exact fit and one-unit margin reflow match native geometry, hits, selections, and reuse", () => {
  const session = new wasm.ExperimentalLayoutSession(font, fixture.fontSize);
  const snapshots = fixture.physical.map(({ input, spec }) => session.layoutPages(input, spec));
  session.free();
  try {
    for (const [i, snapshot] of snapshots.entries()) {
      checkPhysical(snapshot, fixture.physical[i].snapshot, i + 1);
    }

    const [exact, exactWarm, tight, tightWarm] = snapshots;
    const [exactSpec, tightSpec] = [fixture.physical[0].spec, fixture.physical[2].spec];
    assert.deepEqual(tightSpec, { ...exactSpec, bottom: exactSpec.bottom + 1 });
    const exactRows = exact.page(0).rows;
    assert.equal(exactRows.length, 12); // Two six-word rows fit exactly.
    assert.equal(exactRows.at(-1), exactSpec.height - exactSpec.bottom);
    assert.equal(tight.page(0).rows.length, 6);
    assert.deepEqual([...tight.page(1).rows.slice(0, 2)], [...exactRows.slice(6, 8)]);
    assert.equal(exact.reuse.pageHits, 0);
    assert.equal(exactWarm.reuse.pageHits, exact.pageCount);
    assert.equal(tight.reuse.lineHits, fixture.physical[2].input.length);
    assert.equal(tight.reuse.pageHits, 0);
    assert.equal(tightWarm.reuse.pageHits, tight.pageCount);

    // A wrap boundary can select different rows (and pages) by affinity.
    const point = { paragraph: exactRows[0], offset: exact.page(0).lines[3] };
    const upstream = tight.caretBox(point, "upstream");
    const downstream = tight.caretBox(point, "downstream");
    assert.equal(upstream.page, 0);
    assert.equal(downstream.page, 1);
    const hit = tight.hitTest(upstream.page, upstream.x, upstream.top);
    assert.deepEqual(hit, { ...point, revision: tight.revision });
    const tail = fixture.physical[2].input.length - 1;
    const end = { paragraph: tail, offset: 2 };
    const selection = tight.selectionRects(point, end);
    assert.ok(selection.some((rect) => rect.page > 0));
    assert.deepEqual(tight.selectionRects(end, point), selection);
    assert.deepEqual(tight.selectionRects(end, end), []);
  } finally {
    for (const snapshot of snapshots) snapshot.free();
  }
});

test("plain DOCX section projects to the same input and physical layout as native", async () => {
  const docx = await readFile(new URL(
    "../../crates/rocx-layout/tests/fixtures/split-runs.docx", import.meta.url));
  const section = wasm.plainSection(docx);
  const { snapshot: native, ...expected } = fixture.imported;
  assert.deepEqual(section, expected);
  assert.deepEqual(section.paragraphs, ["hello", "hello"]);
  assert.throws(() => wasm.plainSection(new Uint8Array()), Error);

  const session = new wasm.ExperimentalLayoutSession(font, fixture.fontSize);
  const snapshot = session.layoutPages(section.paragraphs, section.spec);
  try {
    checkPhysical(snapshot, native, 1);
  } finally {
    snapshot.free();
    session.free();
  }
});

test("opened DOCX previews match native bytes and stay bound to the original revision", async () => {
  const docx = await readFile(new URL(
    "../../crates/rocx-layout/tests/fixtures/physical-plain.docx", import.meta.url));
  const native = fixture.opened;
  const session = new wasm.ExperimentalLayoutSession(font, fixture.fontSize);
  const handles = new Set([session]);
  const dispose = (handle) => {
    handle.free();
    handles.delete(handle);
  };
  const open = (bytes) => {
    const handle = session.openPages(bytes);
    handles.add(handle);
    return handle;
  };
  const geometry = (handle) => {
    const snapshot = handle.geometry();
    handles.add(snapshot);
    return snapshot;
  };
  try {
    assert.throws(() => open([]), /bytes must be a Uint8Array/);
    assert.throws(() => open(new Uint8Array()), Error);
    const input = Uint8Array.from(docx);
    const original = open(input);
    input.fill(0); // The opened source is not a live view of caller bytes.
    const initial = geometry(original);
    const alias = geometry(original);
    assert.equal(original.revision, 1);
    assert.deepEqual(original.spec, native.spec);
    checkPhysical(initial, native.snapshot, 1);
    const warm = open(docx);
    const warmGeometry = geometry(warm);
    assert.equal(warm.revision, 2);
    assert.equal(warmGeometry.revision, 2);
    assert.deepEqual(warmGeometry.page(0).rows, initial.page(0).rows);

    const { page, x, top } = native.insert.at;
    const oldCaret = initial.hitPosition(page, x, top);
    assert.deepEqual(alias.moveCaret(oldCaret, "left"), initial.moveCaret(oldCaret, "left"));
    assert.throws(() => warmGeometry.moveCaret(oldCaret, "left"), /stale caret position revision/);
    const { anchor, focus } = native.replace;
    const insert = () => original.previewInsert(page, x, top, native.insert.text);
    assert.throws(() => original.previewInsert(page, x, top, "\uD800"), /unpaired surrogate/);
    assert.throws(() => original.previewReplace(anchor, focus, "\uDC00"), /unpaired surrogate/);
    assert.throws(() => original.previewReplace(anchor, focus, 0), /text must be a string/);
    assert.throws(() => original.previewInsert(page, x + 0.5, top, "!"), /x must be an integer/);
    assert.throws(() => original.previewInsert(initial.pageCount, x, top, "!"), /no text at page point/);
    assert.throws(() => original.previewReplace(anchor, anchor, "!"), /nonempty selection/);
    assert.throws(() => original.previewReplace(anchor, { paragraph: 0, offset: 0 }, "!"),
      /one paragraph/);
    assert.throws(() => original.previewReplace(anchor, { ...focus, offset: 99 }, "!"),
      /not a caret stop/);
    const hard = native.refused.at;
    assert.throws(() => original.previewInsert(hard.page, hard.x, hard.top, "!"),
      { message: native.refused.insertError });
    assert.throws(() => original.previewReplace({ paragraph: 2, offset: 4 }, { paragraph: 2, offset: 5 }, "!"),
      { message: native.refused.replaceError });

    const inserted = insert();
    assert.deepEqual(inserted, Uint8Array.from(native.insert.bytes));
    assert.deepEqual(original.previewReplace(anchor, focus, native.replace.text),
      Uint8Array.from(native.replace.bytes));
    assert.deepEqual(original.previewReplace(anchor, focus, ""), Uint8Array.from(native.delete));
    assert.deepEqual(wasm.plainSection(inserted).paragraphs, ["hello", "", "one\ntwo", "a😀a hello 😀"]);

    const edited = open(inserted);
    assert.equal(edited.revision, 3);
    assert.notDeepEqual(geometry(edited).page(1).lines, initial.page(1).lines);
    dispose(session);
    assert.deepEqual(insert(), inserted); // Later openings never retarget the original.
    dispose(original);
    assert.deepEqual(initial.hitTest(page, x, top), { revision: 1, paragraph: 3, offset: 3 });
  } finally {
    for (const handle of handles) handle.free();
  }
});

test("visual hits distinguish wrap affinity and moves match native across short rows", () => {
  const session = new wasm.ExperimentalLayoutSession(font, fixture.fontSize);
  const other = new wasm.ExperimentalLayoutSession(font, fixture.fontSize);
  const { input, spec, navigation } = fixture.physical[2];
  const wrapped = session.layoutPages(input, spec);
  const foreign = other.layoutPages(input, spec);
  const vertical = session.layoutPages(fixture.navigation.input, fixture.navigation.spec);
  try {
    const [upstream, downstream, back] = checkNavigation(wrapped, navigation);
    assert.equal(upstream.affinity, "upstream");
    assert.equal(downstream.affinity, "downstream");
    assert.deepEqual(upstream.point, downstream.point);
    assert.deepEqual(back, upstream);
    const landed = wrapped.caretBox(downstream.point, downstream.affinity);
    assert.equal(landed.page, 1);
    assert.deepEqual(wrapped.hitPosition(landed.page, landed.x, landed.top), downstream);
    assert.equal(wrapped.hitPosition(wrapped.pageCount, 0, 0), undefined);
    assert.throws(() => wrapped.hitPosition(0, 0, 0.25), /y must be an integer/);

    const [begin, short, recovered, , , rowStart, , rowEnd, past] =
      checkNavigation(vertical, fixture.navigation.path);
    assert.equal(short.point.offset, 1);
    assert.equal(recovered.point.offset, begin.point.offset);
    assert.equal(short.preferredX, fixture.navigation.path.at[1]);
    assert.equal(recovered.preferredX, short.preferredX);
    assert.equal(rowStart.point.offset, 0);
    assert.equal(rowStart.preferredX, undefined);
    assert.equal(rowEnd.point.offset, begin.point.offset);
    assert.equal(past, undefined);

    assert.throws(() => wrapped.moveCaret(upstream, "line-start"), /invalid caret move direction/);
    assert.throws(() => vertical.moveCaret(upstream, "right"), /stale caret position revision/);
    assert.equal(foreign.revision, upstream.revision);
    assert.throws(() => foreign.moveCaret(upstream, "right"), /another snapshot/);
    assert.throws(() => wrapped.moveCaret({ ...upstream, preferredX: 0.5 }, "right"), /preferredX/);
    assert.equal(wrapped.moveCaret({ ...upstream, point: { paragraph: 0, offset: 999 } }, "right"),
      undefined);
  } finally {
    vertical.free();
    foreign.free();
    wrapped.free();
    other.free();
    session.free();
  }
});

test("opened formatted DOCX rows and glyphs match native", async () => {
  const docx = await readFile(new URL(
    "../../crates/rocx-layout/tests/fixtures/physical-formatted.docx", import.meta.url));
  const session = new wasm.ExperimentalLayoutSession(font, fixture.fontSize);
  const opened = session.openPages(docx);
  const geometry = opened.geometry();
  try {
    assert.deepEqual(opened.spec, fixture.formatted.spec);
    checkPhysical(geometry, fixture.formatted.snapshot, 1);
    const rows = geometry.page(0).rows;
    const [center, centerWrap, right, left] = [rows[2], rows[8], rows[14], rows[20]];
    assert.equal(left, opened.spec.left);
    assert.ok(center > left && centerWrap > center && right > left);
    assert.ok(rows[3] > opened.spec.top);
  } finally {
    geometry.free();
    opened.free();
    session.free();
  }
});

test("first-line and hanging indents match native glyphs, origins and hits", async () => {
  const docx = await readFile(new URL(
    "../../crates/rocx-layout/tests/fixtures/physical-indents.docx", import.meta.url));
  const session = new wasm.ExperimentalLayoutSession(font, fixture.fontSize);
  const opened = session.openPages(docx);
  const geometry = opened.geometry();
  try {
    assert.deepEqual(opened.spec, fixture.indented.spec);
    checkPhysical(geometry, fixture.indented.snapshot, 1);
  } finally {
    geometry.free();
    opened.free();
    session.free();
  }
});

test("cache control preserves live geometry and resets reuse", () => {
  const session = wasm.ExperimentalLayoutSession.withCacheBudget(font, fixture.fontSize, 65536);
  const { input, spec } = fixture.physical[0];
  const snapshot = session.layoutPages(input, spec);
  try {
    assert.ok(session.cacheStats.estimatedBytes > 0 && session.cacheStats.estimatedBytes <= 65536);
    session.clearCaches();
    assert.deepEqual(session.cacheStats, { paragraphs: 0, wrapVariants: 0, estimatedBytes: 0, budgetBytes: 65536 });
    const cold = session.layoutPages(input, spec);
    try {
      assert.equal(cold.reuse.pageHits, 0);
      assert.deepEqual(cold.page(0).rows, snapshot.page(0).rows);
    } finally {
      cold.free();
    }
  } finally {
    snapshot.free();
    session.free();
  }
});

test("physical inputs are checked and transferred pages outlive their handles", () => {
  const { input, spec } = fixture.physical[0];
  const session = new wasm.ExperimentalLayoutSession(font, fixture.fontSize);
  let transferred;
  try {
    assert.throws(() => session.layoutPages(input, null), /page spec must be an object/);
    assert.throws(() => session.layoutPages(input, { ...spec, width: 2 ** 32 + 1 }), /width/);
    assert.throws(() => session.layoutPages(input, { ...spec, spaceAfter: 0.5 }), /spaceAfter/);
    assert.throws(() => session.layoutPages(input, { ...spec, left: spec.width }), /content area/);
    assert.throws(() => session.layoutPages(["\uD800"], spec), /unpaired surrogate/);
    const grid = session.layout(input, fixture.width, fixture.linesPerPage);
    const snapshot = session.layoutPages(input, spec);
    try {
      assert.equal(grid.revision, 1);
      assert.equal(snapshot.revision, 2);
      assert.equal(snapshot.page(snapshot.pageCount), undefined);
      assert.equal(snapshot.hitTest(snapshot.pageCount, 0, 0), undefined);
      assert.equal(snapshot.hitTest(0, 0, 0), undefined);
      assert.throws(() => snapshot.hitTest(0, 0, 0.1), /y must be an integer/);
      assert.throws(() => snapshot.caretBox({ paragraph: 0, offset: -1 }, "upstream"), /offset/);
      assert.throws(() => snapshot.caretBox({ paragraph: 0, offset: 0 }, "left"), /affinity/);
      assert.throws(() => snapshot.selectionRects({ paragraph: 0, offset: 0 }, 0), /focus/);
      assert.equal(snapshot.caretBox({ paragraph: 0, offset: 2 ** 32 - 1 }, "upstream"), undefined);
      const page = snapshot.page(0);
      transferred = structuredClone(page, {
        transfer: [page.rows.buffer, page.lines.buffer, page.glyphs.buffer, page.carets.buffer],
      });
      assert.equal(page.rows.byteLength, 0);
      assert.deepEqual(transferred, snapshot.page(0));
    } finally {
      snapshot.free();
      grid.free();
    }
  } finally {
    session.free();
  }
  assert.equal(transferred.rows.length, 12);
  assert.ok(transferred.glyphs.length > 0);
});

test("layout checks input and owns page copies independently of handles", () => {
  assert.throws(() => new wasm.ExperimentalLayoutSession(font, 1.1), /fontSize/);
  assert.throws(() => new wasm.ExperimentalLayoutSession(new Uint8Array(), 768), /font face/);
  const session = new wasm.ExperimentalLayoutSession(font, fixture.fontSize);
  try {
    assert.throws(() => session.layout(["A"], 2 ** 32 + 1, 2), /width/);
    assert.throws(() => session.layout(["A"], 2000, 0), /linesPerPage/);
    assert.throws(() => session.layout("A", 2000, 2), /array of strings/);
    assert.throws(() => session.layout(["A", 1], 2000, 2), /paragraph 1 must be a string/);
    assert.throws(() => session.layout(["\uD800"], 2000, 2), /unpaired surrogate/);
    assert.throws(() => session.layout(["\t"], 2000, 2), /paragraph 0:/);

    const snapshot = session.layout(["A"], 2000, 2);
    let page;
    try {
      assert.equal(snapshot.revision, 1);
      assert.equal(snapshot.page(1), undefined);
      assert.equal(snapshot.hitRow(0, 1, 0), undefined);
      assert.throws(() => snapshot.page(-1), /page/);
      assert.throws(() => snapshot.hitRow(0, 0, 0.5), /x must be an integer/);
      page = snapshot.page(0);
      page.lines.fill(0);
      assert.notDeepEqual(page.lines, snapshot.page(0).lines);
    } finally {
      snapshot.free();
    }
    assert.equal(page.carets.length, 4);
    assert.deepEqual([...page.carets.subarray(0, 2)], [0, 0]);
  } finally {
    session.free();
  }
});
