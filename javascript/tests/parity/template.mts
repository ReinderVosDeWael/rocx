import {
  compileTemplate, future, resolveDocument,
  type DocumentProgram, type PackagePart, type PackagePlan, type ReadLimits,
  type RenderOptions, type ResolvedDocument, type TemplateEdits, type TemplateSource,
} from "rocx-docx";
import { imageFile, saveDocx, templateFile, type SaveDocxOptions } from "rocx-docx/node";

export const snapshotDate = "2026-11-21T00:00:00Z";
export const outputDate = "2026-12-01T12:30:00.123Z";
export const clean = () => resolveDocument({ sections: [], timestamp: snapshotDate });
export const report = (count: () => void) => resolveDocument({
  sections: [{
    kind: "section",
    children: [{ kind: "paragraph", text: future(async () => { count(); return "TOKEN"; }) }],
  }],
  timestamp: snapshotDate,
} satisfies DocumentProgram);

export const indexed = () => resolveDocument({
  sections: [
    { kind: "section", children: [{ kind: "paragraph", text: "Added section one" }] },
    { kind: "section", children: [{ kind: "paragraph", text: "Added section two" }] },
  ],
  timestamp: snapshotDate,
} satisfies DocumentProgram);

export const merge = () => resolveDocument({
  sections: [{
    kind: "section",
    children: [{ kind: "paragraph", text: "Merged body" }],
    properties: { pageMargins: { left: 914400 } },
    headers: { default: { kind: "header", children: [{ kind: "paragraph", text: "Merged header" }] } },
  }],
  metadata: { title: "Merged title" },
  styles: [{ kind: "paragraph", name: "Normal", keepWithNext: true }],
  timestamp: snapshotDate,
} satisfies DocumentProgram);

export const revisionReport = () => resolveDocument({
  sections: [{
    kind: "section",
    children: [{ kind: "paragraph", runs: [{
      kind: "text", text: "New author revision",
      revision: { kind: { kind: "insertion" }, author: "New author", date: snapshotDate },
    }] }],
  }],
  timestamp: snapshotDate,
} satisfies DocumentProgram);

export const override = { profile: "transitional", timestamp: outputDate } satisfies RenderOptions;
export const strict = { profile: "strict" } satisfies RenderOptions;
export function budgets(maxParts: number, maxPartBytes: number, maxExpandedBytes: number): ReadLimits {
  return { maxParts, maxPartBytes, maxExpandedBytes };
}
export const losslessBudgets = {
  maxParts: 4_294_967_295,
  maxPartBytes: 9_007_199_254_740_993n,
  maxExpandedBytes: 18_446_744_073_709_551_615n,
} satisfies ReadLimits;

export const ordered = {
  replacements: [["TOKEN", "first"], ["first", "second"], ["TOKEN", "WRONG"],
    ["DELETE", ""], ["left\tright", "tabs"], ["ABCD", "across marker"]],
} satisfies TemplateEdits;
export const scopes = ["body", "bodyHeadersFooters", "supportedStories"] satisfies TemplateEdits["scope"][];
export const actions = ["accept", "reject"] satisfies TemplateEdits["textRevisionAction"][];
export const controlEdits = {
  replacements: [["ordinary", "changed"], ["TARGET", "must not run after control replacement"]],
  contentControlReplacements: { client: "TARGET" },
} satisfies TemplateEdits;
export const oracleEdits = {
  paragraphIndex: 1, replacements: [["Insert table after me", "Patched"]],
} satisfies TemplateEdits;

export function source(bytes: Uint8Array, edits?: TemplateEdits): TemplateSource {
  return { bytes, edits };
}

export function openImage(data: Uint8Array | string, condition = true) {
  return resolveDocument({
    timestamp: snapshotDate,
    sections: [{
      kind: "section",
      children: [{ kind: "paragraph", runs: [{
        kind: "image", condition, data: typeof data === "string" ? imageFile(data) : data,
      }] }],
    }],
  });
}

export const newTemplateFile = (path: string, edits?: TemplateEdits) => templateFile(path, edits);
export const save = (document: ResolvedDocument, path: string, options?: SaveDocxOptions) =>
  saveDocx(document, path, options);

// Compile-time output and byte-only constraint checks; never run this function.
async function typeContracts(document: ResolvedDocument, bytes: Uint8Array, path: string) {
  const plan: PackagePlan = await compileTemplate(document, source(bytes));
  const parts: readonly PackagePart[] = plan.parts();
  const payload: Uint8Array<ArrayBuffer> | undefined = plan.part("word/document.xml");
  const encoded: Uint8Array<ArrayBuffer> = plan.toBytes();
  const fresh: Uint8Array<ArrayBuffer> = document.toDocx(override);
  document.toDocx({ readLimits: losslessBudgets });
  // @ts-expect-error Part count is a number, not a byte-budget bigint.
  document.toDocx({ readLimits: { maxParts: 1n } });
  const fromBytes: Uint8Array<ArrayBuffer> = document.toDocxWithTemplate(source(bytes));
  const fromFile: Promise<Uint8Array<ArrayBuffer>> = document.toDocxWithTemplate(newTemplateFile(path));
  const validated: void = plan.validate();
  void [parts, payload, encoded, fresh, fromBytes, fromFile, validated];
  // @ts-expect-error filesystem sources are not allowed in byte-only compile
  await compileTemplate(document, newTemplateFile(path));
  // @ts-expect-error replacements are ordered tuples, not an object
  source(bytes, { replacements: { x: "y" } });
}
void typeContracts;
