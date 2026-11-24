import {
  createResolvedInline, deferred, future, lazy, resolveComponent, resolveDocument,
  validateValue,
} from "rocx-docx";
import type { imageFile as nodeImageFile } from "rocx-docx/node";
import type {
  BreakDeclaration, ColumnChartRunDeclaration, ImageAnchor, ImageRunDeclaration,
  ImageTransformation, InlineDeclaration, InlineFractionDeclaration, ResolvedImage, ResolvedImageView,
  ShapeRunDeclaration, TabDeclaration, V,
} from "rocx-docx";

const contour = [[-10, 0], [21600, 0], [0, 21600]] as const;
export const placements = [
  { horizontalReference: "column", verticalReference: "paragraph", x: -2, y: 3, wrap: "none" },
  {
    horizontalReference: "margin", verticalReference: "line",
    horizontalAlignment: "left", verticalAlignment: "top",
    wrap: "square", wrapSide: "largest", distanceTop: 1, distanceBottom: 2,
    distanceLeft: 3, distanceRight: 4, relativeHeight: 17, behindText: true,
    locked: false, layoutInCell: false, allowOverlap: false,
  },
  {
    horizontalReference: "character", verticalReference: "topMargin",
    horizontalAlignment: "center", verticalAlignment: "center",
    wrap: "tight", wrapSide: "left", wrapPolygon: contour,
  },
  {
    horizontalReference: "leftMargin", verticalReference: "bottomMargin",
    horizontalAlignment: "right", verticalAlignment: "bottom",
    wrap: "through", wrapSide: "right", wrapPolygon: contour,
  },
  {
    horizontalReference: "rightMargin", verticalReference: "insideMargin",
    horizontalAlignment: "inside", verticalAlignment: "inside",
    wrap: "topAndBottom", distanceBottom: 5, locked: true, layoutInCell: true, allowOverlap: true,
  },
  {
    horizontalReference: "outsideMargin", verticalReference: "outsideMargin",
    horizontalAlignment: "outside", verticalAlignment: "outside",
    wrap: "square", wrapSide: "bothSides", behindText: false,
  },
  { horizontalReference: "insideMargin", verticalReference: "margin", wrap: "none" },
  { horizontalReference: "page", verticalReference: "page", wrap: "none" },
] as const satisfies readonly ImageAnchor[];

export const pictureTransform = {
  width: 30, height: 20, crop: { left: 1, top: 2, right: 3, bottom: 4 },
  rotation: -10, flipHorizontal: false, flipVertical: true, anchor: placements[1],
} as const satisfies ImageTransformation;
export const alt = { name: "Logo", title: "Brand", description: "Logo & art" } as const;

export function pictures(data: Uint8Array) {
  return [
    {
      kind: "image",
      data: deferred(future(async () => data)),
      transformation: lazy(() => pictureTransform),
      altText: future(async () => alt),
      type: "ignored/image-type",
      condition: () => true,
    },
    {
      kind: "image", data, transformation: { height: 10, flipHorizontal: true, flipVertical: false },
      altText: lazy(() => undefined),
    },
    { kind: "image", data, transformation: future(async () => undefined) },
    { kind: "image", data, transformation: { width: 15 } },
    ...placements.filter((_, i) => i !== 1).map(anchor =>
      ({ kind: "image", data, transformation: { anchor } } as const)),
  ] satisfies readonly ImageRunDeclaration[];
}

export const filePicture = (imageFile: typeof nodeImageFile, path: string) =>
  ({ kind: "image", data: imageFile(path) } satisfies ImageRunDeclaration);

// Keep the standalone overload and the installed readback type in the runtime case.
export async function resolvePicture(data: Uint8Array): Promise<ResolvedImageView> {
  const run = await resolveComponent({ kind: "image", data });
  if (!run) throw new Error("an unconditional image is missing");
  return run.view.image.view;
}

const ellipse = {
  width: 60, height: 31, geometry: "ellipse", fill: [1, 2, 3], text: "Shape & text",
  outline: { width: 2, color: [10, 20, 30] },
  textFormat: {
    fontSize: 9.25, color: [30, 20, 10], bold: false, italic: true,
    alignment: "right", verticalAlignment: "bottom", font: "Aptos",
  },
  anchor: placements[0],
} as const;
const otherShapes = [
  { kind: "shape", properties: { text: "Rectangle", textFormat: { bold: true, italic: false, alignment: "center", verticalAlignment: "center" } } },
  { kind: "shape", properties: { geometry: "textBox", text: "Text box", textFormat: { alignment: "left", verticalAlignment: "top" } } },
] satisfies readonly ShapeRunDeclaration[];
const chart = () => ({
  kind: "columnChart",
  properties: lazy(() => ({ width: 120, height: 48, categories: ["One", "Two & 三"], values: [-1.5, 0] })),
} satisfies ColumnChartRunDeclaration);
const fraction = () => ({
  kind: "fraction", numerator: lazy(() => "α"), denominator: future(async () => "β"),
} satisfies InlineFractionDeclaration);
const tab = { kind: "tab" } satisfies TabDeclaration;
const breaks = [
  { kind: "break" }, { kind: "break", breakType: "line" }, { kind: "break", breakType: "textWrapping" },
  { kind: "break", breakType: "page" }, { kind: "break", breakType: "column" },
] satisfies readonly BreakDeclaration[];

// New single-use producers for every run of the fixture.
export function drawingRuns() {
  return [
    { kind: "shape", properties: future(async () => ellipse) },
    ...otherShapes, chart(), fraction(), tab, ...breaks,
  ] satisfies readonly InlineDeclaration[];
}

export const timestamp = "2024-02-03T04:05:06Z";
export const paragraph = (runs: readonly V<InlineDeclaration>[]) =>
  ({ kind: "paragraph", runs } as const);
export const document = (runs: readonly V<InlineDeclaration>[]) => resolveDocument({
  timestamp, sections: [{ kind: "section", children: [paragraph(runs)] }],
});

export async function standaloneAndConditionalComponents() {
  const skip = () => { throw new Error("excluded drawing's producer ran"); };
  const result = await Promise.all([
    resolveComponent({ kind: "image", condition: false, data: lazy(skip),
      transformation: lazy(skip), altText: lazy(skip) }),
    resolveComponent({ kind: "shape", condition: () => false, properties: lazy(skip) }),
    resolveComponent({ kind: "columnChart", condition: false, properties: future(skip) }),
    resolveComponent({ kind: "fraction", condition: false, numerator: lazy(skip), denominator: future(skip) }),
    resolveComponent({ kind: "tab", condition: false }),
    resolveComponent({ kind: "break", condition: false }),
  ]);
  return result;
}

export async function construct(image: ResolvedImage) {
  const comment = { text: "Annotated drawing run", author: "Reviewer" };
  return Promise.all([
    createResolvedInline({ kind: "image", image, format: { bold: true, italic: false }, comment }),
    createResolvedInline({ kind: "tab", format: { bold: false }, comment }),
    createResolvedInline({ kind: "break", breakType: "column", format: { italic: true }, comment }),
    createResolvedInline({ kind: "shape", properties: { text: "Constructed shape" } }),
    createResolvedInline({ kind: "columnChart", properties: { width: 10, height: 20, categories: ["A"], values: [10] } }),
    createResolvedInline({ kind: "fraction", numerator: "1", denominator: "2" }),
  ]);
}

export function validateImageTransformation(value: ImageTransformation) {
  return validateValue({ kind: "imageTransformation", value });
}
