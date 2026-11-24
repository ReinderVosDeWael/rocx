// docs:start quickstart
import { writeFile } from "node:fs/promises";
import { declarative as d, generateDocx } from "rocx-docx";

const document = d.Document([d.Section([
  d.Paragraph("Hello, world!"),
  d.Paragraph("A second paragraph, with Unicode: café 🌍"),
])], { timestamp: "2026-11-21T00:00:00Z" });

await writeFile(process.argv[2] ?? "plain.docx", await generateDocx(document));
// docs:end quickstart
