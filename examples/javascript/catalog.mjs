import { openDocx, resolveComponent } from "rocx-docx";
import {
  HEADER_BATCH_LABEL, HEADER_INLINE_LABEL, HEADER_LINK_RUNS, STORY_VARIANTS,
  assert, assertBytes, authorCatalog, storyLabel,
} from "./catalog/authoring.mjs";
import { EDIT_LABEL, INLINE_LABEL, INLINE_LINK_RUNS, para, row, run, table, tableCell, text } from "./catalog/blocks.mjs";
import { hyperlink } from "./catalog/stories.mjs";

export const catalogNames = Object.freeze({
  final: "all_features-javascript.docx",
  template: "all_features-javascript.template.docx",
  edited: "all_features-javascript.edited.docx",
  headerSource: "all_features-javascript.header-source.docx",
  headerShared: "all_features-javascript.header-shared.docx",
  headerLocal: "all_features-javascript.header-local.docx",
  headerFirstShared: "all_features-javascript.header-first-shared.docx",
  headerFirstLocal: "all_features-javascript.header-first-local.docx",
  entryText: "all_features-javascript.entry-text.docx",
  entryEdits: "all_features-javascript.entry-edits.docx",
  storyBlocks: "all_features-javascript.story-blocks.docx",
  footerEven: "all_features-javascript.footer-even.docx",
  runText: "all_features-javascript.run-text.docx",
});
const EDIT_TABLE_LABEL = "Imported editor table: bold label";
const SECOND_CELL_RUNS = ["Fixed-width ", "second column"];

function checkText(items, expected) {
  assert(items.length === expected.length &&
    items.every((item, i) => item.kind === "text" && item.text === expected[i]), "typed text items differ");
}

function checkLink(document, structure, label, children) {
  const [first, link] = structure;
  assert(structure.length === 2 && first.kind === "run" && link.kind === "hyperlink", "expected a run and a hyperlink");
  checkText(first.items, [label]);
  assert(document.inlineAt(first.anchor).anchor === first.anchor, "run lookup must retain anchor identity");
  assert(document.inlineAt(link.anchor).anchor === link.anchor, "hyperlink lookup must retain anchor identity");
  assert(link.children.length === children.length, "expected separate hyperlink runs");
  for (const [i, child] of link.children.entries()) {
    assert(child.kind === "run", "expected a hyperlink run");
    const found = document.inlineAt(child.anchor);
    assert(found.kind === "run" && found.anchor === child.anchor, "child lookup must retain its run anchor");
    checkText(found.items, [children[i]]);
  }
  return link;
}

function previewTable(document, target, bodyIndex, table, headerImages) {
  const before = document.body();
  const preview = document.preview({
    alignments: [{ target: target.anchor, alignment: "center" }],
    tableInsertions: [{ target: target.anchor, table }],
    headerImages,
  });
  const changes = preview.changes();
  const edits = preview.insertions();
  assert(changes.length === 1 && changes[0].bodyIndex === bodyIndex &&
    changes[0].before === target.alignmentXml && changes[0].after === "center", "unexpected typed alignment change");
  assert(edits.length === 1 && edits[0].afterBodyIndex === bodyIndex &&
    edits[0].rows === 1 && edits[0].columns === 2, "unexpected typed table insertion");
  const after = document.body();
  assert(after.length === before.length && after[bodyIndex].anchor === target.anchor &&
    after[bodyIndex].alignmentXml === target.alignmentXml, "preview must not change source");
  return preview;
}

