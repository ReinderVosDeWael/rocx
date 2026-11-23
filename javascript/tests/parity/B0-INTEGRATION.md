# B0 integration contract

- Each owner implements its reserved `src/<owner>.rs` `HOOKS` and
  `types/<owner>.d.ts`; no parallel edits to `lib.rs`, `program.rs`, entrypoints,
  manifests, or the type barrel. Use `..Hooks::EMPTY` for unused entries.
  `components` routes `(kind, scope, Resolve)` and `blocks` / `inlines` route
  `kind` in the appropriate context. Register additional standalone types in
  the corresponding `*Components` declaration map with `{declaration,
  resolved, scope?}`; map keys are unique registration names, not wire tags.
  `text_layout::SECTION` replaces the bootstrap section converter.
- `Decode<T> = fn(&Wire, &Pass, &str) -> rocx::Result<T>`. Use `Record::new`
  with allowed camelCase fields; use `required` / `optional` and `values` for
  Rust `Value` positions. `blocks` and `inlines` decode item-wise `Value`s.
  `optional` distinguishes a missing producer from a producer returning
  `undefined`; decode `Option<T>` at the latter's field site. Do not read or
  normalize deferred payloads in a component converter. `Pass::condition`
  builds Rust guards; `Pass::alias` maps authored names that differ from generic
  snake-to-camel conversion (e.g. paragraph `children` → `runs`). Use
  `program::resolve(component, projector)` to call `Component::resolve`.
- `Wire` is an owned ticket, not a `JsValue`; `wire.js()` borrows the local
  value for synchronous conversion only. `context.get` queues a request for
  the calling Rust producer’s future to poll. Never call a top-level resolve
  for that dependency. Rust supplies sharing, guard order and cycle detection.
  Abort drops owned work and its pending dependencies; shared work belongs to
  the shared token, not a cancelled waiter. Late host results are discarded.
- Use `boundary::handle(kind, typed_rust_value, view)` and `borrow::<T>` for
  immutable reusable handles. Table block/edit handles store `ResolvedTable`;
  inline handles store `ResolvedInline`, with a separate `ResolvedImage`
  handle for image edits. The JS wrapper is instance-bound and readonly.
  Build opaque anchors with an undefined view; reuse *the same envelope*
  for equal anchors in one snapshot to get stable JS object identity. Borrow
  bytes with `boundary::bytes` (copies the exact `Uint8Array` view); construct
  fresh typed arrays for output. Byte-valued view properties return copies.
- `document_fields`, `document`, and `document_view` extend the program and
  its snapshot projection; `section_view` / `paragraph_view` extend direct
  projections; `block_views` / `inline_views` take precedence over bootstrap
  projections. `handle_with_methods` maps JS method names to sync operations;
  these receive `{target, args}`. Document features register method pairs in
  `document_methods`. Specify `Paths::Package` for byte-package diagnostics
  (part names unchanged), `Paths::Declaration` otherwise. Adapter failures
  carry path and no synthetic rule; leave document/OPC validation to Rust.
- Root dispatch routes already reserved in both entrypoints (initialize
  asynchronously): `openDocx({bytes,options})`,
  `compileTemplate({document,source,options})`, `inlineText({handles})`,
  `createResolvedInline(input)`, `createResolvedBlockControl({properties,children})`,
  `validateValue(input)`, `validateAuthoringId({kind,value})`. Their public types
  ship when the corresponding hook is implemented. `./node` currently resolves
  to the installed Node entry; integration wires the forthcoming `file-io.js`
  and its Node-only types into that reserved subpath.

**Known Rust baseline limitation:** A single-pass indirect property cycle
rejects with `dependency_cycle`. With separate Rust root passes that start
different properties before the cycle closes, the wait edges can be split
between graphs and neither pass settles. This was reproduced natively on
`8c3cc2e`. B0 intentionally does not add a JS cycle detector or claim cross-pass
cycle rejection. The packed concurrency case uses two acyclic waiters.
