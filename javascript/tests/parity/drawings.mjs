import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { inflateRawSync } from "node:zlib";
import { createResolvedInline, deferred, future, inlineText, lazy, resolveComponent, validateValue } from "rocx-docx";
import { imageFile } from "rocx-docx/node";
import {
  alt, construct, document, drawingRuns, filePicture, pictures, placements, pictureTransform,
  resolvePicture, standaloneAndConditionalComponents, validateImageTransformation,
} from "./drawings.mts";

// Read only what the package assertions need, without making this consumer
// install a ZIP dependency or relying on the binding to inspect its own output.
function zipParts(docx) {
  const bytes = Buffer.from(docx);
  const end = bytes.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
  assert.ok(end >= 0, "ZIP has an end-of-central-directory record");
  const parts = new Map();
  let entry = bytes.readUInt32LE(end + 16);
  for (let i = 0; i < bytes.readUInt16LE(end + 10); i++) {
    assert.equal(bytes.readUInt32LE(entry), 0x02014b50);
    const method = bytes.readUInt16LE(entry + 10);
    const size = bytes.readUInt32LE(entry + 20);
    const nameSize = bytes.readUInt16LE(entry + 28);
    const name = bytes.toString("utf8", entry + 46, entry + 46 + nameSize);
    const local = bytes.readUInt32LE(entry + 42);
    assert.equal(bytes.readUInt32LE(local), 0x04034b50);
    const start = local + 30 + bytes.readUInt16LE(local + 26) + bytes.readUInt16LE(local + 28);
    const payload = bytes.subarray(start, start + size);
    assert.ok(method === 0 || method === 8, `supported ZIP compression for ${name}`);
    parts.set(name, method === 0 ? payload : inflateRawSync(payload));
    entry += 46 + nameSize + bytes.readUInt16LE(entry + 30) + bytes.readUInt16LE(entry + 32);
  }
  return parts;
}

const ruleAt = (rule, path) => err => {
  assert.equal(err.rule, rule);
  assert.equal(err.path, path);
  return true;
};
const files = process.argv[2];
assert.ok(files, "usage: node drawings.mjs <image-fixtures-directory> [native-drawings.docx] [native-pictures.docx] [native-image-path.docx]");
const png = await readFile(join(files, "sample.png"));

