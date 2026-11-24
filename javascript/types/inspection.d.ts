import type { ReadLimits, ResolvedHandle } from "./boundary.js";
import type { TableLayout } from "./tables.js";

declare const anchor: unique symbol;
export type MetadataField = "title" | "subject" | "creator" | "description" | "category" | "keywords" | "version"
  | "created" | "modified" | "lastPrinted" | "lastModifiedBy" | "revision"
  | "identifier" | "language" | "contentStatus" | "company" | "manager";

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
export interface SectionAnchor { readonly [anchor]: "section" }
export interface LayoutTab {
  readonly position: number;
  readonly alignment: "left" | "center" | "right" | "decimal" | "bar" | "clear" | "start" | "end";
  readonly leader?: "none" | "dot" | "hyphen" | "underscore" | "heavy" | "middleDot";
}
export interface DirectParagraphLayout {
  readonly heading?: number;
  readonly keepWithNext?: boolean;
  readonly keepTogether?: boolean;
  readonly pageBreakBefore?: boolean;
  readonly widowControl?: boolean;
  readonly suppressLineNumbers?: boolean;
  readonly paragraphBidi?: boolean;
  readonly snapToGrid?: boolean;
  readonly contextualSpacing?: boolean;
  readonly mirrorIndents?: boolean;
  readonly styleId?: string;
  readonly alignment?: "left" | "start" | "center" | "right" | "end" | "both" | "distribute" | "mediumKashida" | "highKashida" | "lowKashida" | "thaiDistribute";
  readonly outlineLevel?: number;
  readonly spaceBefore?: number;
  readonly spaceAfter?: number;
  readonly lineHeight?: number;
  readonly lineRule?: "auto" | "exact" | "atLeast";
  readonly leftIndent?: number;
  readonly rightIndent?: number;
  readonly firstLineIndent?: number;
  readonly tabStops?: readonly LayoutTab[];
}
export interface DirectSectionLayout {
  readonly margins?: true;
  readonly lineNumbering?: true;
  readonly borderTop?: LayoutBorder;
  readonly borderLeft?: LayoutBorder;
  readonly borderBottom?: LayoutBorder;
  readonly borderRight?: LayoutBorder;
  readonly pageWidth?: number;
  readonly pageHeight?: number;
  readonly orientation?: "portrait" | "landscape";
  readonly marginTop?: number;
  readonly marginBottom?: number;
  readonly marginLeft?: number;
  readonly marginRight?: number;
  readonly marginHeader?: number;
  readonly marginFooter?: number;
  readonly marginGutter?: number;
  readonly columnCount?: number;
  readonly columnSpace?: number;
  readonly columnSeparator?: boolean;
  readonly gridMode?: "default" | "lines" | "linesAndChars" | "snapToChars";
  readonly gridPitch?: number;
  readonly gridCharSpace?: number;
  readonly pageStart?: number;
  readonly pageFormat?: "decimal" | "upperRoman" | "lowerRoman" | "upperLetter" | "lowerLetter" | "ordinal" | "cardinalText" | "ordinalText" | "decimalZero" | "none";
  readonly lineStart?: number;
  readonly lineCountBy?: number;
  readonly lineDistance?: number;
  readonly lineRestart?: "newPage" | "newSection" | "continuous";
  readonly verticalAlign?: "top" | "center" | "bottom" | "both";
  readonly sectionType?: ImportedSectionType;
  readonly titlePage?: boolean;
  readonly bidi?: boolean;
  readonly rtlGutter?: boolean;
  readonly borderOffset?: "page" | "text";
  readonly borderDisplay?: "allPages" | "firstPage" | "notFirstPage";
  readonly borderZOrder?: "front" | "back";
}
export interface LayoutBorder { readonly size?: number; readonly space?: number; readonly color?: string }
/** Snapshot-scoped address; physical-owner and ordinary body anchors are not interchangeable. */
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

export interface DefaultTab {
  readonly positionTwips: number;
  readonly alignment: "left" | "right" | "center" | "decimal" | "bar" | "clear" | "start" | "end" | "num";
  readonly leader?: "none" | "dot" | "hyphen" | "underscore" | "heavy" | "middleDot";
}
type DefaultToggle = "bold" | "italic" | "boldCs" | "italicCs" | "runRtl" | "complexScript"
  | "caps" | "smallCaps" | "strike" | "runSnapToGrid" | "paragraphBidi" | "keepTogether"
  | "keepWithNext" | "pageBreakBefore" | "widowControl" | "suppressLineNumbers" | "contextualSpacing"
  | "mirrorIndents" | "paragraphSnapToGrid" | "mirrorMargins" | "gutterAtTop" | "trackRevisions"
  | "beforeAutoSpacing" | "afterAutoSpacing";
type DefaultInteger = "sizeHalfPoints" | "sizeCsHalfPoints" | "beforeTwips" | "afterTwips" | "line"
  | "leftTwips" | "rightTwips" | "firstLineTwips" | "hangingTwips" | "leftChars" | "rightChars"
  | "firstLineChars" | "hangingChars" | "footnoteStart" | "endnoteStart" | "startTwips" | "endTwips"
  | "startChars" | "endChars" | "beforeLines" | "afterLines";
