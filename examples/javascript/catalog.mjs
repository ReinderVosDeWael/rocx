import { openDocx, resolveComponent } from "rocx-docx";
import {
  HEADER_BATCH_LABEL, HEADER_INLINE_LABEL, HEADER_LINK_RUNS, assert, assertBytes, authorCatalog,
} from "./catalog/authoring.mjs";
import { EDIT_LABEL, INLINE_LABEL, INLINE_LINK_RUNS, cell, row, run, table, tableCell } from "./catalog/blocks.mjs";

export const catalogNames = Object.freeze({
  final: "all_features-javascript.docx",
  template: "all_features-javascript.template.docx",
  edited: "all_features-javascript.edited.docx",
  headerSource: "all_features-javascript.header-source.docx",
  headerShared: "all_features-javascript.header-shared.docx",
  headerLocal: "all_features-javascript.header-local.docx",
});
const EDIT_TABLE_LABEL = "Imported editor table: bold label";

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

async function checkEdited(bytes, anchor) {
  const reopened = await openDocx(bytes);
  const body = reopened.body();
  assert(body[anchor].kind === "paragraph" && body[anchor].alignment === "center", "expected reopened center alignment");
  const inserted = body[anchor + 1];
  assert(inserted.kind === "table" && inserted.children.length === 1, "expected inserted physical table");
  const [firstRow] = inserted.children;
  assert(firstRow.kind === "row" && firstRow.children.length === 2, "expected two physical cells");
  const [firstCell] = firstRow.children;
  assert(firstCell.kind === "cell" && firstCell.blocks[0].kind === "paragraph", "expected an inserted cell paragraph");
  checkText(firstCell.blocks[0].inlineItems, [EDIT_TABLE_LABEL]);
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
 * Generate all six DOCX artifacts without host I/O assumptions.
 * `fixtures` maps fixture filenames to raw Uint8Array image bytes, or is an
 * async reader taking a fixture filename and returning the same bytes.
 * The optional `namedPng` declaration lets Node demonstrate lazy imageFile().
 * Returns the six Uint8Array values by their keys in catalogNames.
 */
export async function generateCatalog(fixtures, options) {
  const readFixture = typeof fixtures === "function" ? fixtures : (filename) => fixtures[filename];
  const { template, final, headerSource, image } = await authorCatalog(readFixture, options);
  const resolvedTable = await resolveComponent(table([
    row([tableCell([run(EDIT_TABLE_LABEL, { bold: true })]), cell("Fixed-width second column")]),
  ], { columnWidths: [2160, 2160], layout: "fixed" }));

  const imported = await openDocx(final);
  const body = imported.body();
  const inline = body.find((node) => node.kind === "paragraph" && node.text === INLINE_LABEL + INLINE_LINK_RUNS.join(""));
  assert(inline, "expected labeled body inline structure");
  const internalLink = checkLink(imported, inline.inlineStructure, INLINE_LABEL, INLINE_LINK_RUNS);
  assert(internalLink.bookmark === "CatalogBookmark" && internalLink.relationshipId === undefined, "expected bookmark hyperlink");
  const editTargets = body.filter((node) => node.kind === "paragraph" && node.text === EDIT_LABEL);
  assert(editTargets.length === 1 && editTargets[0].alignment === "left", "expected unique left-aligned target");
  const [target] = editTargets;
  checkText(target.inlineItems, [EDIT_LABEL]);
  const bodyIndex = body.indexOf(target);
  const preview = previewTable(imported, target, bodyIndex, resolvedTable);
  const edited = preview.toDocx();
  assertBytes(edited, preview.toDocx(), "retained body preview must encode identically");
  await checkEdited(edited, bodyIndex);
  const otherSnapshot = await openDocx(final);
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
  const headerTarget = headers.body()[0];
  assert(headerTarget.kind === "paragraph" && headerTarget.text === HEADER_BATCH_LABEL, "expected header batch anchor");
  const resolvedImage = (await resolveComponent(image("png"))).view.image;

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
    await checkEdited(bytes, 0);
    return bytes;
  }
  const headerShared = await headerEdit("shared", [0, 1]);
  const headerLocal = await headerEdit("sectionLocal", [1]);
  return { final, template, edited, headerSource, headerShared, headerLocal };
}
