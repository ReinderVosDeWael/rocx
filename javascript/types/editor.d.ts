import type { ResolvedImage } from "./drawings.js";
import type { StyleDefinition, NumberingStyleDefinition, NumberingLevel, NumberingDefinition, TableSectionFormat } from "./styles.js";
import type { ControlAnchor, DefaultValues, HeaderAnchor, InlineAnchor, ParagraphAnchor, StoryParagraphAnchor, TableAnchor, ParagraphFlag, SectionFlag, ImportedSectionType } from "./inspection.js";
import type { ResolvedTable, TableLayout } from "./tables.js";
import type { Alignment, ResolvedInline, ResolvedParagraph, RunFormat } from "./text-layout.js";

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
  | { readonly kind: "paragraphFlag"; readonly target: ParagraphAnchor; readonly flag: ParagraphFlag; readonly value: boolean | null }
  | { readonly kind: "sectionFlag"; readonly target: ParagraphAnchor; readonly flag: SectionFlag; readonly value: boolean | null }
  | { readonly kind: "sectionType"; readonly target: ParagraphAnchor; readonly value: ImportedSectionType | null }
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
  | { readonly kind: "patchFormat"; readonly format?: RunFormat; readonly clear?: readonly
      (Exclude<keyof RunFormat, "superscript" | "subscript" | "language"> |
        "verticalAlignment" | "languageLatin" | "languageEastAsia" | "languageBidi" | "hidden")[] }
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
  | { readonly op: "field"; readonly target: StyleAnchor; readonly region?: StyleRegion; readonly field: StyleField; readonly value: string | null }
  | { readonly op: "region"; readonly target: StyleAnchor; readonly region: StyleRegion; readonly format: TableSectionFormat | null }
  | { readonly op: "add"; readonly definition: StyleDefinition }
  | { readonly op: "patch"; readonly target: StyleAnchor; readonly bold?: boolean; readonly italic?: boolean }
  | { readonly op: "replace"; readonly target: StyleAnchor; readonly definition: EditableDefinition }
  | { readonly op: "remove"; readonly target: StyleAnchor };

type StyleRegion = "wholeTable" | "firstRow" | "lastRow" | "firstCol" | "lastCol" | "band1Horz" | "band2Horz" | "band1Vert" | "band2Vert" | "nwCell" | "neCell" | "swCell" | "seCell";
type StyleField = "basedOn" | "next" | "link" | "b" | "i" | "bCs" | "iCs" | "rtl" | "cs" | "strike" | "caps" | "smallCaps" | "vanish" | "keepNext" | "keepLines" | "pageBreakBefore" | "widowControl" | "bidi" | "contextualSpacing" | "mirrorIndents" | "snapToGrid" | "jc";

export type ResourceEdit =
  | { [K in keyof DefaultValues]: {readonly kind: "documentDefault"; readonly field: K; readonly value: DefaultValues[K] | null} }[keyof DefaultValues]
  | { readonly kind: "metadata"; readonly field: import("./inspection.js").MetadataField; readonly value: string | null }
  | { readonly kind: "customProperty"; readonly name: string; readonly value: string | boolean | null }
  | { readonly kind: "replaceInstance"; readonly definition: Omit<NumberingDefinition, "abstractId" | "inheritStyle" | "levels"> & {readonly abstractId: number} }
  | { readonly kind: "replaceAbstract"; readonly abstractId: number; readonly levels: readonly NumberingLevel[] }
  | { readonly kind: "abstractLevel"; readonly abstractId: number; readonly level: number; readonly value: NumberingLevel | null }
  | { readonly kind: "relinkInstance"; readonly numId: number; readonly abstractId: number }
  | { readonly kind: "storyParagraphNumbering"; readonly target: StoryParagraphAnchor; readonly value: { readonly id: number; readonly level: number } | null }
  | { readonly kind: "addAbstract"; readonly abstractId: number; readonly levels: readonly NumberingLevel[] }
  | { readonly kind: "instanceLevel"; readonly numId: number; readonly level: number; readonly value: NumberingLevel | null }
  | { readonly kind: "removeAbstract"; readonly abstractId: number }
  | { readonly kind: "addInstance"; readonly numId: number; readonly abstractId: number }
  | { readonly kind: "removeInstance"; readonly numId: number }
  | { readonly kind: "instanceStart"; readonly numId: number; readonly level: number; readonly value: number | null }
  | { readonly kind: "paragraphNumbering"; readonly target: ParagraphAnchor; readonly value: { readonly id: number; readonly level: number } | null }
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
    editNoteReference(edit: { readonly ownerPart: string; readonly target: InlineAnchor } & (
      | { readonly kind: "create"; readonly id: number; readonly text: string }
      | { readonly kind: "delete" | "detach" }
      | { readonly kind: "attach" | "retarget" | "renumber"; readonly id: number }
    )): Uint8Array;
    editEntry(edit: { readonly ownerPart: string; readonly id: string } & (
      | { readonly kind: "insertParagraph" | "replaceParagraph"; readonly index: number; readonly value: string }
      | { readonly kind: "removeParagraph"; readonly index: number }
      | { readonly kind: "author" | "date" | "initials"; readonly value: string }
      | { readonly kind: "clearDate" | "clearInitials" | "deleteUnusedNote" }
    )): Uint8Array;
    /** Atomically stage edits against this snapshot's original anchors. */
    preview(batch: EditBatch): EditPreview;
    /** Append an inline image to a shared header/footer or a section-local fork. */
    previewHeaderImage(edit: HeaderImageEdit): HeaderImagePreview;
  }
}
