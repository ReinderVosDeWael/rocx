import assert from "node:assert/strict";
import { resolve } from "node:path";
import { chromium } from "playwright";
import { build, createServer, preview } from "vite";

const [image, outputDir] = process.argv.slice(2);
const host = "127.0.0.1";
const browser = await chromium.launch();

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

try {
  const dev = await createServer({ server: { host } });
  try {
    await dev.listen(0);
    await checkDownload(dev.httpServer.address().port, "browser-dev");
  } finally {
    await dev.close();
  }
  await build();
  const production = await preview({ preview: { host, port: 0 } });
  try {
    await checkDownload(production.httpServer.address().port, "browser");
  } finally {
    await production.close();
  }
} finally {
  await browser.close();
}