async function imageSourcesTransformsAndLifetime() {
  for (const [file, extension, contentType, width] of [
    ["sample.png", "png", "image/png", 152400],
    ["sample_zero_dpi.png", "png", "image/png", 304800],
    ["sample.jpg", "jpg", "image/jpeg", 152400],
    ["sample.gif", "gif", "image/gif", 304800],
    ["sample.bmp", "bmp", "image/bmp", 152400],
    ["sample.tiff", "tiff", "image/tiff", 152400],
  ]) {
    const data = await readFile(join(files, file));
    const envelope = Buffer.concat([Buffer.of(11), data, Buffer.of(22)]);
    const view = await resolvePicture(envelope.subarray(1, -1));
    assert.equal(view.name, `image.${extension}`);
    assert.equal(view.contentType, contentType);
    assert.deepEqual(view.dimensionsEmu, [width, width / 2]);
    assert.deepEqual(view.bytes, Uint8Array.from(data));
    envelope.fill(0);
    view.bytes.fill(255);
    assert.deepEqual(view.bytes, Uint8Array.from(data));
    await assert.rejects(resolveComponent({ kind: "image", data: data.subarray(0, 8) }),
      ruleAt("invalid_content", "imageRun"), `truncated ${file}`);
  }

  const bmpWithoutDensity = await readFile(join(files, "sample.bmp"));
  bmpWithoutDensity.fill(0, 38, 46);
  const bmp = await resolveComponent({ kind: "image", data: bmpWithoutDensity });
  assert.deepEqual(bmp.view.image.view.dimensionsEmu, [228600, 114300]); // 96-DPI fallback

  const jfif = await readFile(join(files, "sample.jpg"));
  const start = jfif.indexOf("JFIF\0");
  assert.ok(start > 0);
  jfif[start + 7] = 2; // dots per centimeter
  jfif.writeUInt16BE(50, start + 8);
  jfif.writeUInt16BE(100, start + 10);
  const jpeg = await resolveComponent({ kind: "image", data: jfif });
  assert.deepEqual(jpeg.view.image.view.dimensionsEmu, [172800, 43200]);
  jfif[start + 7] = 0; // aspect ratio only
  const noUnits = await resolveComponent({ kind: "image", data: jfif });
  assert.deepEqual(noUnits.view.image.view.dimensionsEmu, [304800, 152400]);

  const tiff = await readFile(join(files, "sample.tiff"));
  // Fixture IFD: XResolution/YResolution rationals and ResolutionUnit (296).
  assert.equal(tiff.readUInt16LE(154), 296);
  tiff.writeUInt16LE(1, 162);
  assert.deepEqual((await resolvePicture(tiff)).dimensionsEmu, [304800, 152400]);
  tiff.writeUInt16LE(3, 162);
  tiff.writeUInt32LE(50, 176);
  tiff.writeUInt32LE(100, 184);
  assert.deepEqual((await resolvePicture(tiff)).dimensionsEmu, [172800, 43200]);
  // APP1 following JFIF: EXIF overrides both densities without changing pixel dimensions.
  jfif[start + 7] = 1;
  const exif = Buffer.concat([Buffer.from("Exif\0\0"), tiff]);
  const app1 = Buffer.from([0xff, 0xe1, 0, 0]);
  app1.writeUInt16BE(exif.length + 2, 2);
  const endOfJfif = start - 2 + jfif.readUInt16BE(start - 2);
  const exifJpeg = Buffer.concat([jfif.subarray(0, endOfJfif), app1, exif, jfif.subarray(endOfJfif)]);
  assert.deepEqual((await resolvePicture(exifJpeg)).dimensionsEmu, [172800, 43200]);

  const snapshot = await document(pictures(png));
  const { runs } = snapshot.view.sections[0].view.paragraphs[0].view;
  assert.deepEqual(runs[0].view.image.view.altText, alt);
  assert.deepEqual(runs.slice(0, 4).map(r => r.view.image.view.dimensionsEmu), [
    [381000, 254000], [254000, 127000], [152400, 76200], [190500, 95250],
  ]);
  assert.equal(runs[1].view.image.view.altText, undefined);
  const output = snapshot.toDocx();
  if (process.argv[4]) assert.deepEqual(Buffer.from(output), await readFile(process.argv[4]));
  const parts = zipParts(output);
  const media = [...parts.entries()].filter(([name]) => name.startsWith("word/media/"));
  assert.equal(media.length, 1);
  assert.deepEqual(media[0][1], png);
  const xml = parts.get("word/document.xml").toString();
  for (const attrs of [
    /<a:srcRect\b(?=[^>]*\bl="1000")(?=[^>]*\bt="2000")(?=[^>]*\br="3000")(?=[^>]*\bb="4000")/,
    /<a:xfrm\b(?=[^>]*\brot="-600000")(?=[^>]*\bflipH="0")(?=[^>]*\bflipV="1")/,
    /<a:xfrm\b(?=[^>]*\bflipH="1")(?=[^>]*\bflipV="0")/,
    /<wp:docPr\b(?=[^>]*\bname="Logo")(?=[^>]*\btitle="Brand")(?=[^>]*\bdescr="Logo &amp; art")/,
    /<wp:anchor\b(?=[^>]*\brelativeHeight="17")(?=[^>]*\bbehindDoc="1")(?=[^>]*\blocked="0")(?=[^>]*\blayoutInCell="0")(?=[^>]*\ballowOverlap="0")/,
    /<wp:anchor\b(?=[^>]*\blocked="1")(?=[^>]*\blayoutInCell="1")(?=[^>]*\ballowOverlap="1")/,
    /<wp:wrapSquare\b(?=[^>]*\bwrapText="largest")(?=[^>]*\bdistT="12700")(?=[^>]*\bdistB="25400")(?=[^>]*\bdistL="38100")(?=[^>]*\bdistR="50800")/,
    /<wp:wrapSquare\b[^>]*wrapText="bothSides"/,
    /<wp:wrapTight\b[^>]*wrapText="left"/, /<wp:wrapThrough\b[^>]*wrapText="right"/,
    /<wp:start\b(?=[^>]*\bx="-10")(?=[^>]*\by="0")/, /<wp:wrapNone\b/,
    /<wp:wrapTopAndBottom\b[^>]*distB="63500"/,
    /<wp:posOffset>-25400<\/wp:posOffset>/, /<wp:posOffset>38100<\/wp:posOffset>/,
  ]) {
    assert.match(xml, attrs);
  }
  for (const anchor of placements) {
    for (const [axis, reference, align] of [
      ["H", anchor.horizontalReference, anchor.horizontalAlignment],
      ["V", anchor.verticalReference, anchor.verticalAlignment],
    ]) {
      if (!reference) continue;
      const tag = `wp:position${axis}`;
      assert.match(xml, new RegExp(`<${tag} relativeFrom="${reference}">` +
        (align ? `<wp:align>${align}</wp:align>` : "<wp:posOffset>")));
    }
  }

  const dir = await mkdtemp(join(tmpdir(), "rocx-drawings-"));
  try {
    const path = join(dir, "local-image");
    const declaration = filePicture(imageFile, path);
    assert.equal(await resolveComponent({ ...declaration, condition: false }), undefined);
    await writeFile(path, png); // file did not exist when the token was created
    const diskImage = await resolveComponent(declaration);
    await rm(path);
    assert.equal(diskImage.view.image.view.name, "local-image");
    assert.deepEqual(diskImage.view.image.view.bytes, Uint8Array.from(png));
    const fromDisk = await document([{ kind: "resolved", value: diskImage }]);
    const parts = zipParts(fromDisk.toDocx());
    assert.match(parts.get("word/document.xml").toString(), /<pic:cNvPr\b[^>]*name="local-image"/);
    const media = [...parts.entries()].filter(([name]) => name.startsWith("word/media/"));
    assert.equal(media.length, 1);
    assert.deepEqual(media[0][1], png);
  } finally {
    await rm(dir, { force: true, recursive: true });
  }

  const badData = { kind: "image", data: Uint8Array.of(1, 2, 3) };
  await assert.rejects(resolveComponent(badData), ruleAt("invalid_content", "imageRun"));
  await assert.rejects(resolveComponent({ kind: "image",
    type: "image/svg+xml", data: new TextEncoder().encode('<svg xmlns="http://www.w3.org/2000/svg"/>') }),
    ruleAt("invalid_content", "imageRun"));
  await assert.rejects(resolveComponent({ kind: "image", data: png, altText: { description: "\0" } }),
    ruleAt("invalid_xml", "imageRun"));
  await assert.rejects(resolveComponent({ kind: "image", data: png, transformation: { rotation: 361 } }),
    ruleAt("invalid_measurement", "imageRun.rotation"));
}

