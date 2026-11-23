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

Audit all nine issues open at the start of this sweep. Use one isolated topic
worktree per issue that needs changes. For reproducible regressions, record
the failing test before the fix and verify it again after integration.
Close issues only after their acceptance criteria are met on the default
branch; keep umbrella or externally blocked work open.

| Issue | Scope |
| --- | --- |
| #2 | Closed as already fixed by `ab59df3`; all three builder regression tests passed on the default branch. |
| #4 | Canonical Pages URLs returned HTTP 200; the old site path returned 404. Both hosted quickstarts ran outside the checkout against registry packages and passed independent readback. Strict build: 516 HTML pages; docs tests: 11 passed. The 0.3.0 PyPI OIDC run succeeded; crates.io used the bootstrap token. Private crates.io trusted-publisher verification remains. |
| #7 | LibreOffice displayed three identified fixtures under Xvfb; the [checklist](VIEWER-COMPATIBILITY-CHECKLIST.md) records observed features and a separate headless conversion check. Desktop Word and a Word-authored fixture remain unverified. |
| #3 | Direct body run and hyperlink inspection now exposes snapshot-scoped read-only anchors in Rust and Python. The umbrella remains open for broader typed stories and structured edits, including revisions. |
| #9 | The [JavaScript package](../javascript/README.md) replaces the prototype. It offers an installable tarball, TypeScript declarations, and the shared Rust/Wasm authoring slice; local Node 22/24 and Chromium dev/production checks passed exact DOCX byte parity with Rust/Python. Verify the packed-artifact workflow on GitHub before closing. |
| #16 | Independent fixture reproduced tab-stop corruption; the fix preserves tab-stop markup and still replaces literal run tabs. Source and output passed schema/OPC validation. |
| #17 | Block, row, and cell fixtures reproduced unguarded replacement and passed schema/OPC validation. Ordinary matches inside these controls are rejected; unrelated paragraphs and supported targeted replacement still work. |
| #18 | A schema-valid template without margins reproduced invalid output. Partial patches now fill required built-in defaults and preserve existing margins. Both cases passed schema/OPC validation and readback. |
| #19 | Eight fresh-document checks reproduced shared/explicit references before the fix. Explicit stories now use separate parts and omissions inherit; the independent template append check remains unchanged. Focused integration (including exact DOCX byte parity) passed 59 tests. |

The combined #16–#18 fixes passed the Rust all-targets and doctest suites,
Clippy, and 777 schema-required Python tests, including exact Rust/Python DOCX
catalog byte parity. After the #19 fix and #3 inline slice were integrated,
the Rust suite, three doctests, Clippy, Rust 1.88 check, 787 schema-required
Python tests, exact catalog DOCX byte parity, and a strict 520-page docs build
passed.

The next release is conditional on resolving the open issues, confirming
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
Finish editor checkpoint task 4 before moving on to task 5. Extend the typed
inspection contract beyond the current direct body and default-header slice.
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
paragraphs expose run and hyperlink structure with snapshot-scoped read-only
anchors alongside their existing flat projections. Inline edits are not yet
available. The section view reports body ranges and explicit section types,
without inferring defaults. Tables can contain formatted paragraphs, nested
tables, fractions, and external links. The editor sizes
automatic tables only in simple single-column sections and rejects insertion
inside active or ambiguous supported ranges. Other inline objects, numbering,
comments, revisions, and content controls in inserted tables remain open.

The editor inspects existing default headers with inherited section references,
ordered paragraph projections, and opaque unsupported direct children. It
also appends an inline image through a separate preview. Shared mode lists all sections that
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
paragraphs have flat and structured inline readback, including read-only
run/hyperlink anchors, and direct sections, body tables, and default headers
are inspectable. The paired catalogs passed exact byte parity. Table-cell and
header paragraphs still expose flat inline projections; other stories are
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
