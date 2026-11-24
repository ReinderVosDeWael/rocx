import type {
  BreakDeclaration, ColumnChartRunDeclaration, ImageRunDeclaration, InlineFractionDeclaration,
  ShapeRunDeclaration, TabDeclaration,
} from "./drawings.js";
import type {
  BlockContentControlDeclaration, CommentDeclaration, FieldDeclaration, HyperlinkDeclaration,
  InlineContentControlDeclaration, NoteDeclaration, NoteReferenceDeclaration, RangeEndDeclaration,
  RangeStartDeclaration,
} from "./structured.js";
import type {
  CharacterStyleDefinition, NumberingStyleDefinition, ParagraphStyleDefinition, TableStyleDefinition,
} from "./styles.js";
import type { TableCellDeclaration, TableDeclaration, TableRowDeclaration } from "./tables.js";
import type {
  DocumentProgram, FooterDeclaration, HeaderDeclaration, InlineChildren, ParagraphDeclaration,
  SectionDeclaration, TextRunDeclaration,
} from "./text-layout.js";
import type { V } from "./resolution.js";

type Options<D, Positional extends keyof D = never> = Omit<D, "kind" | Positional>;
type ParagraphOptions = Options<ParagraphDeclaration, "text" | "runs">;
type TextParagraph = Extract<ParagraphDeclaration, { readonly runs?: never }> & { readonly text: V<string> };
type RunsParagraph = Extract<ParagraphDeclaration, { readonly runs: V<InlineChildren> }>;

interface ParagraphFactory {
  (text: string, options?: ParagraphOptions): TextParagraph;
  (runs: InlineChildren, options?: ParagraphOptions): RunsParagraph;
  (textOrRuns: string | InlineChildren, options?: ParagraphOptions): TextParagraph | RunsParagraph;
  /** Select the text field explicitly, including for a deferred value. */
  readonly Text: (text: V<string>, options?: ParagraphOptions) => TextParagraph;
  /** Select the runs field explicitly, including for a deferred list. */
  readonly Runs: (runs: V<InlineChildren>, options?: ParagraphOptions) => RunsParagraph;
}

/** Synchronous factories for ordinary declarations; evaluation happens in the resolver. */
export declare const declarative: {
  readonly Document: (
    sections: DocumentProgram["sections"], options?: Options<DocumentProgram, "sections">,
  ) => DocumentProgram;
  readonly Section: (
    children: NonNullable<SectionDeclaration["children"]>, options?: Options<SectionDeclaration, "children">,
  ) => SectionDeclaration;
  readonly Header: (
    children: HeaderDeclaration["children"], options?: Options<HeaderDeclaration, "children">,
  ) => HeaderDeclaration;
  readonly Footer: (
    children: FooterDeclaration["children"], options?: Options<FooterDeclaration, "children">,
  ) => FooterDeclaration;
  readonly Paragraph: ParagraphFactory;
  readonly TextRun: (
    text: TextRunDeclaration["text"], options?: Options<TextRunDeclaration, "text">,
  ) => TextRunDeclaration;
  readonly Table: (
    rows: TableDeclaration["rows"], options?: Options<TableDeclaration, "rows">,
  ) => TableDeclaration;
  readonly TableRow: (
    cells: TableRowDeclaration["children"], options?: Options<TableRowDeclaration, "children">,
  ) => TableRowDeclaration;
  readonly TableCell: (
    children: TableCellDeclaration["children"], options?: Options<TableCellDeclaration, "children">,
  ) => TableCellDeclaration;
  readonly ImageRun: (
    data: ImageRunDeclaration["data"], options?: Options<ImageRunDeclaration, "data">,
  ) => ImageRunDeclaration;
  readonly ShapeRun: (
    properties: ShapeRunDeclaration["properties"], options?: Options<ShapeRunDeclaration, "properties">,
  ) => ShapeRunDeclaration;
  readonly ColumnChartRun: (
    properties: ColumnChartRunDeclaration["properties"], options?: Options<ColumnChartRunDeclaration, "properties">,
  ) => ColumnChartRunDeclaration;
  readonly InlineFraction: (
    numerator: InlineFractionDeclaration["numerator"], denominator: InlineFractionDeclaration["denominator"],
    options?: Options<InlineFractionDeclaration, "numerator" | "denominator">,
  ) => InlineFractionDeclaration;
  readonly Tab: (options?: Options<TabDeclaration>) => TabDeclaration;
  readonly Break: (options?: Options<BreakDeclaration>) => BreakDeclaration;
  readonly Hyperlink: (
    target: HyperlinkDeclaration["target"], children: HyperlinkDeclaration["children"],
    options?: Options<HyperlinkDeclaration, "target" | "children">,
  ) => HyperlinkDeclaration;
  readonly Field: (
    instruction: FieldDeclaration["instruction"], children: FieldDeclaration["children"],
    options?: Options<FieldDeclaration, "instruction" | "children">,
  ) => FieldDeclaration;
  readonly Note: (
    target: NoteDeclaration["target"], children: NoteDeclaration["children"],
    options?: Options<NoteDeclaration, "target" | "children">,
  ) => NoteDeclaration;
  readonly NoteReference: (
    target: NoteReferenceDeclaration["target"], options?: Options<NoteReferenceDeclaration, "target">,
  ) => NoteReferenceDeclaration;
  readonly Comment: (
    children: CommentDeclaration["children"], options?: Options<CommentDeclaration, "children">,
  ) => CommentDeclaration;
  readonly RangeStart: (
    id: RangeStartDeclaration["id"], range: RangeStartDeclaration["range"],
    options?: Options<RangeStartDeclaration, "id" | "range">,
  ) => RangeStartDeclaration;
  readonly RangeEnd: (
    id: RangeEndDeclaration["id"], options?: Options<RangeEndDeclaration, "id">,
  ) => RangeEndDeclaration;
  readonly InlineContentControl: (
    children: InlineContentControlDeclaration["children"], options?: Options<InlineContentControlDeclaration, "children">,
  ) => InlineContentControlDeclaration;
  readonly BlockContentControl: (
    children: BlockContentControlDeclaration["children"], options?: Options<BlockContentControlDeclaration, "children">,
  ) => BlockContentControlDeclaration;
  readonly ParagraphStyleDefinition: (
    name: ParagraphStyleDefinition["name"], options?: Options<ParagraphStyleDefinition, "name">,
  ) => ParagraphStyleDefinition;
  readonly CharacterStyleDefinition: (
    name: CharacterStyleDefinition["name"], options?: Options<CharacterStyleDefinition, "name">,
  ) => CharacterStyleDefinition;
  readonly NumberingStyleDefinition: (
    name: NumberingStyleDefinition["name"], numberingId: NumberingStyleDefinition["numberingId"],
  ) => NumberingStyleDefinition;
  readonly TableStyleDefinition: (
    name: TableStyleDefinition["name"], options?: Options<TableStyleDefinition, "name">,
  ) => TableStyleDefinition;
};
