import { openDocx, resolveComponent } from "rocx-docx";
import {
  HEADER_BATCH_LABEL, HEADER_INLINE_LABEL, HEADER_LINK_RUNS, STORY_VARIANTS,
  assert, assertBytes, authorCatalog, storyLabel,
} from "./catalog/authoring.mjs";
import { EDIT_LABEL, INLINE_LABEL, INLINE_LINK_RUNS, para, row, run, table, tableCell, text } from "./catalog/blocks.mjs";

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
    const source = await openDocx(entryText);
    const entry = source.entryStories().find(e => e.kind === kind && (kind === "comment" ? e.author === "Structured reviewer" : e.id === "0"));
    const run = entry.blocks[0].inlineStructure.find(n => n.kind === "run" && n.items.length === 1 && n.items[0].kind === "text");
    entryText = source.editStoryText({ownerPart: entry.partName, runText: [{target: run.anchor, text: `Edited ${kind} entry`}]});
  }
  const textEdit = { target: inline.inlineStructure[0].anchor, text: "Imported editor: replaced this run by its anchor. " };
  const insertedParagraph = await resolveComponent({ kind: "paragraph", text: "Imported editor: inserted paragraph." });
  const textPreview = imported.preview({
    runText: [textEdit],
    paragraphInsertions: [{ target: inline.anchor, paragraph: insertedParagraph }],
  });
  assert(textPreview.paragraphInsertions()[0] === body.indexOf(inline), "incorrect paragraph insertion");
  const [change] = textPreview.runTextChanges();
  const [removed] = imported.preview({runRemovals: [textEdit.target]}).runRemovals();
  assert(removed.before === INLINE_LABEL, "incorrect removal report");
  assert(change.before === INLINE_LABEL && change.after === textEdit.text &&
    change.bodyIndex === body.indexOf(inline) && change.inlineIndex === 0, "incorrect text change");
  assertThrows(() => imported.preview({runText: [textEdit, textEdit]}), "accepted duplicate text target");
  const runText = textPreview.toDocx();
  const internalLink = checkLink(imported, inline.inlineStructure, INLINE_LABEL, INLINE_LINK_RUNS);
  assert(internalLink.bookmark === "CatalogBookmark" && internalLink.relationshipId === undefined, "expected bookmark hyperlink");
  const editTargets = body.filter((node) => node.kind === "paragraph" && node.text === EDIT_LABEL);
  assert(editTargets.length === 1 && editTargets[0].alignment === "left", "expected unique left-aligned target");
  const [target] = editTargets;
  checkText(target.inlineItems, [EDIT_LABEL]);
  const bodyIndex = body.indexOf(target);
  const preview = previewTable(imported, target, bodyIndex, resolvedTable);
  let edited = preview.toDocx();
  assertBytes(edited, preview.toDocx(), "retained body preview must encode identically");
  await checkEdited(edited, bodyIndex, imported);
  const reopenedForCell = await openDocx(edited);
  const insertedTable = reopenedForCell.body()[bodyIndex + 1];
  assert(insertedTable.kind === "table", "expected inserted table");
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
  return { final, template, edited, headerSource, headerShared, headerLocal, headerFirstShared, headerFirstLocal, runText, footerEven, entryText };
}
