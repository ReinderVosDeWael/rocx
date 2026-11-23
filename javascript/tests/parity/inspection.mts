import { openDocx } from "rocx-docx";
import type {
  BodyBlock, HeaderAnchor, ImportedCellBlock, ImportedDocument, ImportedHeader, ImportedHeaderBlock,
  ImportedHyperlink, ImportedInline, ImportedInlineNode, ImportedParagraph, ImportedRowChild,
  ImportedRun, ImportedSection, ImportedTable, ImportedTableCell, ImportedTableChild,
  ImportedTableRow, InlineAnchor, OpenDocxOptions, ParagraphAnchor, TableAnchor,
} from "rocx-docx";

function inline(item: ImportedInline): string {
  switch (item.kind) {
    case "text": return item.text;
    case "tab": return "\t";
    case "break": return item.breakType;
    case "opaque": return item.name;
    default: return item satisfies never;
  }
}

function run(node: ImportedRun): readonly string[] {
  return node.items.map(inline);
}

function link(node: ImportedHyperlink): readonly (string | undefined)[] {
  return [node.relationshipId, node.bookmark, ...node.children.map(structure).flat()];
}

function structure(node: ImportedInlineNode): readonly (string | undefined)[] {
  switch (node.kind) {
    case "run": return run(node);
    case "hyperlink": return link(node);
    case "opaque": return [node.name];
    default: return node satisfies never;
  }
}

function block(node: ImportedCellBlock | ImportedHeaderBlock): readonly string[] {
  switch (node.kind) {
    case "paragraph": return node.inlineItems.map(inline);
    case "opaque": return [node.name];
    default: return node satisfies never;
  }
}

const cell = (node: ImportedTableCell) => node.blocks.flatMap(block);
const rowChild = (node: ImportedRowChild) => node.kind === "cell" ? cell(node) : [node.name];
const row = (node: ImportedTableRow) => node.children.flatMap(rowChild);
const tableChild = (node: ImportedTableChild) => node.kind === "row" ? row(node) : [node.name];

function paragraph(p: ImportedParagraph, doc: ImportedDocument) {
  const anchor: ParagraphAnchor = p.anchor;
  const projection = p.inlineItems.map(inline);
  const nodes = p.inlineStructure.map((node) => {
    if (node.kind === "opaque") return structure(node);
    const target: InlineAnchor = node.anchor;
    return structure(doc.inlineAt(target));
  });
  // @ts-expect-error Paragraph and inline anchors are distinct nominal types.
  doc.inlineAt(anchor);
  // @ts-expect-error A structural object cannot implement an opaque anchor.
  doc.inlineAt({});
  // @ts-expect-error Views are readonly.
  p.text = "";
  return { text: p.text, projection, nodes, alignment: p.alignment, raw: p.alignmentXml, opaque: p.hasOpaqueContent };
}

function table(t: ImportedTable, doc: ImportedDocument) {
  const anchor: TableAnchor = t.anchor;
  // @ts-expect-error Table anchors are not inline addresses.
  doc.inlineAt(anchor);
  return t.children.flatMap(tableChild);
}

function bodyBlock(node: BodyBlock, doc: ImportedDocument) {
  switch (node.kind) {
    case "paragraph": return paragraph(node, doc);
    case "table": return table(node, doc);
    case "opaque": return node.name;
    default: return node satisfies never;
  }
}

export async function inspectInstalled(bytes: Uint8Array) {
  const options: OpenDocxOptions = {
    readLimits: { maxParts: 100, maxPartBytes: 1_000_000, maxExpandedBytes: 2_000_000 },
  };
  const doc: ImportedDocument = await openDocx(bytes, options);
  const sections: readonly ImportedSection[] = doc.sections();
  const headers: readonly ImportedHeader[] = doc.defaultHeaders();
  return {
    body: doc.body().map((node) => bodyBlock(node, doc)),
    sections: sections.map((s) => [s.bodyStart, s.bodyEnd, s.sectionType, s.sectionTypeXml]),
    headers: headers.map((header) => {
      const { anchor, sectionIndex, partName, inherited, blocks } = header.view;
      const target: HeaderAnchor = anchor;
      // @ts-expect-error Header anchors are not inline addresses.
      doc.inlineAt(target);
      // @ts-expect-error Header block arrays are readonly.
      blocks.push({ kind: "opaque", name: "w:unknown" });
      return {
        sectionIndex, partName, inherited, blocks: blocks.flatMap(block),
        inlines: header.inlineStructure(0)?.flatMap(structure),
      };
    }),
  };
}
