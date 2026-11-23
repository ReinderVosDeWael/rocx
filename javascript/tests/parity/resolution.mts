import {
  createResolvedBlockControl, createResolvedInline, deferred, future, inlineText, lazy,
  property, ready, resolveComponent, resolveDocument, resolveValue, validateAuthoringId, validateValue,
  type CreateResolvedInlineInput, type InlineChildren, type ProducerContext, type Property,
  type PropertyId, type ResolvedBlock, type ResolvedImage, type ResolvedInline,
  type ResolvedInlineView, type ResolvedParagraph, type ValidateValueInput, type Value,
} from "rocx-docx";

export async function recursiveValuesMapAndSingleUse() {
  const nested: Value<number> = lazy(() => future(async () => lazy(() => 6)));
  const mapped: Value<string> = nested.map((number, { signal }) => {
    const aborted: boolean = signal.aborted;
    return ready(`${number.toFixed(1)}:${aborted}`);
  });
  const result: string = await resolveValue(mapped);
  // @ts-expect-error Future work must be passed as a factory, not a Promise.
  future(Promise.resolve(1));
  // @ts-expect-error Lazy work cannot return a Promise directly.
  lazy(async () => 1);
  // @ts-expect-error A map stage must return a Value, not a Promise.
  nested.map(async (n) => n + 1);
  return result;
}

export async function sharedValuesAndNamedDependencyGraph() {
  const value = deferred(lazy(() => 1));
  let named: Property<number>;
  named = property("named", future(async (context: ProducerContext) => {
    await Promise.resolve();
    const underlying: number = await context.get(value);
    return underlying + 1;
  }), { condition: () => true });
  const id: PropertyId = named.reference().id;
  const waited = await resolveValue(future((context) => context.get(named.reference())));
  // @ts-expect-error A Property itself isn't a dependency reference for context.get.
  future((context) => context.get(named));
  // @ts-expect-error PropertyId is opaque, not a string.
  const forged: PropertyId = "named";
  return { id, waited, forged };
}

export async function standaloneAndConditionalComponents(signal: AbortSignal) {
  const block = await resolveComponent({
    kind: "contentControl", condition: false, children: [{ kind: "paragraph", text: "block" }],
  }, { scope: "block", signal });
  const inline = await resolveComponent({
    kind: "contentControl", condition: false, children: [{ kind: "text", text: "inline" }],
  }, { scope: "inline", signal });
  const text = await resolveComponent({ kind: "text", text: "may be excluded" }, { signal });
  // @ts-expect-error Guarded standalone components may be undefined.
  const run: ResolvedInline = text;
  const footer = await resolveComponent({ kind: "footer", children: [], condition: false }, { signal });
  return { block: block?.view.children, inline: inline?.view.children, footer: footer?.view.blocks, run };
}

export async function cancelledWaiterDoesNotPoisonSharedWork(signal: AbortSignal) {
  const shared = deferred(future(({ signal: producerSignal }) => {
    const ownSignal: AbortSignal = producerSignal;
    return ownSignal.aborted ? "stopped" : "shared";
  }));
  const wait: Promise<string> = resolveValue(shared, { signal });
  const survivor: Promise<string> = resolveValue(shared);
  return [wait, survivor] as const;
}

export async function resolvedBlockViewsAndReuse(paragraph: ResolvedParagraph) {
  const table = await resolveComponent({
    kind: "table",
    rows: [{ kind: "tableRow", children: [{ kind: "tableCell",
      properties: { gridSpan: 2 }, children: [{ kind: "resolved", value: paragraph }],
    }] }],
  });
  if (!table) return;
  const columns: number = table.view.columns;
  const span: number = table.view.rows[0].view.cells[0].view.gridSpan;
  const control = await createResolvedBlockControl({}, [table]);
  const blocks: readonly ResolvedBlock[] = [paragraph, control, table];
  const document = await resolveDocument({ sections: [{
    kind: "section", children: blocks.map((value) => ({ kind: "resolved", value })),
  }] });
  const direct: readonly ResolvedParagraph[] = document.view.sections[0].view.paragraphs;
  return { columns, span, direct };
}

