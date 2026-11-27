import init, { ExperimentalLayoutSession } from "./pkg/rocx.js";

const pt = 64;
const fontSize = 13 * pt;
const sample = "sample.docx";
const spec = {
  width: 350 * pt, height: 190 * pt,
  left: 30 * pt, right: 30 * pt, top: 24 * pt, bottom: 24 * pt,
  spaceBefore: 0, spaceAfter: 8 * pt,
};
const paragraphs = [
  "Office hours in the rocx layout lab",
  "Click this paragraph to place a caret, or drag across rows and pages to select text. The browser paints the glyphs, but Wasm decides where each glyph and caret belongs.",
  "Try the ffi in office, the combining accent in cafe\u0301, or this smile 😀. The scalar offset shown below the controls is not a JavaScript string index.",
  "The same pinned font supplies both the shaping data and these outlines. Even when the page is scaled to fit your screen, clicks still map to its original coordinates.",
  "Changing the sample submits a new revision to the same layout session. It clears the previous selection instead of reusing stale geometry.",
];

async function loadAsset(name, method) {
  const response = await fetch(new URL(name, import.meta.url));
  if (!response.ok) throw new Error(`${name}: HTTP ${response.status}`);
  return response[method]();
}

async function start() {
  const [font, atlas] = await Promise.all([
    loadAsset("./font.ttf", "arrayBuffer"),
    loadAsset("./font-atlas.json", "json"),
    init(),
  ]);
  const bytes = new Uint8Array(font);
  let fingerprint = 0x811c9dc5;
  for (const byte of bytes) fingerprint = Math.imul(fingerprint ^ byte, 0x01000193);
  if ((fingerprint >>> 0) !== atlas.fingerprint) {
    throw new Error("font atlas does not match the shaping font");
  }
  const session = new ExperimentalLayoutSession(bytes, fontSize);
  let current;
  let latestLayoutId = 0;
  let latestPointId = 0;
  let opening = false;
  let anchor;
  let interaction;

  function release(state) {
    state?.snapshot?.free();
    state?.opened?.free();
  }

  // Prepare transferable geometry before releasing the previous opening.
  function publish(next, editId) {
    try {
      const { snapshot, layoutId, spec, source, edition } = next;
      const pages = Array.from({ length: snapshot.pageCount }, (_, i) => {
        const { index, lines, glyphs, rows } = snapshot.page(i);
        return { index, lines, glyphs, rows };
      });
      const transfer = pages.flatMap(({ lines, glyphs, rows }) => [
        lines.buffer, glyphs.buffer, rows.buffer,
      ]);
      self.postMessage({
        type: "layout", id: layoutId, editId, revision: snapshot.revision, pages, spec, source,
        edition,
      }, transfer);
    } catch (error) {
      release(next);
      throw error;
    }
    release(current);
    current = next;
    anchor = interaction = undefined;
  }

  function openDocx(sourceBytes, layoutId, edition) {
    const opened = session.openPages(sourceBytes);
    const next = { opened, bytes: sourceBytes, layoutId, source: sample, edition };
    try {
      next.spec = opened.spec;
      next.snapshot = opened.geometry();
      return next;
    } catch (error) {
      release(next);
      throw error;
    }
  }

  function edit({ id, pointId, text }) {
    if (!current.opened || !interaction?.focus || pointId !== latestPointId) {
      throw new Error("Select a caret or range in the current DOCX before applying an edit.");
    }
    const { focus, page, x, y } = interaction;
    const range = anchor.paragraph !== focus.paragraph || anchor.offset !== focus.offset;
    const preview = range
      ? current.opened.previewReplace(anchor, focus, text)
      : current.opened.previewInsert(page, x, y, text);
    let next;
    try {
      next = openDocx(preview, current.layoutId, current.edition + 1);
    } catch (error) {
      throw new Error(`Preview could not reopen: ${error.message}`);
    }
    publish(next, id);
  }

  self.onmessage = async ({ data }) => {
    try {
      if (data.type === "layout") {
        if (data.id <= latestLayoutId) return;
        latestLayoutId = data.id;
        opening = true;
        if (data.docx) {
          const sourceBytes = new Uint8Array(await loadAsset(`./${sample}`, "arrayBuffer"));
          if (data.id === latestLayoutId) publish(openDocx(sourceBytes, data.id, 0));
        } else {
          const text = paragraphs.slice();
          if (data.alternate) text[0] += " — second edition";
          publish({ snapshot: session.layoutPages(text, spec), layoutId: data.id, spec });
        }
        return;
      }

      const { snapshot, layoutId } = current ?? {};
      if (opening || !snapshot || data.layoutId !== layoutId ||
          data.revision !== snapshot.revision) return;
      if (data.type === "edit") {
        edit(data);
        return;
      }
      if (data.type === "download" && current.bytes) {
        const bytes = current.bytes.slice(); // Never detach the worker's committed bytes.
        self.postMessage({
          type: "download", id: data.id, layoutId, revision: snapshot.revision, bytes,
        }, [bytes.buffer]);
        return;
      }
      if (data.type !== "point" || data.id <= latestPointId) return;
      latestPointId = data.id;

      const focus = snapshot.hitTest(data.page, data.x, data.y);
      if (data.begin) anchor = focus;
      // At a soft wrap either affinity may report the same scalar point.
      // Keep the painted caret on the row the pointer actually hit.
      if (data.begin || (anchor && focus)) {
        let caret;
        if (focus) {
          for (const affinity of ["downstream", "upstream"]) {
            const box = snapshot.caretBox(focus, affinity);
            if (box?.page === data.page && data.y >= box.top && data.y < box.bottom) {
              caret = box;
              break;
            }
          }
        }
        interaction = {
          page: data.page, x: data.x, y: data.y,
          focus, caret, selection: anchor && focus ? snapshot.selectionRects(anchor, focus) : [],
        };
      }
      self.postMessage({
        type: "point", id: data.id, layoutId, revision: snapshot.revision,
        focus: interaction?.focus, caret: interaction?.caret, selection: interaction?.selection,
      });
    } catch (error) {
      if (data.type === "layout" && data.id !== latestLayoutId) return;
      const { focus } = interaction ?? {};
      self.postMessage({
        type: "error", id: data.id, request: data.type, layoutId: data.layoutId,
        revision: data.revision,
        source: data.type === "layout" && data.docx ? sample : current?.source,
        focus: data.type === "edit" ? focus : undefined,
        message: error.message,
      });
    } finally {
      if (data.type === "layout" && data.id === latestLayoutId) opening = false;
    }
  };
  self.postMessage({ type: "ready", atlas, fontSize });
}

start().catch((error) => self.postMessage({ type: "error", message: error.message }));
