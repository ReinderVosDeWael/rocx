import type { Condition, ResolvedHandle, Rgb } from "./boundary.js";
import type { V } from "./resolution.js";
import type { ResolvedComment } from "./structured.js";
import type { ResolvedRunFormat, RunFormat } from "./text-layout.js";

export type ImageAnchorReference =
  | "page" | "margin" | "column" | "character" | "paragraph" | "line"
  | "leftMargin" | "rightMargin" | "topMargin" | "bottomMargin" | "insideMargin" | "outsideMargin";
export type ImageAnchorHorizontalAlignment = "left" | "center" | "right" | "inside" | "outside";
export type ImageAnchorVerticalAlignment = "top" | "center" | "bottom" | "inside" | "outside";
export type ImageWrap = "none" | "square" | "topAndBottom" | "tight" | "through";
export type ImageWrapSide = "bothSides" | "left" | "right" | "largest";

/** Floating placement for pictures and shapes. Offsets and wrap distances are in points. */
export interface ImageAnchor {
  readonly horizontalReference?: ImageAnchorReference;
  readonly verticalReference?: ImageAnchorReference;
  readonly horizontalAlignment?: ImageAnchorHorizontalAlignment;
  readonly verticalAlignment?: ImageAnchorVerticalAlignment;
  readonly x?: number;
  readonly y?: number;
  readonly wrap?: ImageWrap;
  readonly wrapSide?: ImageWrapSide;
  /** Ordered signed DrawingML coordinates; required for tight and through wrap. */
  readonly wrapPolygon?: readonly (readonly [x: number, y: number])[];
  readonly distanceTop?: number;
  readonly distanceBottom?: number;
  readonly distanceLeft?: number;
  readonly distanceRight?: number;
  readonly relativeHeight?: number;
  readonly behindText?: boolean;
  readonly locked?: boolean;
  readonly layoutInCell?: boolean;
  readonly allowOverlap?: boolean;
}
/** Insets as percentages of the original image, measured from each physical edge. */
export interface ImageCrop {
  readonly left?: number;
  readonly top?: number;
  readonly right?: number;
  readonly bottom?: number;
}
export interface ImageTransformation {
  readonly anchor?: ImageAnchor;
  readonly crop?: ImageCrop;
  /** Clockwise degrees. */
  readonly rotation?: number;
  readonly flipHorizontal?: boolean;
  readonly flipVertical?: boolean;
  /** Width in points. Omit one dimension to retain the image's physical aspect ratio. */
  readonly width?: number;
  /** Height in points. */
  readonly height?: number;
}
export interface AltText {
  readonly title?: string;
  readonly description?: string;
  readonly name?: string;
}
/** Host-loaded encoded image bytes with a source name, never a filesystem path. */
export interface NamedImageData {
  readonly bytes: Uint8Array;
  readonly sourceName: string;
}
/** Plain bytes use a format-derived name; Node's `imageFile` supplies named bytes lazily. */
export type ImageData = Uint8Array | NamedImageData;

interface ConditionalDrawing {
  readonly condition?: Condition;
}

export interface ImageRunDeclaration extends ConditionalDrawing {
  readonly kind: "image";
  readonly data: V<ImageData>;
  readonly transformation?: V<ImageTransformation | undefined>;
  readonly altText?: V<AltText | undefined>;
  /** Ignored hint; Rust always detects the format from the bytes. */
  readonly type?: string;
}

export type ShapeGeometry = "rectangle" | "ellipse" | "textBox";
export type ShapeTextAlignment = "left" | "center" | "right";
export type ShapeTextVerticalAlignment = "top" | "center" | "bottom";
export interface ShapeOutline {
  /** Width in points. */
  readonly width: number;
  readonly color: Rgb;
}
export interface ShapeTextFormat {
  readonly fontSize?: number;
  readonly bold?: boolean;
  readonly color?: Rgb;
  readonly alignment?: ShapeTextAlignment;
  readonly verticalAlignment?: ShapeTextVerticalAlignment;
  readonly italic?: boolean;
  readonly font?: string;
}
export interface ShapeProperties {
  /** Width in points; Rust defaults to 72. */
  readonly width?: number;
  /** Height in points; Rust defaults to 72. */
  readonly height?: number;
  readonly geometry?: ShapeGeometry;
  readonly fill?: Rgb;
  readonly text?: string;
  readonly outline?: ShapeOutline;
  readonly textFormat?: ShapeTextFormat;
  readonly anchor?: ImageAnchor;
}
export interface ShapeRunDeclaration extends ConditionalDrawing {
  readonly kind: "shape";
  readonly properties: V<ShapeProperties>;
}
export interface ColumnChartProperties {
  /** Width in points. */
  readonly width: number;
  /** Height in points. */
  readonly height: number;
  readonly categories: readonly string[];
  readonly values: readonly number[];
}
export interface ColumnChartRunDeclaration extends ConditionalDrawing {
  readonly kind: "columnChart";
  readonly properties: V<ColumnChartProperties>;
}
export interface InlineFractionDeclaration extends ConditionalDrawing {
  readonly kind: "fraction";
  readonly numerator: V<string>;
  readonly denominator: V<string>;
}
export interface TabDeclaration extends ConditionalDrawing {
  readonly kind: "tab";
}
export type BreakType = "line" | "textWrapping" | "page" | "column";
export interface BreakDeclaration extends ConditionalDrawing {
  readonly kind: "break";
  /** Defaults to Rust's line break, not an OOXML break without a type. */
  readonly breakType?: BreakType;
}

