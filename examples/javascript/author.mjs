// docs:start declaration
import { declarative as d, generateDocx } from "rocx-docx";

// The same byte-only declaration runs in Node and browsers.
export function reportBlocks(imageBytes) {
  const textCell = (text) => d.TableCell([d.Paragraph(text)]);
  const details = d.TableCell([
    d.Paragraph([
      d.TextRun("2", { format: { bold: true } }),
      d.Tab(),
      d.Hyperlink({ kind: "external", url: "https://example.org/report" }, [
        d.TextRun("Report details", { format: { italic: true } }),
      ]),
      d.Break(),
      d.TextRun("Verified"),
    ]),
    d.Table([d.TableRow([textCell("Nested detail")])]),
  ]);
  return [
    d.Paragraph([
      d.TextRun("JavaScript authoring: ", { format: { bold: true, italic: false } }),
      d.TextRun("Café 東京", { format: { italic: true } }),
    ], { format: { alignment: "center" } }),
    d.Table([
      d.TableRow([textCell("Item"), textCell("Details")]),
      d.TableRow([textCell("Pages"), details]),
    ]),
    d.Paragraph([d.ImageRun(imageBytes, {
      transformation: { width: 72, height: 36 },
      altText: { description: "JavaScript authoring sample image" },
    })]),
  ];
}

export function makeReport(imageBytes) {
  return generateDocx(d.Document([d.Section(reportBlocks(imageBytes))], {
    timestamp: "2026-11-21T00:00:00Z",
  }));
}
// docs:end declaration
