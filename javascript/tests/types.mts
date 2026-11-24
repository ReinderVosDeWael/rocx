import { generateDocx, type DocumentDeclaration, type GenerateDocxError, type Hyperlink } from "rocx-docx";

const declaration = {
  blocks: [
    { kind: "paragraph", runs: [
      { kind: "text", text: "test", bold: true },
      { kind: "image", bytes: new Uint8Array([1]), altText: "An image" },
    ] },
    { kind: "table", rows: [["A", "B"]] },
  ],
} as const satisfies DocumentDeclaration;
const result: Promise<Uint8Array> = generateDocx(declaration);
void result;
new Blob([await generateDocx(declaration)]);

function errorContext(error: GenerateDocxError): [string | undefined, string | undefined] {
  return [error.path, error.rule];
}
void errorContext;

// @ts-expect-error Images take bytes, not a filename.
void generateDocx({ blocks: [{ kind: "paragraph", runs: [{ kind: "image", bytes: "test.png" }] }] });
// @ts-expect-error Table cells are text or an object containing blocks.
void generateDocx({ blocks: [{ kind: "table", rows: [[42]] }] });
const nestedLink = {
  kind: "hyperlink", target: "https://example.org", runs: [
    // @ts-expect-error Hyperlinks cannot contain another hyperlink.
    { kind: "hyperlink", target: "https://example.org", runs: [] },
  ],
} satisfies Hyperlink;
void nestedLink;
