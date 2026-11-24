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
from docx.enum.section import WD_HEADER_FOOTER, WD_SECTION_START
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
merged.paragraphs[0]._p.append(elem("fldSimple", instr=" PAGE "))
link(merged.paragraphs[0], "https://example.invalid/cell", "Cell", "Link")
merged.add_table(rows=1, cols=1).cell(0, 0).text = "Nested"
merged.add_paragraph("After nested")
table.cell(0, 2).text = "Tail"
row = table.rows[0]._tr
row.insert(row.index(row.tc_lst[-1]), elem("sdt"))
table._tbl.insert(table._tbl.index(row), elem("sdt"))
control = elem("sdt")
props = elem("sdtPr")
for name, value in [("tag", "customer & id"), ("alias", "Name"), ("lock", "futureLock")]:
    props.append(elem(name, val=value))
control.append(props)
control.append(elem("sdtContent"))
doc.element.body.insert(3, control)

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
initial = doc.sections[0]
initial._sectPr.add_headerReference(
    WD_HEADER_FOOTER.FIRST_PAGE, initial._sectPr.headerReference_lst[0].rId,
)
initial.even_page_header.paragraphs[0].text = "Even header"
initial.footer.paragraphs[0].text = "Footer "
link(initial.footer.paragraphs[0], "https://example.invalid/footer", "Same")
initial.footer.add_paragraph()
initial.footer._element.append(elem("sdt"))
initial.first_page_footer.paragraphs[0].text = "First footer"
initial._sectPr.add_footerReference(
    WD_HEADER_FOOTER.EVEN_PAGE, initial._sectPr.footerReference_lst[0].rId,
)
doc.sections[2].even_page_footer.is_linked_to_previous = False
doc.sections[2].even_page_footer.paragraphs[0].text = "Footer "
link(doc.sections[2].even_page_footer.paragraphs[0], "https://example.invalid/footer", "Same")
assert not initial.different_first_page_header_footer
assert not doc.settings.odd_and_even_pages_header_footer
base = save(doc)
entries = parts(base)
relationships = etree.fromstring(entries["word/_rels/document.xml.rels"])
relation, = [r for r in relationships if r.get("Target") == "https://example.invalid/body"]
assert relation.get("TargetMode") == "External"
cell_relation, = [r for r in relationships if r.get("Target") == "https://example.invalid/cell"]
header_rels = etree.fromstring(entries["word/_rels/header1.xml.rels"])
header_relation, = [r for r in header_rels if r.get("Target") == "https://example.invalid/header"]
assert header_relation.get("TargetMode") == "External"
footer_rels = etree.fromstring(entries["word/_rels/footer1.xml.rels"])
footer_relation, = [r for r in footer_rels if r.get("Target") == "https://example.invalid/footer"]
assert footer_relation.get("TargetMode") == "External"
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
broken_footer = mutate(base, "word/footer1.xml", lambda xml: xml.replace(b"w:ftr", b"w:hdr"))

revision_doc = Document()
revision_p = revision_doc.add_paragraph()
for kind, value in (("ins", "new"), ("del", "old")):
    revision = elem(kind, id="7", author="Ada")
    run = elem("r")
    text_node = elem("t" if kind == "ins" else "delText")
    text_node.text = value
    run.append(text_node)
    run.append(elem("drawing"))
    revision.append(run)
    revision_p._p.append(revision)

