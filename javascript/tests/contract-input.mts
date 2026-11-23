import type { Block, DocumentDeclaration, GenerateOptions, Inline, Run, TableCell } from "rocx-docx";

// Unlike a normal example, cover every field (even optional ones) and every
// discriminated variant. Adding either to the public types requires a sample.
type Variants<T extends { kind: string }> = {
  [Kind in T["kind"]]: Required<Extract<T, { kind: Kind }>>;
};

export function contractInput(bytes: Uint8Array) {
  const runs = {
    text: { kind: "text", text: "Contract sample", bold: true, italic: true },
    image: { kind: "image", bytes, altText: "Contract image", width: 24, height: 36 },
    tab: { kind: "tab" },
    break: { kind: "break", breakType: "line" },
  } satisfies Variants<Run>;
  const inlines = {
    ...runs,
    hyperlink: { kind: "hyperlink", target: "https://example.org/", runs: [{ ...runs.text }] },
  } satisfies Variants<Inline>;
  const paragraph = { kind: "paragraph", runs: Object.values(inlines), alignment: "right" } as const;
  // The runtime test mutates each location separately, so don't share objects.
  const cell = { blocks: [structuredClone(paragraph)] } satisfies Required<TableCell>;
  const blocks = {
    paragraph,
    table: { kind: "table", rows: [["Contract cell", cell]] },
  } satisfies Variants<Block>;
  const declaration = { blocks: Object.values(blocks) } satisfies Required<DocumentDeclaration>;
  const options = { timestamp: "2026-11-21T00:00:00Z" } satisfies Required<GenerateOptions>;
  return [declaration, options] as const;
}
