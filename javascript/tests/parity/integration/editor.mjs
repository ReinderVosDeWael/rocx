import { editorSources, run } from "../editor.mts";

export async function editorFixtures(read) {
  const sources = await Promise.all(editorSources.map(async (name) => [name, await read(`${name}.docx`)]));
  const { reports, mixedReports, relocated, ...files } = await run({
    ...Object.fromEntries(sources), image: await read("image.png"),
  });
  return { files: { ...files, "relocated-out": relocated }, reports: { ...reports, mixed: mixedReports } };
}
