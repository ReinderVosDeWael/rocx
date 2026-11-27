import init, {
  ExperimentalLayoutSession,
  plainSection,
  type ExperimentalLayoutCaretBox,
  type ExperimentalLayoutSelectionRect,
  type ExperimentalLayoutVisualPosition,
} from "./pkg/rocx.js";

await init();
const session = new ExperimentalLayoutSession(new Uint8Array(), 768);
const bounded = ExperimentalLayoutSession.withCacheBudget(new Uint8Array(), 768, 4096);
const cacheCharge: number = bounded.cacheStats.estimatedBytes;
bounded.clearCaches();
bounded.free();
void cacheCharge;
const snapshot = session.layout(["A", ""] as const, 2200, 2);
const glyphs: Uint32Array | undefined = snapshot.page(0)?.glyphs;
const offset: number | undefined = snapshot.hitRow(0, 0, 0)?.offset;
const shapedHits: number = snapshot.reuse.shapedHits;
const page = snapshot.page(0);
if (page) {
  const transfer: Transferable[] = [page.lines.buffer, page.glyphs.buffer, page.carets.buffer];
  void transfer;
}
// @ts-expect-error Page arrays contain numbers, not glyph objects.
snapshot.page(0)?.glyphs[0]?.advance;
// @ts-expect-error Layout accepts known string inputs, not arbitrary values.
session.layout([0], 2200, 2);
// @ts-expect-error A hit exposes a scalar offset, not a UTF-16 index.
snapshot.hitRow(0, 0, 0)?.utf16Index;
// @ts-expect-error The reuse field names use camelCase.
snapshot.reuse.shaped_hits;

const section = plainSection(new Uint8Array());
const physical = session.layoutPages(section.paragraphs, section.spec);
const physicalPage = physical.page(0);
if (physicalPage) {
  const rows: Int32Array<ArrayBuffer> = physicalPage.rows;
  const transfer: Transferable[] = [rows.buffer, physicalPage.glyphs.buffer];
  void transfer;
}
const hit = physical.hitTest(0, 0, 0);
if (hit) {
  const caret: ExperimentalLayoutCaretBox | undefined = physical.caretBox(hit, "upstream");
  const selection: readonly ExperimentalLayoutSelectionRect[] | undefined =
    physical.selectionRects(hit, { paragraph: 0, offset: 1 });
  void [caret, selection];
}
const visual = physical.hitPosition(0, 0, 0);
if (visual) {
  const moved: ExperimentalLayoutVisualPosition | undefined = physical.moveCaret(visual, "lineEnd");
  const preferred: number | undefined = moved?.preferredX;
  // @ts-expect-error Directions are a closed set of camelCase spellings.
  physical.moveCaret(visual, "line-end");
  // @ts-expect-error A text point alone has no snapshot identity.
  physical.moveCaret(visual.point, "right");
  void preferred;
}
// @ts-expect-error The physical API uses x/y coordinates, not line indices.
physical.hitRow(0, 0, 0);
// @ts-expect-error Only physical pages have page-relative row boxes.
snapshot.page(0)?.rows;
// @ts-expect-error The spec requires explicit margins and spacing.
session.layoutPages(["A"], { width: 1000, height: 2000 });
// @ts-expect-error Affinity is a closed union.
physical.caretBox({ paragraph: 0, offset: 0 }, "left");
// @ts-expect-error The adapter accepts DOCX bytes, not a path.
plainSection("document.docx");
const opened = session.openPages(new Uint8Array());
const openedGeometry = opened.geometry();
const openedRevision: number = opened.revision;
const openedWidth: number = opened.spec.width;
const a = { paragraph: 0, offset: 0 };
const b = { paragraph: 0, offset: 1 };
const inserted: Uint8Array = opened.previewInsert(0, 0, 0, "A");
const replaced: Uint8Array = opened.previewReplace(a, b, "");
// @ts-expect-error Previews take text, not replacement bytes.
opened.previewReplace(a, b, new Uint8Array());
// @ts-expect-error Opening takes a byte buffer, not a parsed spec or number array.
session.openPages([0]);
opened.free();
openedGeometry.free();
void [openedRevision, openedWidth, inserted, replaced];
physical.free();
snapshot.free();
session.free();
void [glyphs, offset, shapedHits];
