import {
  future, generateDocx, property, resolveComponent, resolveDocument,
  type DocumentProgram, type ResolvedParagraph, type ResolvedTextRun,
} from "rocx-docx";

const source = property("name", "Lucas");
const program = {
  timestamp: future(async () => undefined),
  sections: [{
    kind: "section",
    children: [{
      kind: "paragraph",
      runs: [{
        kind: "text",
        text: future(async (context) => context.get(source.reference())),
      }],
    }],
  }],
} satisfies DocumentProgram;
const paragraph: ResolvedParagraph | undefined = await resolveComponent({
  kind: "paragraph", text: source,
});
const run: ResolvedTextRun | undefined = await resolveComponent({ kind: "text", text: source });
const document = await resolveDocument(program, { signal: new AbortController().signal });
const fromHandle: Uint8Array<ArrayBuffer> = document.toDocx({ profile: "transitional" });
const fromProgram: Uint8Array<ArrayBuffer> = await generateDocx(program, { readLimits: { maxParts: 100 } });
const fromLegacy: Uint8Array<ArrayBuffer> = await generateDocx({ blocks: [] });
void [paragraph, run, fromHandle, fromProgram, fromLegacy];

// @ts-expect-error legacy and program inputs cannot be combined
await generateDocx({ sections: [], blocks: [] });
// @ts-expect-error two paragraph content arms
await resolveComponent({ kind: "paragraph", text: "a", runs: [] });
// @ts-expect-error resolved views are not forgeable
await resolveDocument({ sections: [{ kind: "section", children: [{ kind: "resolved", value: { view: {} } }] }] });
