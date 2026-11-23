# Active plan — 0.3.0 release preparation

rocx is feature-complete for its current support profile, not for all OOXML.
Keep the feature freeze in place. The editor plan and deferred backlog below
are not release blockers.

## Release checklist

1. [Documentation audit](DOCUMENTATION-AUDIT.md) applied and verified locally.
2. Review the release candidate for packaging completeness, including license
   notices, release notes, and whether internal planning documents should ship.
   Keep unrelated working-tree changes intact.
3. Follow the [release runbook](RELEASING.md) for `0.3.0`. Confirm the hosted
   build matrix and publication destinations before tagging.
4. Commit, push, deploy, or publish only with authorization for that action.
   Only GitHub Actions publishes packages, on a pushed `v*` tag after validation
   and artifact builds. Manual dispatch is preflight only, even on tagged refs.

## Release candidate status

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
- Hosted release preflight has not run. No tag was created or pushed, and no
  deployment or publication was performed.

The [engine audit](ENGINE-AUDIT.md) records four correctness and preservation
defects (#10–#13). Each failed its regression before the fix and passed
afterward in Rust and Python. The integrated Rust suite, three doctests, Clippy,
767 schema-required Python tests, and exact Rust/Python DOCX byte parity passed.
Close the issues when their fixes reach the default branch.
Use the audit's format-preserving report edit as a migration acceptance case
for tasks 4–6.

## Next feature after 0.3.0: agent editing with declarative parity

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
2. Prove the architecture on Word-authored fixtures: insert a declarative
   table at a body anchor, patch a paragraph property without losing unrelated
   markup, and add an image to a header shared by two sections, testing both
   a shared edit and a section-local copy.
3. Close the feature inventory. Read the all-features DOCX back into typed
   values and check them against expected content. Test preservation of
   opaque markup, unchanged part payloads, and rejection of unsafe batches.
   Update both all-features examples, `examples/ALL_FEATURES.md`, and the
   parity and coverage checks. Require exact Rust/Python DOCX byte parity
   for equivalent edit batches, not just normalized XML equality.

The editor opens a Transitional DOCX in Rust and Python, exposes direct body
paragraphs and placeholders for other blocks, and previews alignment patches
and declarative table insertions in one batch. Tables can contain formatted
paragraphs, nested tables, fractions, and external links. The editor sizes
automatic tables only in simple single-column sections and rejects insertion
inside active or ambiguous supported ranges. Other inline objects, numbering,
comments, revisions, and content controls in inserted tables remain open.

The editor also inspects existing default-header references and appends an
inline image through a separate preview. Shared mode lists all sections that
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
Preservation, section-boundary, and inheritance tests use synthetic or
rocx-authored sources; the Word-authored fixture milestone is still open.

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
