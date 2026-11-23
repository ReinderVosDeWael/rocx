import type { Condition, RenderOptions, ResolveOptions, ResolvedHandle, Rgb } from "./boundary.js";
import type { V } from "./resolution.js";
import type { DrawingComponents, ImageRunDeclaration, ResolvedImageRun } from "./drawings.js";
import type {
  ResolvedComment, TextRevision,
  StructuredBlockComponents, StructuredDocumentFields, StructuredDocumentView,
  StructuredInlineComponents, StructuredStandaloneComponents,
} from "./structured.js";
import type { BorderLine, TableComponents, TableStandaloneComponents, VerticalAlignment } from "./tables.js";
import type { DocumentStyleFields, DocumentStyleView, NumberingReference } from "./styles.js";

export interface Metadata {
  readonly creator?: string;
  readonly title?: string;
  readonly subject?: string;
  readonly description?: string;
  readonly keywords?: string;
  readonly category?: string;
  readonly comments?: string;
  readonly version?: string;
  readonly company?: string;
  readonly manager?: string;
  readonly customProperties?: Readonly<Record<string, string>>;
  readonly customBooleanProperties?: Readonly<Record<string, boolean>>;
}
export interface DocumentTextFields {
  readonly metadata?: V<Metadata>;
  readonly defaults?: V<DocumentDefaults | undefined>;
}
export interface DocumentTextView {
  readonly metadata: Metadata & Required<Pick<Metadata, "customProperties" | "customBooleanProperties">>;
  readonly defaults?: ResolvedDocumentDefaults;
}
export interface TextLayoutComponents {
  readonly header: { readonly declaration: HeaderDeclaration; readonly resolved: ResolvedHeaderFooter };
  readonly footer: { readonly declaration: FooterDeclaration; readonly resolved: ResolvedHeaderFooter };
}

export type Alignment = "left" | "center" | "right" | "justify" | "distribute" |
  "justifyMedium" | "justifyHigh" | "justifyLow" | "thaiJustify";
export type Orientation = "portrait" | "landscape";
export type SectionType = "nextPage" | "nextColumn" | "continuous" | "evenPage" | "oddPage";
export type HeaderFooterType = "default" | "first" | "even";
export type LineNumberRestart = "newPage" | "newSection" | "continuous";
export type PageBorderDisplay = "allPages" | "firstPage" | "notFirstPage";
export type PageBorderOffset = "page" | "text";
export type PageBorderZOrder = "front" | "back";
export type DocumentGridMode = "default" | "lines" | "linesAndChars" | "snapToChars";
/** Physical integer lengths given in EMUs, not points or twips. */
export interface PageSize {
  readonly width?: number | bigint;
  readonly height?: number | bigint;
}
/** Physical integer lengths given in EMUs. Omitted fields retain template values. */
export interface PageMargins {
  readonly top?: number | bigint;
  readonly bottom?: number | bigint;
  readonly left?: number | bigint;
  readonly right?: number | bigint;
  readonly header?: number | bigint;
  readonly footer?: number | bigint;
}
export interface Columns {
  readonly count?: number;
  /** Gap in twips. */
  readonly space?: number;
  readonly separator?: boolean;
}
export interface PageNumbering {
  readonly start?: number;
  readonly format?: string;
}
export interface LineNumbering {
  readonly clear?: boolean;
  readonly countBy?: number;
  readonly start?: number;
  /** Gap in twips. */
  readonly distance?: number;
  readonly restart?: LineNumberRestart;
}
export interface PageBorder {
  readonly style: BorderLine;
  /** Width in eighths of a point. */
  readonly size?: number;
  /** Offset in points. */
  readonly space?: number;
  readonly color?: Rgb;
}
export interface PageBorders {
  readonly offsetFrom?: PageBorderOffset;
  readonly display?: PageBorderDisplay;
  readonly zOrder?: PageBorderZOrder;
  readonly top?: PageBorder;
  readonly left?: PageBorder;
  readonly bottom?: PageBorder;
  readonly right?: PageBorder;
}
export interface DocumentGrid {
  readonly mode?: DocumentGridMode;
  readonly linePitch?: number;
  /** Signed pitch difference in 1/4096-point units. */
  readonly characterPitchDelta?: number;
}
export interface SectionProperties {
  readonly pageSize?: PageSize;
  readonly pageOrientation?: Orientation;
  readonly pageMargins?: PageMargins;
  readonly pageBorders?: PageBorders;
  readonly documentGrid?: DocumentGrid;
  readonly pageNumbering?: PageNumbering;
  readonly lineNumbering?: LineNumbering;
  readonly columns?: Columns;
  readonly verticalAlign?: VerticalAlignment;
  readonly titlePage?: boolean;
  readonly type?: SectionType;
  readonly bidi?: boolean;
  readonly rtlGutter?: boolean;
}
export type ResolvedSectionProperties = Omit<SectionProperties, "lineNumbering"> & {
  readonly lineNumbering?: LineNumbering & Required<Pick<LineNumbering, "clear">>;
};

