# Active plan — editing existing DOCX files

The initial editor checkpoint shipped in 0.3.0. The inline inspection slice
below is unreleased; the unrelated backlog remains deferred.

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
| #3 | Direct body and default-header paragraphs expose runs, hyperlinks, and snapshot-scoped read-only anchors in Rust and Python. Aliases of one physical header share inline anchors; distinct parts do not. The umbrella remains open for broader typed stories and structured edits, including revisions. |
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
Finish editor checkpoint task 4 before moving on to task 5. Extend structured
inline inspection to table-cell paragraphs and broaden story inspection.
Keep the JavaScript tarball out of the Rust/Python release path; viewer results and
hosted release preflight remain separate.
Repository commits, pushes, and release tags are authorized by `AGENTS.md`.
The current checkpoint and issue work are split into separate commits.
Normal pushes can deploy the documentation site. Do not push a publishing tag
until the hosted preflight succeeds and registry setup is confirmed.

## Editor goal: declarative parity

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
- Expose one Rust core through the Rust and Python APIs and a thin JSON CLI.
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
   Update both all-features examples, `examples/ALL_FEATURES.md`, and the
   parity and coverage checks. Require exact Rust/Python DOCX byte parity
   for equivalent edit batches, not just normalized XML equality.

The editor opens a Transitional DOCX in Rust and Python, exposes direct body
paragraphs, tables, and placeholders for unsupported blocks, and previews
alignment patches and declarative table insertions in one batch. Direct body
and default-header paragraphs expose run and hyperlink structure with
snapshot-scoped read-only anchors alongside their flat projections. Inline
edits are not yet available. The section view reports body ranges and explicit section types,
without inferring defaults. Tables can contain formatted paragraphs, nested
tables, fractions, and external links. The editor sizes
automatic tables only in simple single-column sections and rejects insertion
inside active or ambiguous supported ranges. Other inline objects, numbering,
comments, revisions, and content controls in inserted tables remain open.

The editor inspects existing default headers with inherited section references,
ordered paragraph projections, and opaque unsupported direct children. Header
inspection is lazy and distinguishes physical part identity from section
selection. A separate preview appends an inline image. Shared mode lists all sections that
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

Work through these tasks in order and checkpoint after task 10. Tasks 1–3
passed on the python-docx fixture. Task 4 is in progress: direct body
and default-header paragraphs have flat and structured inline readback,
including read-only run/hyperlink anchors; direct sections and body tables
are inspectable. The paired catalogs passed exact byte parity. Table-cell
paragraphs still expose only flat inline projections; other stories are
outside the implemented slice. Tasks 5–10 have not started.

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
10. Run the acceptance suite: typed all-features readback, both examples and
    coverage checks, preservation, and exact Rust/Python DOCX byte parity.

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