type DefaultText = "fontAscii" | "fontHAnsi" | "fontEastAsia" | "fontCs" | "fontAsciiTheme"
  | "fontHAnsiTheme" | "fontEastAsiaTheme" | "fontCsTheme" | "language" | "languageEastAsia"
  | "languageBidi" | "underline" | "verticalAlign" | "runColor" | "colorTheme" | "colorTint"
  | "colorShade" | "lineRule" | "footnotePosition" | "endnotePosition" | "footnoteFormat"
  | "endnoteFormat" | "footnoteRestart" | "endnoteRestart" | "justification";
export type DefaultValues = Readonly<Record<DefaultToggle, boolean> & Record<DefaultInteger, number>
  & Record<DefaultText, string> & {tabStops: readonly DefaultTab[]}>;

export interface ImportedOpaque {
  readonly kind: "opaque";
  /** Expanded XML name (usual OOXML names use their canonical prefix). */
  readonly name: string;
}

export type ImportedInline =
  | { readonly kind: "text"; readonly text: string }
  | { readonly kind: "footnoteReference" | "endnoteReference"; readonly id: string }
  | { readonly kind: "tab" }
  | { readonly kind: "break"; readonly breakType: ImportedBreakType }
  | ImportedOpaque;

export interface ImportedRun {
  readonly format: import("./text-layout.js").ResolvedRunFormat;
  /** Direct toggle properties; absent when unset or ambiguous. */
  readonly bold?: boolean;
  readonly italic?: boolean;
  readonly strike?: boolean;
  readonly hidden?: boolean;
  readonly hasOpaqueFormatting: boolean;
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
  readonly layout: DirectParagraphLayout;
  readonly flags: Readonly<Partial<Record<ParagraphFlag, boolean>>>;
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
  readonly properties: Readonly<Record<string, Readonly<Record<string, string>>>>;
  readonly gridSpan?: number;
  readonly verticalMerge?: string;
  readonly kind: "cell";
  readonly blocks: readonly ImportedCellBlock[];
}

export type ImportedRowChild = ImportedTableCell | ImportedOpaque;

export interface ImportedTableRow {
  readonly properties: Readonly<Record<string, Readonly<Record<string, string>>>>;
  readonly kind: "row";
  readonly children: readonly ImportedRowChild[];
}

export type ImportedTableChild = ImportedTableRow | ImportedOpaque;

export interface ImportedTable {
  readonly properties: Readonly<Record<string, Readonly<Record<string, string>>>>;
  readonly layout?: TableLayout;
  readonly gridWidths: readonly (number | null)[];
  readonly kind: "table";
  readonly anchor: TableAnchor;
  /** Physical rows; merged cells are not expanded. */
  readonly children: readonly ImportedTableChild[];
}
export interface LocatedTable {
  readonly anchor: TableAnchor; readonly partName: string; readonly path: readonly number[];
  readonly depth: number; readonly entryId?: string;
  readonly properties: ImportedTable["properties"]; readonly gridWidths: ImportedTable["gridWidths"];
  readonly rowProperties: readonly ImportedTable["properties"][];
  readonly cellProperties: readonly (readonly ImportedTable["properties"][])[];
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
export type ParagraphFlag = "keepWithNext" | "keepTogether" | "pageBreakBefore" | "widowControl" | "suppressLineNumbers" | "bidi" | "snapToGrid" | "contextualSpacing" | "mirrorIndents";
export type SectionFlag = "titlePage" | "bidi" | "rtlGutter";
export interface ImportedSection {
  readonly anchor: SectionAnchor;
  readonly layout: DirectSectionLayout;
  readonly flags: Readonly<Partial<Record<SectionFlag, boolean>>>;
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
  partTables(ownerPart?: string): readonly LocatedTable[];
  /** Undefined when the document has no styles relationship. */
  styles(): ImportedStyles | undefined;
  coreTitle(): string | undefined;
  coreSubject(): string | undefined;
  metadataField(field: MetadataField): string | undefined;
  customProperty(name: string): string | boolean | undefined;
  defaultAlignment(): ImportedAlignment | undefined;
  documentDefaults(): Partial<DefaultValues>;
  abstractStarts(): readonly AbstractStart[];
  numberingInstances(): readonly ImportedNumberingInstance[];
  paragraphNumbering(target: ParagraphAnchor): { readonly id: number; readonly level: number } | undefined;
}

export interface AbstractStart {
  readonly abstractId: number;
  readonly level: number;
  readonly start?: number;
}

export interface ImportedNumberingInstance {
  readonly replacementLevels: readonly number[];
  readonly numId: number;
  readonly abstractId: number;
  readonly startOverrides: readonly { readonly level: number; readonly start: number }[];
}

export interface OpenDocxOptions {
  readonly readLimits?: ReadLimits;
}

/** Open an immutable Transitional DOCX snapshot from the exact supplied byte view. */
export function openDocx(bytes: Uint8Array, options?: OpenDocxOptions): Promise<ImportedDocument>;
