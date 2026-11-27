const pages = document.querySelector("#pages");
const change = document.querySelector("#change");
const open = document.querySelector("#open");
const editor = document.querySelector("#editor");
const replace = document.querySelector("#replacement");
const apply = document.querySelector("#apply");
const download = document.querySelector("#download");
const status = document.querySelector("#status");
const diagnostic = document.querySelector("#diagnostic");
const worker = new Worker(new URL("./layout-worker.mjs", import.meta.url), { type: "module" });
const svgNS = "http://www.w3.org/2000/svg";

let atlas;
let scale;
let views = [];
let alternate = false;
let requestId = 0;
let current;
let pending;
let downloadId;
let pointId = 0;
let picked;
let pointer;

function svg(tag, attributes = {}) {
  const node = document.createElementNS(svgNS, tag);
  for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, value);
  return node;
}

function renderPage({ index, lines, glyphs, rows }, { width, height }, source) {
  const root = svg("svg", {
    class: source ? "page docx" : "page", "data-page": index, viewBox: `0 0 ${width} ${height}`,
    role: "img", "aria-label": `Layout page ${index + 1}`,
  });
  const defs = svg("defs");
  const highlight = svg("g");
  const ink = svg("g");
  const cursor = svg("g");
  root.append(defs, highlight, ink, cursor);
  const defined = new Set();
  const lineStride = 9, rowStride = 6, glyphStride = 5;
  for (let i = 0; i < lines.length / lineStride; i++) {
    const first = lines[i * lineStride + 5];
    const count = lines[i * lineStride + 6];
    const x = rows[i * rowStride + 2];
    const baseline = rows[i * rowStride + 4];
    for (let j = first * glyphStride; j < (first + count) * glyphStride; j += glyphStride) {
      const id = glyphs[j];
      const path = atlas.paths[id];
      if (path === undefined) throw new Error(`missing glyph outline ${id}`);
      if (!path) continue; // e.g. space
      const name = `page-${index}-glyph-${id}`;
      if (!defined.has(id)) {
        defs.append(svg("path", { id: name, d: path }));
        defined.add(id);
      }
      // The wire stores signed glyph positions in Uint32Array words.
      // Only the font outline needs its y axis flipped.
      const gx = x + (glyphs[j + 1] | 0);
      const gy = baseline + (glyphs[j + 2] | 0);
      ink.append(svg("use", {
        href: `#${name}`, transform: `translate(${gx} ${gy}) scale(${scale} ${-scale})`,
      }));
    }
  }
  return { root, highlight, cursor };
}

function sourceStatus() {
  const { source, edition, revision } = current;
  const label = source ? `${source} · ${edition ? `preview ${edition}` : "original"} · ` : "";
  return `${label}Revision ${revision}`;
}

function syncEditor() {
  if (current) change.disabled = open.disabled = pending?.type === "edit";
  editor.hidden = !current?.source;
  apply.disabled = editor.hidden || !!pending || pointer !== undefined || !picked;
  download.disabled = editor.hidden || !!pending;
}

function showGeometry({ caret, focus, selection }) {
  for (const { cursor, highlight } of views) {
    cursor.replaceChildren();
    highlight.replaceChildren();
  }
  for (const { page, left, right, top, bottom } of selection ?? []) {
    views[page].highlight.append(svg("rect", {
      class: "selection", x: left, y: top, width: right - left, height: bottom - top,
    }));
  }
  if (caret) {
    views[caret.page].cursor.append(svg("line", {
      class: "caret", x1: caret.x, x2: caret.x, y1: caret.top, y2: caret.bottom,
    }));
  }
  picked = focus;
  status.textContent = focus
    ? `${sourceStatus()} · paragraph ${focus.paragraph + 1} · scalar offset ${focus.offset}`
    : `${sourceStatus()} · no text hit`;
  syncEditor();
}

function requestLayout(docx = false) {
  if (pointer !== undefined && pages.hasPointerCapture(pointer)) {
    pages.releasePointerCapture(pointer);
  }
  pointer = undefined;
  pending = { type: "layout", id: ++requestId };
  diagnostic.textContent = "";
  syncEditor();
  worker.postMessage({ ...pending, docx, alternate });
}

