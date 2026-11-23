import {
  bookmarkEnd, bookmarkStart, createResolvedBlockControl, createResolvedInline, future, inlineText,
  lazy, resolveComponent, resolveDocument, validateAuthoringId, validateValue,
} from "rocx-docx";
import type {
  BlockContentControlDeclaration, CommentDeclaration, ContentControlLock, ContentControlText,
  DocumentProgram, FieldDeclaration, FieldInstruction, HyperlinkDeclaration, HyperlinkTarget,
  InlineContentControlDeclaration, NoteDeclaration, NoteTarget, RangeStartDeclaration,
  ResolvedBlockContentControl, ResolvedCommentBody, ResolvedInlineView, ResolvedNote, TextRevision,
} from "rocx-docx";

function required<T>(value: T | undefined): T {
  if (value === undefined) throw new Error("unexpected exclusion");
  return value;
}
const rawInstructions = [
  { kind: "page" }, { kind: "numPages" }, { kind: "reference", name: "_Heading" },
  { kind: "tableOfContents" }, { kind: "raw", text: " DATE " },
] as const satisfies readonly FieldInstruction[];
const targets = [
  { kind: "external", url: "https://example.test/" }, { kind: "bookmark", name: "_Heading" },
] as const satisfies readonly HyperlinkTarget[];
const noteTargets = [
  { kind: "footnote", id: 1 }, { kind: "endnote", id: 1 },
] as const satisfies readonly NoteTarget[];
const appearance = [
  { kind: "rich" }, { kind: "plain", multiLine: false },
  { kind: "dropDown", items: [{ displayText: "Alpha", value: "a" }], selected: "a" },
] as const satisfies readonly ContentControlText[];
const locks = ["unlocked", "controlLocked", "contentLocked", "controlAndContentLocked"] as const satisfies readonly ContentControlLock[];
const timestamp = "2024-02-29T01:02:03Z";
const revisions = [
  { kind: { kind: "insertion" }, author: "Ada" },
  { kind: { kind: "deletion" }, author: "Ada" },
  { kind: { kind: "moveSource", name: "Move" }, author: "Ada", date: timestamp },
  { kind: { kind: "moveDestination", name: "Move" }, author: "Ada", date: timestamp },
] as const satisfies readonly TextRevision[];

const text = { kind: "text", text: "Alpha" } as const;
const paragraph = { kind: "paragraph", runs: [text] } as const;
const link: HyperlinkDeclaration = {
  kind: "hyperlink", target: lazy(() => targets[0]),
  children: future(async () => [text]), properties: { tooltip: "outside", history: false }, condition: () => true,
};
const field: FieldDeclaration = {
  kind: "field", instruction: rawInstructions[4], children: [text],
  properties: { form: "complex", dirty: false, locked: true }, condition: true,
};
const note = {
  kind: "note", target: noteTargets[0], children: [paragraph], condition: true,
} satisfies NoteDeclaration;
const comment = {
  kind: "comment", children: [paragraph], metadata: { author: "Ada", initials: "A", date: timestamp }, condition: true,
} satisfies CommentDeclaration;
const inline = {
  kind: "contentControl", children: [text], condition: true,
  properties: { id: 2, alias: "Inline", tag: "inline", lock: locks[2], text: appearance[2] },
} satisfies InlineContentControlDeclaration;
const block = {
  kind: "contentControl", children: [paragraph], condition: true,
  properties: { id: 3, alias: "Block", tag: "block", lock: locks[1], text: appearance[1] },
} satisfies BlockContentControlDeclaration;
const start = await bookmarkStart("_Heading");
const end = await bookmarkEnd("_Heading");
const resolvedComment: ResolvedCommentBody = required(await resolveComponent(comment));
const range = {
  kind: "rangeStart",
  id: lazy(() => "range"),
  range: { kind: "structuredComment", comment: resolvedComment },
} satisfies RangeStartDeclaration;
const doc: DocumentProgram = {
  timestamp,
  notes: future(async () => [note]),
  sections: [{
    kind: "section",
    children: [block, {
      kind: "paragraph",
      runs: [
        start, text, end, link, field, inline, range, text, { kind: "rangeEnd", id: "range" },
        { kind: "noteReference", target: noteTargets[0] },
        ...revisions.map((revision) => ({ kind: "text" as const, text: "same", revision })),
      ],
    }],
  }],
};

const resolvedDoc = await resolveDocument(doc);
const resolvedNote: ResolvedNote = required(await resolveComponent(note));
const blockControl: ResolvedBlockContentControl = required(await resolveComponent(block, { scope: "block" }));
const inlineControl = required(await resolveComponent(inline, { scope: "inline" }));
const resolvedLink = required(await resolveComponent(link));
const resolvedField = required(await resolveComponent(field));
const assembledField = await createResolvedInline({
  kind: "field", instruction: rawInstructions[0], children: [resolvedLink],
});
const assembledRange = await createResolvedInline({
  kind: "rangeStart", id: "resolved", range: { kind: "structuredComment", comment: resolvedComment },
});
const assembledBlock = await createResolvedBlockControl(block.properties, blockControl.view.children);
const assembledText = await createResolvedInline({ kind: "text", text: "inserted", revision: revisions[0] });
const assembledTab = await createResolvedInline({ kind: "tab", comment: { text: "tab" }, format: { bold: false } });
const narrow: ResolvedInlineView = assembledRange.view;
if (narrow.kind === "rangeStart" && narrow.range.kind === "structuredComment") {
  const author: string | undefined = narrow.range.comment.view.metadata.author;
  void author;
}
const contents: string = await inlineText([inlineControl, assembledText, assembledTab, resolvedField, assembledField]);
const directParagraphs = resolvedDoc.view.sections[0].view.paragraphs;
void [contents, directParagraphs, assembledBlock.view.children, resolvedDoc.view.notes[0].view.target.id];
await validateValue({ kind: "resolvedNote", value: resolvedNote });
await validateValue({ kind: "resolvedCommentBody", value: resolvedComment });
await validateValue({ kind: "textRevision", value: revisions[2] });
await validateValue({ kind: "commentMetadata", value: comment.metadata });
await validateAuthoringId("rangeId", "range");

// @ts-expect-error inline/block controls must identify their standalone scope
void resolveComponent(block);
// @ts-expect-error block control cannot be constrained to inline scope
void resolveComponent(block, { scope: "inline" });
// @ts-expect-error a resolved constructor takes handles, not authoring declarations
void createResolvedInline({ kind: "field", instruction: rawInstructions[0], children: [text] });
// @ts-expect-error checked block constructors cannot accept an inline handle
void createResolvedBlockControl({}, [inlineControl]);
// @ts-expect-error standalone resolved validation requires the matching handle type
void validateValue({ kind: "resolvedCommentBody", value: resolvedNote });