// The independent Rust fixture uses ImageData::Path on sample.png, with these
// same settings and the fixture document's timestamp. Byte-source equivalence
// is not evidence of native Path parity.
async function nativeImagePathParity(reference) {
  const file = join(files, "sample.png");
  const altText = { title: "Path probe" };
  const transformation = { width: 15 };
  const run = () => ({ ...filePicture(imageFile, file), altText, transformation });
  const resolved = await resolveComponent(run());
  const { name, dimensionsEmu, contentType, altText: resolvedAlt, bytes } = resolved.view.image.view;
  const snapshot = await document([run()]);
  assert.deepEqual({
    name, dimensionsEmu, contentType, altText: resolvedAlt, bytesMatch: Buffer.from(bytes).equals(png),
    exactDocx: Buffer.from(snapshot.toDocx()).equals(await readFile(reference)),
  }, {
    name: "sample.png", dimensionsEmu: [190500, 95250], contentType: "image/png", altText,
    bytesMatch: true, exactDocx: true,
  });
}

async function shapeChartFractionAndBreakVariants() {
  const standalone = await Promise.all(drawingRuns().map(run => resolveComponent(run)));
  const snapshot = await document(drawingRuns());
  const { runs, text } = snapshot.view.sections[0].view.paragraphs[0].view;
  assert.deepEqual(standalone.map(r => r.view), runs.map(r => r.view));
  const [ellipse, rect, box, chart, fraction, tab, ...breaks] = runs.map(r => r.view);
  assert.deepEqual(ellipse.properties, {
    width: 60, height: 31, geometry: "ellipse", fill: [1, 2, 3], text: "Shape & text",
    outline: { width: 2, color: [10, 20, 30] },
    textFormat: { fontSize: 9.25, color: [30, 20, 10], bold: false, italic: true,
      alignment: "right", verticalAlignment: "bottom", font: "Aptos" },
    anchor: { ...placements[0], wrapPolygon: [], relativeHeight: 0, behindText: false },
  });
  assert.equal(rect.properties.geometry, "rectangle");
  assert.equal(rect.properties.width, 72);
  assert.equal(rect.properties.height, 72);
  assert.equal(rect.properties.textFormat.italic, false);
  assert.equal(box.properties.geometry, "textBox");
  assert.deepEqual(chart.properties, {
    width: 120, height: 48, categories: ["One", "Two & 三"], values: [-1.5, 0],
  });
  assert.deepEqual(fraction, { kind: "fraction", displayText: "", numerator: "α", denominator: "β" });
  assert.equal(tab.kind, "tab");
  assert.equal(tab.displayText, "\t");
  assert.deepEqual(breaks.map(b => b.breakType), ["line", "line", "textWrapping", "page", "column"]);
  assert.deepEqual(breaks.map(b => b.displayText), ["\n", "\n", "\n", "", ""]);
  assert.equal(text, "\t\n\n\n");
  assert.equal(await inlineText(runs), text);
  const output = snapshot.toDocx();
  const parts = zipParts(output);
  const xml = parts.get("word/document.xml").toString();
  for (const expected of [
    /<a:prstGeom prst="ellipse"/, /<a:prstGeom prst="rect"/, /<a:txSp>/,
    /<a:ln w="25400"/, /<a:srgbClr val="010203"/, /<a:srgbClr val="0A141E"/,
    /<a:srgbClr val="1E140A"/, /<a:rPr\b(?=[^>]*\bsz="925")(?=[^>]*\bb="0")(?=[^>]*\bi="1")/,
    /<a:latin typeface="Aptos"/, /<a:bodyPr anchor="b"/, /<a:pPr algn="r"/,
    /<m:f>/, /<m:t[^>]*>α<\/m:t>/, /<m:t[^>]*>β<\/m:t>/,
    /<w:tab\b/, /<w:br w:type="textWrapping"/, /<w:br w:type="page"/, /<w:br w:type="column"/,
  ]) {
    assert.match(xml, expected);
  }
  const chartXml = [...parts.entries()].find(([path]) => /^word\/charts\/[^/]+\.xml$/.test(path))?.[1].toString();
  assert.match(chartXml, /Two &amp; 三/);
  assert.match(chartXml, />-1\.5</);
  assert.ok(![...parts.keys()].some(path => path.startsWith("word/embeddings/")));

  const shape = properties => resolveComponent({ kind: "shape", properties });
  await assert.rejects(shape({ width: 0 }), ruleAt("invalid_measurement", "shapeRun.shape.width"));
  await assert.rejects(shape({ geometry: "textBox", fill: [1, 2, 3] }),
    ruleAt("invalid_content", "shapeRun.shape.geometry"));
  await assert.rejects(shape({ anchor: { horizontalReference: "line" } }),
    ruleAt("invalid_content", "shapeRun.anchor.horizontalReference"));
  await assert.rejects(shape({ outline: { width: -1, color: [1, 2, 3] } }),
    ruleAt("invalid_measurement", "shapeRun.shape.outline.width"));
  await assert.rejects(shape({ textFormat: { fontSize: 0 } }),
    ruleAt("invalid_measurement", "shapeRun.shape.textFormat.fontSize"));
  await assert.rejects(shape({ text: "\0" }), ruleAt("invalid_xml", "shapeRun.properties.text"));
  const columnChart = (categories, values) =>
    resolveComponent({ kind: "columnChart", properties: { width: 20, height: 30, categories, values } });
  await assert.rejects(columnChart([], []), ruleAt("invalid_content", "columnChartRun.properties.categories"));
  await assert.rejects(columnChart(["One"], []), ruleAt("invalid_content", "columnChartRun.properties.values"));
  await assert.rejects(columnChart(["One"], [Infinity]), ruleAt("invalid_content", "columnChartRun.properties.values[0]"));
  await assert.rejects(columnChart(["\0"], [1]), ruleAt("invalid_xml", "columnChartRun.properties.categories[0]"));
  await assert.rejects(resolveComponent({ kind: "fraction", numerator: "", denominator: "1" }),
    ruleAt("invalid_content", "inlineFraction.numerator"));
  await assert.rejects(resolveComponent({ kind: "fraction", numerator: "1", denominator: "2\u2028" }),
    ruleAt("invalid_content", "inlineFraction.denominator"));
  await assert.rejects(resolveComponent({ kind: "fraction", numerator: "\0", denominator: "1" }),
    ruleAt("invalid_xml", "inlineFraction.numerator"));
  if (process.argv[3]) assert.deepEqual(Buffer.from(output), await readFile(process.argv[3]));
}

