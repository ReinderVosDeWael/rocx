import { run } from "../editor.mts";

export async function editorFixtures(read) {
  const names = ["range-source", "base", "unsafe", "ambiguous", "drawing", "relocated", "styles-edge", "styles-missing", "styles-invalid"];
  const sources = await Promise.all(names.map(async (name) => [name, await read(`${name}.docx`)]));
  const result = await run({ ...Object.fromEntries(sources), image: await read("image.png") });
  const files = {
    tracked: result.tracked, reviewed: result.reviewed,
    moved: result.moved, moveAccepted: result.moveAccepted,
    body: result.body,
    tableStructure: result.tableStructure,
    cell: result.cell,
    mixedBody: result.mixedBody,
    blocks: result.blocks,
    noop: result.noop,
    inline: result.inline,
    removal: result.removal,
    range: result.range,
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
