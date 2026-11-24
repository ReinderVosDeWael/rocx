import assert from "node:assert/strict";
import { chmod, mkdtemp, mkdir, readFile, readdir, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { crc32, inflateRawSync } from "node:zlib";
import { compileTemplate } from "rocx-docx";
import * as fx from "./template.mts";

const [defaultPath, imagePath, editorPath, oracleDirectory] = process.argv.slice(2);
const word = "word/document.xml";
const types = "[Content_Types].xml";
const rootRels = "_rels/.rels";
const mainRels = "word/_rels/document.xml.rels";
const w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const r = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const mc = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const rels = "http://schemas.openxmlformats.org/package/2006/relationships";
const mime = "application/vnd.openxmlformats-officedocument.wordprocessingml.";
const enc = (value) => Buffer.from(value);
const str = (value) => new TextDecoder().decode(value);
const text = (value) => `<w:r><w:t>${value}</w:t></w:r>`;
const para = (value) => `<w:p>${text(value)}</w:p>`;
const xml = (plan, name = word) => str(plan.part(name));
const rule = (rule, path) => (error) => error instanceof Error && error.rule === rule &&
  (path === undefined || error.path === path);

// Independent ZIP fixture I/O (only binary fixture plumbing, not OPC semantics).
function unzip(bytes) {
  const zip = enc(bytes);
  const end = zip.lastIndexOf(Buffer.from("504b0506", "hex"));
  const parts = new Map();
  let offset = zip.readUInt32LE(end + 16);
  for (let i = 0; i < zip.readUInt16LE(end + 10); i++) {
    assert.equal(zip.readUInt32LE(offset), 0x02014b50);
    const length = zip.readUInt16LE(offset + 28);
    const name = zip.toString("utf8", offset + 46, offset + 46 + length);
    const local = zip.readUInt32LE(offset + 42);
    const start = local + 30 + zip.readUInt16LE(local + 26) + zip.readUInt16LE(local + 28);
    const raw = zip.subarray(start, start + zip.readUInt32LE(offset + 20));
    const method = zip.readUInt16LE(offset + 10);
    assert.ok(method === 0 || method === 8);
    parts.set(name, method === 0 ? enc(raw) : inflateRawSync(raw));
    offset += 46 + length + zip.readUInt16LE(offset + 30) + zip.readUInt16LE(offset + 32);
  }
  return parts;
}

function zip(parts) {
  const files = [], directory = [];
  let offset = 0;
  for (const [name, contents] of parts) {
    const key = enc(name), data = enc(contents);
    const local = Buffer.alloc(30), central = Buffer.alloc(46);
    local.writeUInt32LE(0x04034b50);
    local.writeUInt16LE(20, 4);
    local.writeUInt32LE(crc32(data), 14);
    local.writeUInt32LE(data.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(key.length, 26);
    central.writeUInt32LE(0x02014b50);
    central.writeUInt16LE(20, 4);
    local.copy(central, 6, 4, 30);
    central.writeUInt32LE(offset, 42);
    files.push(local, key, data);
    directory.push(central, key);
    offset += local.length + key.length + data.length;
  }
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50);
  end.writeUInt16LE(parts.size, 8);
  end.writeUInt16LE(parts.size, 10);
  end.writeUInt32LE(directory.reduce((size, entry) => size + entry.length, 0), 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...files, ...directory, end]);
}

