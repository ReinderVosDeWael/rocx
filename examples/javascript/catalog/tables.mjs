import { declarative as d } from "rocx-docx";
import { cell, heading, lineHeight, p, row, table, tableCell, tabStop } from "./blocks.mjs";

// The visible labels match the Python keywords; the property keys are JavaScript.
const regions = [
  ["whole_table", "wholeTable"], ["first_row", "firstRow"], ["last_row", "lastRow"],
  ["first_column", "firstColumn"], ["last_column", "lastColumn"],
  ["banding_1_row", "banding1Row"], ["banding_2_row", "banding2Row"],
  ["banding_1_column", "banding1Column"], ["banding_2_column", "banding2Column"],
  ["top_left_cell", "topLeftCell"], ["top_right_cell", "topRightCell"],
  ["bottom_left_cell", "bottomLeftCell"], ["bottom_right_cell", "bottomRightCell"],
];

export function tableStyles() {
  const format = {
    bold: true, italic: false, underline: false, font: "Calibri", fontSize: 10, color: [32, 48, 96],
    background: [224, 238, 255], alignment: "center", spacingBefore: 2, spacingAfter: 2,
    lineHeight: lineHeight("minimum", 12), tabStops: [tabStop(36)], bidi: false,
    language: { latin: "en-US", eastAsia: "ja-JP", bidi: "ar-SA" },
    boldComplexScript: false, italicComplexScript: false, sizeComplexScript: 10, rtl: false,
    complexScript: false, fontEastAsia: "Yu Mincho", fontComplexScript: "Arial",
  };
  return regions.map(([label, key]) => d.TableStyleDefinition(`Region ${label}`, { baseStyle: "Table Grid", [key]: format }));
}

const atLeast = (points) => ({ kind: "atLeast", points });
const look = (enabled) => ({
  firstRow: enabled, lastRow: enabled, firstColumn: enabled, lastColumn: enabled,
  horizontalBanding: enabled, verticalBanding: enabled,
});

export function tableBlocks() {
  const top = tableCell([p("Horizontal span=2; vertical merge restart; shaded; vertically centered")], {
    gridSpan: 2, vmerge: "restart", backgroundColor: [255, 238, 204], verticalAlignment: "center",
    margins: { top: 3, start: 0, bottom: 3, end: 6 },
    borders: ["top", "bottom", "start", "end"].map((side) => ({ side, sz: 8, color: [128, 64, 32], val: "dashed" })),
  });
  const blocks = [
    heading("04 Tables: layout, rows, cells, merges and nested content"),
    p("Fixed table: 3 columns of 1440 twips; all six table-border sides; top/bottom/start/end cell borders; padding top=3, start=0, bottom=3, end=6 pt. Header row repeats; cannot split; minimum height=28 pt."),
    table([
      row([top, cell("Top aligned cell")], { height: atLeast(28), repeatHeader: true, cantSplit: true }),
      row([
        tableCell([p("")], { gridSpan: 2, vmerge: "continue" }),
        tableCell([p("Bottom aligned cell")], { verticalAlignment: "bottom" }),
      ], { height: { kind: "exact", points: 48 }, repeatHeader: false, cantSplit: false }),
      row([cell("Short row: padded grid"), cell("Second cell")]),
    ], {
      columnWidths: [1440, 1440, 1440], layout: "fixed",
      borders: ["top", "left", "bottom", "right", "insideH", "insideV"].map((side) => ({ side, sz: 8, color: [32, 64, 96], val: "single" })),
    }),
    p("Autofit table with nested table and trailing paragraph (nested tables need trailing paragraphs)."),
    table([row([
      tableCell([
        table([row([cell("Nested fixed-width table")])], { columnWidths: [1440], layout: "fixed" }),
        p("Paragraph after nested table"),
      ]), cell("Autofit neighbor"),
    ])], { layout: "autofit", columnWidths: [2000, 2000] }),
  ];
  for (const [label, textDirection] of [
    ["lrTb", "leftToRight"], ["tbRl", "topToBottomRightToLeft"], ["btLr", "bottomToTopLeftToRight"],
    ["lrTbV", "leftToRightVertical"], ["tbRlV", "topToBottomRightToLeftVertical"], ["tbLrV", "topToBottomLeftToRightVertical"],
  ]) {
    blocks.push(p(`Cell text_direction=${label}; independent from paragraph bidi.`),
      table([row([tableCell([p(`Text flow ${label}`)], { textDirection, verticalAlignment: "top" })], { height: atLeast(72) })]));
  }
  for (const alignment of ["left", "center", "right", "start", "end"]) {
    blocks.push(p(`Inline table alignment=${alignment}; fixed width=2160 twips. No floating positioning or paragraph alignment is requested.`),
      table([row([cell(`Table alignment: ${alignment}`)])], { alignment, columnWidths: [2160] }));
  }
  blocks.push(
    p("Repeated-header pagination sample: 36 rows follow. The consumer decides actual page breaks."),
    table(Array.from({ length: 36 }, (_, i) => row([
      cell(i === 0 ? "Repeated header: index" : `Row ${i}`), cell("Minimum 20 pt; cannot split"),
    ], { height: atLeast(20), repeatHeader: i === 0, cantSplit: true }))),
    p("Table look: all six activation flags=false (mask 0600). Whole-table formatting remains unconditional; no region formatting is copied to direct cells."),
    table([row([cell("All conditional regions disabled; base whole_table still applies")])], { style: "Region whole_table", look: look(false) }),
    heading("05 Conditional table styles: all 13 regions"),
    p("Each table references a separate region style. All six table-look activation flags are explicitly enabled (mask 01E0). Region fill/font/paragraph/script requests stay in styles.xml, not direct cell formatting; the viewer applies conditional precedence. Base whole_table is unconditional."),
  );
  for (const [name] of regions) {
    blocks.push(p(`Conditional region=${name}: blue shading, bold 10 pt, centered, minimum line=12 pt, tab=36 pt, bidi/RTL=false; language/script/font slots authored.`),
      table([1, 2, 3].map((r) => row([1, 2, 3].map((c) => cell(`${name} r${r}c${c}`)))),
        { style: `Region ${name}`, look: look(true) }));
  }
  return blocks;
}
