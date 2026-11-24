import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { inflateRawSync } from "node:zlib";
import * as cases from "./tables.mts";

// Inspect the packed bytes independently of the Wasm readback view.
function documentXml(docx) {
  const zip = Buffer.from(docx);
  const end = zip.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
  assert.notEqual(end, -1, "ZIP end of central directory");
  for (let entry = zip.readUInt32LE(end + 16); entry < end;) {
    assert.equal(zip.readUInt32LE(entry), 0x02014b50);
    const length = zip.readUInt16LE(entry + 28);
    const name = zip.toString("utf8", entry + 46, entry + 46 + length);
    if (name === "word/document.xml") {
      const offset = zip.readUInt32LE(entry + 42);
      assert.equal(zip.readUInt32LE(offset), 0x04034b50);
      const start = offset + 30 + zip.readUInt16LE(offset + 26) + zip.readUInt16LE(offset + 28);
      const payload = zip.subarray(start, start + zip.readUInt32LE(entry + 20));
      const method = zip.readUInt16LE(entry + 10);
      assert.ok(method === 0 || method === 8, `ZIP compression method ${method}`);
      return (method === 8 ? inflateRawSync(payload) : payload).toString();
    }
    entry += 46 + length + zip.readUInt16LE(entry + 30) + zip.readUInt16LE(entry + 32);
  }
  assert.fail("word/document.xml missing");
}

function compositionXml({ docx, autoDocx }) {
  const xml = documentXml(docx);
  assert.match(xml, /<w:tblStyle w:val="[^"]+"\/>/);
  assert.match(xml, /<w:tblLayout w:type="fixed"\/>/);
  assert.match(xml, /<w:tblLook\b[^>]*w:val="0320"/);
  for (const side of ["top", "left", "bottom", "right", "insideH", "insideV"]) {
    assert.match(xml, new RegExp(`<w:${side}\\b[^>]*w:color="0C2238"`));
  }
  assert.match(xml, /<w:tblGrid><w:gridCol w:w="1200"\/><w:gridCol w:w="2400"\/><w:gridCol w:w="3600"\/><\/w:tblGrid>/);
  assert.match(xml, /<w:tblGrid><w:gridCol w:w="1800"\/><w:gridCol w:w="1800"\/><\/w:tblGrid>/);
  assert.match(xml, /<w:tcW\b[^>]*w:w="3600"[^>]*\/><w:gridSpan w:val="2"\/><w:vMerge w:val="restart"\/>/);
  assert.match(xml, /<w:vMerge w:val="continue"\/><\/w:tcPr><w:p\/><\/w:tc><w:tc><w:tcPr><w:tcW\b[^>]*w:w="3600"/);
  const auto = documentXml(autoDocx);
  assert.match(auto, /<w:tblLayout w:type="autofit"\/>/);
  assert.match(auto, /<w:tblGrid>(?:<w:gridCol w:w="2880"\/>){3}<\/w:tblGrid>/);
  assert.match(auto, /<w:tblGrid>(?:<w:gridCol w:w="2880"\/>){2}<\/w:tblGrid>/);
}

function layoutXml(docx) {
  const xml = documentXml(docx);
  assert.match(xml, /<w:cantSplit w:val="0"\/><w:tblHeader w:val="1"\/><w:trHeight w:val="360" w:hRule="exact"\/>/);
  assert.match(xml, /<w:cantSplit w:val="1"\/><w:tblHeader w:val="0"\/><w:trHeight w:val="225" w:hRule="atLeast"\/>/);
  assert.match(xml, /<w:tcMar><w:top w:w="0" w:type="dxa"\/><w:start w:w="25" w:type="dxa"\/><w:bottom w:w="40" w:type="dxa"\/><w:end w:w="60" w:type="dxa"\/>/);
  assert.match(xml, /<w:shd\b[^>]*w:fill="FF000C"/);
  for (const [index, side] of ["top", "bottom", "start", "end"].entries()) {
    const line = index % 2 === 0 ? "single" : "dashed";
    assert.match(xml, new RegExp(`<w:${side} w:val="${line}" w:sz="${4 + index}" w:color="0C2238"/>`));
  }
  assert.deepEqual([...xml.matchAll(/<w:textDirection w:val="([^"]+)"\/>/g)].map((m) => m[1]),
    ["lrTb", "tbRl", "btLr", "lrTbV", "tbRlV", "tbLrV"]);
  assert.deepEqual([...xml.matchAll(/<w:vAlign w:val="([^"]+)"\/>/g)].map((m) => m[1]),
    ["top", "center", "bottom", "both", "top", "center"]);
  assert.deepEqual([...xml.matchAll(/<w:jc w:val="([^"]+)"\/>/g)].map((m) => m[1]),
    ["left", "center", "right", "start", "end"]);
  assert.match(xml, /<w:tag w:val="row-control"\/>.*?<w:sdtContent><w:tr>/s);
  assert.match(xml, /<w:tag w:val="cell-control"\/>.*?<w:sdtContent><w:tc>/s);
  assert.deepEqual([...xml.matchAll(/<w:id w:val="(\d+)"\/><w:lock w:val="sdtLocked"\/><w:richText\/>/g)]
    .map((m) => Number(m[1])), [101, 102]);
}

const nativePath = process.argv.find((arg) => arg.startsWith("--rust="))?.slice("--rust=".length);
const composition = await cases.structuredTableComposition();
compositionXml(composition);
layoutXml(await cases.rowCellLayoutAndControls());
await cases.standaloneAndConditionalComponents();
await cases.resolvedBlockViewsAndReuse();
await cases.standaloneValidationAndDiagnostics();
if (nativePath) assert.deepEqual(Buffer.from(composition.docx), await readFile(nativePath));
console.log(`Tables packed: Node ${process.version}, five owner cases${nativePath ? ", exact Rust bytes" : ""}`);

if (process.argv.includes("--chromium")) {
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
    await page.goto(`http://${host}:${server.httpServer.address().port}/`);
    const result = await page.evaluate(async () => {
      const cases = await import("/tables.mts");
      const { docx, autoDocx } = await cases.structuredTableComposition();
      const layout = await cases.rowCellLayoutAndControls();
      await cases.standaloneAndConditionalComponents();
      await cases.resolvedBlockViewsAndReuse();
      await cases.standaloneValidationAndDiagnostics();
      return { docx: [...docx], autoDocx: [...autoDocx], layout: [...layout] };
    });
    compositionXml(result);
    layoutXml(result.layout);
    assert.deepEqual(Buffer.from(result.docx), Buffer.from(composition.docx));
    console.log(`Tables packed: Chromium ${browser.version()}, five owner cases, exact Node bytes`);
  } finally {
    await browser?.close();
    await server.close();
  }
}