const defaultBytes = await readFile(defaultPath);
const defaults = unzip(defaultBytes);
function modify(parts, name, transform) {
  parts.set(name, enc(transform(str(parts.get(name)))));
}
function fixture(body, options = {}) {
  const parts = new Map(defaults);
  const ns = options.ns ?? "";
  const sect = options.sect ?? "";
  parts.set(word, enc(`<w:document xmlns:w="${w}" xmlns:r="${r}" ${ns}><w:body>${body}<w:sectPr>${sect}</w:sectPr></w:body></w:document>`));
  return parts;
}
function addStory(parts, kind, root, contents, id) {
  const name = `word/${kind}-fixture.xml`;
  parts.set(name, enc(`<w:${root} xmlns:w="${w}">${contents}</w:${root}>\n`));
  modify(parts, types, (xml) => xml.replace("</Types>", `<Override PartName="/${name}" ContentType="${mime}${kind}+xml"/></Types>`));
  modify(parts, mainRels, (xml) => xml.replace("</Relationships>",
    `<Relationship Id="${id}" Type="${r}/${kind}" Target="${name.slice(5)}"/></Relationships>`));
  return name;
}
function exactLimits(parts) {
  const sizes = [...parts.values()].map((part) => part.length);
  return fx.budgets(sizes.length, Math.max(...sizes), sizes.reduce((a, b) => a + b));
}
function under(limits, key) {
  return { ...limits, [key]: limits[key] - 1 };
}
function samePayload(plan, name, expected) {
  assert.deepEqual(enc(plan.part(name)), expected, name);
}
function inOrder(haystack, ...needles) {
  let offset = -1;
  for (const needle of needles) {
    offset = haystack.indexOf(needle, offset + 1);
    assert.notEqual(offset, -1, `missing/out-of-order marker: ${needle}`);
  }
}

// Serialized by Playwright, so exercise the same routes in Node and Chromium.
async function renderOptionsCases({ entry, sourceBytes, readLimits, snapshot, timestamp }) {
  const { compileTemplate, future, generateDocx, openDocx, resolveDocument } = await import(entry);
  const source = { bytes: Uint8Array.from(sourceBytes) };
  const program = () => ({
    sections: [{ kind: "section", children: [{ kind: "paragraph", text: "Budgets" }] }],
    timestamp: future(async () => snapshot),
  });
  const doc = await resolveDocument(program());
  const options = { timestamp, readLimits };
  const outputs = {
    snapshot: doc.toDocx(),
    override: doc.toDocx(options),
    generate: await generateDocx(program(), options),
    template: doc.toDocxWithTemplate(source, options),
    plan: (await compileTemplate(doc, source, options)).toBytes(),
  };
  const imported = await openDocx(outputs.snapshot, { readLimits });
  if (doc.view.timestamp !== snapshot || imported.body()[0]?.text !== "Budgets") {
    throw new Error("Fractional snapshot or BigInt import did not round-trip");
  }
  async function rejects(run, path, message = /./) {
    try {
      await run();
    } catch (error) {
      if (error instanceof Error && error.path === path && message.test(error.message)) return;
      throw new Error(`Expected error at ${path}, got ${error.path}: ${error.message}`);
    }
    throw new Error(`Expected rejection at ${path}`);
  }
  const render = [
    (overrides) => doc.toDocx(overrides),
    (overrides) => generateDocx(program(), overrides),
    (overrides) => doc.toDocxWithTemplate(source, overrides),
    (overrides) => compileTemplate(doc, source, overrides),
  ];
  const read = [...render, (overrides) => openDocx(source.bytes, overrides)];
  for (const run of read) {
    for (const key of ["maxPartBytes", "maxExpandedBytes"]) {
      for (const value of [-1n, 1n << 64n, Number.MAX_SAFE_INTEGER + 1, 0.5]) {
        await rejects(() => run({ readLimits: { [key]: value } }), `options.readLimits.${key}`);
      }
    }
    for (const maxParts of [-1, 4_294_967_296, 1n]) {
      await rejects(() => run({ readLimits: { maxParts } }), "options.readLimits.maxParts");
    }
  }
  for (const [invalid, message] of [
    [42, /expected a string/],
    ["\ud800", /malformed UTF-16/],
    ["\udc00", /malformed UTF-16/],
  ]) {
    await rejects(() => resolveDocument({ ...program(), timestamp: invalid }), "timestamp", message);
    for (const run of render) {
      await rejects(() => run({ timestamp: invalid }), "options.timestamp", message);
    }
  }
  const legacy = { blocks: [] };
  await generateDocx(legacy, { timestamp: "2026-01-01T00:00:00.100Z" });
  for (const invalid of [snapshot, timestamp]) {
    try {
      await generateDocx(legacy, { timestamp: invalid });
    } catch (error) {
      if (error instanceof TypeError) continue;
      throw error;
    }
    throw new Error("Legacy API accepted a timestamp outside its UTC contract");
  }
  return Object.fromEntries(Object.entries(outputs).map(([key, bytes]) => [key, Array.from(bytes)]));
}