async function resolvedBlockViewsAndReuse() {
  let starts = 0;
  const source = await document([
    { kind: "image", data: lazy(() => { starts++; return png; }) },
    { kind: "tab" }, { kind: "break", condition: false },
  ]);
  const { blocks, paragraphs } = source.view.sections[0].view;
  assert.equal(blocks.length, 1);
  assert.equal(paragraphs.length, 1);
  const handles = paragraphs[0].view.runs;
  assert.equal(handles.length, 2);
  assert.equal(paragraphs[0].view.text, "\t");
  for (let i = 0; i < 2; i++) {
    const copy = await document(handles.map(value => ({ kind: "resolved", value })));
    assert.deepEqual(copy.toDocx(), source.toDocx());
  }
  assert.equal(starts, 1);
}

async function cancelledWaiterDoesNotPoisonSharedWork() {
  const started = Promise.withResolvers();
  const payload = Promise.withResolvers();
  let calls = 0;
  const data = deferred(future(({ signal }) => {
    calls++;
    started.resolve(signal);
    return payload.promise;
  }));
  const controller = new AbortController();
  const declaration = { kind: "image", data };
  const cancelled = resolveComponent(declaration, { signal: controller.signal });
  const sharedSignal = await started.promise;
  // Observe the survivor's guard before dropping the first awaiter.
  const surviving = Promise.withResolvers();
  const survivor = resolveComponent({ ...declaration, condition: () => {
    surviving.resolve();
    return true;
  } });
  try {
    await surviving.promise;
    controller.abort("drawing waiter cancelled");
    await assert.rejects(cancelled, { name: "AbortError", cause: "drawing waiter cancelled" });
    assert.equal(sharedSignal.aborted, false);
  } finally {
    payload.resolve(png);
  }
  assert.deepEqual((await survivor).view.image.view.bytes, Uint8Array.from(png));
  assert.equal(calls, 1);
}

