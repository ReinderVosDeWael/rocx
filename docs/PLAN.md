# Remaining implementation plan and agent handoff

## Purpose and starting point

This file is a standalone handoff for continuing **rv-docx-rs**. It records
product decisions, implemented foundations, remaining work, execution order and
verification requirements. Read it before making changes. A roadmap item is not
an assertion that the feature already works or that a suspected defect has been
reproduced.

Baseline: commit **`b903ab2`**, version **0.2.0**, on `main` in
<https://github.com/ReinderVosDeWael/rv-docx-rs>. The working directory used for
development was `/workspace/cmi-docx-rs`; that historical directory name is not
the package name. Verify the checkout and current Git status before proceeding.
The Cargo package is `rv-docx-rs`, the Rust import is `rv_docx_rs`, the Python
distribution is `rv-docx-rs`, and the Python import is `rv_docx`.

The repository reorganization is implemented. Do not restart it or treat
[REORGANIZATION.md](REORGANIZATION.md) as an unexecuted task list. See
[MODULES.md](MODULES.md) for actual boundaries,
[MIGRATION.md](MIGRATION.md) for deliberate 0.2 API changes,
[COMPATIBILITY.md](../COMPATIBILITY.md) for supported behavior, and
[OOXML.md](OOXML.md) for the specification and coverage map.
[ARCHITECTURE.md](ARCHITECTURE.md) describes the historical starting point.

At the baseline, verification passed: 82 Rust tests, 2 doctests, and 110 Python
tests; one obsolete upstream CMI behavior test was deliberately skipped. These
counts are historical evidence, not fixed targets. Clippy and rustdoc passed
with warnings denied, independent official-schema tests ran with local schemas,
and a clean wheel worked without `docx` or `cmi_docx` installed.

## Product decisions to preserve

- Maintain an ergonomic declarative interface in Rust and Python. CMI Docx is
  inspiration and a historical test source, **not an API compatibility rule**.
  Preserve useful behavior, not CMI bugs, constructor signatures or dataclass
  layouts. Explain breaking changes in the migration guide.
- Favor domain-driven design and functional programming: explicit domain values,
  algebraic content models, immutable resolved snapshots, pure transformations,
  and effects at adapters/application boundaries. Local mutation of an owned
  compiler buffer or cache is acceptable. Do not build a class hierarchy around
  individual XML tags.
- Generate OOXML directly. Keep runtime dependencies small; justify additions
  against actual needs. Do not add a runtime DOCX SDK, a Word dependency, an
  obligatory async runtime, or production `python-docx` dependency.
- Async values and conditions remain in the authoring tree. Dependencies must
  work for **any supported async property**, not just paragraph text. Resolution
  preserves declaration order, shares work according to its documented lifetime,
  and reports explicit-reference cycles and excluded sources clearly.
- Rust owned futures/declarations are consumed once. Python declarations remain
  unchanged; factories support fresh passes, raw coroutines are single-use, and
  resolved snapshots contain no pending producers or asset reads. Rendering a
  snapshot does not reevaluate user code. Conditions are currently synchronous.
- Use **uv** for Python dependencies, environments and commands; no
  `requirements.txt`. Keep python-docx as optional `interop` and a development
  readback tool. Both frontends compile through one Rust domain and serializer.
- Tests must check expected content and relationships, not merely that a reader
  opens a ZIP. Keep Rust-generated-file checks through independent Python
  consumers, schema checks, negative controls and preservation regressions.
- Cite edition-specific OOXML clauses near relevant codecs and in documentation.
  Separate generation, typed interpretation, safe editing and opaque retention.
  No claim of full DOCX support follows from a successful consumer or XSD test.
- Keep the current single-crate structure unless an actual release/build boundary
  warrants another crate. The optional Python extension is a possible later split.
- Work in reviewable, green commits and retain backups on the existing GitHub
  repository. Do not create another repository or rename the project again.

The long-term goal is broad DOCX coverage. It is not a bounded promise to
implement every ECMA/Microsoft extension in one iteration. The ordering below is
a proposed engineering roadmap; keep unimplemented features explicitly marked.
No layout, pagination, field-evaluation or Office automation engine is currently
part of the project.

## What already exists

The current surface includes documents/sections, paragraphs and explicit
text/tab/break/image inlines and rich external hyperlinks, selected run/paragraph formatting, RGB/theme/auto
run colors, underline styles, styles, numbering, nested tables and merges,
page geometry/columns, default/first/even headers and footers, body comments,
inline images, bookmarks/internal anchor links, footnotes/endnotes, cached simple/
complex fields, core metadata, and structural
template text replacement.
Comments support explicit overlapping paired body ranges and structured base
content/metadata, not threaded comments or singleton point anchors. Pictures inspect file headers;
they do not decode pixel data.

OPC loading follows relationships rather than fixed Word filenames. Typed part
names/targets/IDs, content-type coverage, graph checks, archive/XML budgets,
namespace-safe rewriting, edited-part caching, media deduplication and immutable
`PackagePlan` output are implemented. Untouched part payloads retain bytes;
edited XML and the ZIP container are reserialized. Runtime schema-order rules
cover selected generated properties; they are not a full schema validator.

Only the **Transitional** output profile is implemented. `Strict` is an explicit
unsupported profile. DOCX/DOTX main-part inputs are accepted for DOCX output;
this does not mean arbitrary macro-enabled or signed packages are supported.

### Source ownership

| Location | Owns / extension points |
| --- | --- |
| `src/document/{content,hyperlink,range,notes,field,table,resources,values,validation,color,measure,identity,diagnostic}.rs` | Resolved content, domain values and invariants; standard library only |
| `src/authoring/{mod,components,hyperlink,range,notes,field,table}.rs` | Constructors, unresolved values and conditions; no XML, filesystem or interpreter code |
| `src/resolution/{mod,property,interpreter,components}.rs` | Value/shared computation/reference primitives, dependency graphs and native interpretation |
| `src/ooxml/wordprocessing/` | Text, paragraphs, fields, formatting, tables, sections, stories, styles and comments |
| `src/ooxml/drawing/`, `resources.rs`, `ranges.rs`, `notes.rs`, `story.rs`, `style_catalog.rs`, `schema.rs` | Pictures, media/style/range/note/story registries and Word ordering rules |
| `src/opc/{mod,names}.rs` | Generic package identities, relationships, budgets and write plans; no Word defaults |
| `src/xml/{mod,name}.rs` | Generic expanded-name XML tree/writer and parsing/character constraints |
| `src/template/mod.rs` | Pure edit requests, scopes, token spans and edit-boundary rules |
| `src/application.rs`, `src/adapters/` | Compile/render orchestration, clock/file/image effects and optional PyO3 adapters |
| `python/rv_docx/declarative/`, `resolution/` | Python authoring and asyncio interpretation into immutable snapshots |
| `python/rv_docx/{io,interop}.py` | Python effects and optional editable python-docx output |
| `tests/`, `tools/validate_ooxml.py` | Domain/resolution/preservation/readback tests and independent schema tooling |

The flow must remain: **authoring → resolution → resolved domain → pure OOXML
compilation → package plan → encoding/output adapter**. Template bytes, explicit
render options and a captured timestamp are compiler inputs. OPC/XML must not
call the top-level Word compiler. Python conversion must not reconstruct ready
Rust declarations and run a second resolver.