test("nativeTimestampTextAndLosslessBudgetsInNodeAndChromium", async () => {
  const args = {
    entry: "rocx-docx",
    sourceBytes: [...await readFile(editorPath)],
    readLimits: fx.losslessBudgets,
  };
  const { chromium } = await import("playwright");
  const { createServer } = await import("vite");
  const host = "127.0.0.1";
  const server = await createServer({ configFile: false, server: { host } });
  let browser;
  try {
    await server.listen(0);
    browser = await chromium.launch();
    const page = await browser.newPage();
    await page.goto(`http://${host}:${server.httpServer.address().port}/`);
    for (const [kind, snapshot, timestamp, expectedXml = timestamp] of [
      ["fractional", "2026-01-01T00:00:00.1Z", "2026-02-28T01:02:03.123456789Z"],
      ["offset", "2026-02-28T01:02:03.123456789+00:00", "2026-03-01T01:02:03.00001-04:30"],
      ["unparsed", "2026-02-30T01:02:03.1Z", "not a <date> & λ", "not a &lt;date&gt; &amp; λ"],
    ]) {
      const input = { ...args, snapshot, timestamp };
      const node = await renderOptionsCases(input);
      const core = (bytes) => str(unzip(bytes).get("docProps/core.xml"));
      assert.ok(core(node.snapshot).includes(`>${snapshot}</dcterms:created>`));
      assert.ok(core(node.override).includes(`>${expectedXml}</dcterms:created>`));
      const actual = await page.evaluate(renderOptionsCases, {
        ...input, entry: "/node_modules/rocx-docx/browser.js",
      });
      for (const key of ["snapshot", "override", "template"]) {
        const expected = oracleDirectory
          ? await readFile(join(oracleDirectory, `${kind}-${key}.docx`))
          : enc(node[key]);
        assert.deepEqual(enc(node[key]), expected, `${kind}/${key} Node`);
        assert.deepEqual(enc(actual[key]), expected, `${kind}/${key} Chromium`);
      }
      for (const output of [node, actual]) {
        assert.deepEqual(output.generate, output.override);
        assert.deepEqual(output.plan, output.template);
      }
    }
  } finally {
    await browser?.close();
    await server.close();
  }
});

test("freshAndTemplateRepeatRendering", async () => {
  let count = 0;
  const doc = await fx.report(() => count++);
  const fresh = doc.toDocx();
  const changed = doc.toDocx(fx.override);
  assert.notDeepEqual(changed, fresh);
  assert.match(str(unzip(changed).get("docProps/core.xml")), /2026-12-01T12:30:00\.123Z/);
  assert.equal(doc.view.timestamp, fx.snapshotDate);
  const srcA = fx.source(zip(fixture(para("from A"))));
  const srcB = fx.source(zip(fixture(para("from B"))));
  const rendered = doc.toDocxWithTemplate(srcA);
  assert.deepEqual(doc.toDocxWithTemplate(srcA), rendered);
  assert.notDeepEqual(doc.toDocxWithTemplate(srcB), rendered);
  assert.notDeepEqual(doc.toDocxWithTemplate(srcA, fx.override), rendered);
  assert.deepEqual(doc.toDocx(), fresh);
  assert.equal(count, 1);
  for (const render of [doc.toDocx, (options) => doc.toDocxWithTemplate(srcA, options)]) {
    assert.throws(() => render(fx.strict), rule("unsupported_profile", "options.profile"));
    assert.throws(() => render({ profile: "invalid" }), { path: "options.profile" });
    assert.throws(() => render({ readLimits: { maxPartBytes: -1 } }), { path: "options.readLimits.maxPartBytes" });
  }
  const limits = exactLimits(defaults);
  assert.deepEqual(doc.toDocx({ readLimits: limits }), fresh);
  for (const key of Object.keys(limits)) {
    assert.throws(() => doc.toDocx({ readLimits: under(limits, key) }), /read limit/);
  }
  if (oracleDirectory) {
    assert.deepEqual(enc(fresh), await readFile(join(oracleDirectory, "fresh.docx")));
    assert.deepEqual(enc(changed), await readFile(join(oracleDirectory, "override.docx")));
  }
});

