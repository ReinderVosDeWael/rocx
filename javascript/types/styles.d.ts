import type { Rgb } from "./boundary.js";
import type { V } from "./resolution.js";
import type {
  Alignment, LineHeight, ParagraphFormat, ResolvedParagraphFormat, ResolvedRunFormat, RunFormat,
  RunLanguage, TabStop,
} from "./text-layout.js";

export interface DocumentStyleFields {
  readonly styles?: V<readonly StyleDefinition[]>;
  readonly numbering?: V<readonly NumberingDefinition[]>;
}

export interface DocumentStyleView {
  readonly styles: readonly ResolvedStyleDefinition[];
  readonly numbering: readonly ResolvedNumberingDefinition[];
}

interface ScriptStyleFormat {
  readonly language?: RunLanguage;
  readonly boldComplexScript?: boolean;
  readonly italicComplexScript?: boolean;
  readonly sizeComplexScript?: number;
  readonly rtl?: boolean;
  readonly complexScript?: boolean;
  readonly fontEastAsia?: string;
  readonly fontComplexScript?: string;
}

interface StyleTextFormat extends ScriptStyleFormat {
  readonly bold?: boolean;
  readonly italic?: boolean;
  readonly underline?: boolean;
  readonly font?: string;
  readonly fontSize?: number;
  readonly color?: Rgb;
  readonly alignment?: Alignment;
  readonly spacingBefore?: number;
  readonly spacingAfter?: number;
  readonly lineHeight?: LineHeight;
  readonly tabStops?: readonly TabStop[];
  readonly bidi?: boolean;
}

export interface ParagraphStyleDefinition extends StyleTextFormat {
  readonly kind: "paragraph";
  readonly name: string;
  readonly baseStyle?: string;
  readonly nextParagraphStyle?: string;
  readonly linkedStyle?: string;
  /** Legacy multiple of a line; mutually exclusive with lineHeight. */
  readonly lineSpacing?: number;
  /** Inches, unlike point-based ParagraphFormat indents. */
  readonly leftIndent?: number;
  /** Inches, unlike point-based ParagraphFormat indents. */
  readonly rightIndent?: number;
  /** Inches, unlike point-based ParagraphFormat indents. */
  readonly firstLineIndent?: number;
  readonly keepTogether?: boolean;
  readonly keepWithNext?: boolean;
  readonly pageBreakBefore?: boolean;
  readonly widowControl?: boolean;
  readonly suppressLineNumbers?: boolean;
  readonly contextualSpacing?: boolean;
  readonly mirrorIndents?: boolean;
  readonly snapToGrid?: boolean;
}

export interface CharacterStyleDefinition {
  readonly kind: "character";
  readonly name: string;
  readonly baseStyle?: string;
  readonly linkedStyle?: string;
  readonly format?: RunFormat;
}

export interface NumberingStyleDefinition {
  readonly kind: "numbering";
  readonly name: string;
  /** ID of an authored concrete instance with local levels. */
  readonly numberingId: number;
}

export interface TableSectionFormat extends StyleTextFormat {
  readonly background?: Rgb;
}

interface TableStyleRegions {
  readonly wholeTable?: TableSectionFormat;
  readonly firstRow?: TableSectionFormat;
  readonly lastRow?: TableSectionFormat;
  readonly firstColumn?: TableSectionFormat;
  readonly lastColumn?: TableSectionFormat;
  readonly banding1Row?: TableSectionFormat;
  readonly banding2Row?: TableSectionFormat;
  readonly banding1Column?: TableSectionFormat;
  readonly banding2Column?: TableSectionFormat;
  readonly topLeftCell?: TableSectionFormat;
  readonly topRightCell?: TableSectionFormat;
  readonly bottomLeftCell?: TableSectionFormat;
  readonly bottomRightCell?: TableSectionFormat;
}

export interface TableStyleDefinition extends TableStyleRegions {
  readonly kind: "table";
  readonly name: string;
  readonly baseStyle?: string;
}

export type StyleDefinition =
  | ParagraphStyleDefinition | CharacterStyleDefinition | NumberingStyleDefinition | TableStyleDefinition;

type ResolvedTabStops = Pick<ResolvedParagraphFormat, "tabStops">;

export type ResolvedStyleDefinition =
  | (ParagraphStyleDefinition & ResolvedTabStops)
  | (Omit<CharacterStyleDefinition, "format"> & { readonly format: ResolvedRunFormat })
  | NumberingStyleDefinition
  | (Omit<TableStyleDefinition, keyof TableStyleRegions> & {
    readonly [K in keyof TableStyleRegions]?: TableSectionFormat & ResolvedTabStops;
  });

export type NumberingAlignment = "left" | "center" | "right" | "start" | "end";
export type NumberingSuffix = "tab" | "space" | "nothing";

export interface NumberingReference {
  readonly id: number;
  readonly level: number;
}

export interface NumberingStartOverride {
  readonly level: number;
  readonly start: number;
}

export interface NumberingLevel {
  readonly level: number;
  readonly start: number;
  readonly format: string;
  readonly text: string;
  readonly isLegal?: boolean;
  readonly suffix?: NumberingSuffix;
  /** One-based restart trigger; zero requests no restart. */
  readonly restart?: number;
  readonly alignment?: NumberingAlignment;
  readonly labelFormat?: RunFormat;
  /** Lengths use points. */
  readonly paragraphFormat?: ParagraphFormat;
  readonly paragraphStyle?: string;
}

export interface NumberingDefinition {
  readonly id: number;
  readonly abstractId?: number;
  readonly inheritStyle?: string;
  readonly levels?: readonly NumberingLevel[];
  readonly startOverrides?: readonly NumberingStartOverride[];
  readonly levelOverrides?: readonly NumberingLevel[];
}

type ResolvedNumberingLevel = Omit<NumberingLevel, "labelFormat" | "paragraphFormat"> & {
  readonly labelFormat?: ResolvedRunFormat;
  readonly paragraphFormat?: ResolvedParagraphFormat;
};

export type ResolvedNumberingDefinition = Omit<NumberingDefinition, "levels" | "startOverrides" | "levelOverrides"> & {
  readonly levels: readonly ResolvedNumberingLevel[];
  readonly startOverrides: readonly NumberingStartOverride[];
  readonly levelOverrides: readonly ResolvedNumberingLevel[];
};

declare module "./resolution.js" {
  interface ValidationValues {
    readonly numberingDefinition: NumberingDefinition;
  }
}