export interface ResolvedImageView {
  readonly name: string;
  readonly altText?: AltText;
  /** A fresh copy on each read. */
  readonly bytes: Uint8Array<ArrayBuffer>;
  readonly dimensionsEmu: readonly [width: number, height: number];
  readonly contentType: string;
}
export type ResolvedImage = ResolvedHandle<"image", ResolvedImageView>;

interface Display {
  readonly displayText: string;
}
interface Formatted {
  readonly format: ResolvedRunFormat;
  readonly comment?: ResolvedComment;
}
type ResolvedAnchor = ImageAnchor & Required<Pick<ImageAnchor,
  "horizontalReference" | "verticalReference" | "x" | "y" | "wrap" | "wrapPolygon" |
  "relativeHeight" | "behindText">>;
type ResolvedShapeProperties = Omit<ShapeProperties, "anchor"> &
  Required<Pick<ShapeProperties, "width" | "height" | "geometry">> & { readonly anchor?: ResolvedAnchor };
export type ResolvedImageRunView = Display & Formatted & { readonly kind: "image"; readonly image: ResolvedImage };
export type ResolvedShapeRunView = Display & { readonly kind: "shape"; readonly properties: ResolvedShapeProperties };
export type ResolvedColumnChartRunView = Display & { readonly kind: "columnChart"; readonly properties: ColumnChartProperties };
export type ResolvedInlineFractionView = Display & {
  readonly kind: "fraction";
  readonly numerator: string;
  readonly denominator: string;
};
export type ResolvedTabView = Display & Formatted & { readonly kind: "tab" };
export type ResolvedBreakView = Display & Formatted & { readonly kind: "break"; readonly breakType: BreakType };
export type ResolvedImageRun = ResolvedHandle<"inline", ResolvedImageRunView>;
export type ResolvedShapeRun = ResolvedHandle<"inline", ResolvedShapeRunView>;
export type ResolvedColumnChartRun = ResolvedHandle<"inline", ResolvedColumnChartRunView>;
export type ResolvedInlineFraction = ResolvedHandle<"inline", ResolvedInlineFractionView>;
export type ResolvedTab = ResolvedHandle<"inline", ResolvedTabView>;
export type ResolvedBreak = ResolvedHandle<"inline", ResolvedBreakView>;

export interface DrawingComponents {
  readonly image: { readonly declaration: ImageRunDeclaration; readonly resolved: ResolvedImageRun };
  readonly shape: { readonly declaration: ShapeRunDeclaration; readonly resolved: ResolvedShapeRun };
  readonly columnChart: { readonly declaration: ColumnChartRunDeclaration; readonly resolved: ResolvedColumnChartRun };
  readonly fraction: { readonly declaration: InlineFractionDeclaration; readonly resolved: ResolvedInlineFraction };
  readonly tab: { readonly declaration: TabDeclaration; readonly resolved: ResolvedTab };
  readonly break: { readonly declaration: BreakDeclaration; readonly resolved: ResolvedBreak };
}

/** Input arms supplied to the shared resolved-inline constructor. */
type RunProperties = { readonly format?: RunFormat; readonly comment?: ResolvedComment };
export type DrawingResolvedInlineInput =
  | ({ readonly kind: "image"; readonly image: ResolvedImage } & RunProperties)
  | ({ readonly kind: "tab" } & RunProperties)
  | ({ readonly kind: "break"; readonly breakType?: BreakType } & RunProperties)
  | { readonly kind: "shape"; readonly properties: ShapeProperties }
  | { readonly kind: "columnChart"; readonly properties: ColumnChartProperties }
  | { readonly kind: "fraction"; readonly numerator: string; readonly denominator: string };

export type DrawingValidationInput = {
  readonly kind: "imageTransformation";
  readonly value: ImageTransformation;
};
