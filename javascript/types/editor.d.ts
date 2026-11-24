import type { ResolvedImage } from "./drawings.js";
import type { StyleDefinition, NumberingStyleDefinition } from "./styles.js";
import type { ControlAnchor, HeaderAnchor, InlineAnchor, ParagraphAnchor, TableAnchor } from "./inspection.js";
import type { ResolvedTable, TableLayout } from "./tables.js";
import type { Alignment, ResolvedInline, ResolvedParagraph } from "./text-layout.js";

export interface ParagraphAlignmentEdit {
  readonly target: ParagraphAnchor;
  readonly alignment: Alignment;
}

export interface TableInsertion {
  /** Insert immediately after this original body paragraph. */
  readonly target: ParagraphAnchor;
  readonly table: ResolvedTable;
}

/** Physical row/cell indices include opaque children in the inspected view. */
export interface CellAlignmentEdit {
  readonly target: TableAnchor;
  readonly rowIndex: number;
  readonly cellIndex: number;
  readonly alignment: "top" | "center" | "bottom" | "both";
}
export interface CellAlignmentChange {
  readonly bodyIndex: number;
  readonly rowIndex: number;
  readonly cellIndex: number;
  readonly before?: string;
  readonly after: CellAlignmentEdit["alignment"];
}

export interface ParagraphInsertion {
  /** Insert immediately after this original body paragraph. */
  readonly target: ParagraphAnchor;
  readonly paragraph: ResolvedParagraph;
}

export type HeaderEditMode = "shared" | "sectionLocal";

export type BlockEdit =
  | { readonly kind: "replace"; readonly target: ParagraphAnchor; readonly paragraph: ResolvedParagraph }
  | { readonly kind: "remove"; readonly target: ParagraphAnchor }
  | { readonly kind: "replaceSection"; readonly target: ParagraphAnchor; readonly blocks: readonly (ResolvedParagraph | ResolvedTable)[] }
  | { readonly kind: "keepWithNext"; readonly target: ParagraphAnchor; readonly value: boolean };

export interface HeaderImageEdit {
  readonly target: HeaderAnchor;
  readonly mode: HeaderEditMode;
  readonly image: ResolvedImage;
}

export type InlineEdit = { readonly target: InlineAnchor } & (
  | { readonly kind: "insertAfter" | "replace"; readonly inline: ResolvedInline }
  | { readonly kind: "setBold"; readonly bold: boolean }
  | { readonly kind: "setToggle"; readonly property: "bold" | "italic" | "strike" | "hidden"; readonly value: boolean | null }
  | { readonly kind: "removeHyperlink" }
  | { readonly kind: "hyperlinkTooltip"; readonly value?: string }
  | { readonly kind: "hyperlinkHistory"; readonly value?: boolean }
);

export interface EditBatch {
  readonly blockEdits?: readonly BlockEdit[];
  readonly inlineEdits?: readonly InlineEdit[];
  readonly tableStructure?: readonly TableStructureEdit[];
  readonly styleEdits?: readonly StyleEdit[];
  readonly resources?: readonly ResourceEdit[];
  readonly controls?: readonly ControlEdit[];
  readonly textRanges?: readonly TextRangeEdit[];
  readonly textInsertions?: readonly TextInsertion[];
  readonly runText?: readonly RunTextEdit[];
  readonly runRemovals?: readonly InlineAnchor[];
  readonly alignments?: readonly ParagraphAlignmentEdit[];
  readonly tableInsertions?: readonly TableInsertion[];
  readonly paragraphInsertions?: readonly ParagraphInsertion[];
  readonly headerImages?: readonly HeaderImageEdit[];
  readonly cellAlignments?: readonly CellAlignmentEdit[];
}

export interface TextRangeEdit { readonly target: ParagraphAnchor; readonly start: number; readonly end: number; readonly text: string }
export interface TextInsertion { readonly target: ParagraphAnchor; readonly position: number; readonly text: string }
export type TableStructureEdit =
  | { readonly target: TableAnchor; readonly kind: "layout"; readonly value: TableLayout | null }
  | { readonly target: TableAnchor; readonly kind: "insertRow" | "removeRow" | "insertColumn" | "removeColumn"; readonly index: number }
  | { readonly target: TableAnchor; readonly kind: "insertCell" | "removeCell"; readonly row: number; readonly cell: number }
  | { readonly target: TableAnchor; readonly kind: "replaceCell"; readonly row: number; readonly cell: number; readonly blocks: readonly (ResolvedParagraph | ResolvedTable)[] };

declare const styleAnchor: unique symbol;
export interface StyleAnchor { readonly [styleAnchor]: true }
type EditableDefinition = Exclude<StyleDefinition, NumberingStyleDefinition>;
export type StyleEdit =
  | { readonly op: "add"; readonly definition: EditableDefinition }
  | { readonly op: "patch"; readonly target: StyleAnchor; readonly bold?: boolean; readonly italic?: boolean }
  | { readonly op: "replace"; readonly target: StyleAnchor; readonly definition: EditableDefinition }
  | { readonly op: "remove"; readonly target: StyleAnchor };

export type ResourceEdit =
  | { readonly kind: "metadata"; readonly field: import("./inspection.js").MetadataField; readonly value: string | null }
  | { readonly kind: "customProperty"; readonly name: string; readonly value: string | boolean | null }
  | { readonly kind: "coreTitle"; readonly value: string | null }
  | { readonly kind: "coreSubject"; readonly value: string | null }
  | { readonly kind: "defaultAlignment"; readonly value: Alignment | null }
  | { readonly kind: "abstractStart"; readonly abstractId: number; readonly level: number; readonly value: number | null };

