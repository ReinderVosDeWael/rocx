import type { RenderOptions } from "./boundary.js";
import type { ResolvedDocument } from "./text-layout.js";

export type EditScope = "body" | "bodyHeadersFooters" | "supportedStories";
export type TextRevisionAction = "accept" | "reject";

export interface TemplateEdits {
  /** Applied in order to the source stories, before inserting new declarations. */
  readonly replacements?: readonly (readonly [needle: string, replacement: string])[];
  readonly contentControlReplacements?: Readonly<Record<string, string>>;
  readonly textRevisionAction?: TextRevisionAction;
  /** Zero-based among original direct body paragraphs; past the end appends. */
  readonly paragraphIndex?: number;
  readonly scope?: EditScope;
}

export interface TemplateSource {
  readonly bytes: Uint8Array;
  readonly edits?: TemplateEdits;
}

export type PackagePart = readonly [name: string, payload: Uint8Array<ArrayBuffer>];

declare const packagePlan: unique symbol;
export interface PackagePlan {
  readonly [packagePlan]: true;
  /** Sorted by ZIP entry name. Payloads are independent copies. */
  parts(): readonly PackagePart[];
  /** Exact, case-sensitive lookup by OPC part name (leading slash optional). */
  part(name: string): Uint8Array<ArrayBuffer> | undefined;
  validate(): void;
  toBytes(): Uint8Array<ArrayBuffer>;
}

export function compileTemplate(
  document: ResolvedDocument,
  source: TemplateSource,
  options?: RenderOptions,
): Promise<PackagePlan>;

declare module "./text-layout.js" {
  interface ResolvedDocument {
    toDocxWithTemplate(source: TemplateSource, options?: RenderOptions): Uint8Array<ArrayBuffer>;
  }
}
