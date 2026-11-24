import type { Condition, ResolvedHandle } from "./boundary.js";
import type { V } from "./resolution.js";
import type { BlockChildren, InlineChildren, ResolvedBlock, ResolvedInline } from "./text-layout.js";

interface Conditional {
  readonly condition?: Condition;
}
interface InlineView<K extends string> {
  readonly kind: K;
  readonly displayText: string;
}

export type HyperlinkTarget =
  | { readonly kind: "external"; readonly url: string }
  | { readonly kind: "bookmark"; readonly name: string };
export interface HyperlinkProperties {
  readonly tooltip?: string;
  readonly history?: boolean;
}
export interface HyperlinkDeclaration extends Conditional {
  readonly kind: "hyperlink";
  readonly target: V<HyperlinkTarget>;
  readonly children: V<InlineChildren>;
  readonly properties?: V<HyperlinkProperties>;
}
export type ResolvedHyperlink = ResolvedHandle<"inline", InlineView<"hyperlink"> & {
  readonly target: HyperlinkTarget;
  readonly children: readonly ResolvedInline[];
  readonly properties: HyperlinkProperties;
}>;

export type FieldInstruction =
  | { readonly kind: "page" }
  | { readonly kind: "numPages" }
  | { readonly kind: "reference"; readonly name: string }
  | { readonly kind: "tableOfContents" }
  | RawFieldInstruction;
/** Nonempty XML text checked by Rust, not evaluated or parsed as field syntax. */
export interface RawFieldInstruction {
  readonly kind: "raw";
  readonly text: string;
}
export type FieldForm = "simple" | "complex";
export interface FieldProperties {
  readonly form?: FieldForm;
  readonly dirty?: boolean;
  readonly locked?: boolean;
}
export interface FieldDeclaration extends Conditional {
  readonly kind: "field";
  readonly instruction: V<FieldInstruction>;
  readonly children: V<InlineChildren>;
  readonly properties?: V<FieldProperties>;
}
export type ResolvedField = ResolvedHandle<"inline", InlineView<"field"> & {
  readonly instruction: FieldInstruction;
  readonly instructionText: string;
  readonly children: readonly ResolvedInline[];
  readonly properties: FieldProperties & { readonly form: FieldForm };
}>;

/** Rust's u32 authoring identity. Footnotes and endnotes use separate spaces. */
export type NoteId = number;
export type NoteKind = "footnote" | "endnote";
export interface NoteTarget {
  readonly kind: NoteKind;
  readonly id: NoteId;
}
export interface NoteDeclaration extends Conditional {
  readonly kind: "note";
  readonly target: V<NoteTarget>;
  readonly children: V<BlockChildren>;
}
export interface NoteReferenceDeclaration extends Conditional {
  readonly kind: "noteReference";
  readonly target: V<NoteTarget>;
}
export type ResolvedNote = ResolvedHandle<"note", {
  readonly target: NoteTarget;
  readonly blocks: readonly ResolvedBlock[];
}>;
export type ResolvedNoteReference = ResolvedHandle<"inline", InlineView<"noteReference"> & {
  readonly target: NoteTarget;
}>;

export interface ResolvedComment {
  readonly text: string;
  readonly author?: string;
}
export interface CommentMetadata {
  readonly author?: string;
  readonly date?: string;
  readonly initials?: string;
}
export interface CommentDeclaration extends Conditional {
  readonly kind: "comment";
  readonly children: V<BlockChildren>;
  readonly metadata?: V<CommentMetadata>;
}
export type ResolvedCommentBody = ResolvedHandle<"commentBody", {
  readonly blocks: readonly ResolvedBlock[];
  readonly metadata: CommentMetadata;
}>;
export type BookmarkName = string;
export type RangeId = string;
type FlatRangeKind =
  | { readonly kind: "bookmark"; readonly name: BookmarkName }
  | ({ readonly kind: "comment" } & ResolvedComment);
export type ResolvedRangeKind =
  | FlatRangeKind
  | { readonly kind: "structuredComment"; readonly comment: ResolvedCommentBody };
export type RangeKind =
  | ResolvedRangeKind
  | { readonly kind: "structuredComment"; readonly comment: CommentDeclaration };
export interface RangeStartDeclaration extends Conditional {
  readonly kind: "rangeStart";
  readonly id: V<RangeId>;
  readonly range: V<RangeKind>;
}
export interface RangeEndDeclaration extends Conditional {
  readonly kind: "rangeEnd";
  readonly id: V<RangeId>;
}
export type ResolvedRangeStart = ResolvedHandle<"inline", InlineView<"rangeStart"> & {
  readonly id: RangeId;
  readonly range: ResolvedRangeKind;
}>;
export type ResolvedRangeEnd = ResolvedHandle<"inline", InlineView<"rangeEnd"> & {
  readonly id: RangeId;
}>;
/** Derive a range identity using Rust's bookmark policy. */
export function bookmarkStart(name: BookmarkName): Promise<RangeStartDeclaration>;
export function bookmarkEnd(name: BookmarkName): Promise<RangeEndDeclaration>;