test("compileTemplatePlanAndBudgets", async () => {
  const doc = await fx.clean();
  const parts = fixture(para("relocated source"));
  const relocated = "word/relocated.xml";
  parts.set(relocated, parts.get(word));
  parts.delete(word);
  parts.set("word/_rels/relocated.xml.rels", parts.get(mainRels));
  parts.delete(mainRels);
  modify(parts, rootRels, (xml) => xml.replace(word, relocated));
  modify(parts, types, (xml) => xml.replace(word, relocated).replace(`${mime}document.main+xml`, `${mime}template.main+xml`));
  const payload = enc([0, 1, 2, 255]);
  parts.set("vendor/opaque.bin", payload);
  modify(parts, types, (xml) => xml.replace("</Types>", '<Default Extension="bin" ContentType="application/octet-stream"/></Types>'));
  const input = zip(parts);
  const padded = Buffer.concat([enc([6]), input, enc([7])]);
  const src = fx.source(padded.subarray(1, -1));
  const limits = exactLimits(parts);
  const plan = await compileTemplate(doc, src, { readLimits: limits });
  const encoded = plan.toBytes();
  const bigintLimits = {
    ...limits, maxPartBytes: BigInt(limits.maxPartBytes),
    maxExpandedBytes: BigInt(limits.maxExpandedBytes),
  };
  assert.deepEqual((await compileTemplate(doc, src, { readLimits: bigintLimits })).toBytes(), encoded);
  for (const key of ["maxPartBytes", "maxExpandedBytes"]) {
    await assert.rejects(compileTemplate(doc, src, {
      readLimits: { ...bigintLimits, [key]: bigintLimits[key] - 1n },
    }), /read limit/);
  }
  assert.equal(plan.validate(), undefined);
  assert.match(xml(plan, relocated), /relocated source/);
  assert.ok(xml(plan, types).includes(`${mime}document.main+xml`));
  assert.equal(plan.part(word), undefined);
  assert.equal(plan.part("Word/relocated.xml"), undefined);
  assert.throws(() => plan.part("../relocated.xml"), /invalid OPC part name/);
  samePayload(plan, "/vendor/opaque.bin", payload);
  const list = plan.parts();
  const names = list.map(([name]) => name);
  assert.deepEqual(names, [...names].sort());
  const decoded = unzip(encoded);
  assert.deepEqual([...decoded.keys()], names);
  for (const [name, bytes] of list) assert.deepEqual(enc(bytes), decoded.get(name), name);
  list.find(([name]) => name === "vendor/opaque.bin")[1].fill(0);
  plan.part("vendor/opaque.bin").fill(1);
  plan.toBytes().fill(2);
  assert.deepEqual(plan.toBytes(), encoded);
  assert.deepEqual(doc.toDocxWithTemplate(src), encoded);
  for (const key of Object.keys(limits)) {
    const options = { readLimits: under(limits, key) };
    await assert.rejects(compileTemplate(doc, src, options), /read limit/);
    assert.throws(() => doc.toDocxWithTemplate(src, options), /read limit/);
  }
  await assert.rejects(compileTemplate(doc, fx.source(new Uint8Array())), /zip|archive/i);
  await assert.rejects(compileTemplate(doc, { bytes: new Uint16Array(1) }), { path: "source.bytes" });
  padded.fill(0);
  samePayload(plan, "vendor/opaque.bin", payload);
});

