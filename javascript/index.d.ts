export interface DocumentDeclaration {
  /** Blocks in a single section with default page layout. */
  readonly blocks: readonly Block[];
}

export type Block = Paragraph | Table;
export type Run = TextRun | ImageRun;

export interface Paragraph {
  readonly kind: "paragraph";
  readonly runs: readonly Run[];
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
}

export interface Table {
  readonly kind: "table";
  /** Nonempty rectangular grid of plain-text cells. */
  readonly rows: readonly (readonly string[])[];
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