export interface ControlEdit {
  readonly target: ControlAnchor;
  readonly tag?: string;
  readonly alias?: string;
  readonly text?: string;
}

export interface RunTextEdit { readonly target: InlineAnchor; readonly text: string }
export interface RunRemovalChange {
  readonly bodyIndex: number;
  readonly inlineIndex: number;
  readonly before: string;
}
export interface RunTextChange {
  readonly bodyIndex: number;
  readonly inlineIndex: number;
  readonly before: string;
  readonly after: string;
}

export interface AlignmentChange {
  readonly bodyIndex: number;
  /** Original XML attribute value; absent if the alignment was unset. */
  readonly before?: string;
  readonly after: Alignment;
}

export interface TableInsertionChange {
  readonly afterBodyIndex: number;
  readonly rows: number;
  readonly columns: number;
}

export interface HeaderImageChange {
  readonly sourcePart: string;
  readonly outputPart: string;
  readonly affectedSections: readonly number[];
  readonly restoredSection?: number;
  readonly mode: HeaderEditMode;
}

declare const preview: unique symbol;
export interface EditPreview {
  blockChanges(): readonly number[];
  readonly [preview]: "edit";
  /** Changes in original body order, excluding unchanged alignments. */
  changes(): readonly AlignmentChange[];
  cellAlignmentChanges(): readonly CellAlignmentChange[];
  runTextChanges(): readonly RunTextChange[];
  runRemovals(): readonly RunRemovalChange[];
  /** Insertions in original body order. */
  insertions(): readonly TableInsertionChange[];
  inlineChanges(): readonly (readonly [number, number])[];
  /** Original body indices after which paragraphs were inserted. */
  paragraphInsertions(): readonly number[];
  /** Changes in target section order. */
  headerChanges(): readonly HeaderImageChange[];
  /** Encode a fresh copy; does not consume the preview or alter its source. */
  toDocx(): Uint8Array;
}

export interface HeaderImagePreview {
  readonly [preview]: "headerImage";
  change(): HeaderImageChange;
  toDocx(): Uint8Array;
}

declare module "./inspection.js" {
  interface ImportedDocument {
    styleAnchor(id: string): StyleAnchor;
    previewWrapControl(edit: {readonly target: import("./inspection.js").ControlSiteAnchor; readonly tag: string}): EditPreview;
    previewUnwrapControl(target: ControlAnchor): EditPreview;
    previewReplaceControl(edit: {readonly target: ControlAnchor; readonly properties: import("./structured.js").ContentControlProperties}): EditPreview;
    previewControlChoice(edit: {readonly target: ControlAnchor; readonly selected: string;
      readonly choices?: import("./inspection.js").ImportedControl["choices"]}): EditPreview;
    previewTrackedBold(edit: { readonly target: InlineAnchor; readonly bold: boolean; readonly author: string; readonly date?: string }): EditPreview;
    previewTrackedParagraph(edit: { readonly target: ParagraphAnchor; readonly author: string; readonly date?: string } & (
      | { readonly kind: "insertBefore"; readonly text: string }
      | { readonly kind: "delete"; readonly text?: never }
    )): EditPreview;
    previewParagraphRevision(edit: { readonly target: ParagraphAnchor; readonly action: "accept" | "reject" }): EditPreview;
    /** Explicitly track a single-text direct body run edit. */
    previewTrackedText(edit: {
      readonly target: InlineAnchor; readonly author: string; readonly date?: string;
    } & (
      | { readonly kind: "replace" | "insertAfter"; readonly text: string }
      | { readonly kind: "delete"; readonly text?: never }
    )): EditPreview;
    /** Generate both move legs using a fresh name and a required UTC date. */
    previewTrackedMove(edit: {
      readonly target: InlineAnchor; readonly after: InlineAnchor;
      readonly name: string; readonly author: string; readonly date: string;
    }): EditPreview;
    /** Review one insertion/deletion wrapper; moves require both wrappers. */
    previewRevision(edit: {
      readonly target: InlineAnchor; readonly action: "accept" | "reject"; readonly paired?: InlineAnchor;
    }): EditPreview;
    /** Change single-text runs in the named physical story part, including all aliases. */
    editStoryText(batch: { readonly ownerPart: string; readonly runText: readonly RunTextEdit[] }): Uint8Array;
    editEntryInlines(batch: { readonly ownerPart: string; readonly inlineEdits: readonly InlineEdit[] }): Uint8Array;
    replaceField(batch: {readonly target: ParagraphAnchor; readonly inlineIndex: number; readonly field?: import("./structured.js").ResolvedField}): Uint8Array;
    fieldAt(batch: {readonly target: ParagraphAnchor; readonly inlineIndex: number}): {readonly complex: boolean; readonly instruction: string; readonly inlineCount: number; readonly dirtyXml?: string; readonly lockedXml?: string};
    patchField(batch: {readonly target: ParagraphAnchor; readonly inlineIndex: number} & ({readonly property: "instruction"; readonly value: string} | {readonly property: "dirty" | "locked"; readonly value?: boolean})): Uint8Array;
    setPairedRange(batch: {readonly id: number; readonly bookmark?: string; readonly endpoints?: readonly [InlineAnchor, InlineAnchor]}): Uint8Array;
    bodyRangeMarkers(): readonly {readonly bodyIndex: number; readonly inlineIndex: number; readonly kind: "bookmarkStart" | "bookmarkEnd" | "commentRangeStart" | "commentRangeEnd" | "commentReference"; readonly id?: string; readonly name?: string}[];
    /** Atomically stage edits against this snapshot's original anchors. */
    preview(batch: EditBatch): EditPreview;
    /** Append an inline image to a shared header/footer or a section-local fork. */
    previewHeaderImage(edit: HeaderImageEdit): HeaderImagePreview;
  }
}