test("templateOrderedReplacementScopesAndBoundaries", async () => {
  const doc = await fx.report(() => {});
  const parts = fixture(
    '<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>TO</w:t></w:r>' + text("KEN") + "</w:p>" +
    para("DELETE") + '<w:p><w:pPr><w:tabs><w:tab w:val="left" w:pos="720"/></w:tabs></w:pPr>' +
    '<w:r><w:t>left</w:t><w:tab/><w:t>right</w:t></w:r></w:p>' +
    '<w:p><w:r><w:t>AB</w:t><w:commentReference w:id="1"/><w:t>CD</w:t></w:r></w:p>' +
    '<w:p><w:r><w:t>EF</w:t><w:noBreakHyphen/><w:t>GH</w:t></w:r></w:p>' +
    `<w:p>${text("before")}<w:hyperlink w:anchor="target">${text("inside")}</w:hyperlink></w:p>` +
    `<w:p><w:fldSimple w:instr="PAGE">${text("protected")}</w:fldSimple></w:p>`,
    { sect: '<w:headerReference w:type="default" r:id="rId41"/><w:footerReference w:type="default" r:id="rId42"/>' },
  );
  const header = addStory(parts, "header", "hdr", para("TOKEN"), "rId41");
  const footer = addStory(parts, "footer", "ftr", para("TOKEN"), "rId42");
  const comments = addStory(parts, "comments", "comments", `<w:comment w:id="1" w:author="A">${para("TOKEN")}</w:comment>`, "rId43");
  const footnotes = addStory(parts, "footnotes", "footnotes", `<w:footnote w:id="2">${para("TOKEN")}</w:footnote>`, "rId44");
  const endnotes = addStory(parts, "endnotes", "endnotes", `<w:endnote w:id="3">${para("TOKEN")}</w:endnote>`, "rId45");
  const bytes = zip(parts);
  for (const scope of [undefined, ...fx.scopes]) {
    const plan = await compileTemplate(doc, fx.source(bytes, { ...fx.ordered, scope }));
    const body = xml(plan);
    assert.match(body, /<w:b\/><\/w:rPr><w:t\b[^>]*>second<\/w:t>/);
    assert.match(body, /w:pos="720"/);
    assert.match(body, /<w:t\b[^>]*>tabs<\/w:t>/);
    assert.match(body, /across marker.*<w:commentReference/);
    assert.match(body, /<w:t>EF<\/w:t><w:noBreakHyphen\/><w:t>GH<\/w:t>/);
    assert.doesNotMatch(body, /DELETE|WRONG/);
    assert.equal(body.match(/>TOKEN</g).length, 1, "appended content is not edited");
    for (const name of [header, footer, comments, footnotes, endnotes]) {
      const shouldEdit = scope === "supportedStories" || (scope !== "body" && [header, footer].includes(name));
      if (shouldEdit) assert.match(xml(plan, name), /second/);
      else samePayload(plan, name, parts.get(name));
    }
  }
  for (const needle of ["beforeinside", "protected", "EFGH"]) {
    await assert.rejects(compileTemplate(doc, fx.source(bytes, { replacements: [[needle, "unsafe"]] })), rule("unsupported_edit"));
  }
  await assert.rejects(compileTemplate(doc, fx.source(bytes, { replacements: [["", "x"]] })), /empty/);
  await assert.rejects(compileTemplate(doc, fx.source(bytes, { replacements: [["a"]] })), { path: "source.edits.replacements[0]" });
  await assert.rejects(compileTemplate(doc, fx.source(bytes, { replacements: [["a", "\ud800"]] })), { path: "source.edits.replacements[0][1]" });
  await assert.rejects(compileTemplate(doc, fx.source(bytes, { scope: "all" })), { path: "source.edits.scope" });
});