## Execution order

Start with R0, then R1, then R2. R3 depends on R2; R4 and R5 build on R3.
R6–R9 are later feature families and should be delivered in smaller slices.
R10 is a separate conformance project after the current feature vocabulary is
well covered. Run the common completion checklist for every slice.

### R0 — Make the verified baseline repeatable

**Implementation checkpoint (2026-11-17, `171229c`):** CI and the
isolated wheel probe are implemented in `.github/workflows/ci.yml` and
`tools/check_wheel.py`. The standards job requires schemas, caches the
digest-checked download, and validates Rust-generated `feature_parity` output.
Regression tests exercise both required-schema failure and optional local skips.
All five dependency alerts were inspected through GitHub; source/feature-based
retain-for-now dispositions and residual risks are in [DEPENDENCY-AUDIT.md](DEPENDENCY-AUDIT.md).
No versions, alert states, upstream fixtures or normalization rules were changed.

Local evidence (stable Rust, Python 3.12; Python commands used
`UV_PROJECT_ENVIRONMENT=.venv-opencode`):

- `uv sync --locked --group test` provisioned the authorized isolated environment.
- `cargo fmt --check`, default and all-feature `cargo test --locked --all-targets`
  passed (82 tests with all features, including 12 independent Python readbacks).
- `cargo test --locked --all-features --doc` passed (2 doctests); Clippy with
  `-D warnings` and rustdoc with warnings denied passed.
- Required official-schema Python run: 112 passed, 1 intentional upstream skip.
  An explicit missing-schema run failed as expected, rather than skipping.
- `tools/check_wheel.py` passed: clean installed native extension, optional-only
  metadata, exact Unicode/formatting, bytes/save and snapshot checks.

GitHub-hosted CI and the Rust 1.88/Python 3.13–3.14 matrix have not run locally.
R1 completion is recorded below. The hosted R0 version matrix remains unverified
in this local session.

**Goal:** another checkout can reproduce the checks without undocumented local
scripts or silently skipped standards validation.

1. Add repository CI configuration; no `.github` workflows exist at the baseline.
   Cover default Rust builds, all-feature tests including Python readback,
   doctests, formatting, Clippy, rustdoc and Python bindings. Test the declared
   Rust MSRV (1.88) as well as stable; use Python versions supported by the
   `>=3.12` / `abi3-py312` packaging contract.
2. Turn the manual clean-installed-wheel verification into a committed reusable
   tool/test. Build with `uv build --wheel`, install into a fresh uv environment,
   run outside the source tree, and confirm that the installed extension is used.
   Assert that `docx` and `cmi_docx` are absent, native bytes/save works, snapshot
   output is repeatable, exact Unicode text and formatting are present, and
   `to_docx()` explains the missing optional extra. Inspect dependency metadata.
3. Make a CI standards job download/cache the digest-pinned official schemas
   through `tools/validate_ooxml.py`. Missing schemas must fail that job rather
   than turn its tests into skips. Preserve offline validation after cache setup.
   Local optional skipping remains useful. Include representative Rust-generated
   feature files, not only Python-generated minimal documents.
4. Record all intentional skips and normalization policies. Keep the original
   upstream provenance/hash and the one retired trailing-section expectation.
   Do not relax comparisons to get a green result.
5. Triage the five dependency alerts reported by GitHub at the baseline
   (one critical, two high, two moderate). Inspect actual advisory/dependency
   paths before changing versions; the prior session could not read advisory
   details. Distinguish production, optional and reference-test dependencies.

**Done when:** a fresh checkout has executable CI and wheel checks, the standards
job cannot silently skip, and dependency findings have evidence-based fixes or
documented dispositions. Run no automatic publishing as part of this work.

### R1 — Finish cross-language API and invariant consistency

**R1 complete (2026-11-18).** Continuation checkpoint subject:
`feat: complete cross-language R1 consistency`, following `7e69c33`.
The engineer requested all of R1, then a green commit and a stop for compaction.

Completion against the six R1 acceptance areas:

1. Public fields/intentional differences are inventoried in
   [DEFERRED-FIELDS.md](DEFERRED-FIELDS.md). Rust now defers `Metadata`, section
   properties, table/cell property aggregates, paragraph numbering and document
   styles/numbering/comment-author/timestamp through `Value`/`PropertyRef`.
   Static structural shells/discriminants remain deliberate ownership choices.
2. Both interpreters exercise the shared scenario matrix, including metadata and
   child dependencies. `Value::map`, `Block::Resolved` and `Inline::Resolved` allow
   Rust to reuse cloneable immutable content without cloning owned futures or
   reevaluating original guards/assets. Raw authoring child lists are intentionally
   not cloneable; resolved-content reuse is the supported reference contract.
3. Existing guard/lifetime semantics are unchanged. Concurrent roots, nested
   passes, diamond/cycle cases, excluded sources, producer failures, cancellation
   and fresh-versus-explicit sharing are covered by the first and final R1 suites.
4. Native errors expose stable `Rule::code`, location and optional cause text while
   retaining standard Python exception categories and a cause chain. Python
   producers retain identity and original causes; image/output I/O has adapter
   context. Domain traversal uses typed `DeclarationPath`; known package context
   uses canonical OPC names. Legacy generic `Validation` messages remain classified
   as `invalid_content`, not guessed from text. This is the planned incremental
   diagnostic contract, not exhaustive reclassification of every legacy check.
5. Pure domain validators now own supported format/measurement, section-size/
   column-count, image-transformation and cell-span invariants. Both frontends and
   codecs reuse them; template-dependent checks remain in compilation. `Emu`
   converts to twips with integer rounding. Read-only snapshot accessors cover
   metadata, resources, sections/stories and table/cell properties. Application
   limits versus schema constraints are documented.
6. Python `RenderOptions`/`ReadLimits` and Rust output/save option methods expose
   profile, timestamp override and package-read budgets. Pure compilation still
   takes bytes; template file loading stays at application boundaries. Overrides
   do not mutate snapshots; original image mutation/deletion regressions remain.

Final verification (stable Rust, CPython 3.12; Python commands used
`UV_PROJECT_ENVIRONMENT=.venv-opencode`):

- `uv sync --locked --group test` rebuilt the isolated editable extension.
- `cargo fmt --check`; `cargo test --locked --all-targets`; and
  `cargo test --locked --all-features --all-targets` passed (98 all-feature tests,
  including 13 independent Python readbacks and their negative controls).
- `cargo test --locked --all-features --doc` passed (2 doctests).
- `cargo clippy --locked --all-features --all-targets -- -D warnings` passed.
- `RUSTDOCFLAGS='-D warnings' cargo doc --locked --all-features --no-deps
  --document-private-items` passed.
- Full Python suite with `RV_DOCX_REQUIRE_SCHEMAS=1` and
  `RV_DOCX_SCHEMA_DIR=/tmp/opencode/rv-docx-schemas`: 135 passed, 1 intentional
  upstream skip. New errors/options tests initially failed against the old native
  extension; they pass after rebuilding. Test setup/canonical-path expectations
  and Clippy findings encountered during implementation were corrected.
