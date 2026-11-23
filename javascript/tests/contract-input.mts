import type { Block, DocumentDeclaration, GenerateOptions, Run } from "rocx-docx";

// Unlike a normal example, cover every field (even optional ones) and every
// discriminated variant. Adding either to the public types requires a sample.
type Variants<T extends { kind: string }> = {
  [Kind in T["kind"]]: Required<Extract<T, { kind: Kind }>>;
};

export function contractInput(bytes: Uint8Array) {
  const runs = {
    text: { kind: "text", text: "Contract sample", bold: true, italic: true },
    image: { kind: "image", bytes, altText: "Contract image" },
  } satisfies Variants<Run>;
  const blocks = {
    paragraph: { kind: "paragraph", runs: Object.values(runs) },
    table: { kind: "table", rows: [["Contract cell"]] },
  } satisfies Variants<Block>;
  const declaration = { blocks: Object.values(blocks) } satisfies Required<DocumentDeclaration>;
  const options = { timestamp: "2026-11-21T00:00:00Z" } satisfies Required<GenerateOptions>;
  return [declaration, options] as const;
}