function move(kind, id, content) {
  const label = `move${kind}`;
  return `<w:${label}RangeStart w:id="${id}" w:name="pair" w:author="A" w:date="${fx.snapshotDate}"/>` +
    `<w:${label} w:id="${id + 10}" w:author="A">${content}</w:${label}><w:${label}RangeEnd w:id="${id}"/>`;
}
const control = (kind = "text") => `<w:p><w:sdt><w:sdtPr><w:tag w:val="client"/><w:${kind}/>` +
  '</w:sdtPr><w:sdtContent><w:r><w:rPr><w:i/></w:rPr><w:t>cache</w:t></w:r></w:sdtContent></w:sdt></w:p>';

test("templateControlAndRevisionComposition", async () => {
  const doc = await fx.revisionReport();
  const from = move("From", 2, "<w:r><w:delText>moved</w:delText></w:r>");
  const to = move("To", 3, text("moved"));
  const ordinary = para("ordinary");
  const revisions = `<w:p><w:ins w:id="9" w:author="A">${text("insertion")}</w:ins>` +
    '<w:del w:id="10" w:author="A"><w:r><w:delText>deletion</w:delText></w:r></w:del></w:p>';
  const body = `${ordinary}${control()}${revisions}<w:p>${text("from:")}${from}</w:p><w:p>${text("to:")}${to}</w:p>`;
  for (const textRevisionAction of fx.actions) {
    const plan = await compileTemplate(doc, fx.source(zip(fixture(body)), { ...fx.controlEdits, textRevisionAction }));
    const result = xml(plan);
    assert.match(result, /<w:i\/><\/w:rPr><w:t\b[^>]*>TARGET<\/w:t>/);
    assert.match(result, />changed</);
    assert.match(result, /<w:ins\b[^>]*w:author="New author"[^>]*>.*>New author revision<\/w:t>/);
    assert.equal(result.match(/<w:ins\b/g)?.length, 1, "only the declaration retains an insertion");
    assert.doesNotMatch(result, /cache|must not run|<w:(del|moveFrom|moveTo)/);
    if (textRevisionAction === "accept") {
      inOrder(result, ">insertion<", ">from:<", ">to:<", ">moved<");
      assert.doesNotMatch(result, />deletion</);
    } else {
      inOrder(result, ">deletion<", ">from:<", ">moved<", ">to:<");
      assert.doesNotMatch(result, />insertion</);
    }
  }
  for (const [markup, message] of [
    [ordinary, /not found|no matching/],
    [control() + control(), /ambiguous|more than one|multiple/],
    [control("picture"), /plain.text|supported/],
  ]) {
    await assert.rejects(compileTemplate(doc, fx.source(zip(fixture(markup)), fx.controlEdits)), message);
  }
  for (const malformed of [`<w:p>${from}</w:p>`, `<w:p><w:ins>${text("a")}${text("b")}</w:ins></w:p>`]) {
    await assert.rejects(compileTemplate(doc, fx.source(zip(fixture(malformed)), { textRevisionAction: "accept" })),
      rule("unsupported_edit", word));
  }
});