function fail(message) {
  status.textContent = `Layout demo error: ${message}`;
  change.disabled = open.disabled = true;
  current = undefined;
  syncEditor();
  worker.terminate();
}

function isCurrent({ layoutId, revision }) {
  return current?.id === layoutId && current?.revision === revision;
}

function downloadDocx(bytes) {
  const url = URL.createObjectURL(new Blob([bytes], {
    type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  }));
  const link = document.createElement("a");
  link.href = url;
  link.download = "physical-plain-preview.docx";
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 30_000);
}

worker.addEventListener("error", (event) => fail(event.message));
worker.addEventListener("message", ({ data }) => {
  try {
    if (data.type === "ready") {
      atlas = data.atlas;
      scale = data.fontSize / atlas.unitsPerEm;
      change.disabled = open.disabled = false;
      requestLayout();
    } else if (data.type === "layout" && (
      (pending?.type === "layout" && data.id === pending.id && data.editId === undefined) ||
      (pending?.type === "edit" && data.id === current.id && data.editId === pending.id)
    )) {
      views = data.pages.map((page) => renderPage(page, data.spec, data.source));
      pages.replaceChildren(...views.map(({ root }) => root));
      const { id, revision, source, edition } = data;
      current = { id, revision, source, edition };
      pages.dataset.revision = revision;
      status.textContent = `${sourceStatus()} · ${views.length} pages`;
      pending = picked = undefined;
      replace.value = "";
      syncEditor();
    } else if (data.type === "point" && !pending && isCurrent(data) && data.id === pointId) {
      showGeometry(data);
    } else if (data.type === "download" && !pending && isCurrent(data) && data.id === downloadId) {
      downloadDocx(data.bytes);
    } else if (data.type === "error") {
      if (!data.request) {
        fail(data.message);
      } else if ((data.request === pending?.type && data.id === pending.id) ||
                 (!pending && isCurrent(data) && data.request === "point" && data.id === pointId)) {
        const { source, focus } = data;
        const location = focus ? ` · paragraph ${focus.paragraph + 1}, scalar offset ${focus.offset}` : "";
        diagnostic.textContent = `${source ?? "Fixed sample"}${location} — ${data.message}`;
        pending = undefined;
        syncEditor();
      }
    }
  } catch (error) {
    fail(error.message);
  }
});

change.addEventListener("click", () => {
  alternate = !alternate;
  requestLayout();
});
open.addEventListener("click", () => requestLayout(true));
apply.addEventListener("click", () => {
  pending = { type: "edit", id: ++requestId };
  diagnostic.textContent = "";
  syncEditor();
  worker.postMessage({
    ...pending, layoutId: current.id, revision: current.revision, pointId, text: replace.value,
  });
});
download.addEventListener("click", () => {
  downloadId = ++requestId;
  worker.postMessage({
    type: "download", id: downloadId, layoutId: current.id, revision: current.revision,
  });
});

function sendPoint(event, begin) {
  // Pointer capture keeps drag events coming, but elementFromPoint gives the
  // actual page under the pointer rather than the captured container.
  const root = document.elementFromPoint(event.clientX, event.clientY)?.closest("svg.page");
  if (!root || !pages.contains(root) || pending || !current) return false;
  const matrix = root.getScreenCTM();
  if (!matrix) return false;
  const { x, y } = new DOMPoint(event.clientX, event.clientY).matrixTransform(matrix.inverse());
  worker.postMessage({
    type: "point", id: ++pointId, layoutId: current.id, revision: current.revision, begin,
    page: Number(root.dataset.page), x: Math.round(x), y: Math.round(y),
  });
  picked = undefined;
  syncEditor();
  return true;
}

pages.addEventListener("pointerdown", (event) => {
  if (event.button !== 0 || pointer !== undefined || !sendPoint(event, true)) return;
  pages.setPointerCapture(event.pointerId);
  pointer = event.pointerId;
  syncEditor();
});
pages.addEventListener("pointermove", (event) => {
  if (event.pointerId === pointer) sendPoint(event, false);
});
pages.addEventListener("pointerup", (event) => {
  if (event.pointerId !== pointer) return;
  sendPoint(event, false);
  pointer = undefined;
  syncEditor();
});
pages.addEventListener("lostpointercapture", (event) => {
  if (event.pointerId === pointer) {
    pointer = undefined;
    syncEditor();
  }
});
