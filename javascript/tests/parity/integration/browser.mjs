import { catalogFixtures } from "./catalog-contract.mjs";
import { editorFixtures } from "./editor.mjs";
import { portableFixtures } from "./fixtures.mjs";

async function read(name) {
  const response = await fetch(`/fixtures/${name}`);
  if (!response.ok) throw new Error(`Missing browser fixture ${name}: HTTP ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

globalThis.runInstalledParity = async () => {
  const fixtures = await portableFixtures(
    await read("sample.png"), await read("no-margins.docx"), await read("imported-editor.docx"),
  );
  const catalog = await catalogFixtures(read);
  const editor = await editorFixtures((name) => read(`editor/${name}`));
  const arrays = (files) => Object.fromEntries(
    Object.entries(files).map(([name, bytes]) => [name, Array.from(bytes)]),
  );
  return { files: arrays({ ...fixtures, ...catalog }), editor: arrays(editor.files), reports: editor.reports };
};
