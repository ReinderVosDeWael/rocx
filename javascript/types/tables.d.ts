import type { Condition, ResolvedHandle, Rgb } from "./boundary.js";
import type { V } from "./resolution.js";
import type { ContentControlProperties, ResolvedContentControlProperties } from "./structured.js";
import type { BlockChildren, ResolvedBlock } from "./text-layout.js";

export type TableLayout = "fixed" | "autofit";
export type TableAlignment = "left" | "center" | "right" | "start" | "end";
export type VerticalMerge = "restart" | "continue";
export type VerticalAlignment = "top" | "center" | "bottom" | "both";
export type BorderLine = "single" | "dashed";
export type BorderSide = "top" | "left" | "bottom" | "right" | "insideH" | "insideV";
export type CellBorderSide = "top" | "bottom" | "start" | "end";
export type CellTextDirection =
  | "leftToRight"
  | "topToBottomRightToLeft"
  | "bottomToTopLeftToRight"
  | "leftToRightVertical"
  | "topToBottomRightToLeftVertical"
  | "topToBottomLeftToRightVertical";

export interface TableBorder {
  readonly side: BorderSide;
  /** Border size in eighths of a point. */
  readonly sz: number;
  readonly color: Rgb;
  readonly val: BorderLine;
}
export interface CellBorder extends Omit<TableBorder, "side"> {
  readonly side: CellBorderSide;
}
export interface TableLook {
  readonly firstRow?: boolean;
  readonly lastRow?: boolean;
  readonly firstColumn?: boolean;
  readonly lastColumn?: boolean;
  readonly horizontalBanding?: boolean;
  readonly verticalBanding?: boolean;
}
export interface TableProperties {
  /** Column widths in twips. Autofit ignores these during rendering. */
  readonly columnWidths?: readonly number[];
  readonly layout?: TableLayout;
  readonly style?: string;
  readonly borders?: readonly TableBorder[];
  readonly look?: TableLook;
  readonly alignment?: TableAlignment;
}
export type RowHeight =
  | { readonly kind: "exact"; readonly points: number }
  | { readonly kind: "atLeast"; readonly points: number };
export interface TableRowProperties {
  readonly height?: RowHeight;
  readonly repeatHeader?: boolean;
  readonly cantSplit?: boolean;
  /** Row wrappers require rich text. */
  readonly contentControl?: ContentControlProperties;
}
/** Optional cell-local margins in points. */
export interface CellMargins {
  readonly top?: number;
  readonly start?: number;
  readonly bottom?: number;
  readonly end?: number;
}
export interface TableCellProperties {
  readonly gridSpan?: number;
  readonly vmerge?: VerticalMerge;
  readonly borders?: readonly CellBorder[];
  readonly backgroundColor?: Rgb;
  readonly verticalAlignment?: VerticalAlignment;
  readonly margins?: CellMargins;
  readonly textDirection?: CellTextDirection;
  /** Cell wrappers require rich text. */
  readonly contentControl?: ContentControlProperties;
}

export interface TableDeclaration {
  readonly kind: "table";
  readonly rows: V<readonly V<TableRowDeclaration>[]>;
  readonly properties?: V<TableProperties>;
  readonly condition?: Condition;
}
export interface TableRowDeclaration {
  readonly kind: "tableRow";
  readonly children: V<readonly V<TableCellDeclaration>[]>;
  readonly properties?: V<TableRowProperties>;
  readonly condition?: Condition;
}
export interface TableCellDeclaration {
  readonly kind: "tableCell";
  readonly children: V<BlockChildren>;
  readonly properties?: V<TableCellProperties>;
  readonly condition?: Condition;
}

export interface ResolvedTableView {
  readonly columns: number;
  readonly properties: TableProperties & { readonly borders: readonly TableBorder[] };
  /** Physical resolved rows, before rendering pads ragged rows. */
  readonly rows: readonly ResolvedTableRow[];
}
export interface ResolvedTableRowView {
  readonly properties: Omit<TableRowProperties, "contentControl"> & {
    readonly contentControl?: ResolvedContentControlProperties;
  };
  readonly cells: readonly ResolvedTableCell[];
}
export interface ResolvedTableCellView extends TableCellProperties {
  readonly gridSpan: number;
  readonly borders: readonly CellBorder[];
  readonly contentControl?: ResolvedContentControlProperties;
  readonly blocks: readonly ResolvedBlock[];
}
export type ResolvedTable = ResolvedHandle<"table", ResolvedTableView>;
export type ResolvedTableRow = ResolvedHandle<"tableRow", ResolvedTableRowView>;
export type ResolvedTableCell = ResolvedHandle<"tableCell", ResolvedTableCellView>;

export interface TableComponents {
  readonly table: { readonly declaration: TableDeclaration; readonly resolved: ResolvedTable };
}
export interface TableStandaloneComponents {
  readonly tableRow: { readonly declaration: TableRowDeclaration; readonly resolved: ResolvedTableRow };
  readonly tableCell: { readonly declaration: TableCellDeclaration; readonly resolved: ResolvedTableCell };
}

declare module "./resolution.js" {
  interface ValidationValues {
    readonly tableRowProperties: TableRowProperties;
    readonly tableCellProperties: TableCellProperties;
    readonly cellMargins: CellMargins;
  }
}