async function resolvedInlineViewsConstructorsAndReuse() {
  const original = await resolveComponent({ kind: "image", data: png });
  const runs = await construct(original.view.image);
  assert.deepEqual(runs.map(r => r.view.kind), ["image", "tab", "break", "shape", "columnChart", "fraction"]);
  const [image, tab, br] = runs.map(r => r.view);
  assert.equal(image.format.bold, true);
  assert.equal(image.format.italic, false);
  assert.equal(tab.format.bold, false);
  assert.equal(br.format.italic, true);
  assert.deepEqual(image.image.view.bytes, Uint8Array.from(png));
  for (const view of [image, tab, br]) {
    assert.deepEqual(view.comment, { text: "Annotated drawing run", author: "Reviewer" });
  }
  assert.equal(await inlineText(runs), "\t");
  for (const run of runs) await validateValue({ kind: "resolvedInline", value: run });
  const reusable = runs.map(value => ({ kind: "resolved", value }));
  const output = await document(reusable);
  assert.deepEqual(output.toDocx(), (await document(reusable)).toDocx());
  await assert.rejects(createResolvedInline({ kind: "image", image: original }),
    ruleAt(undefined, "image"));
  const invalidFraction = await createResolvedInline({ kind: "fraction", numerator: "1", denominator: "" });
  await assert.rejects(validateValue({ kind: "resolvedInline", value: invalidFraction }),
    ruleAt("invalid_content", "denominator"));
  const contextual = await createResolvedInline({ kind: "shape", properties: { geometry: "textBox", fill: [1, 2, 3] } });
  await validateValue({ kind: "resolvedInline", value: contextual });
  await assert.rejects(document([{ kind: "resolved", value: contextual }]),
    ruleAt("invalid_content", "sections[0].children[0].runs[0].shape.geometry"));
}