/** Values are points for exact/minimum, lines for multiplier. */
export type LineHeight =
  | { readonly kind: "exact"; readonly value: number }
  | { readonly kind: "minimum"; readonly value: number }
  | { readonly kind: "multiplier"; readonly value: number };
export type TabAlignment = "left" | "center" | "right" | "decimal" | "bar" | "clear" | "start" | "end";
export type TabLeader = "none" | "dot" | "hyphen" | "underscore" | "heavy" | "middleDot";
export interface TabStop {
  /** Signed position in points. */
  readonly position?: number;
  readonly alignment?: TabAlignment;
  readonly leader?: TabLeader;
}
export interface ParagraphFormat {
  readonly bidi?: boolean;
  readonly heading?: number;
  readonly style?: string;
  readonly alignment?: Alignment;
  readonly spacingBefore?: number;
  readonly spacingAfter?: number;
  readonly lineSpacing?: number;
  readonly lineHeight?: LineHeight;
  readonly leftIndent?: number;
  readonly rightIndent?: number;
  readonly firstLineIndent?: number;
  readonly keepTogether?: boolean;
  readonly keepWithNext?: boolean;
  readonly pageBreakBefore?: boolean;
  readonly widowControl?: boolean;
  readonly suppressLineNumbers?: boolean;
  readonly contextualSpacing?: boolean;
  readonly mirrorIndents?: boolean;
  readonly snapToGrid?: boolean;
  readonly tabStops?: readonly TabStop[];
}
export type ThemeColor = "dark1" | "light1" | "dark2" | "light2" |
  "accent1" | "accent2" | "accent3" | "accent4" | "accent5" | "accent6" |
  "hyperlink" | "followedHyperlink" | "background1" | "text1" | "background2" | "text2";
export type Color = Rgb | "auto" | {
  readonly kind: "theme";
  readonly name: ThemeColor;
  readonly tint?: number;
  readonly shade?: number;
};
export type Underline = "none" | "single" | "double" | "dotted" | "dash" | "wave" | "words";
export interface RunLanguage {
  readonly latin?: string;
  readonly eastAsia?: string;
  readonly bidi?: string;
}
export interface RunFormat {
  readonly style?: string;
  readonly bold?: boolean;
  readonly italic?: boolean;
  readonly underline?: Underline | boolean;
  readonly font?: string;
  readonly size?: number;
  readonly color?: Color;
  readonly superscript?: boolean;
  readonly subscript?: boolean;
  readonly strike?: boolean;
  readonly allCaps?: boolean;
  readonly smallCaps?: boolean;
  readonly snapToGrid?: boolean;
  readonly language?: RunLanguage;
  readonly boldComplexScript?: boolean;
  readonly italicComplexScript?: boolean;
  readonly sizeComplexScript?: number;
  readonly rtl?: boolean;
  readonly complexScript?: boolean;
  readonly fontEastAsia?: string;
  readonly fontComplexScript?: string;
}
export type ResolvedRunFormat = Omit<RunFormat, "underline"> & { readonly underline?: Underline };
export type ResolvedParagraphFormat = ParagraphFormat & {
  readonly tabStops: readonly (TabStop & Required<Pick<TabStop, "position" | "alignment">>)[];
};

