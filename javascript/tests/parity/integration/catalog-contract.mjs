import { catalogNames, generateCatalog } from "../../catalog.mjs";

export async function catalogFixtures(readFixture, options) {
  const catalog = await generateCatalog(readFixture, options);
  const names = Object.entries(catalogNames);
  if (Object.keys(catalog).length !== names.length || new Set(Object.values(catalogNames)).size !== names.length) {
    throw new Error("catalog filenames and generated artifacts differ");
  }
  return Object.fromEntries(names.map(([key, filename]) => {
    const bytes = catalog[key];
    if (!(bytes instanceof Uint8Array) || bytes.length === 0) {
      throw new TypeError(`catalog ${key}: expected nonempty Uint8Array`);
    }
    const match = /^all_features-javascript((?:\.[\w-]+)?)\.docx$/.exec(filename);
    if (!match) throw new Error(`unexpected catalog filename: ${filename}`);
    return [`catalog${match[1]}`, bytes];
  }));
}
