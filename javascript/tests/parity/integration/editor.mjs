import { run } from "../editor.mts";

export async function editorFixtures(read) {
  const names = ["base", "unsafe", "ambiguous", "drawing", "relocated"];
  const sources = await Promise.all(names.map(async (name) => [name, await read(`${name}.docx`)]));
  const result = await run({ ...Object.fromEntries(sources), image: await read("image.png") });
  const files = {
    body: result.body,
    cell: result.cell,
    noop: result.noop,
    shared: result.shared,
    local: result.local,
    "relocated-out": result.relocated,
    mixed: result.mixed,
    mixedShared: result.mixedShared,
  };
  const reports = { ...result.reports, mixed: result.mixedReports };
  return { files, reports };
}
