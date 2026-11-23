export interface DocumentDeclaration {
  /** Blocks in a single section with default page layout. */
  readonly blocks: readonly Block[];
}

export type Block = Paragraph | Table;
export type Run = TextRun | ImageRun | Tab | Break;
export type Inline = Run | Hyperlink;

export type Alignment =
  | "left" | "center" | "right" | "justify" | "distribute"
  | "justifyMedium" | "justifyHigh" | "justifyLow" | "thaiJustify";

export interface Paragraph {
  readonly kind: "paragraph";
  readonly runs: readonly Inline[];
  readonly alignment?: Alignment;
}

export interface TextRun {
  readonly kind: "text";
  readonly text: string;
  readonly bold?: boolean;
  readonly italic?: boolean;
}

export interface ImageRun {
  readonly kind: "image";
  /** Encoded image bytes (not pixels, a path, or a URL). */
  readonly bytes: Uint8Array;
  readonly altText?: string;
  /** Width in points. If only one dimension is given, preserve the aspect ratio. */
  readonly width?: number;
  /** Height in points. If neither dimension is given, use the intrinsic size. */
  readonly height?: number;
}

export interface Tab {
  readonly kind: "tab";
}

export interface Break {
  readonly kind: "break";
  /** Defaults to a line break. */
  readonly breakType?: "line" | "page" | "column" | "textWrapping";
}

export interface Hyperlink {
  readonly kind: "hyperlink";
  /** Absolute external URI/IRI; no internal bookmarks. */
  readonly target: string;
  readonly runs: readonly Run[];
}

export interface TableCell {
  /** Ordered blocks, including paragraphs and nested tables. */
  readonly blocks: readonly Block[];
}

export interface Table {
  readonly kind: "table";
  /** Nonempty rectangular grid; a string is shorthand for a plain paragraph. */
  readonly rows: readonly (readonly (string | TableCell)[])[];
}

export interface GenerateOptions {
  /** UTC ISO timestamp with seconds and optional milliseconds; defaults to now. */
  readonly timestamp?: string;
}

/** An adapter or rendering error, with context when available. */
export interface GenerateDocxError extends Error {
  /** Location in the JavaScript declaration, when known. */
  readonly path?: string;
  /** Stable diagnostic code from the rocx engine, when available. */
  readonly rule?: string;
}

/** Generate a complete DOCX package. Rejects with an Error on failure. */
export function generateDocx(
  declaration: DocumentDeclaration,
  options?: GenerateOptions,
): Promise<Uint8Array<ArrayBuffer>>;
