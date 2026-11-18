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
text/tab/break/image inlines, selected run/paragraph formatting, RGB/theme/auto
run colors, underline styles, styles, numbering, nested tables and merges,
page geometry/columns, default/first/even headers and footers, body comments,
inline images, core metadata, and structural template text replacement.
Comments are not yet general arbitrary ranges. Pictures inspect file headers;
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
| `src/document/{content,table,resources,values,color,measure,identity,diagnostic}.rs` | Resolved content, domain values and invariants; standard library only |
| `src/authoring/{mod,components,table}.rs` | Constructors, unresolved values and conditions; no XML, filesystem or interpreter code |
| `src/resolution/{mod,property,interpreter,components}.rs` | Value/shared computation/reference primitives, dependency graphs and native interpretation |
| `src/ooxml/wordprocessing/` | Text, paragraphs, formatting, tables, sections, stories, styles and comments |
| `src/ooxml/drawing/`, `resources.rs`, `style_catalog.rs`, `schema.rs` | Pictures, media/style registries and Word ordering rules |
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

**Implementation status (2026-11-17, working tree; not committed):** CI and the
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
**Next concrete slice:** confirm the new CI matrix, then R1's public deferred-field
inventory and shared interpreter scenario matrix. Do not start R2 before R1.

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

Confirm **R0**'s new CI matrix and checkpoint the verified changes before changing
the public content algebra. Proceed to **R1**'s deferred-field inventory/scenario
matrix and then R2 external hyperlinks as the first new feature. Do not reopen
settled CMI compatibility or repository naming decisions.
