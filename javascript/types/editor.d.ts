import type { ResolvedImage } from "./drawings.js";
import type { HeaderAnchor, InlineAnchor, ParagraphAnchor, TableAnchor } from "./inspection.js";
import type { ResolvedTable } from "./tables.js";
import type { Alignment, ResolvedParagraph } from "./text-layout.js";

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

export interface HeaderImageEdit {
  readonly target: HeaderAnchor;
  readonly mode: HeaderEditMode;
  readonly image: ResolvedImage;
}

export interface EditBatch {
  readonly runText?: readonly RunTextEdit[];
  readonly runRemovals?: readonly InlineAnchor[];
  readonly alignments?: readonly ParagraphAlignmentEdit[];
  readonly tableInsertions?: readonly TableInsertion[];
  readonly paragraphInsertions?: readonly ParagraphInsertion[];
  readonly headerImages?: readonly HeaderImageEdit[];
  readonly cellAlignments?: readonly CellAlignmentEdit[];
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
  readonly [preview]: "edit";
  /** Changes in original body order, excluding unchanged alignments. */
  changes(): readonly AlignmentChange[];
  cellAlignmentChanges(): readonly CellAlignmentChange[];
  runTextChanges(): readonly RunTextChange[];
  runRemovals(): readonly RunRemovalChange[];
  /** Insertions in original body order. */
  insertions(): readonly TableInsertionChange[];
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
    /** Atomically stage edits against this snapshot's original anchors. */
    preview(batch: EditBatch): EditPreview;
    /** Append an inline image to the shared header or a section-local fork. */
    previewHeaderImage(edit: HeaderImageEdit): HeaderImagePreview;
  }
}