test("indexedTemplateCompositionAndPreservation", async () => {
  const doc = await fx.indexed();
  const parts = fixture(para("Before") + `<w:tbl><w:tblPr/><w:tblGrid/><w:tr><w:tc>${para("Nested")}</w:tc></w:tr></w:tbl>` +
    '<w:p><w:pPr><w:sectPr><w:type w:val="continuous"/></w:sectPr></w:pPr>' +
    `${text("Boundary")}</w:p>${para("Original")}`,
    { sect: '<w:headerReference w:type="default" r:id="rId41"/><w:pgSz w:w="12240" w:h="15840"/>' });
  const header = addStory(parts, "header", "hdr",
    `<w:p xmlns:r="${r}"><w:hyperlink r:id="link">${text("Original header")}</w:hyperlink></w:p>`, "rId41");
  const headerRels = "word/_rels/header-fixture.xml.rels";
  parts.set(headerRels, enc(`<Relationships xmlns="${rels}"><Relationship Id="link" Type="${r}/hyperlink" Target="https://example.invalid/template" TargetMode="External"/></Relationships>\n`));
  const sourceBytes = zip(parts);
  const at = await compileTemplate(doc, fx.source(sourceBytes, { paragraphIndex: 2 }));
  const append = await compileTemplate(doc, fx.source(sourceBytes, { paragraphIndex: 100 }));
  const first = await compileTemplate(doc, fx.source(sourceBytes, { paragraphIndex: 0 }));
  inOrder(xml(at), "Before", "Nested", "Boundary", "Added section one", "Added section two", "Original");
  inOrder(xml(append), "Before", "Nested", "Boundary", "Original", "Added section one", "Added section two");
  inOrder(xml(first), "Added section one", "Added section two", "Before");
  assert.match(xml(at), /w:headerReference/);
  for (const name of [header, headerRels, "customXml/item1.xml", "word/theme/theme1.xml", "docProps/thumbnail.jpeg"]) {
    samePayload(at, name, parts.get(name));
  }
  assert.match(xml(at, "word/styles.xml"), /w:styleId="Normal"/);
  assert.match(xml(at, "docProps/core.xml"), /2026-11-21T00:00:00Z/);
  const merging = new Map(parts);
  modify(merging, word, (body) => body.replace("</w:sectPr></w:body>",
    '<w:pgMar w:left="500" w:right="801" w:top="802" w:bottom="803" w:header="804" w:footer="805" w:gutter="0"/></w:sectPr></w:body>'));
  modify(merging, "docProps/core.xml", (core) => core.replace(/<dc:subject[^>]*\/>|<dc:subject[^>]*>.*?<\/dc:subject>/,
    "<dc:subject>Retained subject</dc:subject>"));
  const merged = await compileTemplate(await fx.merge(), fx.source(zip(merging)));
  inOrder(xml(merged, header), "Original header", "Merged header");
  samePayload(merged, headerRels, merging.get(headerRels));
  assert.match(xml(merged), /<w:pgMar\b(?=[^>]*w:left="1440")(?=[^>]*w:right="801")/);
  assert.match(xml(merged, "word/styles.xml"), /w:styleId="Normal"[\s\S]*?<w:keepNext/);
  assert.match(xml(merged, "docProps/core.xml"), /Merged title/);
  assert.match(xml(merged, "docProps/core.xml"), /Retained subject/);
  if (editorPath) {
    const external = await readFile(editorPath);
    const original = unzip(external);
    const externalPlan = await compileTemplate(doc, fx.source(external));
    for (const name of ["word/header1.xml", "word/_rels/header1.xml.rels", "customXml/item1.xml"]) {
      samePayload(externalPlan, name, original.get(name));
    }
    const edited = await compileTemplate(doc, fx.source(external, fx.oracleEdits), fx.override);
    assert.match(xml(edited), />Patched</);
    assert.deepEqual(edited.toBytes(), doc.toDocxWithTemplate(fx.source(external, fx.oracleEdits), fx.override));
    if (oracleDirectory) {
      assert.deepEqual(enc(edited.toBytes()), await readFile(join(oracleDirectory, "edited.docx")));
    }
  }
  const signed = new Map(parts);
  signed.set("signing/root.sigs", enc(""));
  modify(signed, rootRels, (xml) => xml.replace("</Relationships>",
    `<Relationship Id="signature" Type="${rels}/digital-signature/origin" Target="signing/root.sigs"/></Relationships>`));
  modify(signed, types, (xml) => xml.replace("</Types>",
    '<Override PartName="/signing/root.sigs" ContentType="application/vnd.openxmlformats-package.digital-signature-origin"/></Types>'));
  await assert.rejects(compileTemplate(doc, fx.source(zip(signed))), rule("unsupported_profile", rootRels));
  const mandatory = fixture(para("Original"), {
    ns: `xmlns:mc="${mc}" xmlns:unknown="urn:rocx:must-understand" mc:MustUnderstand="unknown"`,
  });
  await assert.rejects(compileTemplate(doc, fx.source(zip(mandatory))), /unsupported mandatory namespace/);
  await assert.rejects(compileTemplate(doc, fx.source(sourceBytes, { paragraphIndex: -1 })), { path: "source.edits.paragraphIndex" });
});

