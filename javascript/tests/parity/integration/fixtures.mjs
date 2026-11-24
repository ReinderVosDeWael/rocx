// Browser-safe fixtures shared by both installed-package consumers.
import * as drawings from "../drawings.mts";
import * as styles from "../styles.mjs";
import * as tables from "../tables.mts";
import * as layout from "../text-layout.mjs";
import { declarativeFixtures } from "./declarative.mts";
import { structuredFixture } from "./structured.mts";
import { templateFixtures } from "./template.mjs";

export async function portableFixtures(png, noMargins, editorSource) {
  await layout.sectionGeometryWithoutInheritedMargins(noMargins);

  const pictures = await drawings.document(drawings.pictures(png));
  const image = pictures.view.sections[0].view.paragraphs[0].view.runs[0].view.image.view;
  image.bytes.fill(0);
  if (image.name !== "image.png" || image.dimensionsEmu.join() !== "381000,254000" ||
      image.bytes.length !== png.length || !image.bytes.every((byte, i) => byte === png[i])) {
    throw new Error("unexpected resolved image view or aliased bytes");
  }
  const drawing = await drawings.document(drawings.drawingRuns());
  if (drawing.view.sections[0].view.paragraphs[0].view.text !== "\t\n\n\n") {
    throw new Error("unexpected display text for drawings and breaks");
  }

  const table = await tables.structuredTableComposition();
  await tables.standaloneAndConditionalComponents();
  await tables.resolvedBlockViewsAndReuse();
  await tables.standaloneValidationAndDiagnostics();

  const style = await styles.styleByteFixtures();
  return {
    ...await declarativeFixtures(png),
    "text-layout": await layout.layoutParityFixture(),
    "linked-styles": style.linkedStyles,
    "numbering-replacements": style.numberingReplacements,
    drawings: drawing.toDocx(),
    pictures: pictures.toDocx(),
    tables: table.docx,
    "tables-auto": table.autoDocx,
    "tables-layout": await tables.rowCellLayoutAndControls(),
    structured: await structuredFixture(),
    ...await templateFixtures(editorSource),
  };
}
