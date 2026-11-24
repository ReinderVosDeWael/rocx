import { generateCatalog } from "../../catalog.mjs";

const suffixes = {
  final: "",
  template: ".template",
  edited: ".edited",
  headerSource: ".header-source",
  headerShared: ".header-shared",
  headerLocal: ".header-local",
  headerFirstShared: ".header-first-shared",
  headerFirstLocal: ".header-first-local",
  runText: ".run-text",
};

export async function catalogFixtures(readFixture, options) {
  const catalog = await generateCatalog(readFixture, options);
  return Object.fromEntries(Object.entries(suffixes).map(([key, suffix]) => {
    const bytes = catalog[key];
    if (!(bytes instanceof Uint8Array) || bytes.length === 0) {
      throw new TypeError(`catalog ${key}: expected nonempty Uint8Array`);
    }
    return [`catalog${suffix}`, bytes];
  }));
}