export type FootnotePosition = "pageBottom" | "beneathText";
export type EndnotePosition = "sectionEnd" | "documentEnd";
export type NoteNumberRestart = "continuous" | "eachSection" | "eachPage";
export type NoteNumberFormat = "decimal" | "lowerRoman" | "upperRoman" | "lowerLetter" | "upperLetter";
export interface NoteNumbering {
  readonly format?: NoteNumberFormat;
  readonly start?: number;
  readonly restart?: NoteNumberRestart;
}
export interface DocumentDefaults {
  readonly run?: RunFormat;
  readonly paragraph?: ParagraphFormat;
  readonly footnoteNumbering?: NoteNumbering;
  readonly endnoteNumbering?: NoteNumbering;
  readonly footnotePosition?: FootnotePosition;
  readonly endnotePosition?: EndnotePosition;
  readonly mirrorMargins?: boolean;
  readonly gutterAtTop?: boolean;
  readonly trackRevisions?: boolean;
}
export type ResolvedDocumentDefaults = Omit<DocumentDefaults, "run" | "paragraph"> & {
  readonly run?: ResolvedRunFormat;
  readonly paragraph?: ResolvedParagraphFormat;
};

export interface DocumentProgram extends DocumentStyleFields, DocumentTextFields, StructuredDocumentFields {
  readonly sections: readonly V<SectionDeclaration>[];
  readonly timestamp?: V<string | undefined>;
  readonly commentAuthor?: V<string | undefined>;
}
export interface SectionDeclaration {
  readonly kind: "section";
  readonly properties?: V<SectionProperties | undefined>;
  readonly children?: V<BlockChildren>;
  readonly headers?: Readonly<Partial<Record<HeaderFooterType, V<HeaderDeclaration>>>>;
  readonly footers?: Readonly<Partial<Record<HeaderFooterType, V<FooterDeclaration>>>>;
  readonly condition?: Condition;
}
export interface HeaderDeclaration {
  readonly kind: "header";
  readonly children: V<BlockChildren>;
  readonly condition?: Condition;
}
export interface FooterDeclaration {
  readonly kind: "footer";
  readonly children: V<BlockChildren>;
  readonly condition?: Condition;
}
interface SimpleComments {
  readonly commentText?: V<string>;
  readonly commentAuthor?: V<string>;
}
export type ParagraphDeclaration = SimpleComments & {
  readonly kind: "paragraph";
  readonly format?: V<ParagraphFormat>;
  readonly numbering?: V<NumberingReference | undefined>;
  readonly condition?: Condition;
} & (
  | { readonly text?: V<string>; readonly runs?: never }
  | { readonly text?: never; readonly runs: V<InlineChildren> }
);
export interface TextRunDeclaration extends SimpleComments {
  readonly kind: "text";
  readonly text: V<string>;
  readonly format?: V<RunFormat>;
  readonly revision?: V<TextRevision | undefined>;
  readonly condition?: Condition;
}

/** Module-owned custom component declarations and corresponding resolved handles. */
export type ComponentEntry = { readonly declaration: object; readonly resolved: object };
type Values<T> = T[keyof T];
type Declaration<T extends ComponentEntry> = T["declaration"];
type Resolved<T extends ComponentEntry> = T["resolved"];
type CustomBlocks = Values<TableComponents & StructuredBlockComponents>;
type CustomInlines = Values<DrawingComponents & StructuredInlineComponents>;

export type BlockChildren = readonly V<BlockDeclaration>[];
export type InlineChildren = readonly V<InlineDeclaration>[];
export type BlockDeclaration = ParagraphDeclaration | Declaration<CustomBlocks> |
  { readonly kind: "resolved"; readonly value: ResolvedBlock };
