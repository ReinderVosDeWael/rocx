# Active plan — editing existing DOCX files

Blocks topic (#3): direct paragraph replacement/removal, paragraph-anchored
section replacement with ordered resolved paragraphs/tables, and keep-with-next
patches share the original-snapshot transaction. The section's original break
and header/footer inheritance are preserved. Replacement of source sections
containing tables or opaque content remains unsupported.

Topic verification: 43 Rust editor tests plus the showcase, 32 installed
Python editor tests, native/Wasm Clippy and strict installed TypeScript passed.
Packed Node 24 and Chromium 159 matched 13 native editor outputs. The mixed
block output matched Python and passed independent python-docx, unchanged-part,
OPC and 11-part schema checks. All nine catalogs matched across the bindings.

Documentation pass: publish user guides and API references only. Keep build, architecture, release and verification procedures such as the [viewer observation record](VIEWER-COMPATIBILITY-RECORD.md) in the repository.

## Current milestone: implement the remaining editor capabilities

Ten parallel worktrees start from `fad29a9`. Track their acceptance separately
under #3: text ranges (#42), paragraph and section edits (#43), inline edits
(#44), row and cell edits (#45), story edits (#46), styles (#47), numbering,
defaults and metadata (#48), revisions (#49), controls (#50), and CLI coverage
of the seven previously integrated edit kinds (#51). A partial implementation
leaves its ticket open. Before closing the umbrella, reconcile these results
with the [operation inventory](EDITOR-PARITY.md#operation-contract) and
verify combined batches, preservation and exact cross-binding output again.

## Previous milestone: qualify the combined #3 editor

Style editing slice (#3): paragraph/character/table additions, direct bold and
italic patches, and guarded replacement/removal of simple unused custom styles
are implemented on the topic branch. The four-operation independent fixture
matched complete Rust, installed Python 3.12 and packed Node 24 output, passed
schema/OPC validation, and preserved every other part payload. Focused Rust
and Python tests cover rejection and transaction reuse. Numbering-style editing,
broader property patching and umbrella qualification remain open.

Nine slices based on `aefcec8` are integrated in [#41](https://github.com/ReinderVosDeWael/rocx/pull/41).
Use the [editor inventory](EDITOR-PARITY.md) for the supported targets and the
[CLI contract](CLI.md) for its digest-bound mixed-edit workflow.

| Slice | Scope in this candidate |
| --- | --- |
| Addressing | Snapshot lifetime, physical aliases, duplicate targets and range guards. |
| Paragraphs and inlines | Insert a resolved paragraph after a direct body paragraph; replace text in, or conservatively remove, a supported direct body run. |
| Tables | Patch vertical alignment using a direct body table anchor and physical row/cell indices. |
| Stories | Append an inline image to a default or first-page header with explicit shared or section-local intent. |
| Resources | Inspect relationship-selected style identities, kinds, names and base references. |
| Controls and revisions | Inspect direct body control metadata and inline insertion/deletion wrappers; neither is editable. |
| CLI | Inspect paragraph, run, physical cell and header-slot addresses; preview all seven core edit kinds together and save to a new path. |

**Combined verification passed locally on `6b34ff0`.** All Rust targets, three
doctests, native/Wasm Clippy, the Rust 1.88 check, 822 schema-required Python
tests, the isolated wheel check and the strict 552-page docs build passed.
Packed Node 22 and 24, with Chromium development and production builds, passed
offline package replay, schema/OPC checks and exact byte comparisons for all
nine catalog outputs. A mixed paragraph insertion, cell patch and run removal
produced identical DOCX bytes in Rust, Python, Node and both browser builds.
Readback verified target placement and unchanged part payloads; a conflicting
batch was rejected and the original anchors remained usable. Earlier branch
results below describe their own checkpoints. #3 and release preflight remain open.

Standalone evidence is available at the topic tips: addressing `b03a230`,
CLI `c8bbaff`, controls `bdd2da5`, inlines `ce92fed`, paragraphs `21954ab`,
resources `44d0f23`, revisions `ea3f9d9`, stories `4e3eb4b`, tables
`d25b1bb`. For example, controls and resources recorded 813 and 814 Python
tests respectively; revisions recorded 756 passes and a separate required-schema
run for the other 57. Their packed-consumer runs exercised different artifacts.

The [engine audit](ENGINE-AUDIT.md) records four correctness and preservation
defects (#10–#13). Each failed its regression before the fix and passed
afterward in Rust and Python. The integrated Rust suite, three doctests, Clippy,
767 schema-required Python tests, and exact Rust/Python DOCX byte parity passed.
The fixes are on the default branch and the four issues are closed.
Use the audit's format-preserving report edit as a migration acceptance case
for tasks 4–6.

The separate Verification formatting fix and opt-in prek hook reached `main`
in PR #14. GitHub Actions checks formatting independently of local hook setup.

The Rust, Python, Wasm probe, and GitHub Actions dependencies were refreshed.
The Rust 1.88 check, Rust suite, three doctests, Clippy, Wasm target check,
767 schema-required Python tests, strict docs build, and exact DOCX byte parity
passed locally. See the [dependency audit](DEPENDENCY-AUDIT.md) for alert status.

## Current GitHub issue sweep

### Standalone addressing branch (#3)

The addressing inventory now states snapshot lifetime, physical story aliases,
duplicate-edit rejection and range limitations. Focused Rust and Python imported
editor tests passed locally. The installed JavaScript test adds corresponding
identity, alias and reopen checks. Full packed qualification passed on Node
22.23.5 and 24.22.3 and Chromium 159 development and production, including
offline provenance replay, exact DOCX parity, schema and OPC checks, and
independent readback and untouched-part checks.

### Editor checkpoint (#3, PR #31)

The hosted Rust showcase test left the seventh catalog output (`run-text.docx`)
in its temporary directory. It now reopens that output and removes it alongside
the header outputs. The packed JavaScript checks now build the native editor
oracle as a Cargo example instead of guessing where rustc can find its
dependencies. Locally the showcase test passed, python-docx reopened all seven
native oracle outputs, and the no-op preserved every ZIP part payload. Require
the hosted Rust and packed Node/Chromium checks before integrating this slice.

### Repository evaluation follow-ups

Three independent probes on `19590c9` reproduced defects despite 261 passing
Rust all-targets tests and 28 passing focused Python tests:

- [#25](https://github.com/ReinderVosDeWael/rocx/issues/25): ordinary template
  replacement can match across a visible nonbreaking hyphen.
- [#26](https://github.com/ReinderVosDeWael/rocx/issues/26): an automatic nested
  table inside a fixed-width inserted table can receive a zero-width grid.
- [#27](https://github.com/ReinderVosDeWael/rocx/issues/27): saving to a Python
  stream that accepts a partial write can report success with an incomplete DOCX.

The architecture follow-up tracked preview memory in
[#28](https://github.com/ReinderVosDeWael/rocx/issues/28): retaining three
alignment previews of a document with a 12 MiB image added about 39 MiB RSS
before encoding. For task 5, consider one internal transaction for body and
header edits; the [mixed-batch acceptance case](https://github.com/ReinderVosDeWael/rocx/issues/3#issuecomment-7109658618)
is recorded under #3.

Six isolated development worktrees established these baselines before changes:

| Slice | Baseline evidence | Implemented behavior |
| --- | --- | --- |
| #25 | Replacing `ABCD` changed `AB` + nonbreaking hyphen + `CD` to `X-`. | A shared run-child classifier prevents matching across opaque content. The all-features fixture identified a zero-width comment reference that must stay transparent to matching; its readback and markup are preserved. |
| #26 | Two nested grid widths were `0, 0` instead of `900, 900` in an inserted table; authored spanning cells also used the wrong width. | The shared codec passes the spanned grid width to child blocks. Fragment rendering no longer constructs a synthetic whole document. |
| #27 | `save()` returned after a stream accepted just 97 bytes. | Short writes continue until complete; stalled or invalid counts raise an I/O error. |
| #28 | Separate previews allocated distinct copies of the same unchanged media. | Shared immutable part bytes reduced median peak RSS for 12 retained previews from 268,016 to 54,200 KiB in the developer's probe. |
| #3 | Separate body and header previews succeeded; the combined call failed. | One transaction supports body edits and shared or local header images, checks aliases and conflicts, and rejects the whole batch on failure. |
| #21 | A temporary declaration-only field passed the old packed tests but the installed Wasm rejected it. | The packed consumer requires every declared field and variant in a typed fixture and exercises runtime validation. |

Integrated verification on `9cc432b`: 304 Rust all-features/all-targets tests,
three doctests, 806 schema-required Python tests, formatting, native/Wasm Clippy,
Rust 1.88, and the strict 524-page docs build passed. All six Rust/Python
catalog pairs matched byte for byte. Node 22 and 24 packed consumers passed
the declaration checks, both Chromium modes, and five-way exact DOCX byte
parity with schema and OPC validation. Issues #3 and #21 retain broader work.

### JavaScript parity review follow-up

The adapter now retries after Wasm initialization failure (#24), rejects
malformed UTF-16 rather than replacing it (#20), and accepts valid
`Uint8Array` views from other JavaScript realms (#23). The adapter reports
JavaScript declaration paths; core failures expose stable rule codes when
available (#22). Regressions failed against the previous adapter and passed
with the fixes. Packed consumer verification passed locally on Node 22/24,
including both Chromium modes, exact Rust/Python/JavaScript DOCX byte parity,
and schema/OPC validation. The Rust all-targets suite, doctests, Wasm Clippy,
791 schema-required Python tests, exact catalog byte parity, and strict
523-page docs build also passed.
Track broader authoring parity in #21; keep editor parity separate.

### Full JavaScript binding workstream

**Authoring experience (#29):** the JavaScript `declarative` namespace exposes
28 named factories matching Rust/Python components. The quickstart, report,
and catalog use them; both prior record forms remain supported. The same
tarball passed strict NodeNext/bundler type checks and Node 22/24 with
Chromium development and production. The installed quickstart, report, and
all six catalog outputs matched their native Rust/Python counterparts byte
for byte. The quickstart also passed independent readback and schema/OPC checks.

Use the [Rust-source parity contract](JAVASCRIPT-PARITY.md) for authoring,
resolution, template and implemented editor capabilities. This workstream
includes the implemented Rust editor surface, not the future editor
checkpoint below. The inventory maps Rust requirements to JavaScript APIs
and evidence; focused cases are not proof of full parity.

**B0/B1 checkpoint:** The shared boundary preserves the legacy
`generateDocx({ blocks }, options)` form and adds `DocumentProgram`,
resolution and reusable snapshots, template rendering and package plans,
Node file helpers, and typed imported inspection (E01–E03). The
[focused evidence map](JAVASCRIPT-PARITY.md#implementation-and-focused-evidence)
links installed declarations and individual fixtures. Node image reads carry
the basename with the bytes; Rust adds `ImageData::NamedBytes`, which
downstream exhaustive `ImageData` matches must handle. Separate concurrent
root resolution passes can still hang on a split property cycle; one-pass
cycle checks do not address this baseline limitation.

**B2 implemented in this worktree, not released:** The installed package
exposes anchored alignment, table-insertion and default-header image
previews (E04–E06). Local packed runs on Node 22.23.5 and 24.22.3 and in
Chromium 159.0.8154.6 (Vite development and production) matched Rust
on all six catalog DOCX files. Typed preview reports, invalid inputs,
independent readback and untouched payloads were checked separately.
The [evidence and review notes](JAVASCRIPT-PARITY.md#qualification-and-independent-review)
record the final same-tarball matrix.

On the standalone tables branch, direct physical-cell vertical-alignment edits were staged
alongside other preview operations. It validates explicit grids and vertical
merge chains, rejects unsupported structures, and preserves unrelated cell
properties. The three catalogs exercise the same patch on their inserted table.
Packed Node 22.23.5 and 24.22.3 and Chromium 159 development and production
consumers passed, including clean offline provenance replay. All seven catalog
artifacts matched Rust and Python byte for byte. The cell-edit oracle matched
the installed consumers exactly and passed 11-part schema and OPC validation;
the edited catalog passed 20-part schema and OPC validation. Independent
readback and untouched-part checks also passed.

Workstream acceptance criteria:

1. Inventory the Rust surface. Map each public authoring, resolution, template, and editor capability to a JavaScript API and a test. Use Rust source as the baseline; the all-features example alone is not proof of completeness.
2. Verify behavior from the installed package. Exercise Node 22/24 and Chromium. Require exact DOCX bytes against Rust and Python for equivalent deterministic fixtures, including every catalog artifact. Check typed inspection results, edit previews, invalid inputs, and preservation of untouched parts separately; matching final bytes does not prove those behaviors.
3. Run independent review. Have the reviewer check every inventory row against Rust and its test evidence. Fix validated findings and repeat review. Report any unverified capability as a gap, not a pass.

The integrated #21 checkpoint retained the expanded legacy declarations,
added table-cell and section story inspection, and audited 536 authoring
field entries and 243 variants. Native byte comparisons and independent
review confirmed the timestamp and bigint byte-budget repairs. The same
tarball passed on Node 22/24 and Chromium development and production builds;
all six catalog artifacts matched Rust/Python exactly, and an offline rebuild
reproduced the packaged Wasm and glue.

Integrated native regression checks passed with Python 3.12: 311 Rust tests
including doctests, native Clippy with warnings denied, and all 810 Python
tests with schema validation required. The strict documentation build checked
529 HTML pages and regenerated byte-identical Rust/Python catalogs.

The Rust/Python release gates below still apply.
GitHub has both release environments and a crates.io bootstrap secret; this
does not verify the registry-side trusted-publisher binding tracked in #4.

### Earlier issue sweep

Audit all nine issues open at the start of this sweep. Use one isolated topic
worktree per issue that needs changes. For reproducible regressions, record
the failing test before the fix and verify it again after integration.
Close issues only after their acceptance criteria are met on the default
branch; keep umbrella or externally blocked work open.

| Issue | Scope |
| --- | --- |
| #2 | Closed as already fixed by `ab59df3`; all three builder regression tests passed on the default branch. |
| #4 | Canonical Pages URLs returned HTTP 200; the old site path returned 404. Both hosted quickstarts ran outside the checkout against registry packages and passed independent readback. Strict build: 516 HTML pages; docs tests: 11 passed. The 0.3.0 PyPI OIDC run succeeded; crates.io used the bootstrap token. Private crates.io trusted-publisher verification remains. |
| #7 | Closed under the revised LibreOffice-only scope. The [hosted matrix](https://reindervosdewael.github.io/rocx/viewer-compatibility/) records Writer GUI observations on four synthetic files, including the before/after catalog edit, and separate headless, readback and payload checks. Desktop Word remains untested. |
| #3 | At this earlier checkpoint, direct body and default-header paragraphs gained run and hyperlink inspection in Rust and Python. The current milestone above extends inspection and adds direct body run edits. |
| #9 | The [JavaScript package](../javascript/README.md) replaces the prototype. [Hosted CI](https://github.com/ReinderVosDeWael/rocx/actions/runs/45219366575) verified Node 22/24 and Chromium consumers and uploaded the installable artifacts. The [package guide](https://reindervosdewael.github.io/rocx/javascript/) and [API reference](https://reindervosdewael.github.io/rocx/api/javascript/) are live; the strict build checked 523 pages. No npm registry upload. |
| #16 | Independent fixture reproduced tab-stop corruption; the fix preserves tab-stop markup and still replaces literal run tabs. Source and output passed schema/OPC validation. |
| #17 | Block, row, and cell fixtures reproduced unguarded replacement and passed schema/OPC validation. Ordinary matches inside these controls are rejected; unrelated paragraphs and supported targeted replacement still work. |
| #18 | A schema-valid template without margins reproduced invalid output. Partial patches now fill required built-in defaults and preserve existing margins. Both cases passed schema/OPC validation and readback. |
| #19 | Eight fresh-document checks reproduced shared/explicit references before the fix. Explicit stories now use separate parts and omissions inherit; the independent template append check remains unchanged. Focused integration (including exact DOCX byte parity) passed 59 tests. |

Issues #2, #7, #9, and #16–#19 are closed on the default branch. The integrated Rust
all-targets suite, three doctests, native and Wasm Clippy, Rust 1.88 check,
787 schema-required Python tests, exact catalog DOCX byte parity, and a
strict 520-page docs build passed. Rust crate packaging, the Python source
archive and isolated wheel, and rebuilding the JavaScript tarball from its
bundled source passed locally. The [hosted Verification run](https://github.com/ReinderVosDeWael/rocx/actions/runs/45219366683)
also passed. The default-header follow-up passed the Rust suite, three
doctests, Clippy, Rust 1.88, 791 schema-required Python tests, the strict
523-page docs build, and exact DOCX byte parity and schema/OPC validation
for all six catalog pairs.

The next release is conditional on resolving #3 and #4, confirming
registry setup, and passing the local and hosted preflight in
[RELEASING.md](RELEASING.md). Do not use a publishing tag to test the pipeline.

## Prior parallel GitHub issue work

| Issue | First slice |
| --- | --- |
| [#2](https://github.com/ReinderVosDeWael/rocx/issues/2) | Existing `RunFormat` builders cover the request; three focused tests and three doctests passed without code changes. |
| [#3](https://github.com/ReinderVosDeWael/rocx/issues/3) | Direct-body tables and default-header stories are inspectable in Rust and Python; focused tests and exact catalog parity passed. Continue checkpoint task 4 with richer inline targets. |
| [#4](https://github.com/ReinderVosDeWael/rocx/issues/4) | Canonical links and package metadata updated. `/rocx/` returns HTTP 200; the legacy path returns 404. Docs tests and strict build passed; private publisher verification remains. |
| [#5](https://github.com/ReinderVosDeWael/rocx/issues/5) | Hosted quickstarts passed on Linux x86_64, Windows x86_64 and macOS arm64 (universal2 wheel). Clean installs of the published 0.3.0 crate and Linux x86_64 wheel also passed. |
| [#6](https://github.com/ReinderVosDeWael/rocx/issues/6) | [Branded report](website/branded-report.md) linked from README and the site. Exact Rust/Python output, unchanged template and header parts, and strict docs build verified. |
| [#7](https://github.com/ReinderVosDeWael/rocx/issues/7) | Added a [viewer checklist](VIEWER-COMPATIBILITY-CHECKLIST.md); verified app results are still needed. |
| [#8](https://github.com/ReinderVosDeWael/rocx/issues/8) | [Evaluation complete](PYTHON-SYNC-EVALUATION.md): keep the async API. The local probe passed script execution, active-loop rejection, async producer, exact DOCX byte comparison, and independent readback. |
| [#9](https://github.com/ReinderVosDeWael/rocx/issues/9) | The [original WebAssembly probe](https://github.com/ReinderVosDeWael/rocx/tree/0201e21/tools/js-wasm-probe) cross-built and returned DOCX bytes in Node with a fixed timestamp. It has since been replaced by the JavaScript package. |

After integrating the checkpoint with `main`, all six paired catalog artifacts
matched byte for byte, and 21 Rust editor tests and 29 focused Python tests passed.
That checkpoint preceded structured cell-paragraph and broader story inspection.
Keep the JavaScript tarball out of the Rust/Python release path; viewer results and
hosted release preflight remain separate.
Repository commits, pushes, and release tags are authorized by `AGENTS.md`.
The current checkpoint and issue work are split into separate commits.
Normal pushes can deploy the documentation site. Do not push a publishing tag
until the hosted preflight succeeds and registry setup is confirmed.

## Editor goal: declarative parity

Task 9 CLI follow-up on `fad29a9`: legacy alignment requests and six additional
edit kinds share the core mixed transaction. The insertion wire forms cover
plain paragraphs and rectangular text tables; header images use inline bytes.
Two seven-kind batches matched installed Python output byte for byte, passed
schema/OPC validation and python-docx readback, and preserved source and
untouched part bytes. Richer declarations and future editor operations remain open.

Complete #3 before releasing 0.4.0. Footnote, endnote and comment entries
already have relationship-based inspection.
Re-run release preflight on the integrated candidate, not the earlier docs commit.
The working branch also stages plain single-text run replacements in the mixed
transaction; property, block, table, resource and revision operations remain.
It also stages conservative whole-run removal for direct body paragraphs.
On the standalone inlines branch, packed Node 22/24 and Chromium development/production checks passed, including
exact removal bytes against Rust and Python, independent readback, untouched
payloads, schema/OPC validation, and reproducible package provenance.
Every new editor slice must ship together in Rust, Python and JavaScript,
including the installed JS package's Node and Chromium parity checks.

An agent should be able to open an existing DOCX, inspect it as typed rocx
content, and apply anything rocx can author declaratively at a valid target.
Text replacement is one operation, not the scope of the editor. Keep the
existing declarative and template APIs intact.

### Import and edit the original package

- `open` should return an imported document with a typed view of sections,
  stories, blocks, inline content, and document resources. Reuse resolved rocx
  types where they fit; do not try to reconstruct the callbacks, conditions, or
  inputs that produced the file. Keep snapshot-scoped anchors into the source,
  opaque nodes for unsupported content, and the identities of shared parts.
- Use existing declarations for new content and typed edit operations to say
  where it goes: insert, replace, remove, patch selected properties, or add
  or update a named resource where valid. A property patch preserves unmodeled
  properties on the same node; whole-node replacement may discard opaque
  content inside that explicitly selected target, but must not break
  references outside it.
- Edit the original package rather than rendering the imported view as a new
  `ResolvedDocument`. Reuse the existing resolver, Word codecs, and OPC
  relationship handling. Preserve untouched part payloads byte for byte;
  edited XML and the ZIP container may be reserialized. Keep the existing
  signature and compatibility guards.
- Expose one Rust core through Rust, Python and JavaScript and a thin JSON CLI.
  The workflow is inspect, submit an anchored batch, preview, then save a new
  copy. Preview the intended changes and any effects on shared resources.
  Make edits to a shared header explicit: change the shared part or make a
  section-local copy. Reject stale targets, conflicting operations, and unsafe
  edits without applying part of the batch.

### Milestones and acceptance

1. Use the [declarative parity inventory](EDITOR-PARITY.md) to track inspection,
   addition, patching, replacement, and removal, including styles, numbering,
   defaults, and metadata. Appending content alone does not count as parity.
2. Prove the architecture on fixtures generated independently with python-docx:
   insert a declarative table at a body anchor, patch a paragraph property
   without losing unrelated markup, and add an image to a header shared by
   two sections, testing both a shared edit and a section-local copy.
3. Close the feature inventory. Read the all-features DOCX back into typed
   values and check them against expected content. Test preservation of
   opaque markup, unchanged part payloads, and rejection of unsafe batches.
   Update all three all-features examples, `examples/ALL_FEATURES.md`, and the
   parity and coverage checks. Require exact Rust/Python/JavaScript DOCX byte parity
   for equivalent edit batches, not just normalized XML equality.

The editor opens an unsigned Transitional DOCX in all three bindings, exposes
direct body paragraphs, tables, control metadata and opaque blocks, and previews
alignment patches and declarative paragraph and table insertions. Body, cell
and inspected story paragraphs expose run, hyperlink and revision structure with
snapshot-scoped anchors alongside their flat projections. Direct body
single-text runs support replacement and conservative removal; other inline
edits remain unavailable. The section view reports body ranges and explicit section types,
without inferring defaults. Tables can contain formatted paragraphs, nested
tables, fractions, and external links. The editor sizes
automatic tables only in simple single-column sections and rejects insertion
inside active or ambiguous supported ranges. Other inline objects, numbering,
comments, revisions, and content controls in inserted tables remain open.

The editor inspects existing default, first and even headers and footers with section references,
ordered paragraph projections, and opaque unsupported direct children. Header
inspection is lazy and distinguishes physical part identity from section
selection. Default and first-page headers can receive an inline image. Shared mode lists all sections that
reference or inherit that part, including first/even aliases. Section-local
mode forks a supported text-and-link header, keeps the original part and its
relationships, and reports any following section explicitly redirected back
to the original to preserve inheritance. Section-local copies with existing
drawings, fields, controls, or unknown content are refused rather than
duplicating source identities.

The paired catalogs exercise the batch. Exact final, template, and edited
Rust/Python DOCX byte parity passed, and both edited artifacts validated 20
schema-covered parts plus OPC relationships. The separate header source and
both header edit modes passed exact Rust/Python byte parity and schema
validation (10 covered parts for the source and shared output, 11 for local).
Use python-docx for independent fixtures and for reading basic features of
source and edited DOCX files. These fixtures are not desktop Word-authored
documents; Word-specific interoperability is not a gate for this batch.

### Serial checkpoint: editor work

Table-cell structured inline readback landed in `1c17f03` under #3. The
feature branch passed 808 schema-required Python tests and schema/OPC checks
on all 12 catalog artifacts. Exact Rust/Python catalog byte parity also passed
after integration with the JavaScript changes. Header/footer inspection landed
in `5f103c5`: default, first-page, and even-page stories expose paragraphs,
inline structure, inherited references, and shared part identities. Its
feature branch passed 808 schema-required Python tests. The integrated build
passed 30 Rust and 19 Python editor tests and exact Rust/Python catalog parity.

The independent #21 authoring slice landed in `a58282c`: JavaScript supports
paragraph alignment, tabs, breaks, external links, image dimensions, and
rich table cells (including nested tables). Packed Node 22/24 and Chromium
checks passed, with five byte-identical reports and schema/OPC validation.
The full Rust/Python catalog also passed exact byte parity. The full JavaScript
binding checkpoint above supersedes this initial slice.

Tasks 1–3 passed on the original python-docx fixture. Tasks 4–9 have partial
implementations; see the current milestone above. Entry story inspection includes
notes and comments; table structure edits, general paragraph and inline editing,
resource mutation and broader CLI coverage remain open. Task 10 requires the
combined gate and completion of the feature inventory.

1. Generate the [python-docx editor fixture](../tests/fixtures/README.md#python-docx-editor-fixture)
   with formatted body text, an insertion anchor, and two sections sharing a
   default header.
2. Verify anchored table insertion and paragraph alignment in one batch on
   the fixture, checking unrelated markup and exact untouched part payloads.
3. Verify shared and section-local header image edits on the fixture, checking
   effective headers after reopening and preservation of the original header
   and its relationships for the section-local edit.
4. Expand typed inspection of sections, stories, tables, and inline content,
   retaining snapshot anchors and opaque nodes.
5. Add anchored paragraph and inline insert, replace, and remove operations,
   and extend property patches to preserve unrelated formatting.
6. Complete table editing, including row and cell operations, grid and merge
   validation, and support for remaining declared cell content.
7. Add typed inspection and reference-safe editing of styles, numbering,
   defaults, and metadata.
8. Close outstanding story and inline content gaps using the parity inventory,
   including references and their owning relationships.
9. Expose inspect, batch preview, and save-copy through the thin JSON CLI.
   Verify atomic rejection of stale, conflicting, and unsafe edits.
10. Run the acceptance suite: typed all-features readback, all three examples and
    coverage checks, preservation, and exact Rust/Python/JavaScript DOCX byte parity.

## Released: 0.3.0

### Release checklist

1. [Documentation audit](DOCUMENTATION-AUDIT.md) applied and verified locally.
2. Review the release candidate for packaging completeness, including license
   notices and release notes. Keep internal planning documents and the
   unsupported Wasm probe out of distribution archives.
3. Follow the [release runbook](RELEASING.md) for `0.3.0`. Confirm the hosted
   build matrix and publication destinations before tagging.
4. Once the runbook checks pass, a release tag can be created and pushed under
   `AGENTS.md`. Only GitHub Actions publishes packages,
   on a pushed `v*` tag after validation and artifact builds. Manual dispatch is
   preflight only, even on tagged refs.

The maintainer confirmed the pending PyPI publisher. The `pypi` and `crates-io`
GitHub environments exist, and the repository has the crates.io bootstrap
secret. The [hosted non-publishing preflight](https://github.com/ReinderVosDeWael/rocx/actions/runs/45190233113)
passed for `0a54aaa`, including all three wheel families and the source
distribution. The [tag-triggered publish run](https://github.com/ReinderVosDeWael/rocx/actions/runs/45191346555)
succeeded. Version 0.3.0 is live on [crates.io](https://crates.io/crates/rocx/0.3.0)
and [PyPI](https://pypi.org/project/rocx/0.3.0/). Clean registry quickstarts
passed on Linux x86_64, using Rust 1.88 and CPython 3.12. Follow up by configuring
crates.io trusted publishing and removing the bootstrap token.

### Baseline before the dependency refresh

- The Rust and Python manifests, lockfiles, and Python runtime report `0.3.0`.
- The recorded local baseline passed 269 Rust tests, three doctests, 741
  schema-required Python tests, and Clippy. Both final catalogs validated 20
  covered parts plus OPC relationships; final and template byte parity passed.
- After the audit edits, the strict site build checked 477 HTML pages and
  verified exact catalog parity. All 11 documentation tests passed, and the
  standalone Rust and Python quickstart and template snippets ran locally.
- The earlier version gate, offline lock check, Cargo packaging and isolated
  CPython 3.12 wheel probe also passed. Cargo packaging used `--allow-dirty`
  for local preflight.

## Deferred backlog

- Workbook-free clustered multi-series charts and a literal superscript
  prototype.
- Relationship-based validation for relocated font tables and more OPC
  relationship graph tests.
- An internal `AlternateContent` selector, subject to the
  [proposed editor contract](ALTERNATE-CONTENT-DESIGN.md).
- A glossary preservation prototype.

See the [archived roadmap and handoffs](ROADMAP-HISTORY.md) for earlier
decisions, checkpoint evidence, and prototype notes. Instructions there are
historical, not current assignments or authorization.
