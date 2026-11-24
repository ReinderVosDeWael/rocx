import {
  future, resolveComponent, resolveDocument, validateAuthoringId, validateValue,
} from "rocx-docx";
import type {
  DocumentProgram, NumberingAlignment, NumberingDefinition, NumberingLevel, NumberingReference,
  NumberingStartOverride, NumberingSuffix, ResolvedNumberingDefinition, ResolvedStyleDefinition,
  StyleDefinition, TableSectionFormat, TableStyleDefinition,
} from "rocx-docx";

const region = {
  bold: true, italic: false, underline: true, font: "Latin", fontSize: 10, color: [1, 2, 3],
  background: [4, 5, 6], alignment: "center", spacingBefore: 3, spacingAfter: 6,
  lineHeight: { kind: "exact", value: 14 }, tabStops: [{ alignment: "decimal", position: 18 }],
  bidi: true, language: { latin: "en-US", eastAsia: "ja-JP", bidi: "he-IL" },
  boldComplexScript: false, italicComplexScript: true, sizeComplexScript: 13, rtl: true,
  complexScript: true, fontEastAsia: "East", fontComplexScript: "Complex",
} satisfies TableSectionFormat;

const table = {
  kind: "table", name: "Regional", baseStyle: "Table Base",
  wholeTable: region, firstRow: region, lastRow: region, firstColumn: region, lastColumn: region,
  banding1Row: region, banding2Row: region, banding1Column: region, banding2Column: region,
  topLeftCell: region, topRightCell: region, bottomLeftCell: region, bottomRightCell: region,
} satisfies TableStyleDefinition;

const { background: _background, ...styleText } = region;
const styles = [
  {
    ...styleText, kind: "paragraph", name: "List Paragraph", baseStyle: "Normal",
    nextParagraphStyle: "Next", linkedStyle: "List Character", leftIndent: 0.5, rightIndent: -0.1,
    firstLineIndent: 0.1, keepTogether: false, keepWithNext: true, pageBreakBefore: false,
    widowControl: true, suppressLineNumbers: false, contextualSpacing: true, mirrorIndents: false,
    snapToGrid: true,
  },
  { kind: "paragraph", name: "Next", lineSpacing: 1.5 },
  {
    kind: "character", name: "List Character", baseStyle: "Base Character",
    linkedStyle: "List Paragraph", format: { underline: "double", size: 15 },
  },
  { kind: "character", name: "Base Character" },
  { kind: "numbering", name: "Outline", numberingId: 600 },
  table,
  { kind: "table", name: "Table Base" },
] satisfies readonly StyleDefinition[];

const alignments = ["left", "center", "right", "start", "end"] satisfies readonly NumberingAlignment[];
const suffixes = ["tab", "space", "nothing"] satisfies readonly NumberingSuffix[];
const levels = alignments.map((alignment, level): NumberingLevel => ({
  level, start: 1, format: "lowerRoman", text: "%1.", isLegal: level > 0, restart: 0,
  alignment, suffix: suffixes[level % suffixes.length], labelFormat: { bold: true },
  paragraphFormat: { leftIndent: 18 }, paragraphStyle: "List Paragraph",
}));
const override = { level: 0, start: 0 } satisfies NumberingStartOverride;
const numbering = [
  { id: 600, abstractId: 900, levels },
  { id: 601, abstractId: 900, levels, startOverrides: [override], levelOverrides: [levels[0]] },
  { id: 602, inheritStyle: "Outline" },
] satisfies readonly NumberingDefinition[];

export async function typedStyleCases(): Promise<void> {
  const reference: NumberingReference = { id: 602, level: 1 };
  const paragraph = await resolveComponent({
    kind: "paragraph", text: "reference", numbering: future(async () => reference),
  });
  const readReference: NumberingReference | undefined = paragraph?.view.numbering;
  const declaration: DocumentProgram = { sections: [], styles: future(async () => styles), numbering };
  const { view } = await resolveDocument(declaration);
  const readNumbering: ResolvedNumberingDefinition = view.numbering[1];
  const readStyle: ResolvedStyleDefinition = view.styles[0];
  if (readStyle.kind === "paragraph") {
    const position: number | undefined = readStyle.tabStops[0]?.position;
    void position;
  }
  await validateValue({ kind: "numberingDefinition", value: readNumbering });
  await validateAuthoringId("styleId", "ListParagraph");
  void readReference;
  // @ts-expect-error A numbering style points to a concrete numeric ID.
  const bad: StyleDefinition = { kind: "numbering", name: "Wrong", numberingId: "600" };
  // @ts-expect-error List formats are open strings, but suffixes are not.
  const suffix: NumberingSuffix = "none";
  void [bad, suffix];
}