export type ContentControlLock = "unlocked" | "controlLocked" | "contentLocked" | "controlAndContentLocked";
export interface ContentControlListItem {
  readonly displayText: string;
  readonly value: string;
}
export type ContentControlText =
  | { readonly kind: "rich" }
  | { readonly kind: "plain"; readonly multiLine?: boolean }
  | { readonly kind: "dropDown"; readonly items: readonly ContentControlListItem[]; readonly selected: string };
export interface ContentControlProperties {
  readonly text?: ContentControlText;
  readonly alias?: string;
  readonly tag?: string;
  readonly lock?: ContentControlLock;
  readonly id?: number;
}
export interface InlineContentControlDeclaration extends Conditional {
  readonly kind: "contentControl";
  readonly children: V<InlineChildren>;
  readonly properties?: V<ContentControlProperties>;
}
export interface BlockContentControlDeclaration extends Conditional {
  readonly kind: "contentControl";
  readonly children: V<BlockChildren>;
  readonly properties?: V<ContentControlProperties>;
}
export type ResolvedContentControlProperties = ContentControlProperties & {
  readonly text: ContentControlText;
};
export type ResolvedInlineContentControl = ResolvedHandle<"inline", InlineView<"contentControl"> & {
  readonly children: readonly ResolvedInline[];
  readonly properties: ResolvedContentControlProperties;
}>;
export type ResolvedBlockContentControl = ResolvedHandle<"blockControl", {
  readonly kind: "contentControl";
  readonly children: readonly ResolvedBlock[];
  readonly properties: ResolvedContentControlProperties;
}>;
export function createResolvedBlockControl(
  properties: ContentControlProperties | undefined, children: readonly ResolvedBlock[],
): Promise<ResolvedBlockContentControl>;

export type RevisionKind =
  | { readonly kind: "insertion" }
  | { readonly kind: "deletion" }
  | { readonly kind: "moveSource"; readonly name: BookmarkName }
  | { readonly kind: "moveDestination"; readonly name: BookmarkName };
export interface TextRevision {
  readonly kind: RevisionKind;
  readonly author: string;
  readonly date?: string;
}

type Entry<D, R> = { readonly declaration: D; readonly resolved: R };
type ControlEntry<D, R, S> = Entry<D, R> & { readonly scope: S };
export interface StructuredBlockComponents {
  readonly blockControl: ControlEntry<BlockContentControlDeclaration, ResolvedBlockContentControl, "block">;
}
export interface StructuredInlineComponents {
  readonly hyperlink: Entry<HyperlinkDeclaration, ResolvedHyperlink>;
  readonly field: Entry<FieldDeclaration, ResolvedField>;
  readonly noteReference: Entry<NoteReferenceDeclaration, ResolvedNoteReference>;
  readonly rangeStart: Entry<RangeStartDeclaration, ResolvedRangeStart>;
  readonly rangeEnd: Entry<RangeEndDeclaration, ResolvedRangeEnd>;
  readonly inlineControl: ControlEntry<InlineContentControlDeclaration, ResolvedInlineContentControl, "inline">;
}
export interface StructuredStandaloneComponents {
  readonly note: Entry<NoteDeclaration, ResolvedNote>;
  readonly comment: Entry<CommentDeclaration, ResolvedCommentBody>;
}
export interface StructuredDocumentFields {
  readonly notes?: V<readonly V<NoteDeclaration>[]>;
}
export interface StructuredDocumentView {
  readonly notes: readonly ResolvedNote[];
}

export interface StructuredResolvedInlineArms {
  readonly hyperlink: { readonly target: HyperlinkTarget; readonly properties?: HyperlinkProperties; readonly children: readonly ResolvedInline[] };
  readonly field: { readonly instruction: FieldInstruction; readonly properties?: FieldProperties; readonly children: readonly ResolvedInline[] };
  readonly contentControl: { readonly properties?: ContentControlProperties; readonly children: readonly ResolvedInline[] };
  readonly noteReference: { readonly target: NoteTarget };
  readonly rangeStart: { readonly id: RangeId; readonly range: ResolvedRangeKind };
  readonly rangeEnd: { readonly id: RangeId };
}

declare module "./resolution.js" {
  interface ValidationValues {
    readonly resolvedNote: ResolvedNote;
    readonly resolvedCommentBody: ResolvedCommentBody;
    readonly textRevision: TextRevision;
    readonly commentMetadata: CommentMetadata;
  }
}