- Clean installed-wheel probe passed, including explicit timestamp/Strict-error
  behavior without interop dependencies. `resolution_options` examples ran in
  both languages; official schemas validated each (9 covered parts/relationships).
- `git diff --check` passed. Rust 1.88, other Python versions and hosted CI were
  not run locally. Existing untracked `docs/DEPENDENCY-AUDIT.md` remains untouched.

**Handoff at R1 completion: R2 external hyperlinks.** Start by reading the normative
inline parent model and defining the smallest domain/authoring/codec contract in
both languages. Do not start R3 ranges/bookmarks or change dependency ownership
as part of hyperlinks. R2 was not implemented in that checkpoint; completion is
recorded below.

**First slice implemented and verified (2026-11-18):** the complete field inventory,
intentional lifetime differences, shared scenario matrix and remaining gaps are
in [DEFERRED-FIELDS.md](DEFERRED-FIELDS.md). Rust image transformation and alt-text
fields now use `Value<Option<...>>` and the existing `PropertyRef` mechanism;
they resolve concurrently with data after the image condition passes. Producer
errors retain field paths and causes. Ready assignments now need `.into()`;
see [MIGRATION.md](MIGRATION.md). No dependency ownership semantics were changed.

New Rust/Python contract tests cover image-field references, concurrency, hidden
producers, option errors, concurrent root passes, nested cycle detection,
cancellation and snapshot rendering after asset mutation/deletion. Rust also
adds explicit diamond, direct/indirect cycle, and fresh/shared-pass cases.
Independent Python readback now exercises Rust-deferred image options. Runnable
examples are `examples/image_dependencies.{rs,py}`.

Verification on stable Rust/Python 3.12 (Python commands used
`UV_PROJECT_ENVIRONMENT=.venv-opencode`):

- `cargo fmt --check`; default and all-feature `cargo test --locked --all-targets`
  passed (92 tests with all features, including 10 new Rust contract tests).
- `cargo test --locked --all-features --doc` passed (2 doctests).
- Clippy `--all-features --all-targets -- -D warnings` and rustdoc with
  `RUSTDOCFLAGS='-D warnings'` passed.
- Required-schema Python suite passed: 123 tests, 1 intentional upstream skip,
  including 11 new Python contract cases.
- Clean installed-wheel probe passed; both new examples ran, and the independent
  official-schema validator accepted each (8 covered parts plus relationships).
- `git diff --check` passed. The initial new Rust reference/async-field tests
  failed compilation against the former ordinary image-option fields, as expected.

**Checkpoint:** `feat(resolution): defer image options and document R1 contracts`
(locate its hash with `git log`). The engineer requested a commit and a stop for
conversation compaction at this green checkpoint. The pre-existing untracked
`docs/DEPENDENCY-AUDIT.md` is not part of this R1 change.

The gaps recorded at that first checkpoint are addressed by the R1 completion
entry above. R0 is checkpointed as `171229c`; its hosted/version-matrix results
remain unverified in this session.

**Goal:** shared semantics and diagnostics are deliberate before enlarging the
content algebra. These are follow-ups to the reorganization, not another move of
every source file.

1. Inventory public async/deferred fields in both languages. Rust now supports
   deferred paragraph/run format aggregates, but some image/section/table
   conveniences still contain ordinary optional values. Document intentional
   differences; introduce `Value<T>` or an aggregate where users need deferred
   production. Avoid duplicating each field as both optional and asynchronous.
2. Add a shared scenario matrix exercised by both interpreters: dependencies on
   text, formatting, image data, children and metadata; source after consumer;
   diamond dependencies; excluded source; direct/indirect cycles; producer error;
   cancellation of one waiter and of the entire pass; fresh passes versus an
   explicitly shared computation. Every supported async field must accept the
   same reference mechanism, without text-specific branches.
3. Preserve the documented distinction: Rust `Property<T>` owns its own guard
   and an independently excluded paragraph does not exclude that property;
   Python field references check their source declaration's condition. Do not
   promise that these are identical object models. Assess concurrent root passes
   sharing explicit handles and nested passes with regression tests before
   changing their ownership semantics. Opaque futures cannot be introspected.
4. Complete structured diagnostics incrementally. `Rule`/`Diagnostic` exist, but
   `Error::Validation(String)` and string locations remain, and native Python
   conversion generally returns standard exceptions containing display text.
   Expose stable machine-readable rule/location/cause information in Python
   while retaining meaningful exception categories. Use typed declaration/part/
   story paths where needed; never classify failures by message prefixes.
5. Move shared invariants into pure domain constructors/validators where they
   currently rely on codec guards. Introduce checked unit values and richer
   style/color/property values at useful boundaries without a gratuitous rewrite
   of every numeric convenience. Document application limits separately from
   schema constraints. Audit public snapshot accessors as new content arrives.
6. Expose profile/read-budget/timestamp options consistently where appropriate.
   Rust pure compilation accepts `RenderOptions`; Python currently uses native
   rendering defaults. Keep path-loading effects outside pure compilation and
   test snapshots after source image mutation/deletion.

**Done when:** the field inventory and intentional differences are documented,
both languages pass the scenario matrix, new failures expose structured context,
and no new feature needs its own dependency or numeric-conversion machinery.

### R2 — Rich inline containers and hyperlinks (first new feature)

**R2 complete (2026-11-18).** The engineer authorized committing the verified R2
checkpoint and continuing with R3. Commit subject: `feat: add rich external hyperlinks`.
Existing untracked `docs/DEPENDENCY-AUDIT.md` is untouched; no push is authorized.

Implementation against the five R2 acceptance areas:

1. `HyperlinkTarget` validates an external absolute URI/IRI's lexical envelope
   (scheme, forbidden whitespace/control/URI characters, percent escapes), keeping
   the original Unicode/query/fragment/escape spelling. This is not a full URL
   parser, navigation-safety check or scheme allowlist. `ResolvedHyperlink` has
   immutable target, display children and tooltip/history properties. Supported
   children are explicit text/image/tab/break runs. The official Transitional
   `CT_Hyperlink` uses `EG_PContent`, which permits recursive hyperlinks; this
   library deliberately rejects nesting as an authoring limitation, not an XSD
   restriction. Relative and bookmark-only targets remain unsupported for R3.
2. Rust `Hyperlink` defers target, children and `HyperlinkProperties` through the
   existing `Value`/`PropertyRef` machinery. Python `Hyperlink` resolves target,
   children, tooltip and history through its unchanged generic interpreter and
   converts snapshots directly into the shared Rust domain. Hidden links do not
   execute fields. Concurrent fields and generic references have matched tests.
   Rust resolution and Python native conversion reject nested containers before descending into them;
   resolved-content reuse cannot bypass the domain constructor's nesting check.
3. The inline codec emits `w:hyperlink` containing runs, with optional tooltip/
   history attributes and an external relationship against `PartContext.source`.
   Existing OPC code allocates/deduplicates owner-local IDs; no target is fetched.
   Relocated main/header owners, existing media relationships and repeated link
   targets are covered. No dummy relationship or internal-anchor implementation
   was introduced. Display formatting stays explicit on children.
