import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { openDocx } from "rocx-docx";

// Build independent inputs with python-docx. Inject unsupported nodes with
// OOXML and use ZIP parts to establish the expected raw relationship and limits.
const generated = spawnSync(process.env.PYTHON ?? "python3", ["-"], {
  encoding: "utf8", maxBuffer: 4 * 1024 * 1024,
  input: String.raw`
import base64, io, json, zipfile
from docx import Document
from docx.enum.section import WD_SECTION_START
from docx.enum.text import WD_ALIGN_PARAGRAPH, WD_BREAK
from docx.opc.constants import RELATIONSHIP_TYPE
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from lxml import etree

def elem(name, **attrs):
    el = OxmlElement("w:" + name)
    for key, value in attrs.items():
        el.set(qn("w:" + key), value)
    return el

def save(document):
    output = io.BytesIO()
    document.save(output)
    return output.getvalue()

def parts(data):
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        assert archive.testzip() is None
        return {name: archive.read(name) for name in archive.namelist()}

def mutate(data, part, replacement):
    entries = parts(data)
    with io.BytesIO() as output:
        with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as archive:
            for name, payload in entries.items():
                archive.writestr(name, replacement(payload) if name == part else payload)
        changed = output.getvalue()
    assert parts(changed) == {**entries, part: replacement(entries[part])}
    return changed

def link(paragraph, target, *texts, bookmark=None):
    node = elem("hyperlink")
    if target is not None:
        rid = paragraph.part.relate_to(target, RELATIONSHIP_TYPE.HYPERLINK, is_external=True)
        node.set(qn("r:id"), rid)
    if bookmark is not None:
        node.set(qn("w:anchor"), bookmark)
    for text in texts:
        node.append(paragraph.add_run(text)._r)
    paragraph._p.append(node)

doc = Document()
p = doc.add_paragraph()
p.alignment = WD_ALIGN_PARAGRAPH.RIGHT
p.add_run("A").bold = True
p.add_run("B")
r = p.add_run()
r.add_tab()
r._r.append(elem("cr"))
r.add_break(WD_BREAK.LINE)
r.add_break(WD_BREAK.PAGE)
r.add_break(WD_BREAK.COLUMN)
r._r.append(elem("noBreakHyphen"))
field = elem("fldSimple", instr=" PAGE ")
field.append(p.add_run("cached field")._r)
p._p.append(field)
link(p, "https://example.invalid/body", "L", "ink", bookmark="Somewhere")
link(p, None, "Jump", bookmark="Local")
assert [r.text for r in p.runs[:2]] == ["A", "B"]
assert [h.text for h in p.hyperlinks] == ["Link", "Jump"]

unknown = doc.add_paragraph("Unknown alignment")
unknown._p.get_or_add_pPr().append(elem("jc", val="start"))
table = doc.add_table(rows=1, cols=3)
merged = table.cell(0, 0).merge(table.cell(0, 1))
merged.text = "Span"
merged.add_table(rows=1, cols=1).cell(0, 0).text = "Nested"
table.cell(0, 2).text = "Tail"
row = table.rows[0]._tr
row.insert(row.index(row.tc_lst[-1]), elem("sdt"))
table._tbl.insert(table._tbl.index(row), elem("sdt"))
doc.element.body.insert(3, elem("sdt"))

first = doc.sections[0].header
first.paragraphs[0].text = "Header "
link(first.paragraphs[0], "https://example.invalid/header", "Same")
first.add_paragraph()
first._element.append(elem("sdt"))
doc.add_section(WD_SECTION_START.CONTINUOUS)
doc.add_section(WD_SECTION_START.NEW_COLUMN)
doc.add_section(WD_SECTION_START.NEW_PAGE)
doc.sections[3].header.is_linked_to_previous = False
other = doc.sections[3].header
other.paragraphs[0].text = "Header "
link(other.paragraphs[0], "https://example.invalid/header", "Same")
doc.sections[2]._sectPr.add_headerReference(
    doc.sections[0]._sectPr.headerReference_lst[0].type_,
    doc.sections[0]._sectPr.headerReference_lst[0].rId,
)
base = save(doc)
entries = parts(base)
relationships = etree.fromstring(entries["word/_rels/document.xml.rels"])
relation, = [r for r in relationships if r.get("Target") == "https://example.invalid/body"]
assert relation.get("TargetMode") == "External"
header_rels = etree.fromstring(entries["word/_rels/header1.xml.rels"])
header_relation, = [r for r in header_rels if r.get("Target") == "https://example.invalid/header"]
assert header_relation.get("TargetMode") == "External"
assert b"cached field" in entries["word/document.xml"]
assert b'w:gridSpan w:val="2"' in entries["word/document.xml"]

sections = Document()
sections.add_paragraph("First")
sections.add_paragraph("Second")
sections.add_table(rows=1, cols=1)
for paragraph, value in zip(sections.paragraphs, ("continuous", "unknown")):
    props = elem("sectPr")
    props.append(elem("type", val=value))
    paragraph._p.get_or_add_pPr().append(props)
sections.element.body.remove(sections.element.body.sectPr)
implicit = save(sections)
main = "word/document.xml"
ambiguous = mutate(implicit, main, lambda xml: xml.replace(
    b'<w:type w:val="continuous"/>',
    b'<w:type w:val="continuous"/><w:type w:val="nextPage"/>',
))
strict = mutate(base, main, lambda xml: xml.replace(
    b"http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    b"http://purl.oclc.org/ooxml/wordprocessingml/main",
))
broken_header = mutate(base, "word/header1.xml", lambda xml: xml.replace(b"w:hdr", b"w:ftr"))

print(json.dumps({
    **{key: base64.b64encode(data).decode() for key, data in {
        "base": base, "implicit": implicit, "ambiguous": ambiguous, "strict": strict,
        "brokenHeader": broken_header,
    }.items()},
    "bodyLink": relation.get("Id"),
    "headerLink": header_relation.get("Id"),
    "limits": {
        "maxParts": len(entries),
        "maxPartBytes": max(map(len, entries.values())),
        "maxExpandedBytes": sum(map(len, entries.values())),
    },
}))
`,
});
if (generated.error) throw generated.error;
assert.equal(generated.status, 0, generated.stderr);
const fixtures = JSON.parse(generated.stdout);
const bytes = (key) => Buffer.from(fixtures[key], "base64");
const text = (value) => ({ kind: "text", text: value });
const opaque = (name) => ({ kind: "opaque", name });
const br = (breakType) => ({ kind: "break", breakType });

