import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { imageFile } from "rocx-docx/node";
import { catalogNames, generateCatalog } from "./catalog.mjs";

const [imageDir, outputDir] = process.argv.slice(2);
if (!imageDir || !outputDir || process.argv.length !== 4) {
  throw new Error("Usage: node catalog-node.mjs IMAGE_DIR OUTPUT_DIR");
}
const artifacts = await generateCatalog((filename) => readFile(join(imageDir, filename)), {
  namedPng: imageFile(join(imageDir, "sample.png")),
});
await mkdir(outputDir, { recursive: true });
await Promise.all(Object.entries(catalogNames).map(([key, filename]) => writeFile(join(outputDir, filename), artifacts[key])));
