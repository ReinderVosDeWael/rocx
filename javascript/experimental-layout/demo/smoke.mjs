// Standalone Firefox WebDriver smoke; no npm dependencies.
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { copyFile, mkdtemp, readdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const [url, driver = "http://127.0.0.1:4444"] = process.argv.slice(2);
if (!url) throw new Error("usage: node smoke.mjs DEMO_URL [WEBDRIVER_URL]");

async function command(path, method = "GET", body) {
  const response = await fetch(`${driver}${path}`, {
    method,
    headers: { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const { value } = await response.json();
  if (!response.ok) throw new Error(`${path}: ${value.message}`);
  return value;
}

// Firefox Snap has a private /tmp; use a non-hidden directory in HOME for
// actual WebDriver downloads, and remove it even if the smoke fails.
const scratch = await mkdtemp(join(process.env.HOME, "rocx-layout-download-"));
const options = {
  args: ["-headless"],
  prefs: {
    "browser.download.folderList": 2,
    "browser.download.dir": scratch,
    "browser.download.useDownloadDir": true,
    "browser.download.always_ask_before_handling_new_types": false,
  },
};
if (process.env.FIREFOX_BIN) options.binary = process.env.FIREFOX_BIN;
let sessionId;
const session = (path, method, body) => command(`/session/${sessionId}${path}`, method, body);
const evaluate = (fn, ...args) => session("/execute/sync", "POST", {
  script: `return (${fn})(...arguments);`, args,
});

async function waitFor(check, label) {
  for (let i = 0; i < 100; i++) {
    if (await evaluate(check)) return;
    const status = await evaluate(() => document.querySelector("#status")?.textContent);
    if (status?.includes("error:")) throw new Error(status);
    await delay(50);
  }
  assert.fail(`Timed out: ${label}`);
}

function pagePoint(page, x, y) {
  const svg = document.querySelector(`svg[data-page="${page}"]`);
  const point = new DOMPoint(x * 64, y * 64).matrixTransform(svg.getScreenCTM());
  return { x: Math.round(point.x), y: Math.round(point.y), duration: 50 };
}

const move = (point) => ({ type: "pointerMove", origin: "viewport", ...point });
const down = { type: "pointerDown", button: 0 };
const up = { type: "pointerUp", button: 0 };
const mouse = (...actions) => session("/actions", "POST", {
  actions: [{ type: "pointer", id: "mouse", parameters: { pointerType: "mouse" }, actions }],
});

const click = (selector) => evaluate((selector) => document.querySelector(selector).click(), selector);
const choose = async (from, to = from) => {
  await mouse(move(from), down, move(to), up);
  await waitFor(() => !document.querySelector("#apply").disabled, "DOCX caret or selection");
};

async function applyText(text) {
  await evaluate((text) => {
    document.querySelector("#replacement").value = text;
    document.querySelector("#apply").click();
  }, text);
}

async function saveDownload(name) {
  const filename = "physical-plain-preview.docx";
  await click("#download");
  for (let i = 0; i < 100; i++) {
    const files = await readdir(scratch);
    if (files.includes(filename) && !files.some((file) => file.endsWith(".part"))) {
      await copyFile(join(scratch, filename), join(scratch, `${name}.docx`));
      await rm(join(scratch, filename));
      return;
    }
    await delay(50);
  }
  assert.fail(`Timed out waiting for browser download: ${name}`);
}

try {
  ({ sessionId } = await command("/session", "POST", {
    capabilities: { alwaysMatch: { browserName: "firefox", "moz:firefoxOptions": options } },
  }));
  // A narrow viewport also exercises the screen-to-page scale transform.
  await session("/window/rect", "POST", { width: 435, height: 1000 });
  await session("/url", "POST", { url });
  await waitFor(() => document.querySelector("#pages")?.dataset.revision === "1", "first layout");
  assert.ok(await evaluate(() => {
    const pages = document.querySelectorAll("svg.page");
    const glyph = pages[0].querySelector("use");
    const outline = document.querySelector(glyph.getAttribute("href"));
    return pages.length >= 2 && outline.getAttribute("d").startsWith("M") &&
      glyph.getBoundingClientRect().width > 0 && !pages[0].querySelector("text, canvas");
  }), "positioned outline glyphs across physical pages");

  const start = await evaluate(pagePoint, 0, 31, 30);
  const end = await evaluate(pagePoint, 1, 130, 30);
  await mouse(move(start), down, up);
  await waitFor(() => document.querySelector(".caret")?.getAttribute("x1") === "1920", "caret");
  assert.match(await evaluate(() => document.querySelector("#status").textContent),
    /paragraph 1 · scalar offset 0$/);

  await mouse(move(start), down, move(end), up);
  await waitFor(() => [0, 1].every((page) =>
    document.querySelector(`svg[data-page="${page}"] .selection`)), "cross-page selection");
  assert.ok(await evaluate(() => document.querySelector('svg[data-page="1"] .caret')));

  const margin = await evaluate(pagePoint, 0, 30, 12);
  await mouse(move(margin), down, up);
  await waitFor(() => document.querySelector("#status").textContent.endsWith("no text hit") &&
    !document.querySelector(".caret, .selection"), "click in margin clears selection");

  // Put a caret back so the reflow must clear painted geometry.
  await mouse(move(start), down, up);
  await waitFor(() => document.querySelector(".caret"), "caret before reflow");
  const firstPageInk = () => Array.from(
    document.querySelectorAll('svg[data-page="0"] use'), (glyph) => glyph.outerHTML,
  ).join("");
  const originalInk = await evaluate(firstPageInk);

  // Queue several layouts in one event turn: only the last may paint.
  await evaluate(() => {
    const pages = document.querySelector("#pages");
    const change = document.querySelector("#change");
    window.painted = [];
    const observer = new MutationObserver(() => {
      if (pages.dataset.revision) window.painted.push(pages.dataset.revision);
    });
    observer.observe(pages, { attributes: true, attributeFilter: ["data-revision"] });
    change.click();
    change.click();
    change.click();
  });
  await waitFor(() => document.querySelector("#pages").dataset.revision === "4", "latest layout");
  assert.deepEqual(await evaluate(() => window.painted), ["4"]);
  assert.equal(await evaluate(() => !!document.querySelector(".caret, .selection")), false);
  assert.notEqual(await evaluate(firstPageInk), originalInk, "latest sample has different glyphs");

  const result = await session("/execute/async", "POST", {
    args: [],
    script: `
      const done = arguments[0];
      const worker = new Worker("./layout-worker.mjs", { type: "module" });
      const seen = [];
      let wrapRow;
      const finish = (value) => { worker.terminate(); done(value); };
      worker.onerror = (event) => finish(event.message);
      worker.onmessage = ({ data }) => {
        if (data.type === "error") return finish(data.message);
        if (data.type === "ready") worker.postMessage({ type: "layout", id: 1 });
        if (data.type === "layout") {
          const { lines, rows } = data.pages[0];
          // Click the trailing edge of a wrapped row. Downstream affinity
          // at that scalar stop would incorrectly paint on the next row.
          for (let i = 0; i + 9 < lines.length; i += 9) {
            if (lines[i] === lines[i + 9] && lines[i + 3] === lines[i + 11]) {
              wrapRow = i / 9;
              break;
            }
          }
          if (wrapRow === undefined) return finish("sample lacks a soft wrap");
          const point = {
            type: "point", page: 0, begin: true,
            x: rows[wrapRow * 6 + 2] + (lines[wrapRow * 9 + 4] | 0),
            y: rows[wrapRow * 6 + 3],
          };
          worker.postMessage({ ...point, id: 1, layoutId: 0, revision: data.revision });
          worker.postMessage({ ...point, id: 2, layoutId: 1, revision: data.revision - 1 });
          worker.postMessage({ ...point, id: 3, layoutId: 1, revision: data.revision });
        }
        if (data.type === "point") {
          seen.push(data.id);
          if (data.id === 3) finish({ seen, caretOnHitRow: data.caret?.row === wrapRow });
        }
      };
    `,
  });
  assert.deepEqual(result, { seen: [3], caretOnHitRow: true },
    "worker ignores stale requests and keeps the caret on the hit row at a soft wrap");
  console.log("Firefox smoke: positioned glyphs, click caret, cross-page selection, stale messages");

  await click("#open");
  await waitFor(() => document.querySelector("#status").textContent.includes("sample.docx · original"),
    "opened DOCX");
  assert.equal(await evaluate(() => document.querySelector("svg.docx").getAttribute("viewBox")),
    "0 0 3203 4608");
  await saveDownload("original");

  // 10.5pt is inside the first glyph but nearer offset zero; 12pt is
  // inside the first row. The margins come from the checked-in fixture.
  const originalStart = await evaluate(pagePoint, 0, 10.5, 12);
  await choose(originalStart);
  assert.match(await evaluate(() => document.querySelector("#status").textContent),
    /paragraph 1 · scalar offset 0$/);
  await applyText("A");
  await waitFor(() => document.querySelector("#status").textContent.includes("preview 1"),
    "insert reopens");
  await saveDownload("insert");

  const insertStart = await evaluate(pagePoint, 0, 10.5, 12);
  const insertEnd = await evaluate(pagePoint, 0, 25, 12);
  await choose(insertStart, insertEnd);
  assert.match(await evaluate(() => document.querySelector("#status").textContent),
    /paragraph 1 · scalar offset 2$/);
  assert.ok(await evaluate(() => !!document.querySelector(".selection")));
  await applyText("X");
  await waitFor(() => document.querySelector("#status").textContent.includes("preview 2"),
    "replacement reopens");
  await saveDownload("replace");

  const replaceStart = await evaluate(pagePoint, 0, 10.5, 12);
  const replaceEnd = await evaluate(pagePoint, 0, 19, 12);
  await choose(replaceStart, replaceEnd);
  assert.match(await evaluate(() => document.querySelector("#status").textContent),
    /paragraph 1 · scalar offset 1$/);
  await applyText("");
  await waitFor(() => document.querySelector("#status").textContent.includes("preview 3"),
    "deletion reopens");
  await saveDownload("delete");

  await choose(await evaluate(pagePoint, 0, 10.5, 12));
  const beforeFailure = await evaluate(() => document.querySelector("#pages").dataset.revision);
  await applyText("א");
  await waitFor(() => document.querySelector("#diagnostic").textContent.includes("could not reopen"),
    "unsupported insertion fails reopening");
  assert.match(await evaluate(() => document.querySelector("#diagnostic").textContent),
    /sample\.docx · paragraph 1, scalar offset 0.*U\+05D0/);
  assert.equal(await evaluate(() => document.querySelector("#pages").dataset.revision), beforeFailure);
  assert.ok(await evaluate(() => !document.querySelector("#apply").disabled &&
    !!document.querySelector(".caret")));
  await saveDownload("after-rejection");

  await click("#open");
  await waitFor(() => document.querySelector("#status").textContent.includes("sample.docx · original"),
    "reset to original");
  await saveDownload("after-reset");

  const checker = fileURLToPath(new URL("./check-downloads.py", import.meta.url));
  const { stdout } = await promisify(execFile)(process.env.PYTHON ?? "python3", [checker, scratch]);
  console.log(stdout.trim());
} finally {
  try {
    if (sessionId) await session("", "DELETE");
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
}