async function importedBodySectionsAndPhysicalCells() {
  const input = bytes("base");
  const padded = new Uint8Array(input.length + 8);
  padded.set(input, 4);
  const source = await openDocx(padded.subarray(4, -4), { readLimits: fixtures.limits });
  padded.fill(0);
  const body = source.body();
  const [first, unknown, table, unsupported] = body;
  assert.equal(first.alignment, "right");
  assert.equal(first.alignmentXml, "right");
  assert.equal(unknown.alignment, undefined);
  assert.equal(unknown.alignmentXml, "start");
  assert.equal(unknown.hasOpaqueContent, false);
  assert.deepEqual(unsupported, opaque("w:sdt"));
  assert.deepEqual(table, {
    kind: "table", anchor: table.anchor,
    children: [
      opaque("w:sdt"),
      { kind: "row", children: [
        { kind: "cell", blocks: [
          { kind: "paragraph", inlineItems: [text("Span")] },
          opaque("w:tbl"),
          { kind: "paragraph", inlineItems: [] },
        ] },
        opaque("w:sdt"),
        { kind: "cell", blocks: [{ kind: "paragraph", inlineItems: [text("Tail")] }] },
      ] },
    ],
  });
  assert.equal(table.anchor, source.body()[2].anchor);
  assert.equal(first.anchor, source.body()[0].anchor);
  assert.ok(Object.isFrozen(table.children[1].children[0].blocks));
  assert.notEqual(table.anchor, (await openDocx(input)).body()[2].anchor);
  assert.deepEqual((await openDocx(bytes("implicit"))).sections(), [
    { bodyStart: 0, bodyEnd: 1, sectionType: "continuous", sectionTypeXml: "continuous" },
    { bodyStart: 1, bodyEnd: 2, sectionTypeXml: "unknown" },
    { bodyStart: 2, bodyEnd: 3 },
  ]);
  const ambiguous = await openDocx(bytes("ambiguous"));
  assert.throws(() => ambiguous.sections(), { rule: "unsupported_edit" });
  for (const [limit, boundary] of Object.entries(fixtures.limits)) {
    await assert.rejects(openDocx(input, { readLimits: { [limit]: boundary - 1 } }), Error, limit);
  }
  await assert.rejects(openDocx(input, { readLimits: { maxParts: -1 } }), {
    path: "options.readLimits.maxParts",
  });
  await assert.rejects(openDocx(new Uint16Array([0x4b50])), { path: "bytes" });
  await assert.rejects(openDocx(bytes("strict")), Error);
  await assert.rejects(openDocx(new Uint8Array([0x50, 0x4b])), Error);
}

