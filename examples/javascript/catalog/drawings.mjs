import { future, lazy } from "rocx-docx";
import { br, heading, p, para, tab, text } from "./blocks.mjs";

const formats = ["png", "jpg", "gif", "bmp", "tiff", "zero"];
const filename = (ext) => ext === "zero" ? "sample_zero_dpi.png" : `sample.${ext}`;

export async function loadImages(readFixture, namedPng) {
  async function readRequired(name) {
    const bytes = await readFixture(name);
    if (bytes === undefined) throw new TypeError(`Missing catalog image fixture: ${name}`);
    return bytes;
  }
  const images = Object.fromEntries(await Promise.all(formats.map(async (ext) => [ext, await readRequired(filename(ext))])));
  return {
    image: (ext, transformation) => ({
      kind: "image", data: images[ext], transformation,
      altText: { title: `Image ${ext}`, description: `Accessible ${ext} sample`, name: `Sample ${ext}` },
    }),
    named: namedPng ?? future(async () => ({ bytes: await readRequired(filename("png")), sourceName: filename("png") })),
    deferred: future(() => readRequired(filename("png"))),
  };
}

const shape = (properties) => ({ kind: "shape", properties });

export function drawingBlocks({ image, named, deferred }) {
  const blocks = [heading("07 Inline images and explicit breaks")];
  for (const [ext, label, size] of [
    ["png", "PNG width-only 48 pt (aspect retained)", { width: 48 }],
    ["jpg", "JPEG height-only 36 pt (aspect retained)", { height: 36 }],
    ["gif", "GIF both dimensions 48 x 24 pt", { width: 48, height: 24 }],
    ["bmp", "BMP native density-derived size"], ["tiff", "TIFF native density-derived size"], ["zero", "PNG zero-DPI fallback size"],
  ]) {
    blocks.push(p(`Image: ${label}; accessibility title/description/name authored.`), para([image(ext, size)]));
  }
  blocks.push(
    para([{ kind: "hyperlink", target: { kind: "external", url: "https://example.com/image" }, children: [
      image("png", { width: 24 }), tab, text("Linked image, tab and line break"),
      br(), text("Same PNG bytes are deduplicated in the package."),
    ] }]),
    p("Image crop and rotation: 10% left, 5% top, 20% right and 15% bottom; 30 degrees clockwise, horizontal flip on and vertical flip off."),
    para([image("png", { width: 60, height: 40, rotation: 30, flipHorizontal: true, flipVertical: false,
      crop: { left: 10, top: 5, right: 20, bottom: 15 } })]),
    p("Floating picture: column-relative horizontal offset 250 pt, page-relative bottom alignment, square wrapping on the largest side; gaps top 0, bottom 2, left 3, right 4 pt. Anchor locked; in-cell layout and overlap off."),
    para([image("png", { width: 42, anchor: {
      horizontalReference: "column", x: 250, verticalAlignment: "bottom", wrap: "square", relativeHeight: 4, wrapSide: "largest",
      distanceTop: 0, distanceBottom: 2, distanceLeft: 3, distanceRight: 4, locked: true, layoutInCell: false, allowOverlap: false,
    } })]),
    p("Floating picture: tight text wrapping around an authored triangular contour."),
    para([image("png", { width: 42, anchor: { x: 360, y: 160, wrap: "tight", wrapPolygon: [[0, 0], [21600, 21600], [0, 21600]] } })]),
    p("Inline DrawingML shapes: a blue rectangle with text and a green ellipse with a dark outline."),
    p("Shape text style: blue rectangle, 14.25 pt white Arial text, bold explicitly false, italic true, centered horizontally and bottom anchored. Blank middle line preserved; ellipse has no text body."),
    para([
      shape({ width: 96, height: 64, fill: [32, 80, 176], text: "Blue\n\nText",
        textFormat: { fontSize: 14.25, bold: false, color: [255, 255, 255], alignment: "center", verticalAlignment: "bottom", italic: true, font: "Arial" } }),
      shape({ width: 44, height: 32, geometry: "ellipse", fill: [32, 144, 80], outline: { width: 1.5, color: [16, 48, 32] } }),
    ]),
    p("Standalone inline text box: 144 by 48 points, plain multiline DrawingML text, Arial 12 pt, bold explicitly false; no geometric fill or outline, no rich WordprocessingML story."),
    para([shape({ width: 144, height: 48, geometry: "textBox", text: "Standalone\nText box",
      textFormat: { font: "Arial", fontSize: 12, bold: false } })]),
    para([
      text("Floating text box: 220 pt from the margin, 12 pt below its paragraph, square wrapping with a 6 pt gap on the left."),
      shape({ width: 108, height: 24, geometry: "textBox", text: "Floating note",
        anchor: { horizontalReference: "margin", verticalReference: "paragraph", x: 220, y: 12, wrap: "square", distanceLeft: 6 } }),
    ]),
    p("Inline column chart: North 8, South 13.5, West 5; one series without an embedded workbook."),
    para([{ kind: "columnChart", properties: { width: 288, height: 180, categories: ["North", "South", "West"], values: [8, 13.5, 5] } }]),
    para([text("Inline equation: stacked fraction "), { kind: "fraction", numerator: "1", denominator: "2" }, text(".")]),
    p("Image from a named fixture (filename preserved), followed by async byte data with deferred size/alt text."),
    para([
      { kind: "image", data: named, type: "png", transformation: { width: 24 } },
      { kind: "image", data: deferred, transformation: lazy(() => ({ height: 24 })),
        altText: lazy(() => ({ description: "Deferred image accessibility" })) },
    ]),
  );
  for (const kind of ["line", "textWrapping", "page", "column"]) {
    blocks.push(para([text(`Before explicit ${kind} break`), br(kind), text(`After explicit ${kind} break`)]));
  }
  return blocks;
}
