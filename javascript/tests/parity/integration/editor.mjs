import { run } from "../editor.mts";

export async function editorFixtures(read) {
  const names = ["base", "unsafe", "ambiguous", "drawing", "relocated", "styles-edge", "styles-missing", "styles-invalid"];
  const sources = await Promise.all(names.map(async (name) => [name, await read(`${name}.docx`)]));
  const result = await run({ ...Object.fromEntries(sources), image: await read("image.png") });
  const files = {
    body: result.body,
    noop: result.noop,
    removal: result.removal,
    shared: result.shared,
    local: result.local,
    firstShared: result.firstShared,
    firstLocal: result.firstLocal,
    "relocated-out": result.relocated,
    mixed: result.mixed,
    mixedShared: result.mixedShared,
  };
  const reports = { ...result.reports, mixed: result.mixedReports };
  return { files, reports };
}