4. Shared `inline_text` projection includes nested display text, not target,
   tooltip or image alt text. Links do not cache another derived text field;
   paragraphs retain their immutable private projection. `ResolvedInline::text`
   now returns `Cow<str>` and `format` returns `Option<&RunFormat>` because a
   container has no run format. Migration details/accessors are documented.
   Compiler inline traversal reuses comment/story handling; existing structural
   XML template traversal edits within links without crossing their boundaries
   or rewriting target relationships. No opaque import traversal was added.
5. `tests/hyperlinks.rs`, `tests/async_fields.rs`, and Python hyperlink/standards
   tests cover all story/table contexts, rich/image children, Unicode and target
   escaping, async/reference fields, skipped links, invalid targets/nesting,
   comments and template edits. Independent Rust-to-python-docx readback checks
   exact rich content and external relationships, and rejects a changed target
   after successfully loading the document. Official-schema tests accept legal
   content and reject a paragraph inserted under a hyperlink. Runnable examples
   are `examples/hyperlinks.{rs,py}`; support maps and field inventory are updated.

Verification on stable Rust/CPython 3.12 (Python commands used
`UV_PROJECT_ENVIRONMENT=.venv-opencode`):

- `uv sync --locked --group test` rebuilt the isolated native extension.
- `cargo test --locked --all-targets` passed before the final native-only guard;
  `cargo test --locked --all-features --all-targets` passed after it (106 tests,
  including 14 independent Python readbacks/negative-control test cases).
- `cargo test --locked --all-features --doc` passed (2 doctests).
- `cargo fmt --check`, `cargo clippy --locked --all-features --all-targets --
  -D warnings`, and rustdoc with `RUSTDOCFLAGS='-D warnings'` passed.
- Required-schema full Python suite passed: 150 passed, 1 intentional upstream
  skip, using `RV_DOCX_REQUIRE_SCHEMAS=1` and the existing official schema cache.
- The clean installed-wheel probe passed without interop dependencies and now
  asserts exact hyperlink display text and external target/mode. Both new
  examples ran; official schemas validated each (8 covered parts/relationships).
- `git diff --check` passed. The first feature tests failed against missing
  interpreter/codec support and the old Python extension, then passed after
  implementation/rebuild. The first full Python run found stale standards-test
  count expectations (3 rather than 4); those were updated without weakening
  required-schema failure checks. Additional review produced a failing Python
  regression for native descent into illegal nested links; the parent-context
  guard fixes it. A deferred-target diagnostic regression also passes.
- Hosted CI, MSRV Rust 1.88 and other Python versions remain unverified locally.

**Handoff after R2: R3 story identity/bookmarks/ranges.** Begin with its
story/part identity and paired-marker contracts; do not equate general ranges
with nested inline containers. R2's external-target representation is deliberately
not a fabricated internal-anchor model. R3 continuation is now authorized.

**Goal:** prove the new boundaries with a small feature requiring nested content
and external relationships. Sources: P1 §17.16.22 (`hyperlink`), paragraph/run
content models in §17.3, and P2 §§6.4–6.5.

1. Extend the domain/authoring inline algebra with a hyperlink container and a
   typed target. Support external URI targets first; internal bookmark anchors
   follow R3. Model display children separately from the target. Reject nested
   hyperlinks/illegal children according to their actual parent content model.
2. Resolve target, children and supported properties through the generic field
   machinery. Add Rust/Python declarations and direct native conversion. Keep
   text, images, tabs and breaks as explicit child variants where legal.
3. Add a WordprocessingML hyperlink codec. Register an external relationship
   against the actual story part; preserve URI escaping/fragments correctly,
   allocate owner-local IDs, and never fetch the target. A bookmark-only link
   must not create a dummy external relationship.
4. Introduce shared inline traversal/text-projection helpers where needed.
   `ResolvedParagraph` currently caches flat `text` alongside `runs`; define how
   nested display text contributes and avoid stale/duplicate derived text.
   Reuse traversal for comments, templates and diagnostics without descending
   into unsupported opaque XML.
5. Test body/header/footer/table contexts, relocated owner parts, Unicode and
   query/fragment targets, multiple formatted children, async target/children,
   skipped links and invalid nesting. Verify exact `w:hyperlink` structure and
   relationship type/mode/target independently; retain template boundary tests.

**Done when:** both languages can author equivalent rich external links, targets
resolve from their owning stories, schema/content checks pass and external links
are never fetched. Add runnable Rust/Python examples and the support-map entry.

### R3 — Story identity, bookmarks and general ranges

**R3 complete (2026-11-18; committed in `7ebcdb3` on 2026-11-19).** R2 was committed as
`7d54cfe` (`feat: add rich external hyperlinks`) as requested. R3 implementation
and verification are complete; R4 was not started at that checkpoint (completion
is recorded below). No push or publication was made.
The pre-existing untracked `docs/DEPENDENCY-AUDIT.md` remains untouched.

Implementation against the five R3 acceptance areas:

1. `ooxml::StoryRegistry` registers explicit logical IDs, kind, typed OPC owner
   and legal-content context. A body ID is reused across sections; table contexts
   retain it. Header/footer stories are deliberately reused by physical header/
   footer owner, while every comment definition registers its own logical text
   story even when definitions share a comments part. Tests demonstrate distinct
   logical IDs with identical owner/kind. No note authoring was smuggled into R3.
2. `RangeId` is a stable authoring key. `RangeStart`/`RangeEnd` are explicit paired
   markers; `RangeKind` distinguishes bookmark names from comment payloads.
   `story_inlines` traverses supported domain blocks/containers with structural
   locations. Per-story maps validate ordering, duplicate/unclosed/cross-story
   anchors and empty explicit comment spans, without treating ranges as a stack.
   Overlap, cross-paragraph/section and table-cell ranges work. Names and authoring
   start keys are document-wide. Standalone markers are not complete stories.
3. `BookmarkName` and internal `HyperlinkTarget::bookmark` (Python:
   `BookmarkTarget`) generate paired bookmark elements and anchor-only links.
   Forward/internal references resolve against generated and relationship-driven
   imported names, including relocated headers. Missing/ambiguous names and
   generated collisions fail with locations; no imported references are renamed.
   Bookmark and comment wire IDs have separate document-wide allocation spaces,
   reserving imported markers/definitions. High imported IDs can be preserved or
   linked without generating new IDs; generated IDs have a documented 32-bit cap.
4. Legacy run/paragraph comments and explicit body comment ranges now share
   `RangeKind` allocation and the `WireRange` paired-anchor codec. Existing author
   defaults, reference runs and body-only restrictions remain. Cross-paragraph
   comments and overlapping bookmark/comment spans have exact independent readback
   assertions. A changed endpoint is rejected semantically after successful load.
5. [RANGES.md](RANGES.md) defines stable identities, legal contexts, collision and
   edit policies. Template replacements retain marker positions and put inserted
   text in the first matched run, with no inferred offset/range rebasing. Tests
   preserve those positions and existing hyperlink/field/revision boundaries.

Normative review: consulted the official P1 (2016) PDF, kept outside the repo in
`/tmp/opencode`. §§17.13.6.1–2 specify matching IDs and document-order pairing;
§17.13.4.3 requires the comment reference in the same document story. Bookmark
name/ID scopes and the standard's first-bookmark duplicate-name behavior were
checked. Conservative duplicate/ambiguous-name rejection, paired-only comments,
body-only comments, no explicit markers within hyperlinks, Word-compatible ASCII
name rules and generated-ID caps are documented library policies, not invented
schema constraints. No broad claim of typed range import or navigation/layout
equivalence is made.

