import { readFile, writeFile } from "node:fs/promises";
import { makeLegacyReport } from "./legacy-author.mjs";

const [imagePath, outputPath = "legacy-report.docx"] = process.argv.slice(2);
if (!imagePath) throw new Error("Usage: node legacy-node.mjs IMAGE [OUTPUT.docx]");
await writeFile(outputPath, await makeLegacyReport(await readFile(imagePath)));
