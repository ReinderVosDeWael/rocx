import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

const host = "127.0.0.1";
const server = await createServer({ configFile: false, server: { host } });
let browser;
try {
  await server.listen(0);
  browser = await chromium.launch();
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error));
  await page.goto(`http://${host}:${server.httpServer.address().port}/`);
  const result = await page.evaluate(async () => {
    const { future, generateDocx, property, resolveComponent, resolveDocument } =
      await import("/node_modules/rocx-docx/browser.js");
    const paragraph = (text, condition) => ({ kind: "paragraph", text, condition });
    const program = (...children) => ({
      sections: [{ kind: "section", children }],
      timestamp: "2022-02-03T04:05:06Z",
    });
    let first, second, third;
    function depends(next) {
      return future(async (context) => {
        await Promise.resolve();
        return context.get(next().reference());
      });
    }
    first = property("first", depends(() => second));
    second = property("second", depends(() => third));
    third = property("third", depends(() => first));
    let cycle;
    try {
      await resolveDocument(program(paragraph(first.reference())), { signal: AbortSignal.timeout(3000) });
    } catch (error) {
      cycle = [error instanceof Error, error.rule, error.path];
    }
    let fieldsStarted = 0;
    const field = () => future(async () => { fieldsStarted++; return "hidden"; });
    const excluded = await resolveComponent(paragraph(field(), false));
    const cause = {};
    let guard;
    try {
      await resolveComponent(paragraph(field(), () => { throw cause; }));
    } catch (error) {
      guard = [error instanceof Error, error.cause === cause, error.path];
    }
    const release = Promise.withResolvers();
    const waiting = Promise.withResolvers();
    let starts = 0;
    const shared = property("shared", future(async () => {
      starts++;
      await release.promise;
      return "ready";
    }));
    const left = resolveComponent(paragraph(shared.reference()), { signal: AbortSignal.timeout(3000) });
    const right = resolveComponent(paragraph(future(async (context) => {
      const result = context.get(shared.reference());
      waiting.resolve();
      return result;
    })), { signal: AbortSignal.timeout(3000) });
    await Promise.race([waiting.promise, left, right]);
    release.resolve();
    const texts = (await Promise.all([left, right])).map((p) => p.view.text);
    const bytes = await generateDocx(program(paragraph("browser")));
    return {
      cycle, guard, excluded: excluded === undefined, fieldsStarted, starts, texts,
      signature: Array.from(bytes.subarray(0, 4)),
    };
  });
  assert.deepEqual(result, {
    cycle: [true, "dependency_cycle", "sections[0].children[0].text"],
    guard: [true, true, "paragraph.condition"],
    excluded: true, fieldsStarted: 0, starts: 1, texts: ["ready", "ready"],
    signature: [0x50, 0x4b, 3, 4],
  });
  assert.deepEqual(errors, []);
  console.log(`B0 packed: Chromium ${browser.version()}, direct browser entry, real Rust wait graph, guards and waiters`);
} finally {
  await browser?.close();
  await server.close();
}
