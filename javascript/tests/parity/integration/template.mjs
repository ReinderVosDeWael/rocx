import { compileTemplate, future, resolveDocument } from "rocx-docx";

// Byte-source subset of the Node template suite (filesystem operations are Node-only).
export async function templateFixtures(bytes) {
  const timestamp = "2026-11-21T00:00:00Z";
  const options = { profile: "transitional", timestamp: "2026-12-01T12:30:00.123Z" };
  const sections = (...texts) => texts.map((text) => ({
    kind: "section", children: [{ kind: "paragraph", text }],
  }));
  let calls = 0;
  const report = await resolveDocument({
    timestamp, sections: sections(future(async () => { calls++; return "TOKEN"; })),
  });
  const fresh = report.toDocx();
  const override = report.toDocx(options);
  if (calls !== 1 || report.view.timestamp !== timestamp ||
      (fresh.length === override.length && fresh.every((byte, i) => byte === override[i]))) {
    throw new Error("render override changed the snapshot, reran a producer, or was ignored");
  }
  try {
    report.toDocx({ profile: "strict" });
    throw new Error("strict rendering unexpectedly accepted");
  } catch (error) {
    if (!(error instanceof Error) || error.rule !== "unsupported_profile" || error.path !== "options.profile") {
      throw error;
    }
  }
  const append = await resolveDocument({
    timestamp, sections: sections("Added section one", "Added section two"),
  });
  const source = { bytes, edits: { paragraphIndex: 1, replacements: [["Insert table after me", "Patched"]] } };
  const plan = await compileTemplate(append, source, options);
  plan.validate();
  const document = plan.part("word/document.xml");
  if (!document || !new TextDecoder().decode(document).includes(">Patched<") ||
      plan.part("missing.xml") !== undefined) {
    throw new Error("template plan did not expose expected parts and replacements");
  }
  document.fill(0);
  const edited = plan.toBytes();
  const direct = append.toDocxWithTemplate(source, options);
  if (edited.length !== direct.length || !edited.every((byte, index) => byte === direct[index])) {
    throw new Error("plan payload mutation changed the encoded template");
  }
  return { fresh, override, edited };
}