Verification on stable Rust/CPython 3.12 (Python commands used
`UV_PROJECT_ENVIRONMENT=.venv-opencode`):

- `uv sync --locked --group test` rebuilt the isolated native extension.
- Default `cargo test --locked --all-targets` passed; final all-feature
  `cargo test --locked --all-features --all-targets` passed (115 tests, including
  15 independent Python readback/negative-control test cases).
- `cargo test --locked --all-features --doc` passed (2 doctests).
- `cargo fmt --check`, Clippy all-features/all-targets with `-D warnings`, and
  private-item rustdoc with `RUSTDOCFLAGS='-D warnings'` passed.
- Required-schema Python suite: 165 passed, 1 intentional upstream skip, using
  `RV_DOCX_REQUIRE_SCHEMAS=1` and `/tmp/opencode/rv-docx-schemas`.
- Clean installed-wheel probe passed without interop dependencies and includes
  bookmark pairing/internal-target/no-dummy-relationship assertions. Both new
  `examples/ranges.{rs,py}` ran and passed official-schema validation (9 covered
  parts plus relationships each). Schema tests also cover headers and table cells.
- Compilation initially found moved mutable scanner references; reborrowing fixed
  them. Clippy requested collapsing scanner/target conditionals; corrected.
  Required-schema failure regressions were updated from 4 to 5 tests without
  weakening their assertions. Existing comment/link/template regressions pass.
  Final collision review reproduced a duplicate generated ID when an imported
  ID used the valid decimal spelling `-0`. Numeric reservation now handles signed,
  zero-padded and surrounding-whitespace spellings while retaining original XML;
  four regressions and the final full suites pass.
- `git diff --check` passed. Hosted CI, MSRV Rust 1.88 and other Python versions
  remain unverified locally. No new production dependency was added.

**Handoff after R3: R4 footnotes/endnotes/richer comments**, using the explicit
story registry and range contract rather than filename-derived identity. R3
was ready for a checkpoint commit. Do not start R4 or publish without
continuation authorization.

**Goal:** provide shared structural concepts for notes, fields, comments and
revisions. Sources: P1 §§17.10–17.13; bookmark definitions §§17.13.6.1–17.13.6.2.

1. Separate story identity from part identity fully. The compiler currently keys
   stories by `(part path, StoryKind)`, adequate for current body/header/footer
   work. Multiple notes in one XML part require separate logical stories.
   Give each story explicit kind, owner part, legal-content context and identity.
2. Design typed positions/anchors and paired range markers. Ranges can overlap;
   do not represent all ranges as nested containers. Specify whether references
   use stable content identities or validated structural paths and how edits
   transform them. Define rejection behavior for dangling/cross-story anchors.
3. Add bookmark generation, unique names/IDs, paired anchors and internal
   hyperlink resolution. Handle imported collisions without changing unrelated
   existing references. Verify allocation scopes against the specification.
4. Migrate current body-comment convenience syntax onto the common range model
   while preserving existing output semantics. Add explicit cross-paragraph
   ranges only with validation and tests. Keep currently unsupported comment
   contexts rejected until their rules and codecs are implemented.
5. Document template interactions. Today comment/bookmark markers stay at their
   original positions and replacement text belongs to the first matching run.
   Any richer edit behavior needs an explicit policy and preservation fixtures.

**Done when:** multiple logical stories can share a part, overlapping legal
ranges round-trip semantically through generated files, invalid anchors fail
with locations, and existing comment/link/template tests remain meaningful.

### R4 — Footnotes, endnotes and richer comments

**R4 complete (2026-11-18; committed in `7ebcdb3` on 2026-11-19).** Continuation was
authorized; at implementation completion R3/R4 were uncommitted after R2 `7d54cfe`. The audit
remains untouched. No push/publication or dependency addition was made; R5 was not
started at that checkpoint (completion below). The detailed contract is [NOTES.md](NOTES.md).

Implementation against the four R4 acceptance areas:

1. Both frontends author ordinary footnotes/endnotes with separate kind/ID
   namespaces. `NoteId` is a nonnegative 32-bit authoring value; zero is valid for
   an ordinary note. Note targets/blocks, reference targets and structured-comment
   fields use the existing generic deferred machinery. Pure domain validation
   catches duplicate definitions and illegal/cross-note stories/ranges; compilation
   resolves references against generated/imported definitions. Missing, duplicate,
   collided and non-normal targets fail with locations. Roles are determined by
   `w:type`, not assumed reserved numeric IDs. Special entries cannot be authored
   as ordinary definitions or referenced by ordinary markers.
2. Note roots/content types and existing owners are discovered by exact OPC
   relationships. Each ordinary note registers a distinct logical story in its
   physical part. Images/links use that owner, including relocated footnote and
   endnote parts; media deduplication remains shared. New parts avoid filename
   collisions. Missing separator/continuation-separator entries use unused negative
   IDs (normally -1/-2), with corresponding special-entry settings references.
   Existing types/content/settings are retained and checked, not silently
   remapped. Imported decimal keys use canonical strings, avoiding machine-integer
   narrowing even for a 5,001-digit valid source ID. Notes get their reference mark
   at the start of their first paragraph (or a new leading marker paragraph).
3. `Comment`/`ResolvedCommentBody` and `RangeKind::StructuredComment` provide rich
   paragraphs/tables/images/links through existing blocks and the common anchor
   codec. `CommentMetadata` exposes author/date/initials deliberately. Author
   defaults and legacy text conveniences remain; supplied dates use a checked
   UTC-seconds calendar subset, while absent dates use snapshot time. Structured
   comment resources belong to the actual comments owner; unrelated orphan
   filename collisions are not overwritten. Body-only anchors/references remain
   an explicit supported-authoring policy; nested notes/comments, threaded
   extensions, glossary-note authoring and layout/number evaluation are excluded.
4. Supported-story template edits visit imported note/comment content without
   changing newly appended declarations. Tests cover rich/nested-table blocks,
   source conditions/references/concurrency, captured image data, imported ID
   collisions, special types, relocated owners, exact output and failed-save
   preservation. Independent python-docx OPC/XML readback checks normal/special
   IDs, self marks, formatted note/comment content, media bytes and owner-local
   links. Mutations to note text or a body reference fail after successful load.
   The independent official-schema tool now checks note graphs too: missing,
   duplicate, special and nested targets fail; custom relationship suffixes do
   not masquerade as standard note relations.

Preservation repair discovered by R4: the new untouched-note test failed because
OPC's parse cache treated all reads as edits. `CachedXml` now tracks dirtiness:
only explicit `put` operations reserialize a cached part. Both a focused OPC unit
test and the original byte-for-byte note regression pass, along with the complete
namespace/template/preservation suite. The contract/test was not weakened.