async function importedBodyInlineAnchorsAndProjections() {
  const input = bytes("base");
  const source = await openDocx(input);
  const p = source.body()[0];
  assert.equal(p.text, "AB\t\n\nLinkJump");
  assert.equal(p.hasOpaqueContent, true);
  const breaks = [{ kind: "tab" }, br("line"), br("textWrapping"), br("page"), br("column"),
    opaque("w:noBreakHyphen")];
  assert.deepEqual(p.inlineItems, [
    text("A"), text("B"), ...breaks, opaque("w:fldSimple"), text("L"), text("ink"), text("Jump"),
  ]);
  const [a, b, controls, field, external, internal] = p.inlineStructure;
  assert.deepEqual(a, { kind: "run", anchor: a.anchor, items: [text("A")] });
  assert.deepEqual(b, { kind: "run", anchor: b.anchor, items: [text("B")] });
  assert.deepEqual(controls.items, breaks);
  assert.deepEqual(field, opaque("w:fldSimple"));
  assert.deepEqual(external, {
    kind: "hyperlink", anchor: external.anchor, relationshipId: fixtures.bodyLink,
    bookmark: "Somewhere",
    children: [
      { kind: "run", anchor: external.children[0].anchor, items: [text("L")] },
      { kind: "run", anchor: external.children[1].anchor, items: [text("ink")] },
    ],
  });
  assert.deepEqual(internal, {
    kind: "hyperlink", anchor: internal.anchor, bookmark: "Local",
    children: [{ kind: "run", anchor: internal.children[0].anchor, items: [text("Jump")] }],
  });
  const run = external.children[1];
  for (const node of [a, b, controls, external, internal, ...external.children, ...internal.children]) {
    assert.deepEqual(source.inlineAt(node.anchor), node);
    assert.equal(source.inlineAt(node.anchor).anchor, node.anchor);
  }
  assert.notEqual(a.anchor, b.anchor);
  assert.notEqual(external.anchor, run.anchor);
  assert.equal(source.body()[0].inlineStructure[4].children[1].anchor, run.anchor);
  assert.deepEqual(Reflect.ownKeys(run.anchor), []);
  assert.ok(Object.isFrozen(run.anchor));
  assert.ok(Object.isFrozen(run.items[0]));
  for (const invalid of [p.anchor, source.body()[2].anchor, {}, { ...run.anchor },
    Object.create(run.anchor), structuredClone(run.anchor)]) {
    assert.throws(() => source.inlineAt(invalid), { path: "anchor" });
  }
  const other = await openDocx(input);
  assert.notEqual(p.anchor, other.body()[0].anchor);
  assert.throws(() => other.inlineAt(run.anchor), /different document snapshot/);
  const broken = await openDocx(bytes("brokenHeader"));
  const brokenNode = broken.body()[0].inlineStructure[0];
  assert.deepEqual(broken.inlineAt(brokenNode.anchor), brokenNode);
  assert.throws(() => broken.defaultHeaders(), /invalid header root/);
  assert.deepEqual(broken.inlineAt(brokenNode.anchor), brokenNode);
}

