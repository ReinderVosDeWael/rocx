# Active plan — editing existing DOCX files

Documentation pass: publish user guides and API references only. Keep build, architecture, release and verification procedures such as the [viewer observation record](VIEWER-COMPATIBILITY-RECORD.md) in the repository.

Local build hygiene ([#82](https://github.com/ReinderVosDeWael/rocx/issues/82)):
reuse the checkout's Cargo target for routine checks and clean up isolated
packed JavaScript runs by default. Hosted artifact jobs explicitly retain
output without changing the clean Wasm build and replay requirements.

## Active interface milestone: local MCP server (#52)

Build `crates/rocx-mcp` using the [implementation and acceptance plan](MCP-PLAN.md).
The six-tool interface includes new document creation and the seven #41 edit
kinds. Qualify the installed binary with a real client saving a selected
preview that matches a direct Rust edit byte for byte. This milestone is
independent of the 0.4.0 release.

Local qualification passed with `rmcp` 3.11.0 and Python MCP SDK 1.27.0
negotiating `2025-11-25`: ten Rust tests and eight installed-client tests,
including native byte parity, readback, package preservation, and lifecycle
errors. Rust 1.88 Clippy, the existing CLI tests, the independent Wasm build
check and the strict 699-page documentation build also passed. See the
[installed acceptance instructions](../tests/mcp/README.md) for reproduction.
The hosted checks passed, PR #71 merged to `main`, and #52 is closed.

**In progress — [#72](https://github.com/ReinderVosDeWael/rocx/issues/72):**
follow the [full MCP/Rust editor parity plan](MCP-EDITOR-PARITY.md). The first
increment freezes the compiled editor surface, maps it to coverage obligations,
and adds original-snapshot scalar range replacement/deletion and point
insertion in plain body paragraphs. Qualify these edits independently of
the seven original edit kinds; broad inspection and scoped edits remain open.

Local checks: the compiled snapshot contains 1,908 symbols. The MCP crate's
13 Rust tests and Clippy passed on Rust 1.88 and stable. Nine installed-client
tests passed, including native byte parity and rejection/retry for scalar
edits. A live OpenCode client replaced `😀` and appended a period using
original offsets in `A😀BC`; independent `python-docx` readback gave `AxBC.`.
[PR #74](https://github.com/ReinderVosDeWael/rocx/pull/74) merged after hosted
qualification.

The next #72 increment introduces `inspect_document.view` with bounded entry
pages, issues snapshot-owned anchors for ordinary footnotes, endnotes and
comments, and exposes the seven `EntryEdit` variants as a standalone
`preview_edits.operation`. All 22 installed-client tests passed, including
108-entry pagination, relocated owners, colliding lexical IDs, cursor lifetime
and native byte comparisons. All 21 Rust tests and Clippy passed on 1.88 and
stable. Live OpenCode discovery showed typed ordinary and scoped request
forms; a saved comment author change affected only `word/comments.xml`.
The clean checkout CI failure in
[PR #75](https://github.com/ReinderVosDeWael/rocx/pull/75) was fixed by
checking in the independent Rust test fixture instead of assuming a locally
generated example exists.

The next increment adds note-reference lifecycle, scoped revision
proposals/review, direct paragraph and section layout, per-open import
budgets and an explicit native empty-batch preview. It reuses snapshot-owned
pagination for the new views. Fifty installed-client and fourteen parity
tests passed, plus ten subtests. Live OpenCode discovery remained typed, and
saved examples verified a first footnote, direct paragraph layout, tracked
replacement markup and unchanged payloads for a no-op. Continue with physical
stories, metadata/defaults and the remaining transaction and declaration
families; this increment does not meet the #72 closure gate.

The physical-story and resource increment adds six-slot header/footer
inspection and shared/local image previews, scoped reference lifecycle and
physical text edits for headers, footers, notes and comments. Metadata and
direct defaults work inside ordinary mixed batches, with independent
queries for each native read scope. Live OpenCode created an even-page
footer, edited its run, then saved a title edit that changed only
`docProps/core.xml`. The installed MCP and parity suites passed 90 tests
and ten subtests, including native byte comparisons for all six image
slots in both modes.

Next: build the reusable resolved paragraph/inline/block transport for
body and story splices. Extend the existing owner model to physical run
formatting and tables. Keep rich story creation, linking eligible but
unreferenced story parts, numbering/style edits and remaining slices open
in the ledger until there is installed-client evidence.

The block increment wires rich declarations into ordinary
paragraph insertion, block edits, story creation and story splicing. Section
inspection exposes original body ranges for split/merge. All nine native block
variants passed installed-client checks, including native byte parity and
block change reports. The combined suite passed 99 tests and ten subtests;
35 Rust tests and Clippy passed on Rust 1.88 and stable. Live OpenCode saved a
formatted hyperlink paragraph and a section split. Continue extending the same
declarations for rich tables, fields, controls and physical run edits.

The next increment implements field readback and ordinary field/body-inline
edits, plus single-owner transactions for physical run formatting and
nested/direct tables. Reuse a rich, recursive table declaration for insertion,
cell contents and eligible donor properties. Direct-body table geometry and
whole-node edits run in the ordinary batch; physical owners retain native
scope restrictions. [PR #80](https://github.com/ReinderVosDeWael/rocx/pull/80)
merged after hosted qualification.

The paired-range increment adds paged bookmark/comment marker discovery and
ordinary create/move/remove edits using two original body-inline anchors.
Comment ranges operate on existing definitions without modifying them.
The saved lifecycle outputs match direct Rust bytes, and a live OpenCode
client saved and paged through a cross-paragraph bookmark.

The content-control increment adds paged main-story controls and wrapping
sites, read-only physical story controls, and ordinary patch, wrap, unwrap,
definition, choice and content replacement edits. The Rust 1.88 installed
binary passed 139 MCP and parity tests plus ten subtests. Saved previews
matched direct Rust output for all four wrapping contexts and a mixed
control batch; independent checks validated the packages and confirmed that
only the main document part changed.

The body-object increment exposes drawing and fraction readback, chart-series
queries, direct-body drawing patches, image asset redirection, and object
removal. Inline declarations now include modeled shapes, column charts, and
picture transformation and accessibility. The Rust 1.88 installed binary
passed 141 MCP and parity tests plus ten subtests. Native byte comparisons
covered all ten property groups, shared chart forking, and occurrence-local
image replacement.

The entry-inline increment pages original note and comment inlines and
previews native edits scoped to one physical part, including pictures,
external hyperlinks, and direct formatting. Saved picture lifecycles and
hyperlinks matched direct Rust bytes for comments and relocated footnotes and
endnotes. Reference runs and unrelated parts were preserved. The Rust 1.88
installed binary passed 145 MCP and parity tests plus ten subtests.

The style increment adds paged catalog readback and the seven native style
variants in ordinary previews, including missing catalog creation, conditional
table regions, and guarded reference migration. Saved previews matched
direct Rust bytes for mixed edits, catalog creation, clears, and retargeting.
The Rust 1.88 installed binary passed 148 MCP and parity tests plus ten
subtests.

The numbering increment pages abstract and concrete catalogs, exposes their
independent read scopes, and supports global definition and override edits
with direct body assignments in the ordinary batch. Relocated-catalog
lifecycle previews and first-part creation with a staged numbering style
matched direct Rust bytes. The Rust 1.88 installed binary passed 152 MCP and
parity tests plus ten subtests.

The physical-numbering increment pages direct paragraphs in shared
header/footer parts, note/comment entries, and direct body cells. Ordinary
previews can target these paragraphs alongside staged lists. Assignment and
cleanup batches matched direct Rust bytes, including a cell in a later table
after removing an earlier one. Continue with typed inline discovery in
header/footer and table-cell paragraphs, then reconcile remaining payload and
owner coverage against the compiled ledger.

Continue #72 in parallel: qualify typed inline discovery for physical
stories and cells. The first audit found mappings for all 18 batch slices and
all public `ImportedDocument` methods; it did not establish parity. Prioritize
resolved declaration fields, independently discoverable body and table
targets, empty relocated note owners, and relinking eligible story parts
after their last slot reference is removed. Then qualify full typed readback,
native diagnostics, and reports. Compare each increment with direct Rust
through the installed stdio client. Reconcile the ledger before declaring
parity; a transport mapping alone is not qualification.
The [parity closure gate](MCP-EDITOR-PARITY.md#delivery-slices) still
requires the integrated evidence on `main`.

The paired-range increment merged in [PR #81](https://github.com/ReinderVosDeWael/rocx/pull/81).
The controls, drawings, entry-inline, style and numbering increments merged in
[PR #87](https://github.com/ReinderVosDeWael/rocx/pull/87) after hosted checks.
The [#88](https://github.com/ReinderVosDeWael/rocx/pull/88) checkpoint merged
after hosted checks. It adds physical inline views, richer declaration
inputs, relinking unused story parts and selecting relocated empty note
owners. It also expands direct-Rust evidence for layout, styles, resources,
entry edits and the nine mixed-preview report arrays. On `11a3d2f`, 39 MCP
Rust tests and Clippy passed on Rust 1.88 and stable; 232 installed-client
and ledger tests plus ten subtests passed with schemas enabled. The strict
docs build checked 700 pages. A live OpenCode client retrieved a bold
header run via `inline-at` and saved a centered body paragraph; independent
`python-docx` readback verified both. Continue with logical slot isolation,
scoped diagnostics, guarded edit evidence, and symbol-by-symbol reconciliation.

The follow-up issues logical story slot anchors from `sections` without
reading story bodies, adds paragraph-only layout discovery, and uses the
independent default-header reader for run, image, splice and link targets.
Native diagnostics are retained for the scoped preview operations. Flat
field declarations carry deep native field trees without increasing JSON
parser depth; installed debug previews at depths 64, 200 and 248 matched
Rust and reopened using a bounded worker stack. Focused acceptance
exercises every modeled default/metadata
write and table property donor key, plus protected entry guards. The
coverage validator can group explicit symbol lists with shared evidence;
partial and unexplained symbols must still block #72 closure.
A live OpenCode client set a paragraph's outline level through the
paragraph-only view, and independent readback confirmed both original
section-type elements were retained.

## 0.4.0 release

Release Rust, Python and `rocx-docx` for npm from one validated commit.
[#3](https://github.com/ReinderVosDeWael/rocx/issues/3) and
[#48](https://github.com/ReinderVosDeWael/rocx/issues/48) are closed; MCP
([#52](https://github.com/ReinderVosDeWael/rocx/issues/52)) is not a release gate.
Before pushing `v0.4.0`, complete untagged hosted preflight, verify the registry
publisher setup tracked by #4, and confirm the npm first-release credential.
See [the release runbook](RELEASING.md).

[Untagged preflight](https://github.com/ReinderVosDeWael/rocx/actions/runs/45618855400)
passed crates.io OIDC, qualification and all artifact builds. #4 is closed.
The [v0.4.0 tag run](https://github.com/ReinderVosDeWael/rocx/actions/runs/45623224278)
published Rust and Python. npm misinterpreted its relative tarball path as a
Git repository. The [recovery run](https://github.com/ReinderVosDeWael/rocx/actions/runs/45629145225)
successfully staged the original tarball. The public npm registry now serves
`rocx-docx@0.4.0`; a clean install generated a DOCX, and the registry tarball
matched the original qualified artifact byte for byte (SHA-256
`1c7505c2dd5db0a7fe57e70088c86c95b99fda04dd3abd6b0970b777a2ca1551`).
The [#70](https://github.com/ReinderVosDeWael/rocx/issues/70) cleanup removes
the one-time workflow and runbook exception while retaining the corrected
tarball path and digest check for tag-triggered publication.

## Current milestone: implement the remaining editor capabilities

**#48 follow-up:** merge `26eb84e` into `b89afbc` without changing either
baseline's qualification record. The merged candidate carries the union of
both oracle inventories and `linkedMixed`: a reciprocal
style-owned level edit, inline formatting, removal of an earlier table, and
assignment to a cell addressed in the original snapshot. Keep #48 open
until this follow-up reaches `main` and hosted checks pass; #65 proceeds
independently and #3 stays closed.

**Merged candidate evidence:** 312 default-feature and 349 all-feature
all-target Rust tests, three doctests, Rust 1.88 and stable native/Wasm
Clippy, warning-free Rustdoc and the 698-page strict docs build passed. All
1,019 schema-required tests passed against the installed Python 3.12 wheel.
The full packed gates passed independently on Node 22.23.5 and 24.22.3,
each with Chromium 159 development and production. All 56 editor outputs
and 15 catalog packages matched native Rust byte for byte in each mode;
the installed Python catalog also matched all 15. Strict TypeScript,
offline Wasm/glue replay, schema/OPC, typed readback and installed-package
provenance passed.

`linkedMixed` matched native Rust, CLI, installed Python and all packed
consumers (36,700 bytes, SHA-256
`e77f8cb1512ecc4341c253982504194462b8c085d47c4a7639f1eeb5a41d70bc`).
Independent package checks retain the untouched payloads, including the opaque
header, preserve the numbering alias and concrete overrides, and verify the
new numbering/inline values. Tests separately reject a cell/table overlap,
deletion of the owning concrete instance, and stale anchors; a failed preview
does not affect retry and a failed CLI save does not create a destination.
The original `linkedNumbering` bytes are unchanged. The two historical
inventories are still represented; regenerated comment timestamps and the
extended style fixture account for editor byte differences against the
saved parents. Catalog comparisons use the merged declarations rather than
requiring expanded examples to reproduce an older catalog.

**Historical qualification of `b89afbc`:**
The owner closed #3 manually. [PR #65](https://github.com/ReinderVosDeWael/rocx/pull/65)
merged as `ecc0704`; the modeled resource follow-up merged as `edd24a2`.
The [acceptance map](EDITOR-ACCEPTANCE.md) records the combined operation
inventory and evidence without changing tracker state. Its typed catalog
checks now include the rich `.controls`, `.objects` and `.entry-text` results
in all three bindings. The mixed CLI/Python fixture adds a disjoint structural
replacement and rejects overlapping field/control replacements, a pending
paragraph revision replacement, and incompatible section boundary requests.

Local qualification includes 348 Rust tests and three doctests, Rust 1.88
Clippy, stable native/Wasm Clippy, 1,001 schema-required installed Python
tests, and a strict 683-page docs build. Full packed Node 22/24 and Chromium
159 development/production passed: 50 editor outputs and 15 catalog packages
matched Rust byte for byte, and installed Python also matched the catalog.
The 14 catalogs inherited from `1fda978` retained their exact bytes. Strict
TypeScript, offline source replay, schema/OPC and untouched-payload checks
passed separately from typed readback.

**Historical qualification of `26eb84e`:**
**#48 modeled acceptance on the combined editor:** the direct numbering
catalog reads full modeled levels and replacement levels, including nested
label/paragraph formatting and stored style links. Local levels can be edited
when the `styleLink` / style `numId` relationship is reciprocal; level-free
aliases are kept intact and their paragraph assignments are checked against
the owner. The CLI uses the same readback and accepts rich level declarations.
The authoring audit found no remaining declarative default fields; the
metadata Comments key now explicitly aliases the stored description.

Combined qualification includes `0ba9350` and its structural CLI merge
`c3ea09d`. The 345 Rust all-feature/all-target tests, three doctests, 991
schema-required installed Python 3.12 tests, native and Wasm Clippy, Rust 1.88,
strict installed TypeScript, offline Wasm/glue
replay and 667-page strict docs build passed. All 13 catalogs matched across
Rust, installed Python, packed Node 22/24/26 and Chromium 159 development and
production. All 38 editor oracles matched native Rust in the packed consumers
and passed schema/OPC and independent readback. For the same python-docx
linked-numbering fixture, the native oracle, CLI, installed Python and packed
Node and both browser modes emitted identical DOCX bytes. Tests separately
checked no-op preservation, stale anchors, conflicts, live references and
opaque or broken style links.

Closure criterion: require the combined fix and its cross-binding,
schema/OPC, and preservation qualification on `main`. Do not count
effective numbering, ownership migration, unsafe/dangling references,
protected or arbitrary nested editing targets, legacy numbering tabs,
independent theme fallbacks, or unmodeled default/metadata variants against
the supported direct model. See [modeled acceptance](EDITOR-RESOURCES.md#modeled-acceptance-boundary).
#3 is manually closed; this follow-up tracks #48 without reopening it.

The isolated integration takes the structural merge at `c3ea09d` and the
resource/style recovery at `2d5de8e`. It retains the earlier mixed oracles,
physical-table property no-op guards, and both CLI discovery modes. The new
fixture stages a numbering style and a footer list, applies a staged table
style in the physical footer, and edits a section margin and border while
preserving opaque attributes on the margin group and untouched border edge.
These are three original-snapshot scopes, not a combined cross-owner batch.
The combined verification passed 345 Rust all-feature/all-target tests, three
doctests, native/Wasm Clippy, Rust 1.88, and 77 focused schema-required tests
against the installed Python wheel. The CLI output matched the installed core
at every stage; independent XML/ZIP readback and live-reference rejection
verify that unchanged fields and physical-owner payloads survive.
Packed Node 22/24 and Chromium 159 in development and production matched
all 37 editor outputs and 13 catalogs.

The merge of `f269508` into `c1b6627` combines the resource/style and structural
CLI variants, including both inspection selectors in one request. Physical
table operations remain owner-scoped; body table replacements can use a style
staged alongside numbering at an original paragraph anchor. The mixed CLI
regression checks the installed Python byte oracle, preserves all unrelated
part payloads, and rejects live numbering deletion and cross-scope batching
without creating a destination. Table property no-ops retain the original
body or physical owner; destructive clears still reject annotated markup.

Qualification: 344 Rust all-feature tests, three doctests, native/Wasm Clippy,
Rust 1.88, 974 schema-required installed Python tests (including 46 focused
cases), an isolated wheel probe, and the 652-page strict docs build passed.
Packed Node 24 and
Chromium 159 in development and production matched all 33 editor outputs
and 13 catalogs, including schema/OPC and independent readback. The 32
previous editor outputs and all 13 catalogs remain byte-identical to
`c1b6627`; the physical-table fixture is the 33rd editor output.

The [#3 acceptance map](EDITOR-ACCEPTANCE.md) reconciles the original issue and
the operation contract against the `c1b6627` structural baseline. The Rust
showcase test, installed Python catalog tests and packed JavaScript consumers
now check typed values on every structural catalog artifact, independently of
schema/OPC, package preservation and exact ZIP parity. The divergent inline
cluster (`981d798`) is recorded separately; integrate and extend the typed
checks for the combined artifact set before claiming qualification. Remaining
operation and transport gaps are closure decisions, not implied passes.
On `c1b6627`, the Rust showcase and its Clippy check passed, as did 35
catalog tests against the installed Python 3.12 wheel. Packed Node 22/24
and Chromium 159 development/production passed typed readback; all 13
catalogs in each consumer matched the generated Rust catalog bytes.

The merge of `30ac174` into the qualified `7d5d08b` keeps the #48 resource
guards and the recovered layout, section, story and table APIs. Table removal
is deferred until original body coordinates have been used, while a no-op
legacy table layout patch still leaves the main part untouched. A new combined
oracle replaces and removes tables, consumes a newly staged table style, adds
a hyperlink relationship and custom property part, and numbers the paragraph
after the removed table. It first rejects removal of a still-used numbering
instance, then reuses the original anchors for a successful preview.

**Qualification of `c1b6627`:** 306 default-feature Rust tests, 343
all-features/all-targets tests, three doctests, native/Wasm Clippy, Rust 1.88,
and warning-free Rustdoc passed. The installed Python 3.12 wheel passed all
964 tests with schemas required, the isolated wheel probe, and the 649-page
strict docs build. All 13 catalog outputs matched Rust, installed Python and
packed Node 22/24 and Chromium 159 in both browser modes. All 32 editor
outputs matched Rust in the packed consumers; installed Python also matched
the new combined oracle. Strict installed TypeScript and offline Wasm/glue
replay passed. The 12 catalogs and 26 editor outputs from
`7d5d08b` are byte-identical to their qualified baselines.

The isolated branch now merges the qualified `7d5d08b` resource and structural
integration. The mixed CLI/installed-Python fixture uses a staged character
style and numbering definition, a picture, a field cache referring to a
same-batch bookmark, and a rich wrapper. It checks wrapper/numbering collisions
and retains a pending revision when a disjoint drawing and metadata change
are applied. A wrapper conflicts with same-block paragraph property edits;
an ordinary property patch cannot make a pending paragraph mark unreviewable.
Revision proposals and reviews remain separate requests. Final
style/numbering checks reject new live or opaque references. `ProcessContent`
guards now cover DrawingML properties and relationship-selected chart content.
See [the CLI contract](CLI.md) for the body rich-edit transport.

The combined source at `06ffc46` passed 344 Rust tests, three doctests,
Rust 1.88, native/Wasm Clippy, and 984 schema-required installed Python tests,
including 33 focused Python/CLI cases. Packed Node 22 and Chromium
development/production matched all 43 editor files and 14 catalogs against
Rust, with strict TypeScript, schema/OPC validation and offline archive
replay. Installed Python also matched all 14 catalogs. The same tarball in
Node 24 reproduced all 72 Node DOCX files and the editor reports.

The isolated inline integration starts at `ae0cefd` and merges `ec54da1`,
`f4c9ac6` and snapshots of the outstanding controls, fields and drawings work.
The `richControls` oracle combines rich wrappers, three drawing insertions,
a field flag, a paired bookmark, a run-format patch and paragraph insertion.
It rejects a wrapper crossing a staged bookmark and tracked formatting in that
bookmark after reopen. `fieldBatch` covers a cache spanning body paragraphs;
`formatOwners` covers linked runs in cells and physical header/footer parts.
Standalone gates passed: 342 Rust tests, three doctests, Rust 1.88, native/Wasm
Clippy and 928 schema-required tests from the installed Python 3.12 wheel.
Packed Node 22 and 24 and Chromium 159 development and production matched
all 42 editor packages and 13 catalogs byte for byte against Rust. Installed
Python also matched all 13 catalogs and regenerated the mixed control, field,
revision and rich-inline oracles. Separate checks passed for strict TypeScript,
schema/OPC, relationship ownership and untouched part payloads.

The preceding `ae0cefd` integration combined fields and paired ranges with
inlines, controls and revisions. Its `guardedWorkflow` oracle covers bookmark
boundaries, unique revision IDs, control-site eligibility before and after
review, and retrying a review after rejecting a colliding range. Separate guards
exercise move names and field caches containing controls or revisions.

**Qualification of `ae0cefd`:** 342 Rust all-target/all-feature tests, three
doctests, native and Wasm Clippy, Rust 1.88 and all 915 schema-required
installed Python tests passed. All 38 editor and 12 catalog packages
matched complete Rust ZIPs in packed Node 26 and Chromium. All 50 passed
schema/OPC checks; Node 22 and 24 also matched the 38 editor files. The
installed Python catalog matched all 12 files; the
independent Python editor checks verified the new combined workflow, revision
and field packages, retained part payloads and python-docx readback.

**#48 / style recovery atop `7d5d08b`:** recover the pending numbering work,
compound style changes from `e7c7696`, and the pending numbering-style
ownership regression in a separate worktree. The transaction stages abstract
and instance replacements before styles and payloads, and rechecks removals
against the finished package. Direct footer/header, ordinary note/comment
and physical table-cell paragraphs can take numbering references. The CLI
resolves those addresses against the digest-bound source and exposes abstract
and instance replacement, level edits, relinking and explicit style retargeting.
Equivalent level payloads and compatible default/metadata scalar lexemes
remain untouched.

Qualification: 341 Rust all-feature/all-target tests, three doctests, and
974 schema-required tests against the installed Python 3.12 wheel passed.
Rust 1.88, native/Wasm Clippy, strict installed TypeScript, offline Wasm/glue
replay and the 632-page strict docs build passed. All 12 catalogs matched
exactly across Rust, installed Python, packed Node 22/24/26 and Chromium 159
development/production. All 29 editor outputs matched Rust in those packed
consumers and passed schema/OPC checks. An independent footer/cell fixture
matched Rust CLI, installed Python and packed Node byte for byte, including
no-op, stale-anchor, duplicate, live-reference and opaque-markup probes.

At that recovery checkpoint the direct readers did not reconstruct full
abstract/override level bodies or expose their abstract style links. Edits
were not yet able to change levels of style-linked abstracts or safely relink
across them. Dropping level addresses of surviving instances and editing
nested or protected paragraph targets remained unsupported.
Unmodeled defaults, extended metadata and non-string/non-boolean custom
variants are retained but not editable; the CLI still uses declaration
shorthands. Style retargeting is restricted to provable, same-kind stored IDs;
unknown references and lexical alias collisions remain conservative blockers.

Primary integration ([#65](https://github.com/ReinderVosDeWael/rocx/pull/65)):
the point, section, formatting, story and lifecycle work from `a30c516` is
combined with `ea1c023` and the preservation guards from `ee4f8f5`. The merge
keeps both sets of catalog and editor outputs, and extends the JSON CLI to
defaults, numbering and direct style fields and regions. The resource/style
CLI batch matches the installed Python API byte for byte and rejects stale or
invalid edits without creating a destination. Concrete numbering and
numbering-style edits share the abstract numbering guard for XML integers and
case-insensitive resource media types.

**Local qualification of `7d5d08b`:** 303 default-feature Rust tests, 340
all-features/all-targets tests and three doctests passed, with formatting,
native/Wasm Clippy, Rust 1.88 and warning-free Rustdoc. The installed Python
3.12 wheel passed all 959 tests with schemas required, including the CLI and
preservation cases. The isolated wheel probe and the 628-page strict docs
build passed. All 12 catalog DOCX files
matched exactly in Rust, installed Python 3.12, packed Node 22/24/26 and
Chromium 159 development/production. All 26 editor outputs matched Rust in
those JavaScript consumers; the mixed resource oracle and three other editor
outputs also matched installed Python. Schema/OPC validation, independent
readback and untouched-payload checks passed, as did strict installed
TypeScript and offline Wasm/glue replay from the packed source archive.
The stale table inspection expectation on the hosted PR head is corrected
and passes locally on Node 22 and 24 with Chromium. Hosted checks must rerun
on the updated PR head. The remaining editor and transport limits are recorded
in the acceptance map. This checkpoint preceded the manual closure of #3.

**#3 preservation review of `fe5e5df`:** the editor now rejects matched MCE
`ProcessContent` wrappers before treating a property as absent or a bookmark
as closed. Regression fixtures cover body and entry stories, table layout,
numbering, decimal bookmark identity, XML integer lexical forms, mixed-case
resource media types, and byte preservation for no-op layout edits. Thirteen
cases failed on `fe5e5df`; all 16 pass with the fixes, including controls for
unrelated opaque nodes and string-valued permission IDs.

**#47/#48 integration:** defaults, metadata, numbering and style editing share
one transaction. Allocate absent resource parts without adopting orphan parts,
resolve new numbering-style links against staged instances, reject duplicate
alias edits, and recheck removals against the finished package. Exercise both
a mixed consumer oracle and an independent fixture that creates all six parts.
Combined verification: 63 focused Rust and 96 installed Python tests passed;
strict TypeScript and native/Wasm Clippy passed. All 24 editor outputs matched
between Rust, packed Node 26 and Chromium 159; installed Python also matched
the mixed oracle. All 11 catalog packages matched across Rust, Python and Node
and passed schema/OPC validation and independent readback.

#48 defaults increment: direct run/paragraph defaults and document-wide
settings now support independent inspection and set/clear. All eleven catalogs
matched in Rust, installed Python 3.12 and packed Node 26 and passed schema/OPC
and independent readback. All 23 packed Node editor outputs matched Rust;
the mixed tab-stop edit also matched installed Python. Focused tests covered
relocated settings, explicit false, annotation guards and stale-anchor retry.

**#3 section, story, and table recovery on `a30c516`:** split a snapshot-anchored
section at an original body gap or merge it into its successor. Keep effective
story slots and unrelated package payloads, including on table-first and empty
final sections. The recovered Rust, Python and JavaScript APIs also patch
direct paragraph/section layout, create/link/remove explicit story references,
splice header/footer blocks, and edit modeled table/row/cell properties and
whole nodes. The JSON CLI now transports these operations and the nested and
physical-story table operations from `fc1cad0`. Layout can batch paragraph
and section fields; story and physical table operations keep their core
scopes instead of composing multiple ordinary transactions.

Recovery qualification: Rust all-features/all-targets and doctests, native and
Wasm Clippy, Rust 1.88, and 924 schema-required tests against the installed
Python 3.12 wheel passed. All 13 catalogs matched Rust, Python, packed Node
22/24 and Chromium (development and production). All 30 editor outputs matched
the Rust oracle in the packed consumers. Installed Python independently matched
layout, split/merge, shared story and table output bytes, with separate
anchor, error, schema/OPC and untouched-payload checks.

Follow-up integrated `29be239` and `fc1cad0`: 343 Rust tests, native/Wasm
Clippy, Rust 1.88, and 35 focused installed-wheel tests passed. Mixed ordinary
CLI edits, direct layout, story references/splices and physical-table batches
matched their installed core output byte for byte. Fixtures check stale digests,
overlap and owner rejection, no-op package payloads, and relationships
allocated only to the edited footer. Packed Node 22/24 and Chromium
development/production matched all 31 editor outputs and 13 catalog artifacts.

#48 metadata worktree: core scalars and timestamps, company/manager, and named
string/boolean custom-property edits passed standalone qualification: 55 Rust
editor/showcase tests, nine installed-wheel resource tests, native Clippy and
strict TypeScript declarations. All eleven catalog outputs matched byte for
byte across Rust, Python 3.12 and packed Node 26. The mixed resource output
passed independent readback and schema/OPC checks (20 covered parts).
See [the package metadata contract](EDITOR-RESOURCES.md#package-metadata).

Controls follow-up (#3), based on `b0a28b0`: the two new editor outputs cover
standalone wrap, full definition replacement, unwrap and value-addressed
drop-down changes. Independent python-docx/XML checks verify placement,
retained formatting and unchanged part payloads. All twelve catalog files
matched across Rust, installed Python and packed Node. The two editor outputs
also matched Chromium; both and the new catalog sample passed schema/OPC checks.

#3 revision follow-up: add direct bold-only `w:rPrChange` proposals, typed
run-property metadata and accept/reject on the reopened run anchor. Check
the three preview outputs across bindings against independent Word-package
readback; see [revision contexts](REVISIONS.md#propose-a-formatting-change).
The proposal and both reviewed documents matched exactly in Rust, installed
Python 3.12 and packed Node 26; all three passed schema/OPC validation.
The block-context follow-up marks a plain paragraph's terminator and text
together, exposes mark metadata, and accepts/rejects the pair only next to a
safe following paragraph. Six paragraph outputs (insert, delete, both reviews
for each) matched complete Rust/Python/packed Node bytes and passed schema,
OPC, python-docx and untouched-payload checks.
The 56 Rust imported-editor tests, 15 focused Python tests, native/Wasm
Clippy and strict installed TypeScript passed. All eleven catalog artifacts
matched across Rust, installed Python and packed Node.

Fields/links/ranges slice (#3): direct body field insertion, instruction/flag
patches, whole-field replacement/removal, hyperlink targets/properties, and
cross-paragraph bookmark/comment pairs are implemented on the topic branch.
`fieldsRanges.docx` matched in Rust, installed Python and packed Node; the
package passed schema/OPC validation and independent python-docx readback.
That first result predated mixed field/range batches and balanced
cross-paragraph complex-field cache edits. Non-body range endpoints and
opaque caches remain outside the implementation.
See [fields](FIELDS.md#editing-an-imported-field-or-link) and
[ranges](RANGES.md#imported-body-ranges).
Numbering follow-up (#48): add abstract levels and concrete instances, set or
clear concrete starts/replacement levels and anchored body paragraph references,
and guard removals against live or uninspectable references in the final package.
Standalone verification: 55 Rust editor tests, 74 installed Python tests, strict
TypeScript and packed Node 26 passed. The independent numbering fixture matched
in Rust, Python and Node; all 24 editor outputs matched Rust/Node, and all 11
catalog outputs matched across the three bindings. The mixed catalog passed
schema/OPC checks and independent numbering readback.

The direct layout API addresses each inspected section independently of its
first block, including empty implicit finals. The [layout contract](EDITOR-PARITY.md#direct-paragraph-and-section-layout)
lists fields, units, clear semantics and unsupported imported forms.

The recovered numbering increment allocates missing parts beside the main
document, replaces supported levels and relinks instances without breaking
consumer level addresses, and edits direct story paragraphs through their
original physical owner.

The standalone #3 style increment adds attribute-level formatting and tab edits,
collision-safe creation of an absent styles relationship, and explicit guarded
style-reference migration. The three style oracles matched complete Rust,
installed Python, packed Node 26 and Chromium 159 output and passed schema/OPC
validation. All eleven catalog files matched Rust/Python/Node. The installed
consumer suite compared 25 editor outputs. Exercise style ownership with the
#48 instance/override editor (`c1a8fac`) in a combined checkout.

The ten topic branches start from `fad29a9` and were first combined in
[#63](https://github.com/ReinderVosDeWael/rocx/pull/63). The
[editor inventory](EDITOR-PARITY.md) describes the available calls
and distinguishes them from the broader [operation goals](EDITOR-PARITY.md#operation-contract).

| Ticket | Supported in the candidate | Still outside the implementation |
| --- | --- | --- |
| #42 Text ranges | Replace/delete nonempty scalar ranges or insert text at an original scalar position in a plain direct body paragraph. | Cross-paragraph offsets, mixed/opaque paragraphs and other stories. |
| #43 Blocks | Replace/remove direct body paragraphs and replace supported section contents. Split or merge anchored direct sections. Inspect and patch direct paragraph styles/headings, alignment, tabs, indents, spacing/line height and flags; section geometry, margins, equal columns, grid, page/line numbering, single borders, alignment, flags and break type. | Story/nested paragraph layout; layout in mixed ordinary batches; richer imported spacing/indents, unequal columns, art/theme borders and unmodeled numbering formats. Paragraph numbering references are a separate slice. |
| #44 Inlines | Insert/replace supported direct body inlines. Inspect and patch direct formatting on runs, including hyperlink children, in body paragraphs, physical body cells, effective header/footer parts and ordinary note/comment entries. | Nested tables, control/revision targets, themed or split Latin font selections and destructive edits over unrecognized properties. |
| #45 Tables | Inspect and patch direct properties, insert/remove physical rows/cells and logical columns, and replace cell content in body, nested and physical story tables. Replace guarded table/row/cell nodes or remove eligible whole tables in the direct body. | Splitting protected merges, unsafe destructive changes, whole-node replacement/removal outside direct-body scope and cross-owner batches. |
| #46 Stories | Shared/local images and guarded block splices in all six header/footer slots; create/link/remove explicit story references; direct story text, ordinary entry inlines and simple paragraph edits; note reference create/attach/detach/delete/retarget/renumber; base comment author/date/initials. | Deleting OPC story parts, custom note marks, shared or nested note reference edits, destructive rich block edits, modern comment metadata, reserved entries and local text forks. |
| #47 Styles | Inspect direct properties, regions and base/next/linked IDs; add all four kinds and create an absent catalog; patch direct formatting attributes and tabs, set/clear all 13 conditional wrappers, and migrate provable typed ID references. | Rewriting field instructions or opaque users, arbitrary unmodeled XML edits, and migrating linked numbering styles. |
| #48 Resources | Modeled defaults and metadata; typed direct numbering catalogs and guarded abstract/instance/override edits, reciprocal style-owned levels and direct body/cell/story paragraph numbering. | Effective numbering, ownership migration, unsafe level loss, protected or arbitrary nested targets, and unmodeled defaults or metadata. |
| #49 Revisions and #3 follow-up | Track insertion, replacement, deletion and paired moves of simple direct body runs; track direct bold and simple whole-paragraph insert/delete; review eligible changes. | Mixed ordinary/revision transactions, other property changes, block moves and rich or nested revisions. |
| #50 Controls | Inspect physical story owners; wrap/unwrap compatible main-story siblings in block/inline/row/cell contexts; replace supported rich block or inline content; edit approved metadata, text and choice lists. | Nested and bound edits, other specialized types, and mutation outside the main story. |
| #51 JSON CLI / #3 follow-up | Digest-bound transport for the `b0a28b0` batch families, story images/text, revision operations (including paragraph marks and bold), rich inline/field/range/control/drawing edits, point insertions, block flags, direct layout, section lifecycle, story references/splices, direct and physical table edits, metadata, defaults, concrete numbering and style fields/regions. Preview identifies exact output bytes; save-copy exclusively creates. | Declarations are a subset of authoring; physical-owner rich formatting and entry/note lifecycle are not yet exposed over the CLI. See [CLI contract](CLI.md). |
| #52 MCP | Separate workspace crate under qualification. See the [active milestone](#active-interface-milestone-local-mcp-server-52). | Remote access, cross-session persistence and edit families beyond the initial seven. |

**#3 table follow-up (before section/story/table recovery):** direct-body table layout can be inspected, set or
cleared; grid column widths, physical cell spans and vertical merge markers
are inspectable in all three bindings. The recovery increment covers independent
direct style, alignment, look flags, border sides, grid widths, row flags and
height, cell margins, direction, shading and merge fields. Table/row/cell
replacement and whole-table removal refuse destructive edits over opaque
markup. The physical-owner follow-up discovers nested body/header/footer and
ordinary note/comment tables; edit properties, rows/cells/columns and guarded
cell contents with owner-local relationship allocation. Whole physical table
replacement/deletion outside direct body scope, controlled rows/cells and
local story forks remain open.
Both structural edit artifacts and all eleven catalogs matched across Rust,
installed Python 3.12 and packed Node 26. All 24 Node editor outputs matched
the native reference; the table fixture and both outputs passed schema/OPC
validation. Tests cover physical-coordinate conflicts, merged intervals,
unsupported clears, preserved extension attributes and untouched part payloads.
The complete isolated `check-packed.sh` on `c54e6dd` passed with Node 22.23.5
and Chromium 159 dev/production, including the independent 2880-twip, span-2
inspection fixture and exact offline replay ([#3 evidence](https://github.com/ReinderVosDeWael/rocx/issues/3#issuecomment-7145848269)).

**Combined qualification on `6f11a6a`:** Rust all-features/all-targets and
doctests, native/Wasm Clippy, Rust 1.88, 894 schema-required Python tests and
the 592-page strict docs build passed locally. Packed Node 22/24 and Chromium
development/production passed all eleven catalog and 23 editor comparisons,
schema/OPC validation, independent readback and offline package replay.
The mixed edit was byte-identical in Rust, Python and all three JavaScript
consumers. Regressions cover crossing a bookmark inside math, live style
aliases, protected move and bound-control template edits, proofing ranges
across cells, and sharing a staged style catalog across operations.
Keep #48 open for the remaining resource inventory; #52 is separate.

The standalone #3 block-property slice from `b0a28b0` passed 55 Rust and 57
installed-Python editor tests, native/Wasm Clippy, strict TypeScript, packed
Node inspection and 23 native/Node editor byte comparisons. All eleven catalog
files matched across Rust, Python 3.12 and Node 26. The mixed flag/break-type
batch matched all three bindings, independent python-docx and XML/ZIP readback,
and Transitional schema/OPC validation (11 schema-covered parts).

Standalone #3 CLI follow-up on `b0a28b0`: three Rust process tests, eight
schema-required Python process tests and native binary Clippy passed. A
nine-operation batch matched the installed Python API byte for byte. Separate
fixtures cover all seven structural table operations, revision review, footer
images, physical story/entry text, declaration resolution, digest replay,
exclusive saves and rejected batches. The transport contract is in [CLI.md](CLI.md).

The structural integration extends the CLI to paragraph and section flags,
section break types, table layout, scalar point insertions, core subject and
package metadata in the same digest-bound batch. Named metadata readback and
mixed-batch byte parity are covered by the CLI process tests.
With `fe5e5df` integrated, 59 Rust editor/CLI tests and 92 schema-enabled
Python tests passed. All 11 catalog files matched across Rust, Python and
packed Node; 25 Node editor outputs matched Rust and passed independent readback.

The #48 integration combines core subject and the package metadata work.
Numbering and document defaults remain separate workstreams; #3 closure
requires reconciling the individual slices against the editor inventory and
qualifying their combined behavior.

On merge `fe5e5df`, all 11 catalog DOCX files matched across Rust, installed
Python 3.12 and packed Node 26. The run-text output combines core subject,
creator, extended manager, a boolean custom property and body point insertion;
independent readback and schema/OPC validation passed. All 25 packed Node
editor outputs matched the Rust oracle. The Wasm adapter built and the strict
TypeScript editor consumer compiled. All 338 Rust all-features/all-targets
tests and three doctests passed. The installed Python 3.12 wheel passed
all 907 tests with required schemas. All eleven catalogs also passed
schema/OPC validation.

The #42 follow-up adds `TextInsertion` alongside nonempty `TextRangeEdit`.
The python-docx fixture exercises run-boundary formatting, paragraph end and a
runless paragraph; its complete output matched Rust, installed Python, packed
Node and Chromium and passed schema/OPC checks. All eleven catalog outputs
matched across Rust, Python and Node.

The direct run-toggle follow-up to #3 adds four imported formatting values and
an opaque-formatting indicator. On its standalone branch, 54 Rust and 57
installed Python editor tests passed. The four-toggle DOCX matched across Rust,
Python 3.12, and packed Node 26, with schema/OPC, python-docx, and unchanged-part
checks. All eleven catalogs also matched across these consumers. Browser and
combined-branch qualification remain separate.

The next #3 inline slice reuses the authoring `RunFormat` encoder, with a
shared direct-property decoder and attribute-level patches. It adds character
styles, fonts, sizes, underlines, colors, script positioning, script flags and
language hints, and permits disjoint edits on one run. The rich fixture matched
complete DOCX bytes in Rust, installed Python 3.12, packed Node 26 and Chromium
159, passed schema/OPC validation, and retained unrelated attributes and part
payloads. All 24 editor outputs matched Rust in Node and Chromium; all 11
catalogs matched across Rust, Python and Node. Native/Wasm Clippy, strict
TypeScript, 54 Rust and 58 Python editor tests passed on the standalone branch.

After integrating `8c18d1b`, physical-owner run formatting passed 54 Rust and
84 Python editor/note tests, strict TypeScript and native/Wasm Clippy. Node 26
and Chromium 159 matched all 25 Rust editor outputs. The rich linked drawing
fixture matched Python too, retained the original content and relationships,
and passed schema/OPC checks. All eleven catalogs matched Rust/Python/Node,
including the formatted hyperlink in each ordinary note/comment owner.
**Style follow-up (#3, based on `b0a28b0`):** the seven-edit style fixture
matches full Rust, installed Python 3.12, packed Node 26 and Chromium 159
bytes. Independent readback checks the next-style reference, conditional cell
format, resulting style order, reciprocal numbering link, unchanged levels
and untouched part payloads. Schema/OPC validation passed. All 11 catalog
files matched across Rust, Python and Node. The 60 focused Rust and 74
Python tests passed, including clear/readback for every physical table region,
opaque-leaf protection and a style/numbering/content transaction. Strict
TypeScript and native Clippy passed. See the [style edit contract](EDITOR-PARITY.md#imported-style-edits)
for supported scalar fields and whole-region semantics.

The follow-on layout slice from `7f19e30` passed 56 Rust and 59
installed-Python editor tests, native and Wasm Clippy, and strict TypeScript.
Packed Node 26 and Chromium 159 matched all 24 Rust editor outputs; the
12-field layout batch also matched installed Python. Its independent table-first
and empty-final fixture and output passed schema/OPC validation (10 covered
parts each). All eleven catalogs matched across the three language bindings.

## Selected standalone topic results (before ten-way integration)

**#3 entry inline follow-up (from `b0a28b0`):** the eleven catalog files
matched byte-for-byte across Rust, installed Python 3.12 and packed Node 26.
The edited footnote, endnote and comment each own their added hyperlink;
schema/OPC validation passed for 20 covered parts. Independent python-docx
comment readback and package comparison verify preserved body references.

The follow-up to `8c18d1b` adds a twelfth cross-binding `.entry-edits.docx`
sample. The stories slice owns ordinary note reference/definition lifecycle
and content and metadata inside existing comments. The fields/ranges slice
owns paired bookmark/comment markers, main-story comment references and
associated comment-definition lifecycle ([coordination record](https://github.com/ReinderVosDeWael/rocx/issues/3#issuecomment-7145560023)).

For the individual verification records, read `docs/PLAN.md` at the topic
tips: ranges `6baa54d`, blocks `f4d5a58`, inlines `069f5c6`, tables `ae725f9`,
stories `07bb554`, styles `8e47b52`, resources `dc5c832`, revisions `cba0e66`,
controls `7ac2012`, CLI `9c2a4aa`. For example, the stories branch compared
eleven catalog artifacts across Rust, installed Python and packed Node, and
the CLI branch compared two seven-kind batches with Python. Neither run
exercised the ten-way transaction.

Style editing slice (#3): paragraph/character/table additions, direct bold and
italic patches, and guarded replacement/removal of simple unused custom styles
are implemented on the topic branch. The four-operation independent fixture
matched complete Rust, installed Python 3.12 and packed Node 24 output, passed
schema/OPC validation, and preserved every other part payload. Focused Rust
and Python tests cover rejection and transaction reuse. Numbering-style editing,
broader property patching and umbrella qualification remain open.

Next resources increment (#3): inspect and set/clear direct core title, default
paragraph alignment and shared abstract numbering level starts in mixed batches.
Keep IDs, instance overrides, paragraph references, and unrelated package
payloads intact. Scope and unsupported cases: [resource edit contract](EDITOR-RESOURCES.md).
Standalone verification: 44 Rust and 35 Python focused tests, Clippy, strict
TypeScript consumer check, independent python-docx readback, and exact parity
of nine catalog files across native Rust, Python 3.12 and packed Node 26.
The mixed resource output passed schema/OPC checks (20 schema-covered parts).

**Next revisions slice (#3):** qualify opt-in tracked insert-after, replace,
delete and paired moves of simple direct body text runs, plus anchored review
of existing wrappers. Keep the original editor's ordinary-text projections and
snapshot contract. Four focused package oracles cover tracked replacement,
reviewed insertion, paired move and accepted move. Mixed revision/ordinary
batches, property changes and rich/nested revision editing are outside this slice.
Focused verification passed: Rust editor/revision/showcase tests, native/Wasm
Clippy, 58 Python tests with required schemas, and packed Node 24/Chromium
editor tests. The four new DOCX outputs matched across Rust, installed Python,
Node and Chromium, with independent readback, untouched-part and schema/OPC
checks. All nine catalog artifacts matched across Rust, Python and Node.

## Previous milestone: qualify the #41 editor

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
| CLI | Digest-bound alignment preview and save-copy. The later #51 topic adds the other six edit kinds. |

**Combined verification passed locally on `6b34ff0`.** All Rust targets, three
doctests, native/Wasm Clippy, the Rust 1.88 check, 822 schema-required Python
tests, the isolated wheel check and the strict 552-page docs build passed.
Packed Node 22 and 24, with Chromium development and production builds, passed
offline package replay, schema/OPC checks and exact byte comparisons for all
nine catalog outputs. A mixed paragraph insertion, cell patch and run removal
produced identical DOCX bytes in Rust, Python, Node and both browser builds.
Readback verified target placement and unchanged part payloads; a conflicting
batch was rejected and the original anchors remained usable. Earlier branch
results below describe their own checkpoints. #3 and release preflight were
open at this checkpoint.

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

**Historical implementation baseline (before #42–#51):** the editor opens an unsigned Transitional DOCX in all three bindings, exposes
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

Tasks 1–3 passed on the original python-docx fixture. For the coverage of
tasks 4–9 after #42–#51, see the [current milestone](#current-milestone-implement-the-remaining-editor-capabilities).
Task 10 requires the combined gate and completion of the feature inventory.

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