Normative review used the official P1 (2016) PDF outside the repository:
§§11.3.4/11.3.7 define note parts/owners; §17.11.10 explicitly illustrates ordinary
ID zero and distinguishes non-normal entries by type; §§17.11.7/17.11.14 prohibit
references nested in note stories and require matching definitions. Annex A.1
defines the supported note/comment block models. Application ID/date/context
restrictions versus schema rules are documented in NOTES.md. No visual/note-layout
or full imported-document conformance claim is made.

Verification (stable Rust, CPython 3.12; Python commands used
`UV_PROJECT_ENVIRONMENT=.venv-opencode`):

- Locked isolated extension rebuild: `uv sync --locked --group test`.
- `cargo test --locked --all-targets` and final all-feature/all-target suite passed
  (122 all-feature tests, including 16 independent Python readback/negative-control
  test cases). `cargo test --locked --all-features --doc`: 2 passed.
- Formatting, all-feature/all-target Clippy with `-D warnings`, and private-item
  rustdoc with `RUSTDOCFLAGS='-D warnings'` passed.
- Required-schema full Python suite: 180 passed, 1 intentional upstream skip,
  with `RV_DOCX_REQUIRE_SCHEMAS=1` and the existing official schema cache.
- Clean installed-wheel probe passed without interop dependencies and includes
  ordinary notes/structured comments. `examples/notes.{rs,py}` ran; official
  schema + OPC/note-graph validation passed for each (11 covered parts).
- Initial feature tests failed against missing APIs, then passed. Compilation
  found one borrowed-string call fix; Clippy found collapsible conditions and two
  test initializer issues, corrected. Python readback setup used the non-indexable
  comments collection incorrectly; corrected to iteration. The full Python run
  then exposed the real parse-cache preservation failure, repaired as above.
  Missing-schema guard expectations now cover six standards tests, without
  weakening required-failure behavior.
- `git diff --check` passed. Hosted CI, Rust 1.88 and other Python versions remain
  unverified locally. No source dependency or upstream fixture hash was changed.

**Handoff after R4: R5 fields and dynamic references.** Reuse explicit stories,
range traversal and typed target contracts; cached field results are not computed
layout. R3/R4 were ready for checkpoint commits. Do not start R5,
push or publish without continuation authorization.

**Depends on:** R3. Sources: P1 §17.11, §§11.3.4 and 11.3.7; comments §17.13.4
and §11.3.2. Microsoft threaded-comment features require separately pinned
MS-DOCX extension revisions.

1. Add footnote/endnote definitions, references and story bodies with IDs scoped
   correctly. Handle reserved separator/continuation entries distinctly from
   ordinary note IDs. Validate missing/duplicate definitions and legal contexts.
2. Generate/discover the related parts and content types through OPC. Keep each
   note's media/link ownership tied to its physical part and its own logical
   story. Do not hardcode `word/footnotes.xml` as imported identity.
3. Expand comments to structured content/ranges as supported by the base schema.
   Add author/date metadata deliberately; keep modern threaded comments a
   separate extension feature rather than silently emitting partial metadata.
4. Extend supported template story traversal and independent validator coverage.
   Verify note references, reserved entries, media and nested table content with
   exact XML checks and an independent consumer that understands the feature.

**Done when:** Rust and Python author equivalent notes, references and IDs are
validated across imported/generated content, and unresolved/illegal stories fail
clearly. Merely retaining imported note XML does not satisfy this milestone.

### R5 — Fields and dynamic document references

**R5 complete (2026-11-18; committed in `7ebcdb3` on 2026-11-19).** Continuation authorized
this slice. At implementation completion R3–R5 were uncommitted after R2 `7d54cfe`. The dependency audit
is unchanged. No push, publication, production dependency or lockfile change was
made. Detailed contracts and examples are in [FIELDS.md](FIELDS.md).

Implementation against R5 acceptance:

1. `Field` resolves instruction, cached rich inlines and `FieldProperties`
   concurrently through ordinary `Value<T>` machinery; Python resolves
   `Field`/`FieldInstruction` fields through its unchanged generic interpreter.
   Conditions skip producers. `ResolvedField` has read-only instruction/result/
   properties, and text projections include cached display text, never code.
   `FieldForm` chooses `fldSimple` (default) or balanced begin/instruction/separate/
   result/end runs. Dirty/locked attributes preserve absent/false/true independently
   of cached-run formatting. Complex results may nest fields; no raw delimiter
   components are exposed, so dangling/cross-story markers cannot be authored.
2. Typed PAGE/NUMPAGES/REF/TOC helpers and a checked raw-instruction escape hatch
   distinguish instruction programs from cached display. REF reuses `BookmarkName`
   and the existing imported/generated bookmark catalog, including missing/
   ambiguous targets in nested results. Raw code preserves XML whitespace and
   special characters without parsing syntax or embedded targets. The serializer
   does not calculate pages, evaluate fields or fetch referenced documents.
3. Field cached results reuse rich-inline compilation and flattened story
   traversal. Media/links belong to the enclosing physical owner; schema tests
   exercise headers, notes and structured comments with formatted text, tabs,
   breaks, images and links in both encodings. Snapshots render after image-source
   deletion. Explicit cached-result range markers and fields inside hyperlinks
   are rejected library policies, not invented XSD restrictions. Authored fields
   remain paragraph-local, with nesting only in results, not in instruction code.
   Multi-block field/TOC results and instruction/marker run formatting are not
   exposed. TOC is only a standard instruction plus caller-supplied inline cache
   and optional consumer-update flag, not a generated contents/layout engine.
4. Imported field instructions/caches remain protected from template replacement.
   A new regression first demonstrated an existing hole: field depth reset at
   each paragraph allowed editing cached results in an imported multi-paragraph
   field. Depth now persists within each logical story and resets per note/comment
   entry, including multiple notes sharing one owner. A second regression caught
   overly inherited protection on text after an end marker in the same paragraph;
   token-time depth now permits that unrelated suffix edit. Both regressions pass,
   along with the full template/range/namespace/preservation suites. Unclosed
   imported fields conservatively protect the remaining story; no complete typed
   import or consumer-conformance claim is made.

Normative review used official P1 (2016) §§17.16.18–19/17.16.23 and Annex A.1.
Complex fields require begin/end; the codec always emits a separator and cached
result (possibly empty). The format permits broader nesting/cross-paragraph
constructs than this public authoring subset. Dirty requests consumer updates;
lock inhibits recalculation. Runtime compilation never performs those actions.

Verification on stable Rust / CPython 3.12:

- New Rust acceptance tests initially failed to compile against missing APIs;
  after implementation the focused field/concurrency tests passed. New Python
  tests first failed against the old native adapter, then passed after the locked
  extension rebuild (`UV_PROJECT_ENVIRONMENT=.venv-opencode uv sync --locked --group test`).
- `cargo test --locked --all-targets` and
  `UV_PROJECT_ENVIRONMENT=.venv-opencode TMPDIR=/tmp/opencode cargo test --locked --all-features --all-targets`
  passed: 126 all-feature Rust tests, including 17 independent readback cases.
  Readback checks both forms, exact instructions, formatted cached results and
  marker order through python-docx loaded XML/Run APIs; altered cache/code/markers
  fail after successful document loading. No complete python-docx field API or
  visual-layout evidence is claimed.
