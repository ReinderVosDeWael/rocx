// docs:start declaration
import { generateDocx } from "rocx-docx";

// The same byte-only declaration runs in Node and browsers.
export function makeReport(imageBytes) {
  return generateDocx({
    blocks: [
      { kind: "paragraph", runs: [
        { kind: "text", text: "JavaScript authoring: ", bold: true, italic: false },
        { kind: "text", text: "Café 東京", italic: true },
      ] },
      { kind: "table", rows: [["Item", "Count"], ["Pages", "2"]] },
      { kind: "paragraph", runs: [
        { kind: "image", bytes: imageBytes, altText: "JavaScript authoring sample image" },
      ] },
    ],
  }, { timestamp: "2026-11-21T00:00:00Z" });
}
// docs:end declaration
