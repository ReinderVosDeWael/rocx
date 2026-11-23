import { bookmarkEnd, bookmarkStart, future, lazy, resolveDocument, type DocumentProgram } from "rocx-docx";

// Browser-safe counterpart to the native structured oracle. The separate Node
// owner suite also checks ZIP relationships, rejected inputs and deferred paths.
export async function structuredFixture(): Promise<Uint8Array> {
  const text = (value: string) => ({ kind: "text", text: value } as const);
  const bookmark = "Summary";
  const expected = [
    "block properties", "block children", "field instruction", "field cache", "field properties",
    "inline properties", "inline children", "tracked revision", "absent revision",
  ] as const;
  type Producer = typeof expected[number];
  const calls = new Map<Producer, number>();
  const record = (name: Producer) => {
    calls.set(name, (calls.get(name) ?? 0) + 1);
  };
  const asyncValue = <T,>(name: Producer, value: T) => future(async () => { record(name); return value; });
  const doc = await resolveDocument({
    timestamp: "2024-02-29T01:02:03Z",
    sections: [{
      kind: "section",
      children: [
        { kind: "contentControl", properties: asyncValue("block properties", { id: 10, tag: "block" }),
          children: lazy(() => {
            record("block children");
            return [{ kind: "paragraph", runs: [text("block")] }] as const;
          }) },
        { kind: "paragraph", runs: [
          await bookmarkStart(bookmark),
          { ...text("marker"), revision: asyncValue("absent revision", undefined) },
          await bookmarkEnd(bookmark),
          { kind: "hyperlink", target: { kind: "bookmark", name: bookmark },
            children: [text("link")], properties: { history: false } },
          { kind: "field", instruction: asyncValue("field instruction", { kind: "page" } as const),
            children: lazy(() => { record("field cache"); return [text("3")]; }),
            properties: asyncValue("field properties", { form: "complex", dirty: false, locked: true } as const) },
          { kind: "contentControl", children: asyncValue("inline children", [text("Beta")]),
            properties: lazy(() => {
              record("inline properties");
              return {
                id: 11, text: { kind: "dropDown", selected: "b",
                  items: [{ displayText: "Alpha", value: "a" }, { displayText: "Beta", value: "b" }] },
              } as const;
            }) },
          { ...text("tracked"), revision: lazy(() => {
            record("tracked revision");
            return { kind: { kind: "insertion" }, author: "Ada" } as const;
          }) },
        ] },
      ],
    }],
  } satisfies DocumentProgram);
  const { blocks, paragraphs } = doc.view.sections[0].view;
  const runs = paragraphs[0].view.runs;
  const block = blocks[0].view;
  const blockChild = "kind" in block && block.kind === "contentControl" ? block.children[0]?.view : undefined;
  const plain = runs[1].view;
  const link = runs[3].view;
  const field = runs[4].view;
  const control = runs[5].view;
  const tracked = runs[6].view;
  if (paragraphs.length !== 1 || link.kind !== "hyperlink" || link.properties?.history !== false ||
      field.kind !== "field" || field.instruction.kind !== "page" || field.properties.dirty !== false ||
      field.properties.locked !== true || field.displayText !== "3" ||
      !("kind" in block) || block.kind !== "contentControl" || block.properties.tag !== "block" ||
      !blockChild || !("text" in blockChild) || blockChild.text !== "block" ||
      control.kind !== "contentControl" || control.properties.text.kind !== "dropDown" ||
      control.properties.text.selected !== "b" || control.displayText !== "Beta" ||
      tracked.kind !== "text" || tracked.revision?.kind.kind !== "insertion" || tracked.displayText !== "tracked" ||
      plain.kind !== "text" || plain.revision !== undefined || plain.displayText !== "marker") {
    throw new Error("unexpected resolved view for deferred structured content");
  }
  const bytes = doc.toDocx();
  for (const name of expected) {
    if (calls.get(name) !== 1) throw new Error(`structured producer ${name}: expected one call, got ${calls.get(name) ?? 0}`);
  }
  return bytes;
}
