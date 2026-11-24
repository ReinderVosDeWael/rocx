// From an installed-package consumer directory: copy editor.{mts,mjs,py} here.
// PYTHON=/path/to/python-with-python-docx node editor.mjs --chromium --native=/path/to/oracle
// Build the native oracle: cargo build --locked --example javascript_editor_oracle
// With the default target directory, pass --native=/path/to/repo/target/debug/examples/javascript_editor_oracle.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { run } from "./editor.mts";

function command(binary, args) {
  const result = spawnSync(binary, args, { stdio: "inherit" });
  if (result.error) throw result.error;
  assert.equal(result.status, 0, `${binary} ${args.join(" ")}`);
}

const python = process.env.PYTHON ?? "python3";
const script = fileURLToPath(new URL("./editor.py", import.meta.url));
const directory = await mkdtemp(join(tmpdir(), "rocx-editor-"));
const native = process.argv.find((arg) => arg.startsWith("--native="))?.slice("--native=".length);

async function installedChromium(fixtures, expected, reports) {
  const { chromium } = await import("playwright");
  const { createServer } = await import("vite");
  const host = "127.0.0.1";
  const server = await createServer({
    configFile: false, optimizeDeps: { exclude: ["rocx-docx"] }, server: { host },
  });
  let browser;
  try {
    await server.listen(0);
    browser = await chromium.launch();
    const page = await browser.newPage();
    const failures = [];
    page.on("pageerror", (error) => failures.push(error));
    await page.goto(`http://${host}:${server.httpServer.address().port}/`);
    const input = Object.fromEntries(Object.entries(fixtures).map(([name, bytes]) => [name, bytes.toString("base64")]));
    const actual = await page.evaluate(async (encoded) => {
      const { run } = await import("/editor.mts");
      const input = Object.fromEntries(Object.entries(encoded).map(([key, data]) => [key, Uint8Array.fromBase64(data)]));
      const { reports, mixedReports, ...docx } = await run(input);
      return {
        reports: { ...reports, mixed: mixedReports },
        docx: Object.fromEntries(Object.entries(docx).map(([key, bytes]) => [key, bytes.toBase64()])),
      };
    }, input);
    assert.deepEqual(actual.reports, reports);
    for (const [name, bytes] of Object.entries(expected)) {
      assert.deepEqual(Buffer.from(actual.docx[name], "base64"), Buffer.from(bytes), name);
    }
    assert.deepEqual(failures, []);
    console.log(`E04–E06 installed Chromium ${browser.version()}: all named cases and exact Node bytes`);
  } finally {
    await browser?.close();
    await server.close();
  }
}

try {
  command(python, [script, "generate", directory]);
  const fixtures = Object.fromEntries(await Promise.all(
    ["range-source", "base", "unsafe", "ambiguous", "drawing", "relocated", "styles-edge", "styles-missing", "styles-invalid", "image"].map(async (n) =>
      [n, await readFile(join(directory, n + (n === "image" ? ".png" : ".docx")))]),
  ));
  const { reports, mixedReports, ...outputs } = await run(fixtures);
  const allReports = { ...reports, mixed: mixedReports };
  await writeFile(join(directory, "reports.json"), JSON.stringify(allReports));
  if (native) command(native, [directory]);
  for (const [name, bytes] of Object.entries(outputs)) {
    const file = name === "relocated" ? "relocated-out.docx" : `${name}.docx`;
    await writeFile(join(directory, file), bytes);
    if (native) assert.deepEqual(Buffer.from(bytes), await readFile(join(directory, `native-${file}`)), file);
  }
  command(python, [script, "verify", directory]);
  console.log(`E04–E06 installed Node ${process.version}: all named cases${native ? `, ${Object.keys(outputs).length} exact Rust byte comparisons` : ""}`);
  if (process.argv.includes("--chromium")) await installedChromium(fixtures, outputs, allReports);
} finally {
  await rm(directory, { recursive: true, force: true });
}
