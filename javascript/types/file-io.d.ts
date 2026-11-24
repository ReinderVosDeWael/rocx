import type { RenderOptions } from "./boundary.js";
import type { NamedImageData } from "./drawings.js";
import type { Value } from "./resolution.js";
import type { TemplateEdits, TemplateSource } from "./template.js";
import type { ResolvedDocument } from "./text-layout.js";

export * from "../index.js";

declare const templateFileSource: unique symbol;
export interface TemplateFileSource {
  readonly [templateFileSource]: true;
}

/** Read a local DOCX or DOTX each time it is used for rendering or saving. */
export function templateFile(path: string, edits?: TemplateEdits): TemplateFileSource;

/** Read image bytes only when resolved and retain the path's basename. Single-use. */
export function imageFile(path: string): Value<NamedImageData>;

export interface SaveDocxOptions extends RenderOptions {
  readonly template?: TemplateSource | TemplateFileSource;
}

/** Encode fully, then replace the local destination via a sibling temporary file. */
export function saveDocx(document: ResolvedDocument, path: string, options?: SaveDocxOptions): Promise<void>;

declare module "./text-layout.js" {
  interface ResolvedDocument {
    toDocxWithTemplate(source: TemplateFileSource, options?: RenderOptions): Promise<Uint8Array<ArrayBuffer>>;
  }
}