test("nodeFileSourcesAndAtomicSave", async (t) => {
  const dir = await mkdtemp(join(tmpdir(), "rocx-template-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const path = join(dir, "template.docx"), target = join(dir, "out.docx"), image = join(dir, "image.png");
  const doc = await fx.report(() => {});
  const edits = { replacements: [["original", "replacement"]] };
  const file = fx.newTemplateFile(path, edits);
  await assert.rejects(() => doc.toDocxWithTemplate(file), { code: "ENOENT" });
  await assert.rejects(compileTemplate(doc, file), { path: "source.bytes" });
  assert.ok(await fx.openImage(image, false), "guarded image must not open nonexistent input");
  await assert.rejects(fx.openImage(image), (error) => error.cause?.code === "ENOENT");
  const sample = await readFile(imagePath);
  await writeFile(image, sample);
  const fromFileImage = await fx.openImage(image);
  const fromByteImage = await fx.openImage(sample);
  await rm(image);
  assert.deepEqual(fromFileImage.toDocx(), fromByteImage.toDocx());

  const a = zip(fixture(para("original")));
  const b = zip(fixture(para("second file")));
  const options = { ...fx.override, readLimits: fx.losslessBudgets };
  await writeFile(path, a);
  const result = await doc.toDocxWithTemplate(file, options);
  assert.deepEqual(result, doc.toDocxWithTemplate(fx.source(a, edits), options));
  const overBudget = { readLimits: { maxExpandedBytes: 1n << 64n } };
  await assert.rejects(doc.toDocxWithTemplate(file, overBudget), { path: "options.readLimits.maxExpandedBytes" });
  await writeFile(path, b);
  assert.deepEqual(await doc.toDocxWithTemplate(file), doc.toDocxWithTemplate(fx.source(b, edits)));
  await rename(path, target);
  await assert.rejects(() => doc.toDocxWithTemplate(file), { code: "ENOENT" });
  assert.deepEqual(await readFile(target), b);
  await writeFile(path, a);
  await fx.save(doc, path, { template: file, ...options });
  assert.deepEqual(await readFile(path), enc(result), "same input/output must encode before replacing");
  await fx.save(doc, target, { template: fx.source(a, edits), ...fx.override });
  assert.deepEqual(await readFile(target), enc(result));
  await fx.save(doc, target, { readLimits: fx.losslessBudgets });
  assert.deepEqual(await readFile(target), enc(doc.toDocx()));
  const previous = await readFile(target);
  for (const template of [undefined, file]) {
    await assert.rejects(fx.save(doc, target, { template, ...overBudget }), {
      path: "options.readLimits.maxExpandedBytes",
    });
  }
  await assert.rejects(fx.save(doc, target, fx.strict), rule("unsupported_profile"));
  await assert.rejects(fx.save(doc, target, { template: fx.source(enc("not a ZIP")) }));
  const forged = { toDocx: () => enc("not a DOCX"), toDocxWithTemplate: () => enc("not a DOCX") };
  await assert.rejects(fx.save(forged, target), { path: "document" });
  await assert.rejects(fx.save(forged, target, { template: fx.source(a) }), { path: "document" });
  assert.deepEqual(await readFile(target), previous);

  const directoryTarget = join(dir, "destination");
  await mkdir(directoryTarget);
  await writeFile(join(directoryTarget, "sentinel"), "keep");
  await assert.rejects(fx.save(doc, directoryTarget), { code: "EISDIR" });
  assert.equal(await readFile(join(directoryTarget, "sentinel"), "utf8"), "keep");
  assert.deepEqual((await readdir(dir)).sort(), ["destination", "out.docx", "template.docx"]);
  if (process.getuid?.() > 0) {
    await chmod(dir, 0o555);
    try {
      await assert.rejects(fx.save(doc, target), { code: "EACCES" });
      assert.deepEqual(await readFile(target), previous);
    } finally {
      await chmod(dir, 0o700);
    }
  }
});