async function standaloneValidationAndDiagnostics() {
  await validateImageTransformation(pictureTransform);
  await assert.rejects(validateImageTransformation({ width: NaN }), ruleAt("invalid_measurement", "width"));
  await assert.rejects(validateImageTransformation({ crop: { left: 55, right: 45 } }),
    ruleAt("invalid_measurement", "crop"));
  await assert.rejects(validateImageTransformation({ anchor: { horizontalAlignment: "center", x: 5 } }),
    ruleAt("invalid_content", "anchor"));
  await assert.rejects(validateImageTransformation({ anchor: { verticalReference: "character" } }),
    ruleAt("invalid_content", "anchor.verticalReference"));
  await assert.rejects(validateImageTransformation({ anchor: { wrap: "tight", wrapPolygon: [[0, 0], [1, 1], [2, 2]] } }),
    ruleAt("invalid_content", "anchor.wrapPolygon"));
  await assert.rejects(validateImageTransformation({ anchor: { wrap: "none", distanceLeft: 0 } }),
    ruleAt("invalid_content", "anchor.wrap"));
  await assert.rejects(validateImageTransformation({ anchor: { wrap: "square", distanceLeft: -1 } }),
    ruleAt("invalid_measurement", "anchor.distanceLeft"));
  await assert.rejects(validateImageTransformation({ anchor: { wrap: "\uD800" } }),
    ruleAt(undefined, "value.anchor.wrap"));
  await assert.rejects(validateImageTransformation({ flipHorizontal: "no" }),
    ruleAt(undefined, "value.flipHorizontal"));
  await validateImageTransformation({});
}

async function chromiumDrawings() {
  const { chromium } = await import("playwright");
  const { createServer } = await import("vite");
  const host = "127.0.0.1";
  const server = await createServer({
    configFile: false, server: { host, port: 0 }, optimizeDeps: { exclude: ["rocx-docx"] },
    plugins: [{
      name: "drawings-test-page",
      configureServer(vite) {
        vite.middlewares.use("/drawings-test.html", (_, response) => {
          response.setHeader("Content-Type", "text/html");
          response.end("<!doctype html><title>Installed drawings</title>");
        });
      },
    }],
  });
  let browser;
  try {
    await server.listen(0);
    browser = await chromium.launch();
    const page = await browser.newPage();
    await page.goto(`http://${host}:${server.httpServer.address().port}/drawings-test.html`);
    const result = await page.evaluate(async (source) => {
      const fixtures = await import("/drawings.mts");
      const pictureSnapshot = await fixtures.document(fixtures.pictures(Uint8Array.from(source)));
      const drawingSnapshot = await fixtures.document(fixtures.drawingRuns());
      const picture = pictureSnapshot.view.sections[0].view.paragraphs[0].view.runs[0].view.image.view;
      picture.bytes.fill(0);
      return {
        name: picture.name,
        dimensions: picture.dimensionsEmu,
        bytes: [...picture.bytes],
        text: drawingSnapshot.view.sections[0].view.paragraphs[0].view.text,
        pictures: [...pictureSnapshot.toDocx()],
        drawings: [...drawingSnapshot.toDocx()],
      };
    }, [...png]);
    assert.equal(result.name, "image.png");
    assert.deepEqual(result.dimensions, [381000, 254000]);
    assert.deepEqual(Buffer.from(result.bytes), png);
    assert.equal(result.text, "\t\n\n\n");
    if (process.argv[3]) assert.deepEqual(Buffer.from(result.drawings), await readFile(process.argv[3]));
    if (process.argv[4]) assert.deepEqual(Buffer.from(result.pictures), await readFile(process.argv[4]));
    console.log(`Drawings packed: Chromium ${browser.version()}`);
  } finally {
    await browser?.close();
    await server.close();
  }
}

await imageSourcesTransformsAndLifetime();
await shapeChartFractionAndBreakVariants();
assert.deepEqual(await standaloneAndConditionalComponents(), Array(6).fill(undefined));
await cancelledWaiterDoesNotPoisonSharedWork();
await resolvedBlockViewsAndReuse();
await resolvedInlineViewsConstructorsAndReuse();
await standaloneValidationAndDiagnostics();
if (process.env.ROCX_CHECK_DRAWINGS_BROWSER === "1") await chromiumDrawings();
if (process.argv[5]) await nativeImagePathParity(process.argv[5]);
console.log(`Drawing cases passed${process.argv[5] ? ", including native ImageData::Path parity" : " (native ImageData::Path parity not checked)"}`);