- Required-schema full Python suite: **198 passed, 1 intentional upstream skip**,
  using `UV_PROJECT_ENVIRONMENT=.venv-opencode RV_DOCX_SCHEMA_DIR=/tmp/opencode/rv-docx-schemas RV_DOCX_REQUIRE_SCHEMAS=1 uv run --no-sync pytest`.
  The validator supplements XSDs with independent per-story complex-field balance
  checks, including simple-field boundaries and individual note/comment entries.
  Removed end markers/duplicate separators fail; seven standards tests retain the
  missing-schema required-failure guard. The strict checker rejects malformed
  marker streams that consumers could otherwise ignore; it is not a claim that
  the base XSD pairs markers.
- `cargo fmt --check`, `git diff --check`, all-feature/all-target Clippy with
  `-D warnings`, 2 doctests and private-item rustdoc with `RUSTDOCFLAGS='-D warnings'`
  passed. The pure domain still builds without XML/OPC/external dependencies.
- Clean installed-wheel verification passed without interop dependencies and
  covers simple/complex fields and caches. Both `examples/fields.{rs,py}` ran;
  official schema + OPC/field-graph validation passed (9 covered parts each),
  producing `/tmp/opencode/rv-docx-r5-{rust,python}.docx`.
- Hosted CI, Rust 1.88 and other Python versions remain unverified locally.

**Next concrete slice: R6 formatting/styles/lists/tables.** Select one independent
vertical slice with a property-level inventory rather than attempting all R6 at
once. R3–R5 are checkpointed in `7ebcdb3`; the OPC repair is `9adaa5f`. Do not push/publish
or start the next slice without continuation authorization.

**Depends on:** R2/R3. Sources: P1 §17.16, including §17.16.18 (`fldChar`).

1. Model simple fields and complex begin/instruction/separate/result/end fields
   explicitly, with typed helpers for common PAGE/NUMPAGES/REF-style uses and an
   intentional escape hatch for instructions. Keep cached display content
   distinct from instructions and updates requested from a consuming app.
2. Validate nesting/balance, story boundaries and target references. Preserve
   instruction whitespace and field-associated properties. Reuse range and
   rich-inline traversal instead of inventing another string replacement engine.
3. Add field codecs and declarations in both languages, including async field
   properties. Keep template edits inside field structures rejected until a
   field-aware operation is specifically supported.
4. Add tables of contents/cross-reference helpers only as instruction + cached
   result/update contracts. Do not claim the crate calculates pages or fields.

**Done when:** both field forms serialize legally, cached content survives an
independent reader, invalid structures are rejected and evaluation limits are
clear in API docs and examples.

### R6 — Expand existing formatting, styles, lists and tables

**Dependencies:** R1; advanced inline/story cases additionally use R2–R4.
Sources: P1 §§17.3–17.9, §17.15 and §22.9.

Deliver independent slices, each with a property-level coverage inventory:

- Formatting: absence/true/false and toggle inheritance, script/language/font
  selection, bidi and complex-script properties, tabs, exact/at-least line
  spacing, keep/widow/page-break rules, text decorations and theme references.
- Styles: document defaults, character styles, based-on/next/linked references,
  cycle validation and richer table conditional formatting. Keep direct
  formatting separate from computed/effective style inspection.
- Lists: abstract/concrete numbering identities, multilevel definitions, restarts,
  overrides, style links and imported-ID collisions. Current numbering structs
  are a starting subset, not a complete numbering system.
- Tables: grid/row/cell content-model checks, legal horizontal/vertical merges,
  row heights/repetition/splitting, cell margins/direction, positioning and
  conditional styles. Validate trailing paragraph requirements in cells.
- Sections/settings: additional page borders/geometry, header/footer linking,
  line numbering, note settings, compatibility options and protection settings
  as independently scoped features.

**Done per slice:** one domain meaning and shared codec, both authoring APIs,
checked units/references, positive and negative schema/readback tests, and
updated support/migration documentation. Do not map unrelated properties into
one generic unvalidated attribute dictionary.

### R7 — Revisions, content controls and structured editing

**Depends on:** R3/R5. Sources: P1 §17.13.5 (revisions), §17.5 (custom markup
and content controls), and §17.14 (mail merge).

1. Add typed insert/delete/move revisions with author/date metadata and legal
   run/block/property contexts; implement any accept/reject transformations as
   pure domain operations with explicit supported cases.
2. Model inline/block/row/cell content controls by context, not as one universal
   wrapper. Add tags/aliases/locks and selected control types, then custom XML
   binding and relationship support as separate slices.
3. Add structured template operations on understood nodes/ranges. Define match
   scopes, multiple-match behavior, boundary rules and anchor adjustment. Preserve
   opaque extensions and fail explicitly for unsupported destructive rewrites.
4. Add mail-merge document structures after fields and data-binding contracts
   exist. External data retrieval remains an adapter effect, never an implicit
   compiler action.

**Done per slice:** schema-legal contextual models, explicit edit/preservation
guarantees and imported-document fixtures proving unrelated content survives.

### R8 — Broader DrawingML, charts and equations

**Depends on:** stable resources/story contexts and R2 rich-inline handling.
Sources: P1 §§20.1–20.4, §§21.2–21.4 and §22.1.

1. Floating pictures: model positioning reference frames, wrapping, z-order,
   cropping/rotation and extents. Separate inline and anchored drawings; reuse
   media registration and checked physical units.
2. Shapes/text boxes: introduce legal drawing/text stories and geometry values;
   do not reuse body paragraph serialization without validating its context.
3. Charts: begin with one documented chart type, then add series/axes/styles and
   cached values. Specify embedded workbook versus external data behavior and
   register the dependent part graph. Keep spreadsheet generation scoped and
   justified rather than pulling in a large runtime SDK.
4. Math: introduce an OMML algebra/codec in small slices with exact structure
   assertions. Diagrams/SmartArt require separate data/layout/style/color parts
   and should follow only after dependent resource support is proven.

**Done per feature:** expected XML, parts, IDs, references and units are validated;
independent consumers and, where needed, documented manual Office rendering
checks support the claims. No visual-equivalence claim follows from XSD validity.

### R9 — Import/preservation, package features and extensions

These are separate capabilities, not automatic consequences of writing features.
Sources: P1 §§17.8, 17.12, 22.2–22.3; P2 §§8–10; P3 §§7–9; P4 legacy vocabularies.

- Introduce public typed imported-document views only when a feature needs them.
  Retain opaque nodes/parts next to understood content; distinguish typed import
  from the existing template loader. Verify QName-valued attributes and namespace
  shadowing on every newly edited vocabulary.
- Extend MCE support deliberately, including AlternateContent, MustUnderstand
  and extension capability selection. The development validator's preprocessing
  is not proof of a complete runtime MCE implementation.
- Add extended/custom properties, font metadata/embedding, glossary/building
  blocks and embedded objects in independently tested slices. Preserve existing
  license/provenance notices and document font/embedded-resource contracts.
- Define handling of digitally signed packages before claiming safe edits:
  rewriting signed content invalidates signatures. Choose explicit rejection,
  explicit signature removal or separately implemented re-signing behavior.
  Do not silently advertise retained signature parts as valid signatures.
- Assess legacy VML, macro-enabled types and external relationships independently;
  do not broaden accepted input types without a preservation/profile policy.
