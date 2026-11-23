import { readFile, writeFile } from "node:fs/promises";
import { makeReport } from "./author.mjs";

const [imagePath, outputPath = "report.docx"] = process.argv.slice(2);
if (!imagePath) throw new Error("Usage: node node.mjs IMAGE [OUTPUT.docx]");
const bytes = await makeReport(await readFile(imagePath));
if (!(bytes instanceof Uint8Array)) throw new TypeError("Expected DOCX bytes");
await writeFile(outputPath, bytes);