print(json.dumps({
    **{key: base64.b64encode(data).decode() for key, data in {
        "base": base, "implicit": implicit, "ambiguous": ambiguous, "strict": strict,
        "brokenHeader": broken_header, "brokenFooter": broken_footer,
        "revisions": save(revision_doc),
    }.items()},
    "bodyLink": relation.get("Id"),
    "cellLink": cell_relation.get("Id"),
    "headerLink": header_relation.get("Id"),
    "footerLink": footer_relation.get("Id"),
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
  assert.deepEqual(unsupported, { kind: "contentControl", tag: "customer & id", alias: "Name", lock: "futureLock" });
  const [merged, , tail] = table.children[1].children;
  const [spanRun, field, cellLink] = merged.blocks[0].inlineStructure;
  const [left, right] = cellLink.children;
  const [afterRun] = merged.blocks[3].inlineStructure;
  const [tailRun] = tail.blocks[0].inlineStructure;
  const run = (node, value) => ({ kind: "run", format: {}, hasOpaqueFormatting: false, anchor: node.anchor, items: [text(value)] });
  assert.deepEqual(table, {
    kind: "table", anchor: table.anchor,
    children: [
      opaque("w:sdt"),
      { kind: "row", children: [
        { kind: "cell", blocks: [
          {
            kind: "paragraph",
            inlineItems: [text("Span"), opaque("w:fldSimple"), text("Cell"), text("Link")],
            inlineStructure: [
              run(spanRun, "Span"), opaque("w:fldSimple"),
              { kind: "hyperlink", anchor: cellLink.anchor, relationshipId: fixtures.cellLink,
                children: [run(left, "Cell"), run(right, "Link")] },
            ],
          },
          opaque("w:tbl"),
          { kind: "paragraph", inlineItems: [], inlineStructure: [] },
          { kind: "paragraph", inlineItems: [text("After nested")],
            inlineStructure: [run(afterRun, "After nested")] },
        ] },
        opaque("w:sdt"),
        { kind: "cell", blocks: [
          { kind: "paragraph", inlineItems: [text("Tail")], inlineStructure: [run(tailRun, "Tail")] },
        ] },
      ] },
    ],
  });
  assert.equal(table.anchor, source.body()[2].anchor);
  assert.equal(first.anchor, source.body()[0].anchor);
  assert.ok(Object.isFrozen(merged.blocks));
  assert.ok(Object.isFrozen(merged.blocks[0].inlineStructure));
  assert.equal(field.anchor, undefined);
  for (const node of [spanRun, cellLink, left, right, afterRun, tailRun]) {
    assert.deepEqual(source.inlineAt(node.anchor), node);
    assert.equal(source.inlineAt(node.anchor).anchor, node.anchor);
  }
  assert.equal(new Set([spanRun, cellLink, left, right, afterRun, tailRun, first.inlineStructure[0]]
    .map((node) => node.anchor)).size, 7);
  assert.equal(source.body()[2].children[1].children[0].blocks[0].inlineStructure[2].anchor,
    cellLink.anchor);
  const reopened = await openDocx(input);
  assert.notEqual(table.anchor, reopened.body()[2].anchor);
  assert.throws(() => reopened.inlineAt(right.anchor), /different document snapshot/);
  assert.deepEqual((await openDocx(bytes("implicit"))).sections(), [
    { bodyStart: 0, bodyEnd: 1, flags: {}, sectionType: "continuous", sectionTypeXml: "continuous" },
    { bodyStart: 1, bodyEnd: 2, flags: {}, sectionTypeXml: "unknown" },
    { bodyStart: 2, bodyEnd: 3, flags: {} },
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
  assert.deepEqual(a, { kind: "run", format: {bold:true}, hasOpaqueFormatting: false, bold: true, anchor: a.anchor, items: [text("A")] });
  assert.deepEqual(b, { kind: "run", format: {}, hasOpaqueFormatting: false, anchor: b.anchor, items: [text("B")] });
  assert.deepEqual(controls.items, breaks);
  assert.deepEqual(field, opaque("w:fldSimple"));
  assert.deepEqual(external, {
    kind: "hyperlink", anchor: external.anchor, relationshipId: fixtures.bodyLink,
    bookmark: "Somewhere",
    children: [
      { kind: "run", format: {}, hasOpaqueFormatting: false, anchor: external.children[0].anchor, items: [text("L")] },
      { kind: "run", format: {}, hasOpaqueFormatting: false, anchor: external.children[1].anchor, items: [text("ink")] },
    ],
  });
  assert.deepEqual(internal, {
    kind: "hyperlink", anchor: internal.anchor, bookmark: "Local",
    children: [{ kind: "run", format: {}, hasOpaqueFormatting: false, anchor: internal.children[0].anchor, items: [text("Jump")] }],
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

async function importedSectionStoriesAndCrossViewIdentity() {
  const input = bytes("base");
  const source = await openDocx(input);
  // Issue a legacy anchor first, then read all stories without projecting any
  // story inlines. Rust's lookup can now prefer another allocation.
  const legacy = source.defaultHeaders()[0].inlineStructure(0);
  const stories = source.stories();
  assert.ok(Object.isFrozen(stories));
  assert.deepEqual(stories.map(({ view }) =>
    [view.sectionIndex, view.kind, view.variant, view.partName, view.inherited]),
  [
    [0, "header", "default", "word/header1.xml", false],
    [0, "header", "first",   "word/header1.xml", false],
    [0, "header", "even",    "word/header3.xml", false],
    [0, "footer", "default", "word/footer1.xml", false],
    [0, "footer", "first",   "word/footer2.xml", false],
    [0, "footer", "even",    "word/footer1.xml", false],
    [1, "header", "default", "word/header1.xml", true],
    [1, "header", "first",   "word/header1.xml", true],
    [1, "header", "even",    "word/header3.xml", true],
    [1, "footer", "default", "word/footer1.xml", true],
    [1, "footer", "first",   "word/footer2.xml", true],
    [1, "footer", "even",    "word/footer1.xml", true],
    [2, "header", "default", "word/header1.xml", false],
    [2, "header", "first",   "word/header1.xml", true],
    [2, "header", "even",    "word/header3.xml", true],
    [2, "footer", "default", "word/footer1.xml", true],
    [2, "footer", "first",   "word/footer2.xml", true],
    [2, "footer", "even",    "word/footer3.xml", false],
    [3, "header", "default", "word/header2.xml", false],
    [3, "header", "first",   "word/header1.xml", true],
    [3, "header", "even",    "word/header3.xml", true],
    [3, "footer", "default", "word/footer1.xml", true],
    [3, "footer", "first",   "word/footer2.xml", true],
    [3, "footer", "even",    "word/footer3.xml", true],
  ]);
  const [label, headerLink] = legacy;
  assert.equal(source.inlineAt(label.anchor).anchor, label.anchor);
  const lookedUpLink = source.inlineAt(headerLink.anchor);
  assert.equal(lookedUpLink.anchor, headerLink.anchor);
  assert.equal(lookedUpLink.children[0].anchor, headerLink.children[0].anchor);
  for (const alias of stories.filter((story) => story.view.partName === "word/header1.xml")) {
    assert.deepEqual(alias.inlineStructure(0), legacy);
    assert.equal(alias.inlineStructure(0)[0].anchor, label.anchor);
    assert.equal(alias.inlineStructure(0)[1].children[0].anchor, headerLink.children[0].anchor);
  }
  assert.notEqual(stories[18].inlineStructure(0)[0].anchor, label.anchor);

  const footer = stories[3];
  assert.deepEqual(footer.view.blocks, [
    { kind: "paragraph", inlineItems: [text("Footer "), text("Same")] },
    { kind: "paragraph", inlineItems: [] },
    opaque("w:sdt"),
  ]);
  assert.ok(Object.isFrozen(footer.view.blocks));
  assert.ok(footer.view.anchor);
  assert.notEqual(footer.view.anchor, stories[0].view.anchor);
  assert.deepEqual(footer.inlineStructure(1), []);
  for (const index of [2, 3, Number.MAX_SAFE_INTEGER]) {
    assert.equal(footer.inlineStructure(index), undefined);
  }
  assert.throws(() => footer.inlineStructure(-1), { path: "blockIndex" });
  assert.throws(() => footer.inlineStructure(0.5), { path: "blockIndex" });
  const [, footerLink] = footer.inlineStructure(0);
  assert.equal(footerLink.relationshipId, fixtures.footerLink);
  assert.deepEqual(footerLink.children[0].items, [text("Same")]);
  assert.equal(source.inlineAt(footerLink.anchor).anchor, footerLink.anchor);
  assert.equal(source.inlineAt(footerLink.children[0].anchor).anchor, footerLink.children[0].anchor);
  assert.equal(stories[11].inlineStructure(0)[1].anchor, footerLink.anchor);
  assert.equal(source.stories()[5].inlineStructure(0)[1].anchor, footerLink.anchor);
  assert.notEqual(stories[17].inlineStructure(0)[1].anchor, footerLink.anchor);
  assert.notEqual(footerLink.anchor, headerLink.anchor);

  const reverse = await openDocx(input);
  const reverseLink = reverse.stories()[0].inlineStructure(0)[1];
  assert.equal(reverse.defaultHeaders()[0].inlineStructure(0)[1].anchor, reverseLink.anchor);
  assert.throws(() => reverse.inlineAt(footerLink.anchor), /different document snapshot/);
  assert.deepEqual((await openDocx(bytes("implicit"))).stories(), []);
  const malformed = await openDocx(bytes("brokenFooter"));
  assert.throws(() => malformed.stories(), /invalid footer root/);
  const validHeader = malformed.defaultHeaders()[0].inlineStructure(0)[0];
  assert.equal(malformed.inlineAt(validHeader.anchor).anchor, validHeader.anchor);
}

await importedBodySectionsAndPhysicalCells();
await importedBodyInlineAnchorsAndProjections();
await importedDefaultHeaderAliasesAndLazyInspection();
await importedSectionStoriesAndCrossViewIdentity();
{
  const source = await openDocx(bytes("revisions"));
  const [insertion, deletion] = source.body()[0].inlineStructure;
  for (const [revision, kind, value] of [[insertion, "insertion", "new"], [deletion, "deletion", "old"]]) {
    assert.equal(revision.kind, "revision");
    assert.equal(revision.revisionKind, kind);
    assert.equal(revision.id, "7");
    assert.equal(revision.author, "Ada");
    assert.equal(revision.date, undefined);
    assert.deepEqual(revision.children[0].items, [text(value), opaque("w:drawing")]);
    assert.equal(source.inlineAt(revision.anchor).anchor, revision.anchor);
    assert.equal(source.inlineAt(revision.children[0].anchor).anchor, revision.children[0].anchor);
    const other = await openDocx(bytes("revisions"));
    assert.throws(() => other.inlineAt(revision.anchor), /different document snapshot/);
  }
}
console.log(`E01–E03 installed runtime: sections, structured cells, inline anchors, lazy header/footer variants and aliases (${process.version})`);

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
    const result = await page.evaluate(async ({ base, brokenHeader, revisions }) => {
      const { openDocx } = await import("/node_modules/rocx-docx/browser.js");
      const source = await openDocx(Uint8Array.fromBase64(base));
      const p = source.body()[0];
      const firstNode = p.inlineStructure[0];
      const [header, inherited] = source.defaultHeaders();
      const cellLink = source.body()[2].children[1].children[0].blocks[0].inlineStructure[2];
      const [defaultStory, firstStory, , footerStory, , evenFooter] = source.stories();
      const legacyAnchor = header.inlineStructure(0)[0].anchor;
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
      const revisionSource = await openDocx(Uint8Array.fromBase64(revisions));
      const revisionNodes = revisionSource.body()[0].inlineStructure;
      let headerError;
      try {
        malformed.defaultHeaders();
      } catch (error) {
        headerError = error.message;
      }
      return {
        body: p.text,
        control: source.body()[3],
        bodyIdentity: source.inlineAt(firstNode.anchor).anchor === firstNode.anchor,
        cellIdentity: source.inlineAt(cellLink.children[1].anchor).anchor === cellLink.children[1].anchor,
        headerIdentity: legacyAnchor === inherited.inlineStructure(0)[0].anchor
          && legacyAnchor === defaultStory.inlineStructure(0)[0].anchor
          && legacyAnchor === firstStory.inlineStructure(0)[0].anchor,
        footerIdentity: footerStory.view.kind === "footer" && evenFooter.view.variant === "even"
          && footerStory.inlineStructure(0)[1].anchor === evenFooter.inlineStructure(0)[1].anchor,
        malformedBody: malformed.body()[0].text,
        headerError,
        foreignError,
        revisions: revisionNodes.map((revision) => [
          revision.revisionKind, revision.author, revision.children[0].items[0].text,
          revisionSource.inlineAt(revision.children[0].anchor).anchor === revision.children[0].anchor,
        ]),
      };
    }, fixtures);
    assert.deepEqual({ ...result, headerError: undefined }, {
      body: "AB\t\n\nLinkJump", bodyIdentity: true, cellIdentity: true,
      control: { kind: "contentControl", tag: "customer & id", alias: "Name", lock: "futureLock" },
      headerIdentity: true, footerIdentity: true,
      malformedBody: "AB\t\n\nLinkJump", headerError: undefined, foreignError: "anchor",
      revisions: [["insertion", "Ada", "new", true], ["deletion", "Ada", "old", true]],
    });
    assert.match(result.headerError, /invalid header root/);
    assert.deepEqual(errors, []);
    console.log(`E01–E03 installed Chromium ${browser.version()}: structured cells, lazy header errors, physical story identity`);
  } finally {
    await browser?.close();
    await server.close();
  }
}