async function checkEdited(bytes, anchor, source) {
  const reopened = await openDocx(bytes);
  const body = reopened.body();
  assert(body[anchor].kind === "paragraph" && body[anchor].alignment === "center", "expected reopened center alignment");
  const inserted = body[anchor + 1];
  assert(inserted.kind === "table" && inserted.children.length === 1, "expected inserted physical table");
  const [firstRow] = inserted.children;
  assert(firstRow.kind === "row" && firstRow.children.length === 2, "expected two physical cells");
  const [firstCell, secondCell] = firstRow.children;
  assert(firstCell.kind === "cell" && firstCell.blocks[0].kind === "paragraph", "expected an inserted cell paragraph");
  checkText(firstCell.blocks[0].inlineItems, [EDIT_TABLE_LABEL]);
  assert(secondCell.kind === "cell" && secondCell.blocks[0].kind === "paragraph", "expected a second cell paragraph");
  checkText(secondCell.blocks[0].inlineItems, SECOND_CELL_RUNS);
  const [label] = firstCell.blocks[0].inlineStructure;
  const runs = secondCell.blocks[0].inlineStructure;
  assert(firstCell.blocks[0].inlineStructure.length === 1 && label.kind === "run" &&
    runs.length === SECOND_CELL_RUNS.length, "expected separate cell runs");
  for (const [i, node] of runs.entries()) {
    assert(node.kind === "run" && node.anchor !== label.anchor, "expected a distinct run in the second cell");
    const found = reopened.inlineAt(node.anchor);
    assert(found.kind === "run" && found.anchor === node.anchor, "expected cell run lookup by anchor");
    checkText(found.items, [SECOND_CELL_RUNS[i]]);
    assertThrows(() => source.inlineAt(node.anchor), "accepted a cell run anchor from another snapshot");
  }
  assert(runs[0].anchor !== runs[1].anchor, "cell runs must have distinct anchors");
}

function checkInheritedStories(document, defaultHeader) {
  const stories = document.stories();
  const slots = ["header", "footer"].flatMap((kind) => STORY_VARIANTS.map((variant) => [kind, variant]));
  assert(stories.length === 2 * slots.length, "expected all six story slots in each section");
  for (const [i, [kind, variant]] of slots.entries()) {
    const explicit = stories[i];
    const inherited = stories[i + slots.length];
    for (const [sectionIndex, story] of [explicit, inherited].entries()) {
      const view = story.view;
      assert(view.sectionIndex === sectionIndex && view.kind === kind && view.variant === variant &&
        view.inherited === (sectionIndex === 1), "unexpected effective story slot");
    }
    assert(inherited.view.partName === explicit.view.partName, "inherited slot must use the same physical part");
    const [first] = explicit.inlineStructure(1);
    const [second] = inherited.inlineStructure(1);
    assert(first.kind === "run" && second.kind === "run" && first.anchor === second.anchor,
      "story aliases must share run anchors");
    const found = document.inlineAt(second.anchor);
    assert(found.kind === "run" && found.anchor === first.anchor, "expected story run lookup by anchor");
    if (kind === "header" && variant === "default") {
      assert(found.anchor === defaultHeader.inlineStructure(1)[0].anchor,
        "default-header and story projections must share run anchors");
      checkText(found.items, [HEADER_INLINE_LABEL]);
    } else {
      checkText(found.items, [storyLabel(kind, variant)]);
    }
  }
}

function assertThrows(operation, message) {
  try {
    operation();
  } catch (error) {
    if (error instanceof Error) return;
    throw error;
  }
  throw new Error(`Catalog: ${message}`);
}

/**
 * Generate the catalog artifacts without host I/O assumptions.
 * `fixtures` maps fixture filenames to raw Uint8Array image bytes, or is an
 * async reader taking a fixture filename and returning the same bytes.
 * The optional `namedPng` declaration lets Node demonstrate lazy imageFile().
 * Returns Uint8Array values by their keys in catalogNames.
 */
