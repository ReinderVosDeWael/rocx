import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { imageFile } from "rocx-docx/node";
import { catalogFixtures } from "./catalog-contract.mjs";
import { editorFixtures } from "./editor.mjs";
import { portableFixtures } from "./fixtures.mjs";

const [fixtureDir, outputDir] = process.argv.slice(2);
const read = async (name) => new Uint8Array(await readFile(join(fixtureDir, name)));
const fixtures = await portableFixtures(
  await read("sample.png"), await read("no-margins.docx"), await read("imported-editor.docx"),
);
const catalog = await catalogFixtures(read, { namedPng: imageFile(join(fixtureDir, "sample.png")) });
for (const [name, bytes] of Object.entries({ ...fixtures, ...catalog })) {
  await writeFile(join(outputDir, `${name}.docx`), bytes);
}
const editor = await editorFixtures((name) => read(join("editor", name)));
for (const [name, bytes] of Object.entries(editor.files)) {
  await writeFile(join(outputDir, "editor", `${name}.docx`), bytes);
}
await writeFile(join(outputDir, "editor", "reports.json"), JSON.stringify(editor.reports));
console.log(`Installed Node ${process.version}: text/layout, styles, tables, drawing snapshots, template and editor fixtures passed`);
