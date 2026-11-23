import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { chromium } from "playwright";
import { build, createServer, preview } from "vite";

const [image, outputDir] = process.argv.slice(2);
const host = "127.0.0.1";
const browser = await chromium.launch();
const parityPage = "parity/integration/browser.html";

async function checkDownload(port, name) {
  const origin = `http://${host}:${port}`;
  const page = await browser.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error));
  try {
    await page.route("**/*", (route) =>
      new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
    await page.goto(origin);
    await page.getByLabel("Image").setInputFiles(image);
    const [wasm, download] = await Promise.all([
      page.waitForResponse((response) => new URL(response.url()).pathname.endsWith(".wasm")),
      page.waitForEvent("download"),
      page.getByRole("button", { name: "Download DOCX" }).click(),
    ]);
    assert.equal(wasm.status(), 200);
    assert.match(wasm.headers()["content-type"], /^application\/wasm\b/);
    assert.equal(download.suggestedFilename(), "report.docx");
    await download.saveAs(resolve(outputDir, `${name}.docx`));
    await page.getByLabel("Image").setInputFiles({
      name: "invalid.png",
      mimeType: "image/png",
      buffer: Buffer.from([1]),
    });
    await page.getByRole("button", { name: "Download DOCX" }).click();
    await page.getByText(/Could not create DOCX: .*invalid or unsupported image/).waitFor();
    assert.deepEqual(pageErrors, []);
  } finally {
    await page.close();
  }
}

async function checkParity(port, name, direct) {
  const origin = `http://${host}:${port}`;
  const page = await browser.newPage();
  const pageErrors = [];
  const servedModules = [];
  page.on("pageerror", (error) => pageErrors.push(error));
  page.on("response", (response) => {
    if (response.ok()) servedModules.push(new URL(response.url()).pathname);
  });
  try {
    await page.route("**/*", (route) =>
      new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
    await page.goto(`${origin}/${parityPage}`);
    await page.waitForFunction(() => typeof globalThis.runInstalledParity === "function");
    const outputs = await page.evaluate(() => globalThis.runInstalledParity());
    assert.deepEqual(pageErrors, []);
    assert.ok(servedModules.some((path) => path.endsWith(".wasm")), "installed Rust/Wasm fetched");
    assert.equal(servedModules.includes("/node_modules/rocx-docx/browser.js"), direct);
    const directory = resolve(outputDir, "parity", name);
    for (const [dir, files] of [[directory, outputs.files], [resolve(directory, "editor"), outputs.editor]]) {
      for (const [stem, bytes] of Object.entries(files)) {
        assert.match(stem, /^[a-z]+(?:[.-][a-z]+)*$/i);
        await writeFile(resolve(dir, `${stem}.docx`), Buffer.from(bytes));
      }
    }
    await writeFile(resolve(directory, "editor", "reports.json"), JSON.stringify(outputs.reports));
    console.log(`Chromium ${browser.version()} ${name}: portable suites passed; ` +
      `${Object.keys(outputs.files).length + Object.keys(outputs.editor).length} DOCX files`);
  } finally {
    await page.close();
  }
}

try {
  const dev = await createServer({ server: { host } });
  try {
    await dev.listen(0);
    await checkDownload(dev.httpServer.address().port, "browser-dev");
    await checkParity(dev.httpServer.address().port, "browser-dev", true);
  } finally {
    await dev.close();
  }
  await build({
    build: { rollupOptions: { input: { legacy: resolve("index.html"), parity: resolve(parityPage) } } },
  });
  const production = await preview({ preview: { host, port: 0 } });
  try {
    await checkDownload(production.httpServer.address().port, "browser");
    await checkParity(production.httpServer.address().port, "browser", false);
  } finally {
    await production.close();
  }
} finally {
  await browser.close();
}
