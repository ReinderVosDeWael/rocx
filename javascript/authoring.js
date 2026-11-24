// Factories only assemble declaration records. Keep values, conditions and
// children untouched so the existing resolver owns evaluation and validation.
function paragraphText(text, options) {
  return { ...options, kind: "paragraph", text };
}

function paragraphRuns(runs, options) {
  return { ...options, kind: "paragraph", runs };
}

const Paragraph = Object.freeze(Object.assign(
  function Paragraph(textOrRuns, options) {
    return typeof textOrRuns === "string"
      ? paragraphText(textOrRuns, options)
      : paragraphRuns(textOrRuns, options);
  },
  { Text: paragraphText, Runs: paragraphRuns },
));

export const declarative = Object.freeze({
  Document: (sections, options) => ({ ...options, sections }),
  Section: (children, options) => ({ ...options, kind: "section", children }),
  Header: (children, options) => ({ ...options, kind: "header", children }),
  Footer: (children, options) => ({ ...options, kind: "footer", children }),
  Paragraph,
  TextRun: (text, options) => ({ ...options, kind: "text", text }),
  Table: (rows, options) => ({ ...options, kind: "table", rows }),
  TableRow: (cells, options) => ({ ...options, kind: "tableRow", children: cells }),
  TableCell: (children, options) => ({ ...options, kind: "tableCell", children }),
  ImageRun: (data, options) => ({ ...options, kind: "image", data }),
  ShapeRun: (properties, options) => ({ ...options, kind: "shape", properties }),
  ColumnChartRun: (properties, options) => ({ ...options, kind: "columnChart", properties }),
  InlineFraction: (numerator, denominator, options) => ({
    ...options, kind: "fraction", numerator, denominator,
  }),
  Tab: (options) => ({ ...options, kind: "tab" }),
  Break: (options) => ({ ...options, kind: "break" }),
  Hyperlink: (target, children, options) => ({ ...options, kind: "hyperlink", target, children }),
  Field: (instruction, children, options) => ({ ...options, kind: "field", instruction, children }),
  Note: (target, children, options) => ({ ...options, kind: "note", target, children }),
  NoteReference: (target, options) => ({ ...options, kind: "noteReference", target }),
  Comment: (children, options) => ({ ...options, kind: "comment", children }),
  RangeStart: (id, range, options) => ({ ...options, kind: "rangeStart", id, range }),
  RangeEnd: (id, options) => ({ ...options, kind: "rangeEnd", id }),
  InlineContentControl: (children, options) => ({ ...options, kind: "contentControl", children }),
  BlockContentControl: (children, options) => ({ ...options, kind: "contentControl", children }),
  ParagraphStyleDefinition: (name, options) => ({ ...options, kind: "paragraph", name }),
  CharacterStyleDefinition: (name, options) => ({ ...options, kind: "character", name }),
  NumberingStyleDefinition: (name, numberingId) => ({ kind: "numbering", name, numberingId }),
  TableStyleDefinition: (name, options) => ({ ...options, kind: "table", name }),
});