export type InlineDeclaration = TextRunDeclaration | ImageRunDeclaration | Declaration<CustomInlines> |
  { readonly kind: "resolved"; readonly value: ResolvedInline };

export type ResolvedBlock = ResolvedParagraph | Resolved<CustomBlocks>;
export type ResolvedInline = ResolvedTextRun | ResolvedImageRun | Resolved<CustomInlines>;
export type ResolvedTextRun = ResolvedHandle<"inline", {
  readonly kind: "text";
  readonly text: string;
  readonly displayText: string;
  readonly format: ResolvedRunFormat;
  readonly comment?: ResolvedComment;
  readonly revision?: TextRevision;
}>;
export type ResolvedParagraph = ResolvedHandle<"paragraph", {
  readonly text: string;
  readonly runs: readonly ResolvedInline[];
  readonly format: ResolvedParagraphFormat;
  readonly numbering?: NumberingReference;
  readonly comment?: ResolvedComment;
}>;
export type ResolvedHeaderFooter = ResolvedHandle<"headerFooter", {
  readonly blocks: readonly ResolvedBlock[];
}>;
export type ResolvedSection = ResolvedHandle<"section", {
  readonly properties?: ResolvedSectionProperties;
  readonly headers: Readonly<Partial<Record<HeaderFooterType, ResolvedHeaderFooter>>>;
  readonly footers: Readonly<Partial<Record<HeaderFooterType, ResolvedHeaderFooter>>>;
  readonly blocks: readonly ResolvedBlock[];
  readonly paragraphs: readonly ResolvedParagraph[];
}>;
export interface ResolvedDocumentView extends DocumentStyleView, DocumentTextView, StructuredDocumentView {
  readonly timestamp: string;
  readonly commentAuthor?: string;
  readonly sections: readonly ResolvedSection[];
}
export interface ResolvedDocument extends ResolvedHandle<"document", ResolvedDocumentView> {
  toDocx(options?: RenderOptions): Uint8Array<ArrayBuffer>;
}

export function resolveDocument(program: DocumentProgram, options?: ResolveOptions): Promise<ResolvedDocument>;
export function resolveComponent(declaration: SectionDeclaration, options?: ResolveOptions): Promise<ResolvedSection | undefined>;
export function resolveComponent(declaration: ParagraphDeclaration, options?: ResolveOptions): Promise<ResolvedParagraph | undefined>;
export function resolveComponent(declaration: TextRunDeclaration, options?: ResolveOptions): Promise<ResolvedTextRun | undefined>;
export function resolveComponent(declaration: ImageRunDeclaration, options?: ResolveOptions): Promise<ResolvedImageRun | undefined>;

type Unscoped<T> = T extends ComponentEntry ? T extends { readonly scope: string } ? never : T : never;
type Scoped<T, S> = T extends { readonly scope: S } ? T : never;
type Matching<T, D> = T extends ComponentEntry ? D extends T["declaration"] ? T["resolved"] : never : never;
type Custom = CustomBlocks | CustomInlines |
  Values<TextLayoutComponents & TableStandaloneComponents & StructuredStandaloneComponents>;

export function resolveComponent<D extends Declaration<Unscoped<Custom>>>(
  declaration: D, options?: ResolveOptions,
): Promise<Matching<Unscoped<Custom>, D> | undefined>;
export function resolveComponent<S extends "inline" | "block", D extends Declaration<Scoped<Custom, S>>>(
  declaration: D, options: ResolveOptions & { readonly scope: S },
): Promise<Matching<Scoped<Custom, S>, D> | undefined>;

declare module "./resolution.js" {
  interface ValidationValues {
    readonly runFormat: RunFormat;
    readonly paragraphFormat: ParagraphFormat;
    readonly sectionProperties: SectionProperties;
    readonly documentDefaults: DocumentDefaults;
  }
}