async function importedDefaultHeaderAliasesAndLazyInspection() {
  const source = await openDocx(bytes("base"));
  const [first, inherited, explicit, distinct] = source.defaultHeaders();
  assert.equal(source.defaultHeaders().length, 4);
  assert.deepEqual(first.view, {
    anchor: first.view.anchor, sectionIndex: 0, partName: "word/header1.xml", inherited: false,
    blocks: [
      { kind: "paragraph", inlineItems: [text("Header "), text("Same")] },
      { kind: "paragraph", inlineItems: [] },
      opaque("w:sdt"),
    ],
  });
  assert.deepEqual([inherited, explicit, distinct].map(({ view }) =>
    [view.sectionIndex, view.partName, view.inherited]), [
    [1, first.view.partName, true], [2, first.view.partName, false], [3, "word/header2.xml", false],
  ]);
  assert.ok(Object.isFrozen(first.view.blocks));
  assert.equal(new Set([first, inherited, explicit, distinct].map((h) => h.view.anchor)).size, 4);
  assert.equal(source.defaultHeaders()[1].view.anchor, inherited.view.anchor);
  const [label, link] = first.inlineStructure(0);
  assert.equal(link.relationshipId, fixtures.headerLink);
  assert.equal(link.bookmark, undefined);
  for (const alias of [inherited, explicit, source.defaultHeaders()[0]]) {
    const [aliasLabel, aliasLink] = alias.inlineStructure(0);
    assert.deepEqual([aliasLabel, aliasLink], [label, link]);
    assert.equal(aliasLabel.anchor, label.anchor);
    assert.equal(aliasLink.anchor, link.anchor);
    assert.equal(aliasLink.children[0].anchor, link.children[0].anchor);
  }
  assert.notEqual(distinct.inlineStructure(0)[0].anchor, label.anchor);
  assert.notEqual(source.body()[0].inlineStructure[0].anchor, label.anchor);
  assert.deepEqual(source.inlineAt(link.anchor), link);
  assert.equal(source.inlineAt(link.children[0].anchor).anchor, link.children[0].anchor);
  assert.deepEqual(first.inlineStructure(1), []);
  for (const index of [2, 3, Number.MAX_SAFE_INTEGER]) {
    assert.equal(first.inlineStructure(index), undefined);
  }
  assert.throws(() => first.inlineStructure(-1), { path: "blockIndex" });
  assert.throws(() => first.inlineStructure(0.5), { path: "blockIndex" });
  assert.throws(() => source.inlineAt(first.view.anchor), { path: "anchor" });
  const other = await openDocx(bytes("base"));
  assert.throws(() => other.inlineAt(link.anchor), /different document snapshot/);
  assert.deepEqual((await openDocx(bytes("implicit"))).defaultHeaders(), []);
}

await importedBodySectionsAndPhysicalCells();
await importedBodyInlineAnchorsAndProjections();
await importedDefaultHeaderAliasesAndLazyInspection();
console.log(`E01–E03 installed runtime: sections/physical cells, inline lookup/anchors, lazy header aliases (${process.version})`);

if (process.argv.includes("--browser")) {
  const { chromium } = await import("playwright");
  const { createServer } = await import("vite");
  const host = "127.0.0.1";
  const server = await createServer({ configFile: false, server: { host } });
  let browser;
  try {
    await server.listen(0);
    browser = await chromium.launch();
    const page = await browser.newPage();
    const errors = [];
    page.on("pageerror", (error) => errors.push(error));
    await page.goto(`http://${host}:${server.httpServer.address().port}/`);
    const result = await page.evaluate(async ({ base, brokenHeader }) => {
      const { openDocx } = await import("/node_modules/rocx-docx/browser.js");
      const source = await openDocx(Uint8Array.fromBase64(base));
      const p = source.body()[0];
      const firstNode = p.inlineStructure[0];
      const [header, inherited] = source.defaultHeaders();
      // A separate same-origin realm is still a separate Wasm instance. Pass
      // the original object, not a serialized imitation of an inline anchor.
      const iframe = document.createElement("iframe");
      const foreign = new Promise((resolve) => {
        iframe.onload = () => resolve(iframe.contentWindow.openInspectionSource);
      });
      iframe.srcdoc = `<script type="module">
        import { openDocx } from "/node_modules/rocx-docx/browser.js";
        window.openInspectionSource = openDocx;
      </script>`;
      document.body.append(iframe);
      const foreignSource = await (await foreign)(Uint8Array.fromBase64(base));
      let foreignError;
      try {
        source.inlineAt(foreignSource.body()[0].inlineStructure[0].anchor);
      } catch (error) {
        foreignError = error.path;
      } finally {
        iframe.remove();
      }
      const malformed = await openDocx(Uint8Array.fromBase64(brokenHeader));
      let headerError;
      try {
        malformed.defaultHeaders();
      } catch (error) {
        headerError = error.message;
      }
      return {
        body: p.text,
        bodyIdentity: source.inlineAt(firstNode.anchor).anchor === firstNode.anchor,
        headerIdentity: header.inlineStructure(0)[0].anchor === inherited.inlineStructure(0)[0].anchor,
        malformedBody: malformed.body()[0].text,
        headerError,
        foreignError,
      };
    }, fixtures);
    assert.deepEqual({ ...result, headerError: undefined }, {
      body: "AB\t\n\nLinkJump", bodyIdentity: true, headerIdentity: true,
      malformedBody: "AB\t\n\nLinkJump", headerError: undefined, foreignError: "anchor",
    });
    assert.match(result.headerError, /invalid header root/);
    assert.deepEqual(errors, []);
    console.log(`E01–E03 installed Chromium ${browser.version()}: synchronous DTOs, lazy header errors, physical inline identity`);
  } finally {
    await browser?.close();
    await server.close();
  }
}