export async function generateCatalog(fixtures, options) {
  const readFixture = typeof fixtures === "function" ? fixtures : (filename) => fixtures[filename];
  const { template, final, headerSource, image } = await authorCatalog(readFixture, options);
  const resolvedTable = await resolveComponent(table([
    row([
      tableCell([run(EDIT_TABLE_LABEL, { bold: true })]),
      tableCell([para(SECOND_CELL_RUNS.map((value) => text(value)))]),
    ]),
  ], { columnWidths: [2160, 2160], layout: "fixed" }));

  const imported = await openDocx(final);
  const styled = imported.preview({ styleEdits: [
    { op: "add", definition: { kind: "paragraph", name: "Catalog Imported Child", baseStyle: "Catalog Base" } },
    { op: "patch", target: imported.styleAnchor("CatalogBase"), italic: true },
    { op: "field", target: imported.styleAnchor("Regionfirst_row"), region: "firstRow", field: "b", value: null },
    { op: "region", target: imported.styleAnchor("Regionlast_row"), region: "lastRow", format: { italic: true } },
    { op: "add", definition: { kind: "numbering", name: "Catalog Imported List", numberingId: 60 } },
  ] });
  assert((await openDocx(styled.toDocx())).styles().styles.some(s =>
    s.id === "CatalogImportedChild" && s.basedOn === "CatalogBase"), "imported style edit readback");
  assert(imported.styles()?.styles.some((style) => style.name === "Catalog Bullets"
    && style.kind === "numbering" && style.id), "expected imported numbering style");
  const body = imported.body();
  const revisions = body.flatMap((block) => block.kind === "paragraph" ? block.inlineStructure : [])
    .filter((node) => node.kind === "revision" && node.author === "Catalog editor");
  for (const [kind, text] of [["insertion", "inserted wording"], ["deletion", "old wording"]]) {
    const revision = revisions.find((node) => node.revisionKind === kind && node.children[0]?.items[0]?.text === text);
    assert(revision && imported.inlineAt(revision.children[0].anchor).items[0].text === text, "missing typed revision");
  }
  const controls = body.filter((node) => node.kind === "contentControl" && node.tag === "review-block");
  assert(controls.length === 1 && controls[0].alias === "Review block" &&
    controls[0].lock === "sdtLocked", "incorrect block control metadata");
  const inline = body.find((node) => node.kind === "paragraph" && node.text === INLINE_LABEL + INLINE_LINK_RUNS.join(""));
  assert(inline, "expected labeled body inline structure");
  const entries = imported.entryStories();
  for (const kind of ["footnote", "endnote"]) {
    assert(entries.some((entry) => entry.kind === kind && entry.id === "0"), `missing ${kind}`);
  }
  const review = entries.find((e) => e.author === "Structured reviewer");
  assert(review?.kind === "comment" && review.initials === "SR", "missing structured review");
  const reviewRun = review.blocks[0].inlineStructure[0];
  assert(imported.inlineAt(reviewRun.anchor).items[0].text.startsWith("Structured comment:"), "incorrect comment run");
  let entryText = final;
  for (const kind of ["footnote", "endnote", "comment"]) {
    let source = await openDocx(entryText);
    const select = doc => doc.entryStories().find(e => e.kind === kind && (kind === "comment" ? e.author === "Structured reviewer" : e.id === "0"));
    let entry = select(source);
    const first = entry.blocks[0].inlineStructure.find(n => n.kind === "run" && n.items.length === 1 && n.items[0].kind === "text");
    const label = `Edited ${kind} entry`;
    source = await openDocx(source.editStoryText({ownerPart: entry.partName, runText: [{target: first.anchor, text: label}]}));
    entry = select(source);
    const run = entry.blocks[0].inlineStructure.find(n => n.kind === "run" && n.items[0]?.text === label);
    const inline = await resolveComponent(hyperlink({kind: "external", url: "https://example.com/entry-review"}, [text("Review link")]));
    const edit = {kind: "insertAfter", target: run.anchor, inline};
    const batch = {ownerPart: entry.partName, inlineEdits: [edit]};
    assertThrows(() => source.editEntryInlines({...batch, inlineEdits: [edit, edit]}), "accepted duplicate entry targets");
    entryText = source.editEntryInlines(batch);
    const reopened = await openDocx(entryText);
    assertThrows(() => reopened.editEntryInlines(batch), "accepted a stale entry anchor");
    assert(reopened.entryStories().find(e => e.kind === kind && e.id === entry.id)
      .blocks[0].inlineStructure.some(n => n.kind === "hyperlink" && n.children[0].items[0].text === "Review link"), "missing inserted link");
  }
  const ownerPart = entries.find(e => e.kind === "footnote").partName;
  const noteBody = doc => doc.body().find(p => p.kind === "paragraph" && p.text === "Footnote reference; endnote reference");
  const targetRun = noteBody(imported).inlineStructure[0].anchor;
  const create = {ownerPart, target: targetRun, kind: "create", id: 47, text: "Lifecycle note"};
  assertThrows(() => imported.editNoteReference({...create,id:0}), "accepted duplicate note ID");
  const created = await openDocx(imported.editNoteReference(create));
  assertThrows(() => created.editNoteReference(create), "accepted stale note anchor");
  const inserted = noteBody(created).inlineStructure[1].anchor;
  assert(created.inlineAt(inserted).items[0].kind === "footnoteReference" && created.inlineAt(inserted).items[0].id === "47", "incorrect note reference identity");
  const detached = await openDocx(created.editNoteReference({ownerPart,target:inserted,kind:"detach"}));
  const attached = await openDocx(detached.editNoteReference({ownerPart,target:noteBody(detached).inlineStructure[0].anchor,kind:"attach",id:47}));
  assert(noteBody(attached).inlineStructure[1].items[0].id === "47", "reattachment lost the note");
  const retargeted = await openDocx(detached.editNoteReference({ownerPart,target:noteBody(detached).inlineStructure[1].anchor,kind:"retarget",id:47}));
  assert(noteBody(retargeted).inlineStructure[1].items[0].id === "47" && retargeted.entryStories().some(e => e.partName === ownerPart && e.id === "0"), "retarget discarded the old definition");
  assertThrows(() => created.editEntry({ownerPart,id:"47",kind:"deleteUnusedNote"}), "removed a referenced note");
  const purged = await openDocx(detached.editEntry({ownerPart,id:"47",kind:"deleteUnusedNote"}));
  assert(!purged.entryStories().some(e => e.partName === ownerPart && e.id === "47"), "did not remove unused note");
  const renamed = await openDocx(created.editNoteReference({ownerPart,target:inserted,kind:"renumber",id:48}));
  assert(renamed.entryStories().some(e => e.partName === ownerPart && e.id === "48"), "missing renamed definition");
  const deleted = await openDocx(renamed.editNoteReference({ownerPart,target:noteBody(renamed).inlineStructure[1].anchor,kind:"delete"}));
  assert(!deleted.entryStories().some(e => e.partName === ownerPart && e.id === "48"), "definition was not deleted");
  const expandedNote = await openDocx(created.editEntry({ownerPart,id:"47",kind:"insertParagraph",index:1,value:"Second note paragraph"}));
  const entryEdits = expandedNote.editEntry({ownerPart:review.partName,id:review.id,kind:"initials",value:"ED"});
  const textEdit = { target: inline.inlineStructure[0].anchor, text: "Imported editor: replaced this run by its anchor. " };
  const insertedParagraph = await resolveComponent({ kind: "paragraph", text: "Imported editor: inserted paragraph." });
  const editTargets = body.filter((node) => node.kind === "paragraph" && node.text === EDIT_LABEL);
  assert(editTargets.length === 1 && editTargets[0].alignment === "left", "expected unique left-aligned target");
  const [target] = editTargets;
  checkText(target.inlineItems, [EDIT_LABEL]);
  assert(imported.documentDefaults().sizeHalfPoints === 22, "expected direct 11-point default");
  assertThrows(() => imported.preview({resources: [{kind:"documentDefault", field:"bold", value:1}]}), "accepted numeric default toggle");
  assert(imported.coreTitle() === "All features visual inspection" && imported.defaultAlignment() === undefined,
    "unexpected direct catalog defaults");
  assert(imported.abstractStarts().some(s => s.abstractId === 100 && s.level === 0 && s.start === 1),
    "missing shared abstract numbering level");
  const textPreview = imported.preview({
    styleEdits: [{op:"add", definition:{kind:"numbering", name:"Reviewed List", numberingId:9049}}],
    controls: [{ target: imported.controls().find((c) => c.context === "inline" && c.id === "5001").anchor,
      alias: "Imported editor caption" }],
    resources: [{kind: "coreTitle", value: "Edited resource sample"},
      {kind: "coreSubject", value: "Imported resource edits"},
      {kind:"documentDefault", field:"bold", value:false}, {kind:"documentDefault", field:"footnoteStart", value:5},
      {kind:"documentDefault", field:"afterTwips", value:null},
      {kind: "metadata", field: "creator", value: "Review team"},
      {kind: "metadata", field: "manager", value: "Review lead"},
      {kind: "customProperty", name: "Reviewed", value: false},
      {kind: "defaultAlignment", value: "center"}, {kind: "abstractStart", abstractId: 100, level: 0, value: 7},
      {kind: "addAbstract", abstractId: 9048, levels: [{level: 0, start: 2, format: "decimal", text: "%1."}]},
      {kind: "instanceLevel", numId: 9049, level: 0, value: {level: 0, start: 1, format: "lowerRoman", text: "(%1)"}},
      {kind: "addInstance", numId: 9049, abstractId: 9048},
      {kind: "instanceStart", numId: 9049, level: 0, value: 6},
      {kind: "paragraphNumbering", target: target.anchor, value: {id: 9049, level: 0}}],
    runText: [textEdit],
    textRanges: [{target: target.anchor, start: 0, end: 8, text: "Ranged 📝"}],
    textInsertions: [{target: target.anchor, position: 8, text: " + point"}],
    paragraphInsertions: [{ target: inline.anchor, paragraph: insertedParagraph }],
  });
  assert(textPreview.paragraphInsertions()[0] === body.indexOf(inline), "incorrect paragraph insertion");
  const change = textPreview.runTextChanges().find(c => c.before === INLINE_LABEL);
  assert(inline.inlineStructure[0].italic === undefined, "unexpected direct italic");
  const whole = imported.preview({inlineEdits: [
    {kind: "setToggle", target: textEdit.target, property: "italic", value: true},
    {kind: "patchFormat", target: textEdit.target, format: {size: 12.5}, clear: ["fontEastAsia"]},
    {kind: "insertAfter", target: inline.inlineStructure[1].anchor, inline: insertedParagraph.view.runs[0]},
  ]});
  assert(whole.inlineChanges().at(-1)[1] === 1, "incorrect whole-inline report");
  const [removed] = imported.preview({runRemovals: [textEdit.target]}).runRemovals();
  assert(removed.before === INLINE_LABEL, "incorrect removal report");
  assert(change.before === INLINE_LABEL && change.after === textEdit.text &&
    change.bodyIndex === body.indexOf(inline) && change.inlineIndex === 0, "incorrect text change");
  assertThrows(() => imported.preview({runText: [textEdit, textEdit]}), "accepted duplicate text target");
  const runText = textPreview.toDocx();
  const editedResources = await openDocx(runText);
  const defaults = editedResources.documentDefaults();
  assert(defaults.bold === false && defaults.footnoteStart === 5 && defaults.afterTwips === undefined, "incorrect direct defaults");
  assert(editedResources.metadataField("manager") === "Review lead" &&
    editedResources.customProperty("Reviewed") === false, "missing metadata values");
  assertThrows(() => imported.preview({resources: [{kind: "customProperty", name: "Reviewed"}]}),
    "accepted missing custom value");
  assertThrows(() => imported.preview({resources: [{kind: "metadata", field: "created", value: "yesterday"}]}),
    "accepted invalid core date");
  assert(editedResources.numberingInstances().some(n => n.numId === 9049 && n.abstractId === 9048 &&
    n.replacementLevels.includes(0) && n.startOverrides.some(o => o.level === 0 && o.start === 6)), "missing instance override");
  const listTarget = editedResources.body().find(p => p.kind === "paragraph" && p.text.startsWith("Ranged 📝"));
  assert(editedResources.paragraphNumbering(listTarget.anchor)?.id === 9049, "missing direct numbering reference");
  assertThrows(() => editedResources.paragraphNumbering(target.anchor), "accepted stale paragraph anchor");
  assertThrows(() => editedResources.preview({resources: [{kind: "removeInstance", numId: 9049}]}), "deleted a live instance");
  assert(editedResources.defaultAlignment() === "center" && editedResources.coreTitle() === "Edited resource sample" &&
    editedResources.coreSubject() === "Imported resource edits",
    "missing resource edits");
  const cleared = await openDocx(editedResources.preview({resources: [
    {kind: "metadata", field: "manager", value: null}, {kind: "customProperty", name: "Reviewed", value: null},
    {kind: "coreTitle", value: null}, {kind: "defaultAlignment", value: null},
    {kind: "coreSubject", value: null},
    {kind: "abstractStart", abstractId: 100, level: 0, value: null}]}).toDocx());
  assert(cleared.coreSubject() === undefined && cleared.coreTitle() === undefined && cleared.defaultAlignment() === undefined &&
    cleared.abstractStarts().some(s => s.abstractId === 100 && s.level === 0 && s.start === undefined),
    "clearing direct properties should return absence");
  assert(cleared.customProperty("Reviewed") === undefined && cleared.metadataField("manager") === undefined,
    "metadata clear retained the property");
  assertThrows(() => editedResources.preview({runText: [textEdit], resources: [
    {kind: "customProperty", name: "Reviewed", value: true}]}), "accepted stale anchor with metadata");
  assertThrows(() => imported.preview({resources: [{kind: "coreTitle", value: "uncommitted"},
    {kind: "abstractStart", abstractId: 9001, level: 0, value: 6}]}), "accepted missing numbering level");
  assertThrows(() => imported.preview({resources: [{kind: "coreTitle"}]}), "treated missing value as clear");
  const internalLink = checkLink(imported, inline.inlineStructure, INLINE_LABEL, INLINE_LINK_RUNS);
  assert(internalLink.bookmark === "CatalogBookmark" && internalLink.relationshipId === undefined, "expected bookmark hyperlink");
  const bodyIndex = body.indexOf(target);
  const preview = previewTable(imported, target, bodyIndex, resolvedTable);
  let edited = preview.toDocx();
  assertBytes(edited, preview.toDocx(), "retained body preview must encode identically");
  await checkEdited(edited, bodyIndex, imported);
  const reopenedForCell = await openDocx(edited);
  const insertedTable = reopenedForCell.body()[bodyIndex + 1];
  assert(insertedTable.kind === "table", "expected inserted table");
  // Layout sample: change the direct layout while preserving grid widths.
  assert(insertedTable.layout === "fixed" && insertedTable.gridWidths.join() === "2160,2160", "expected direct layout and widths");
  const layout = reopenedForCell.preview({ tableStructure: [{ target: insertedTable.anchor, kind: "layout", value: "autofit" }] });
  const adjusted = (await openDocx(layout.toDocx())).body()[bodyIndex + 1];
  assert(adjusted.layout === "autofit" && adjusted.gridWidths.join() === "2160,2160", "layout patch preserved grid");
  // Direct property sample: clear a single grid width.
  const clearedWidth = reopenedForCell.preview({tableStructure: [{target: insertedTable.anchor, kind:"patchProperty", field:"width", column:1, source:null}]});
  const partial = (await openDocx(clearedWidth.toDocx())).body()[bodyIndex + 1];
  assert(partial.gridWidths[1] === null && partial.properties.layout.type === "fixed", "cleared one width, retained layout");
  // Structural sample: append an empty row to this original table.
  const appended = reopenedForCell.preview({ tableStructure: [{ target: insertedTable.anchor, kind: "insertRow", index: 1 }] });
  const larger = (await openDocx(appended.toDocx())).body()[bodyIndex + 1];
  assert(larger.kind === "table" && larger.children.length === 2, "expected appended row");
  const cellPreview = reopenedForCell.preview({ cellAlignments: [
    { target: insertedTable.anchor, rowIndex: 0, cellIndex: 0, alignment: "center" },
  ] });
  assert(cellPreview.cellAlignmentChanges().length === 1, "expected cell alignment change");
  edited = cellPreview.toDocx();
  const otherSnapshot = await openDocx(final);
  assertThrows(() => otherSnapshot.preview({runText: [textEdit]}), "accepted a foreign run-text edit");
  assertThrows(() => imported.preview({runText: [{target: reviewRun.anchor, text: "Wrong story"}]}),
    "accepted a comment run as a body text target");
  assertThrows(() => otherSnapshot.inlineAt(internalLink.anchor), "accepted a foreign inline anchor");
  assertThrows(() => otherSnapshot.preview({ alignments: [{ target: target.anchor, alignment: "center" }] }),
    "accepted a foreign edit anchor");

  const headers = await openDocx(headerSource);
  // Tracked replacement and paired move are independent opt-in previews.
  const [from, after] = [0, 2].map(i => headers.body()[i].inlineStructure[0].anchor);
  const metadata = {author: "Reviewer", date: "2026-11-19T00:00:00Z"};
  const tracked = headers.previewTrackedText({target: from, kind: "replace", text: "Reviewed label", ...metadata});
  assert((await openDocx(tracked.toDocx())).body()[0].text === "", "tracked replacement projection");
  const moved = headers.previewTrackedMove({target: from, after, name: "CatalogMove", ...metadata});
  const pair = await openDocx(moved.toDocx());
  const restored = pair.previewRevision({target: pair.body()[0].inlineStructure[1].anchor,
    paired: pair.body()[2].inlineStructure[2].anchor, action: "reject"});
  assert((await openDocx(restored.toDocx())).body()[0].text === HEADER_BATCH_LABEL, "rejected move");
  const blockTarget = headers.body()[2].anchor;
  const directLayout = headers.previewLayout([{target: blockTarget, field: "spaceAfter", value: 180},
    {target: headers.sections()[1].anchor, field: "gridPitch", value: 360}]);
  assert((await openDocx(directLayout.toDocx())).sections()[1].layout.gridPitch === 360, "section grid pitch");
  const split = headers.preview({blockEdits:[{kind:"splitSection",target:headers.sections()[1].anchor,offset:1}]});
  const divided = await openDocx(split.toDocx());
  assert(divided.sections().length === 3, "split section");
  const merged = divided.preview({blockEdits:[{kind:"mergeSection",target:divided.sections()[1].anchor}]});
  assert((await openDocx(merged.toDocx())).sections().length === 2, "merged section");
  const blockParagraph = await resolveComponent({kind: "paragraph", text: "Block editor: replacement paragraph."});
  for (const edit of [
    {kind: "replace", paragraph: blockParagraph}, {kind: "remove"},
    {kind: "replaceSection", blocks: [blockParagraph]}, {kind: "keepWithNext", value: true},
    {kind: "paragraphFlag", flag: "keepTogether", value: true},
    {kind: "sectionFlag", flag: "rtlGutter", value: true}, {kind: "sectionType", value: "evenPage"},
  ]) {
    assert(headers.preview({blockEdits: [{target: blockTarget, ...edit}]}).blockChanges()[0] === 2,
      "incorrect block preview target");
  }
  const sections = headers.sections();
  assert(sections.length === 2 && sections[0].bodyStart === 0 && sections[0].bodyEnd === 2 &&
    sections[1].bodyStart === 2 && sections[1].bodyEnd === 3, "unexpected section body ranges");
  const [first, inherited] = headers.defaultHeaders();
  assert(inherited.view.inherited && inherited.view.sectionIndex === 1 && first.view.partName === inherited.view.partName,
    "second section should inherit the physical header");
  assert(first.view.anchor !== inherited.view.anchor, "header edit targets must be section-scoped");
  const [empty, paragraph] = inherited.view.blocks;
  assert(empty.kind === "paragraph" && inherited.inlineStructure(0).length === 0, "expected built-in empty header paragraph");
  assert(paragraph.kind === "paragraph", "expected header text paragraph");
  checkText(paragraph.inlineItems, [HEADER_INLINE_LABEL, ...HEADER_LINK_RUNS]);
  const headerLink = checkLink(headers, inherited.inlineStructure(1), HEADER_INLINE_LABEL, HEADER_LINK_RUNS);
  assert(headerLink.relationshipId && headerLink.anchor === first.inlineStructure(1)[1].anchor,
    "header aliases must share the physical hyperlink anchor");
  checkInheritedStories(headers, first);
  const rich = await resolveComponent(para([hyperlink({kind:"external",url:"https://example.com/story-block"},[text("Story block link")])]));
  const footerSlot = {sectionIndex:1, storyKind:"footer", variant:"first"};
  const createdFooter = await openDocx(headers.editStoryReference({...footerSlot,action:"create",blocks:[rich]}));
  const partName = createdFooter.stories().find(s => s.view.sectionIndex===1 && s.view.kind==="footer" && s.view.variant==="first").view.partName;
  const removedFooter = await openDocx(createdFooter.editStoryReference({...footerSlot,action:"remove"}));
  const linkedFooter = await openDocx(removedFooter.editStoryReference({...footerSlot,action:"link",partName}));
  const localBlocks = {target:linkedFooter.defaultHeaders()[0].view.anchor,mode:"sectionLocal",index:2,remove:0,blocks:[resolvedTable,rich]};
  assertThrows(() => headers.editStoryBlocks(localBlocks),"accepted a stale story anchor");
  const storyBlocks = linkedFooter.editStoryBlocks(localBlocks);
  const localHeader = (await openDocx(storyBlocks)).defaultHeaders();
  assert(localHeader[0].view.partName !== localHeader[1].view.partName && localHeader[1].view.partName === first.view.partName, "next section lost its header");
  const headerTarget = headers.body()[0];
  assert(headerTarget.kind === "paragraph" && headerTarget.text === HEADER_BATCH_LABEL, "expected header batch anchor");
  const resolvedImage = (await resolveComponent(image("png"))).view.image;
  const footer = headers.stories().at(-1);
  const edit = {target: footer.inlineStructure(1)[0].anchor, text: "Edited shared even footer"};
  const scope = {ownerPart: footer.view.partName, runText: [edit]};
  assertThrows(() => headers.editStoryText({...scope, runText: [edit, edit]}), "accepted a duplicate physical target");
  assertThrows(() => headers.editStoryText({...scope, ownerPart: first.view.partName}), "accepted a wrong owner");
  const changedFooter = await openDocx(headers.editStoryText(scope));
  assertThrows(() => changedFooter.editStoryText(scope), "accepted a foreign story target");
  const footerEven = changedFooter.previewHeaderImage({target: changedFooter.stories().at(-1).view.anchor,
    mode: "sectionLocal", image: resolvedImage}).toDocx();
  const firstPage = headers.stories().find(({view}) => view.sectionIndex === 1 && view.kind === "header" && view.variant === "first").view;
  const firstOutputs = [];
  for (const mode of ["shared", "sectionLocal"]) {
    const firstPreview = headers.previewHeaderImage({target: firstPage.anchor, mode, image: resolvedImage});
    const change = firstPreview.change();
    assert(JSON.stringify(change.affectedSections) === JSON.stringify(mode === "shared" ? [0, 1] : [1]),
      "unexpected first-page header scope");
    firstOutputs.push(firstPreview.toDocx());
  }

  async function headerEdit(mode, affectedSections) {
    const headerImages = [{ target: inherited.view.anchor, mode, image: resolvedImage }];
    const result = previewTable(headers, headerTarget, 0, resolvedTable, headerImages);
    const changes = result.headerChanges();
    const [change] = changes;
    assert(changes.length === 1 && change.mode === mode && change.sourcePart === first.view.partName &&
      (change.outputPart === change.sourcePart) === (mode === "shared") &&
      change.affectedSections.length === affectedSections.length &&
      change.affectedSections.every((section, i) => section === affectedSections[i]), "unexpected header scope/effects");
    const bytes = result.toDocx();
    assertBytes(bytes, result.toDocx(), "retained header transaction must encode identically");
    await checkEdited(bytes, 0, headers);
    return bytes;
  }
  const headerShared = await headerEdit("shared", [0, 1]);
  const headerLocal = await headerEdit("sectionLocal", [1]);
  const [headerFirstShared, headerFirstLocal] = firstOutputs;
  return { final, template, edited, headerSource, headerShared, headerLocal, headerFirstShared, headerFirstLocal, runText, footerEven, entryText, entryEdits, storyBlocks };
}
