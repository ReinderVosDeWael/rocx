import type { ReadLimits, ResolvedHandle } from "./boundary.js";

declare const anchor: unique symbol;

/** Direct body paragraph target, valid only in the snapshot that issued it. */
export interface ParagraphAnchor { readonly [anchor]: "paragraph" }
/** Read-only direct body table address. */
export interface TableAnchor { readonly [anchor]: "table" }
/** Snapshot-local run or hyperlink address; eligible direct body runs support text replacement. */
export interface InlineAnchor { readonly [anchor]: "inline" }
/** Section-scoped target for a default-header edit. */
export interface HeaderAnchor { readonly [anchor]: "header" }

export type ImportedAlignment =
  | "left" | "center" | "right" | "justify" | "distribute"
  | "justifyMedium" | "justifyHigh" | "justifyLow" | "thaiJustify";
export type ImportedBreakType = "line" | "page" | "column" | "textWrapping";
export type ImportedSectionType = "nextPage" | "nextColumn" | "continuous" | "evenPage" | "oddPage";

export interface ImportedOpaque {
  readonly kind: "opaque";
  /** Expanded XML name (usual OOXML names use their canonical prefix). */
  readonly name: string;
}

export type ImportedInline =
  | { readonly kind: "text"; readonly text: string }
  | { readonly kind: "tab" }
  | { readonly kind: "break"; readonly breakType: ImportedBreakType }
  | ImportedOpaque;

export interface ImportedRun {
  readonly kind: "run";
  readonly anchor: InlineAnchor;
  readonly items: readonly ImportedInline[];
}

export interface ImportedHyperlink {
  readonly kind: "hyperlink";
  readonly anchor: InlineAnchor;
  /** Source relationship ID local to the owning part, not a URL. */
  readonly relationshipId?: string;
  readonly bookmark?: string;
  readonly children: readonly ImportedInlineNode[];
}

export type ImportedInlineNode = ImportedRun | ImportedHyperlink | ImportedOpaque;

export interface ImportedFlatParagraph {
  readonly kind: "paragraph";
  readonly inlineItems: readonly ImportedInline[];
}

export interface ImportedParagraph extends ImportedFlatParagraph {
  readonly anchor: ParagraphAnchor;
  /** Projection of ordinary runs and links, not rendered text. */
  readonly text: string;
  readonly alignment?: ImportedAlignment;
  readonly alignmentXml?: string;
  readonly hasOpaqueContent: boolean;
  readonly inlineStructure: readonly ImportedInlineNode[];
}

/** Direct cell paragraph; only runs and hyperlinks have anchors. */
export interface ImportedCellParagraph extends ImportedFlatParagraph {
  readonly inlineStructure: readonly ImportedInlineNode[];
}

/** Nested tables and unsupported blocks remain opaque. */
export type ImportedCellBlock = ImportedCellParagraph | ImportedOpaque;

export interface ImportedTableCell {
  readonly kind: "cell";
  readonly blocks: readonly ImportedCellBlock[];
}

export type ImportedRowChild = ImportedTableCell | ImportedOpaque;

export interface ImportedTableRow {
  readonly kind: "row";
  readonly children: readonly ImportedRowChild[];
}

export type ImportedTableChild = ImportedTableRow | ImportedOpaque;

export interface ImportedTable {
  readonly kind: "table";
  readonly anchor: TableAnchor;
  /** Physical rows; merged cells are not expanded. */
  readonly children: readonly ImportedTableChild[];
}

export type BodyBlock = ImportedParagraph | ImportedTable | ImportedOpaque;

/** Half-open body range with explicit section type only. */
export interface ImportedSection {
  readonly bodyStart: number;
  readonly bodyEnd: number;
  readonly sectionType?: ImportedSectionType;
  readonly sectionTypeXml?: string;
}

export type ImportedHeaderBlock = ImportedFlatParagraph | ImportedOpaque;

export interface ImportedHeaderView {
  readonly anchor: HeaderAnchor;
  readonly sectionIndex: number;
  readonly partName: string;
  readonly inherited: boolean;
  readonly blocks: readonly ImportedHeaderBlock[];
}

export interface ImportedHeader extends ResolvedHandle<"importedHeader", ImportedHeaderView> {
  /** Empty paragraphs return `[]`; opaque and out-of-range blocks return `undefined`. */
  inlineStructure(blockIndex: number): readonly ImportedInlineNode[] | undefined;
}

export type SectionStoryKind = "header" | "footer";
export type ImportedStoryVariant = "default" | "first" | "even";
export type ImportedStoryBlock = ImportedHeaderBlock;

/** A section's effective reference to a physical header or footer part. */
export interface ImportedStoryView {
  readonly sectionIndex: number;
  readonly kind: SectionStoryKind;
  /** The section reference slot, not a pagination result. */
  readonly variant: ImportedStoryVariant;
  /** Physical package part name; aliases share inline anchors. */
  readonly partName: string;
  readonly inherited: boolean;
  readonly blocks: readonly ImportedStoryBlock[];
}

export interface ImportedStory extends ResolvedHandle<"importedStory", ImportedStoryView> {
  /** Empty paragraphs return `[]`; opaque and out-of-range blocks return `undefined`. */
  inlineStructure(blockIndex: number): readonly ImportedInlineNode[] | undefined;
}

declare const importedDocument: unique symbol;

export interface ImportedEntry {
  readonly kind: "footnote" | "endnote" | "comment";
  readonly partName: string;
  readonly id: string;
  readonly noteType?: string;
  readonly author?: string;
  readonly date?: string;
  readonly initials?: string;
  readonly blocks: readonly (ImportedOpaque | (ImportedFlatParagraph & {
    readonly inlineStructure: readonly ImportedInlineNode[];
  }))[];
}

export interface ImportedDocument {
  readonly [importedDocument]: true;
  body(): readonly BodyBlock[];
  sections(): readonly ImportedSection[];
  /** Rejects anchors issued by any other snapshot, including reopening identical bytes. */
  inlineAt(anchor: InlineAnchor): ImportedRun | ImportedHyperlink;
  /** Lazily inspect effective default headers; sections without one are omitted. */
  defaultHeaders(): readonly ImportedHeader[];
  /**
   * Lazily inspect effective header/footer slots in section order, headers
   * before footers and default, first, then even within each kind. Unset slots
   * are omitted, and page selection settings do not filter the result.
   */
  stories(): readonly ImportedStory[];
  entryStories(): readonly ImportedEntry[];
}

export interface OpenDocxOptions {
  readonly readLimits?: ReadLimits;
}

/** Open an immutable Transitional DOCX snapshot from the exact supplied byte view. */
export function openDocx(bytes: Uint8Array, options?: OpenDocxOptions): Promise<ImportedDocument>;