- Add targeted fuzz/property tests for XML namespaces, OPC URI resolution,
  archive limits, relationship graphs and text/range transformations. Performance
  work should follow measured representative documents, not speculative caches.

**Done per slice:** documented import/edit/retention capabilities, supported
profile and failure behavior, byte preservation of untouched payloads, semantic
preservation of edited unrelated markup and bounded resource use.

### R10 — Strict profile and release/conformance expansion

Strict is not a namespace switch. Inventory every emitted namespace, relationship
type, lexical value, content type and Transitional-only construct. Introduce
profile-owned mappings, Strict-specific fixtures and independent Strict schemas
before enabling output. Unsupported template/profile combinations must fail
explicitly. Keep Transitional fixtures and tests active.

For broader releases, define a finite documented support profile, platform/wheel
matrix and semver policy. Automate build/test artifacts first; publication and
release credentials are a separate operation. Maintain LGPL-2.1-or-later project
licensing and the distinct MIT notice for bundled python-docx assets. Do not claim
full ECMA conformance, complete Microsoft-extension support or 1.0 readiness
without a coverage audit tied to operations and independent evidence.

## Common completion checklist

For every feature slice:

1. Write the domain contract, supported contexts, invariants and exact output
   profile. Read the normative parent/content model, not only the element name.
2. Implement the smallest end-to-end path through domain, authoring, both
   interpreters, native Python conversion and a shared codec. Reuse OPC/media/
   style/story registries and checked conversions. Avoid temporary duplicate
   document models or frontend-specific serializers.
3. Test expected values through Rust-generated DOCX files and Python readback;
   use XML/schema checks where python-docx lacks an API. Add a negative control
   for the actual failure mode: wrong order, missing target, illegal context,
   bad ID or changed content. A file merely opening is insufficient.
4. Add template preservation cases for relevant imported parts, including
   nonstandard paths and namespace collisions. Keep unsupported contexts explicit.
5. Exercise async/reference/condition semantics for new fields in both languages.
   Verify snapshot rendering does not run producers or read assets again.
6. Add a runnable example in Rust and Python, update public exports and migration
   notes when necessary, and update support status in `COMPATIBILITY.md` and
   `docs/OOXML.md`. Include codec-local source citations.
7. Run relevant checks below, inspect the diff, commit a green checkpoint and
   push the existing branch when continuing the authorized implementation work.
   Do not publish a package or create another repository as an incidental step.
8. Update this file with the completed milestone, commit, evidence and next
   concrete slice, so the next agent does not depend on a conversation transcript.

## Verification and environment runbook

Prerequisites: Rust (manifest MSRV 1.88, edition 2024), uv, and Python >=3.12
with development/link libraries when running embedding tests. On a new machine,
network access may be needed once for Cargo/uv packages and official schemas.
Keep existing lockfiles; do not replace the pinned CMI source casually.

From the repository root:

```sh
uv sync --locked --group test
cargo fmt --check
cargo test --all-targets
cargo test --all-features --all-targets
cargo test --all-features --doc
cargo clippy --all-features --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --all-features --no-deps --document-private-items
uv run --locked --group test python tools/validate_ooxml.py --download-schemas
RV_DOCX_SCHEMA_DIR=/tmp/rv-docx-validation-schemas uv run --locked --group test pytest
uv build --wheel
git diff --check
```

The schema downloader verifies official archive digests; do not update digests to
bypass a mismatch. Point `RV_DOCX_SCHEMA_DIR` at the directory containing
`wml.xsd` and the downloaded companion schemas. See [tests/README.md](../tests/README.md)
for details. The optional `python-tests` Cargo feature runs substantive readback;
`python-bindings` builds PyO3. Maturin separately enables extension-module linkage
for wheels. Do not add that linkage mode to ordinary all-feature embedding tests.

In the previous managed workspace, sysconfig reported an incorrect library
directory. The actual libpython was at
`/opt/codex/runtimes/codex-primary-runtime/dependencies/python/lib`; setting
`LIBRARY_PATH` and `LD_LIBRARY_PATH` to that directory allowed Cargo embedding
tests to link/run. Discover the correct path on a new machine instead of making
this absolute path a repository requirement. Optional offline cache settings in
that workspace were `UV_CACHE_DIR=/tmp/rv-docx-uv-cache`, `UV_OFFLINE=1` and
`UV_LINK_MODE=copy`, plus `cargo --offline`; only use offline mode after caching.

Clean wheel check: use `uv venv <temporary-env>` and `uv pip install --python
<temporary-env>/bin/python <absolute-wheel-path>`, without reference/test extras.
Run from outside the source tree, verify the imported module lives in that
environment, inspect ZIP/XML using the standard library and check optional
interop failure. Run interop/readback separately with the extra installed.
`uv run --no-sync python tools/check_wheel.py` now automates this recipe.

Useful focused entrypoints: `tests/domain.rs` (std-only domain boundary),
`tests/{deferred,async_fields,resolution,compilation}.rs`,
`tests/{preservation,templates}.rs`, `tests/python_docx.rs` with
`tests/python_docx/verify.py`, and `tests/python_bindings/test_standards.py`.
Keep shared test builders in `tests/support`, independent from examples.

## Specification and documentation policy

Use the official sources linked in [OOXML.md](OOXML.md): **P1 = Part 1 (2016)**,
**P2 = Part 2 (2021)**, **P3 = Part 3 (2015)**, **P4 = Part 4 (2016)**. Clause
numbers above refer to those editions, not PDF pages. Verify any more-specific
clause before adding it to a codec. Microsoft's MS-DOCX extensions and
MS-OI29500/MS-OE376 implementation notes have different authority; pin the
revision when implementing an extension. Do not commit the standards PDFs.

For every support claim record: domain behavior, owning parts/relationships,
elements/attributes, edition/clause, output profile, generation/import/edit/
preservation status, unsupported cases, fixtures/provenance and independent
validation. Schema validity, consumer readability, preserved opaque payloads and
visual rendering are different evidence and must not be substituted for each other.

## First assignment for the next agent

**Committed checkpoints (2026-11-19):** the engineer requested commits for the
completed work. `9adaa5f` isolates the OPC read-cache preservation fix;
`7ebcdb3` checkpoints the combined R3–R5 implementation/tests/examples (the
feature changes share domain, traversal and adapter edits). The accompanying
documentation checkpoint records contracts and this handoff. The unrelated
untracked `docs/DEPENDENCY-AUDIT.md` remains excluded. Pre-commit verification
reran the all-feature/all-target Rust suite (126 passed), required-schema Python
suite (198 passed, 1 intentional skip), formatting and diff checks successfully.
No push, publication or R6 implementation was authorized by this commit request.

R2 is committed as `7d54cfe`; R3–R5 are committed as `7ebcdb3`. The next
implementation is **one independent R6 formatting/styles/lists/tables slice**,
after continuation authorization. Use [RANGES.md](RANGES.md), [NOTES.md](NOTES.md),
[FIELDS.md](FIELDS.md), the current
deferred-field inventory and migration guide rather than baseline API shapes.
Confirm hosted CI/version-matrix results when available; no package publication
was authorized. Do not reopen settled CMI compatibility or repository naming
decisions. Preserve the existing untracked dependency-audit document.