export async function resolvedInlineViewsConstructorsAndReuse(image: ResolvedImage) {
  const text = await createResolvedInline({
    kind: "text", text: "before", format: { bold: false },
    revision: { kind: { kind: "deletion" }, author: "author" },
  });
  const raw: string = text.view.text;
  const tab = await createResolvedInline({ kind: "tab", format: { bold: true } });
  const page = await createResolvedInline({ kind: "break", breakType: "page" });
  const picture = await createResolvedInline({ kind: "image", image, comment: { text: "image" } });
  const variants = [
    { kind: "shape", properties: {} },
    { kind: "columnChart", properties: { width: 100, height: 50, categories: ["a"], values: [1] } },
    { kind: "fraction", numerator: "1", denominator: "4" },
    { kind: "hyperlink", target: { kind: "external", url: "https://example.com/" }, children: [text] },
    { kind: "field", instruction: { kind: "page" }, children: [text] },
    { kind: "contentControl", children: [text] },
    { kind: "noteReference", target: { kind: "footnote", id: 1 } },
    { kind: "rangeStart", id: "r", range: { kind: "bookmark", name: "_valid" } },
    { kind: "rangeEnd", id: "r" },
  ] as const satisfies readonly CreateResolvedInlineInput[];
  const constructed = await Promise.all(variants.map((variant) => createResolvedInline(variant)));
  const inlines: readonly ResolvedInline[] = [text, tab, page, picture, ...constructed];
  const runs: InlineChildren = inlines.map((value) => ({ kind: "resolved", value }));
  const doc = await resolveComponent({ kind: "paragraph", runs });
  const renderedText: Promise<string> = inlineText(inlines);
  const view: ResolvedInlineView = page.view;
  // @ts-expect-error Handles cannot be constructed by copying their public projections.
  inlineText([{ view }]);
  // @ts-expect-error A checked container constructor takes resolved handles, not declarations.
  createResolvedInline({ kind: "contentControl", children: [{ kind: "text", text: "not resolved" }] });
  // @ts-expect-error A ready run constructor cannot have a component guard.
  createResolvedInline({ kind: "tab", condition: false });
  return { raw, doc, renderedText, bold: tab.view.format.bold, breakType: page.view.breakType };
}

export async function standaloneValidationAndDiagnostics(paragraph: ResolvedParagraph) {
  const note = await resolveComponent({ kind: "note", target: { kind: "footnote", id: 1 }, children: [] });
  const comment = await resolveComponent({ kind: "comment", children: [] });
  const document = await resolveDocument({ sections: [] });
  const inputs = [
    { kind: "runFormat", value: {} },
    { kind: "paragraphFormat", value: {} },
    { kind: "sectionProperties", value: {} },
    { kind: "numberingDefinition", value: { id: 1, levels: [] } },
    { kind: "imageTransformation", value: {} },
    { kind: "tableCellProperties", value: {} },
    { kind: "cellMargins", value: {} },
    { kind: "tableRowProperties", value: { height: { kind: "exact", points: 12 } } },
    { kind: "resolvedInline", value: await createResolvedInline({ kind: "tab" }) },
    { kind: "resolvedParagraph", value: paragraph },
    { kind: "resolvedDocument", value: document },
    { kind: "documentDefaults", value: {} },
    { kind: "textRevision", value: { author: "a", kind: { kind: "insertion" } } },
    { kind: "commentMetadata", value: {} },
    ...(note ? [{ kind: "resolvedNote", value: note }] as const : []),
    ...(comment ? [{ kind: "resolvedCommentBody", value: comment }] as const : []),
  ] as const satisfies readonly ValidateValueInput[];
  await Promise.all(inputs.map((value) => validateValue(value)));
  await validateAuthoringId("bookmarkName", "_valid");
  // @ts-expect-error The validation discriminant requires a matching handle type.
  await validateValue({ kind: "resolvedInline", value: document });
  // @ts-expect-error Authoring-ID kinds are fixed.
  await validateAuthoringId("bookmark", "_valid");
}
