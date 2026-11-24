// Browser-safe: inspect the bytes produced by the installed consumer itself.
import { openDocx } from "rocx-docx";

function equal(actual, expected, label) {
  if (Array.isArray(expected)) {
    if (!Array.isArray(actual) || actual.length !== expected.length) {
      throw new Error(`${label}: expected ${expected.length} items, got ${actual?.length}`);
    }
    expected.forEach((value, i) => equal(actual[i], value, `${label}[${i}]`));
  } else if (actual !== expected) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

function one(items, predicate, label) {
  const matches = items.filter(predicate);
  equal(matches.length, 1, label);
  return matches[0];
}

const paragraph = (doc, prefix) =>
  one(doc.body(), p => p.kind === "paragraph" && p.text.startsWith(prefix), prefix);
const entry = (doc, kind, id) =>
  one(doc.entryStories(), e => e.kind === kind && e.id === id, `${kind} ${id}`);
const scope = (doc, kind, variant) =>
  doc.stories().filter(s => s.view.kind === kind && s.view.variant === variant);
const text = items => items.filter(i => i.kind === "text").map(i => i.text);

function reject(action, pattern) {
  try {
    action();
  } catch (error) {
    equal(pattern.test(String(error)), true, `rejection: ${String(error)}`);
    return;
  }
  throw new Error(`expected rejection matching ${pattern}`);
}

async function authored(doc) {
  equal(doc.coreTitle(), "All features visual inspection", "core title");
  equal(doc.customProperty("Published"), false, "custom boolean");
  const sections = doc.sections();
  equal(sections.map(s => s.sectionType), [undefined, "nextPage", "nextColumn", "continuous", "evenPage", "oddPage"],
    "section types");
  equal(sections.map((s, i) => s.bodyStart === (i ? sections[i - 1].bodyEnd : 0)), Array(6).fill(true),
    "contiguous section ranges");
  equal(sections.at(-1).bodyEnd, doc.body().length, "end of final section");
  const { pageWidth, columnCount, pageFormat } = sections[1].layout;
  equal([pageWidth, columnCount, pageFormat], [15840, 2, "lowerRoman"], "section layout");
  const indent = paragraph(doc, "First-line indent=");
  equal([indent.layout.firstLineIndent, indent.flags.widowControl, indent.flags.bidi], [360, false, false],
    "paragraph layout and explicit false");
  const toggles = paragraph(doc, "Builder bold unspecified").inlineStructure;
  equal(toggles.map(r => [r.kind, r.format.bold, r.format.italic]),
    [["run", undefined, undefined], ["run", true, undefined], ["run", false, true]], "run formatting");
  equal(paragraph(doc, "Before explicit page break").inlineItems[1].breakType, "page", "explicit page break");
  equal(paragraph(doc, "Tab left\t").inlineItems[1].kind, "tab", "explicit tab");
  const link = paragraph(doc, "Internal hyperlink:").inlineStructure[0];
  equal([link.kind, doc.inlineAt(link.anchor).bookmark], ["hyperlink", "CatalogBookmark"], "bookmark link");
  const style = one(doc.styles().styles, s => s.name === "Catalog Child", "catalog child style");
  equal([style.id, style.basedOn], ["CatalogChild", "CatalogBase"], "style inheritance");
  const [table, nested] = doc.body().filter(b => b.kind === "table");
  equal([table.layout, ...table.gridWidths], ["fixed", 1440, 1440, 1440], "table grid");
  equal(table.children.slice(0, 2).map(r => [r.children[0].gridSpan, r.children[0].verticalMerge]),
    [[2, "restart"], [2, "continue"]], "physical merges");
  equal(nested.children[0].children[0].blocks.some(b => b.kind === "opaque" && b.name === "w:tbl"),
    true, "opaque nested table");
  const nodes = paragraph(doc, "Current: ").inlineStructure;
  equal(nodes.slice(1).map(r => [r.revisionKind, r.author, r.date, ...text(doc.inlineAt(r.children[0].anchor).items)]),
    [["insertion", "Catalog editor", "2026-11-20T00:00:00Z", "inserted wording"],
      ["deletion", "Catalog editor", undefined, "old wording"]], "revision metadata and child anchors");
  const field = paragraph(doc, "Field simple PAGE:");
  equal([field.hasOpaqueContent, field.inlineStructure[1].kind, field.inlineStructure[1].name],
    [true, "opaque", "w:fldSimple"], "opaque field at structural baseline");
  const caption = one(doc.controls(), c => c.id === "5001", "locked caption");
  equal([caption.context, caption.tag, caption.lock, caption.bound],
    ["inline", "review-caption", "contentLocked", false], "control metadata");
  const resources = [{kind: "coreTitle", value: "Catalog retry"}];
  reject(() => doc.preview({resources, controls: [{target: caption.anchor, text: "forbidden"}]}), /locked/);
  equal(doc.coreTitle(), "All features visual inspection", "failed batch left original unchanged");
  const controls = [{target: caption.anchor, alias: "Allowed alias"}];
  const retry = await openDocx(doc.preview({resources, controls}).toDocx());
  equal(retry.coreTitle(), "Catalog retry", "valid retry title");
  equal(one(retry.controls(), c => c.id === "5001", "reopened caption").alias, "Allowed alias", "valid retry alias");
  reject(() => retry.preview({controls}), /snapshot/);
}

function checkStory(doc, kind, variant, local, drawings, expectedText) {
  const slots = scope(doc, kind, variant);
  equal(slots.length, 2, `${kind} ${variant} slot count`);
  const [first, second] = slots.map(s => s.view);
  equal([first.sectionIndex, second.sectionIndex, first.inherited, second.inherited, first.partName !== second.partName],
    [0, 1, false, !local, local], `${kind} ${variant} slot identity`);
  const flattened = [first, second].map(s => s.blocks.flatMap(b => b.inlineItems ?? []));
  equal(flattened.map(items => items.filter(i => i.kind === "opaque" && i.name === "w:drawing").length),
    drawings, `${kind} ${variant} image counts`);
  if (expectedText) {
    equal(flattened.map(items => text(items).includes(expectedText)), [true, true], "shared text visible in both slots");
  }
}

export async function verifyTypedCatalog(files) {
  const remaining = new Map(Object.entries(files));
  async function take(name) {
    const bytes = remaining.get(name);
    if (!remaining.delete(name)) throw new Error(`Missing catalog artifact: ${name}`);
    return openDocx(bytes);
  }
  const source = await take("catalog");
  await authored(source);
  equal((await take("catalog.template")).body()[0].text, "Template {{replace}}: cross-run replacement.", "template");
  const edited = await take("catalog.edited");
  const p = paragraph(edited, "Imported editor: this paragraph changes");
  const table = edited.body()[edited.body().findIndex(b => b.kind === "paragraph" && b.text === p.text) + 1];
  const label = table.children[0].children[0].blocks[0].inlineStructure[0];
  equal([p.alignment, table.kind, label.format.bold, ...text(label.items)],
    ["center", "table", true, "Imported editor table: bold label"], "edited alignment and inserted table");
  const changed = await take("catalog.run-text");
  const ranged = paragraph(changed, "Ranged 📝 + point editor:");
  equal([changed.coreSubject(), changed.defaultAlignment(), changed.metadataField("manager"), changed.customProperty("Reviewed")],
    ["Imported resource edits", "center", "Review lead", false], "resource edits");
  const a = source.documentDefaults(), b = changed.documentDefaults();
  equal([a.bold, b.bold, a.afterTwips, b.afterTwips, a.footnoteStart, b.footnoteStart],
    [undefined, false, 80, undefined, 3, 5], "default patches and clears");
  const reference = changed.paragraphNumbering(ranged.anchor);
  const instance = one(changed.numberingInstances(), n => n.numId === reference.id, "concrete numbering instance");
  equal([reference.id, reference.level, instance.abstractId, ...instance.replacementLevels],
    [9049, 0, 9048, 0], "numbering reference and replacement level");

  const linked = await take("catalog.entry-text");
  for (const [kind, id] of [["footnote", "0"], ["endnote", "0"], ["comment", "1"]]) {
    const block = entry(linked, kind, id).blocks[0];
    const link = block.inlineStructure.at(-1);
    equal([link.kind, Boolean(linked.inlineAt(link.anchor).relationshipId)], ["hyperlink", true], `${kind} link`);
    equal(text(block.inlineItems), [`Edited ${kind} entry`, "Review link"], `${kind} edited text`);
  }
  const lifecycle = await take("catalog.entry-edits");
  equal(entry(lifecycle, "comment", "1").initials, "ED", "edited comment initials");
  equal(entry(lifecycle, "footnote", "47").blocks.map(b => text(b.inlineItems)),
    [["Lifecycle note"], ["Second note paragraph"]], "created note");
  equal(paragraph(lifecycle, "Footnote reference;").inlineItems.filter(i => i.kind.endsWith("Reference")).map(i => [i.kind, i.id]),
    [["footnoteReference", "47"], ["footnoteReference", "0"], ["endnoteReference", "0"]], "note reference order");

  const headerSource = await take("catalog.header-source");
  checkStory(headerSource, "header", "default", false, [0, 0]);
  for (const [name, kind, variant, local, drawings, value] of [
    ["header-shared", "header", "default", false, [1, 1]],
    ["header-local", "header", "default", true, [0, 1]],
    ["header-first-shared", "header", "first", false, [1, 1]],
    ["header-first-local", "header", "first", true, [0, 1]],
    ["footer-even", "footer", "even", true, [0, 1], "Edited shared even footer"],
  ]) {
    checkStory(await take(`catalog.${name}`), kind, variant, local, drawings, value);
  }
  const storyBlocks = await take("catalog.story-blocks");
  const [first, second] = scope(storyBlocks, "header", "default");
  const [original] = scope(headerSource, "header", "default");
  equal([first.view.partName !== second.view.partName, second.view.partName, second.view.inherited],
    [true, original.view.partName, false], "story splice preserves next section's physical header");
  equal(first.view.blocks.filter(b => b.kind === "opaque").map(b => b.name), ["w:tbl"], "inserted opaque story table");
  const footer = one(scope(storyBlocks, "footer", "first"), s => s.view.sectionIndex === 1, "new first footer");
  for (const story of [first, footer]) {
    const [link] = story.inlineStructure(story.view.blocks.length - 1);
    equal([link.kind, Boolean(link.relationshipId)], ["hyperlink", true], "story link");
    equal(text(story.view.blocks.at(-1).inlineItems), ["Story block link"], "inserted story paragraph");
  }
  equal([...remaining.keys()], [], "catalog artifacts missing typed readback");
}
