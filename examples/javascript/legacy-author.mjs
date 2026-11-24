// docs:start declaration
import { generateDocx } from "rocx-docx";

// Compatibility example: the original single-section record API.
export function makeLegacyReport(imageBytes) {
  return generateDocx({
    blocks: [
      { kind: "paragraph", alignment: "center", runs: [
        { kind: "text", text: "JavaScript authoring: ", bold: true, italic: false },
        { kind: "text", text: "Café 東京", italic: true },
      ] },
      { kind: "table", rows: [
        ["Item", "Details"],
        ["Pages", { blocks: [
          { kind: "paragraph", runs: [
            { kind: "text", text: "2", bold: true },
            { kind: "tab" },
            { kind: "hyperlink", target: "https://example.org/report", runs: [
              { kind: "text", text: "Report details", italic: true },
            ] },
            { kind: "break", breakType: "line" },
            { kind: "text", text: "Verified" },
          ] },
          { kind: "table", rows: [["Nested detail"]] },
        ] }],
      ] },
      { kind: "paragraph", runs: [
        { kind: "image", bytes: imageBytes, altText: "JavaScript authoring sample image",
          width: 72, height: 36 },
      ] },
    ],
  }, { timestamp: "2026-11-21T00:00:00Z" });
}
// docs:end declaration
