import { openDocx } from "rocx-docx";
import type {
  BodyBlock, HeaderAnchor, ImportedCellBlock, ImportedCellParagraph, ImportedDocument, ImportedHeader,
  ImportedHyperlink, ImportedInline, ImportedInlineNode, ImportedParagraph, ImportedRowChild,
  ImportedRun, ImportedSection, ImportedStory, ImportedStoryBlock, ImportedStoryVariant,
  ImportedStoryView, ImportedTable, ImportedTableCell, ImportedTableChild, ImportedTableRow,
  InlineAnchor, OpenDocxOptions, ParagraphAnchor, SectionStoryKind, TableAnchor,
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
    case "revision": return [node.revisionKind, node.id, node.author, node.date, ...node.children.map(structure).flat()];
    case "opaque": return [node.name];
    default: return node satisfies never;
  }
}

function block(node: ImportedStoryBlock): readonly string[] {
  switch (node.kind) {
    case "paragraph": return node.inlineItems.map(inline);
    case "opaque": return [node.name];
    default: return node satisfies never;
  }
}

function cellBlock(node: ImportedCellBlock, doc: ImportedDocument): readonly (string | undefined)[] {
  if (node.kind === "opaque") return [node.name];
  const paragraph: ImportedCellParagraph = node;
  // @ts-expect-error Cell paragraphs are not edit/lookup targets.
  doc.inlineAt(paragraph.anchor);
  // @ts-expect-error Cell paragraph structure is readonly.
  paragraph.inlineStructure.push({ kind: "opaque", name: "w:unknown" });
  return [
    ...paragraph.inlineItems.map(inline),
    ...paragraph.inlineStructure.flatMap((item) =>
      item.kind === "opaque" ? structure(item) : structure(doc.inlineAt(item.anchor))),
  ];
}

const cell = (node: ImportedTableCell, doc: ImportedDocument) =>
  node.blocks.flatMap((block) => cellBlock(block, doc));
const rowChild = (node: ImportedRowChild, doc: ImportedDocument) =>
  node.kind === "cell" ? cell(node, doc) : [node.name];
const row = (node: ImportedTableRow, doc: ImportedDocument) =>
  node.children.flatMap((child) => rowChild(child, doc));
const tableChild = (node: ImportedTableChild, doc: ImportedDocument) =>
  node.kind === "row" ? row(node, doc) : [node.name];

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
  return t.children.flatMap((child) => tableChild(child, doc));
}

function bodyBlock(node: BodyBlock, doc: ImportedDocument) {
  switch (node.kind) {
    case "paragraph": return paragraph(node, doc);
    case "table": return table(node, doc);
    case "opaque": return node.name;
    default: return node satisfies never;
  }
}

function sectionStory(story: ImportedStory, doc: ImportedDocument) {
  const view: ImportedStoryView = story.view;
  const { kind, variant, partName, sectionIndex, inherited, blocks } = view;
  const kindLabel: Record<SectionStoryKind, string> = { header: "Header", footer: "Footer" };
  const variantLabel: Record<ImportedStoryVariant, string> = {
    default: "Default", first: "First", even: "Even",
  };
  // @ts-expect-error Story slots do not have default-header edit anchors.
  const editTarget: HeaderAnchor = view.anchor;
  // @ts-expect-error A block index is numeric.
  story.inlineStructure("1");
  return {
    kind: kindLabel[kind], variant: variantLabel[variant], partName, sectionIndex, inherited,
    blocks: blocks.flatMap(block),
    inlines: story.inlineStructure(0)?.flatMap((node) =>
      node.kind === "opaque" ? structure(node) : structure(doc.inlineAt(node.anchor))),
  };
}

export async function inspectInstalled(bytes: Uint8Array) {
  const options: OpenDocxOptions = {
    readLimits: { maxParts: 100, maxPartBytes: 1_000_000, maxExpandedBytes: 2_000_000 },
  };
  const doc: ImportedDocument = await openDocx(bytes, options);
  const sections: readonly ImportedSection[] = doc.sections();
  const headers: readonly ImportedHeader[] = doc.defaultHeaders();
  const stories: readonly ImportedStory[] = doc.stories();
  return {
    body: doc.body().map((node) => bodyBlock(node, doc)),
    sections: sections.map((s) => [s.bodyStart, s.bodyEnd, s.sectionType, s.sectionTypeXml]),
    stories: stories.map((story) => sectionStory(story, doc)),
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
