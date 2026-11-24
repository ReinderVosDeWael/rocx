import type { ReadLimits, ResolvedHandle } from "./boundary.js";

declare const anchor: unique symbol;

export interface ControlAnchor { readonly [anchor]: "control" }
export interface ImportedControl {
  readonly anchor: ControlAnchor;
  readonly context: "block" | "inline" | "row" | "cell";
  readonly id?: string;
  readonly tag?: string;
  readonly alias?: string;
  readonly lock?: string;
  readonly bound: boolean;
}

/** Direct body paragraph target, valid only in the snapshot that issued it. */
export interface ParagraphAnchor { readonly [anchor]: "paragraph" }
/** Direct body table address for cell alignment edits. */
export interface TableAnchor { readonly [anchor]: "table" }
/** Snapshot-local run, hyperlink or revision address; eligible direct body runs support text replacement or removal. */
export interface InlineAnchor { readonly [anchor]: "inline" }
/** Section-scoped target for a header/footer image edit. */
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

export interface ImportedRevision {
  readonly kind: "revision";
  readonly revisionKind: "insertion" | "deletion" | "moveSource" | "moveDestination";
  readonly anchor: InlineAnchor;
  readonly id?: string;
  readonly author?: string;
  readonly date?: string;
  readonly children: readonly ImportedInlineNode[];
}

export type ImportedInlineNode = ImportedRun | ImportedHyperlink | ImportedRevision | ImportedOpaque;

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

/** Direct body control metadata; its content is opaque and has no edit anchor.
 * Values are source XML strings, including unrecognized lock values. */
export interface ImportedContentControl {
  readonly kind: "contentControl";
  readonly tag?: string;
  readonly alias?: string;
  readonly lock?: string;
}

export type BodyBlock = ImportedParagraph | ImportedTable | ImportedContentControl | ImportedOpaque;

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
  /** Image-edit target for this section, story kind and variant. */
  readonly anchor: HeaderAnchor;
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

/** Source metadata only; absent fields were not present in the source. */
export interface ImportedStyle {
  readonly id?: string;
  readonly kind: "paragraph" | "character" | "table" | "numbering" | "missing" | "unsupported";
  readonly kindXml?: string;
  readonly name?: string;
  readonly basedOn?: string;
  readonly nextStyle?: string;
  readonly linkedStyle?: string;
  readonly propertyXml: readonly string[];
}
export interface ImportedStyles {
  readonly partName: string;
  readonly styles: readonly ImportedStyle[];
}
export interface ImportedDocument {
  controls(): readonly ImportedControl[];
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
  /** Undefined when the document has no styles relationship. */
  styles(): ImportedStyles | undefined;
  coreTitle(): string | undefined;
  defaultAlignment(): ImportedAlignment | undefined;
  abstractStarts(): readonly AbstractStart[];
}

export interface AbstractStart {
  readonly abstractId: number;
  readonly level: number;
  readonly start?: number;
}

export interface OpenDocxOptions {
  readonly readLimits?: ReadLimits;
}

/** Open an immutable Transitional DOCX snapshot from the exact supplied byte view. */
export function openDocx(bytes: Uint8Array, options?: OpenDocxOptions): Promise<ImportedDocument>;
