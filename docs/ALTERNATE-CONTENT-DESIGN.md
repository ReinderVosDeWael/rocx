# AlternateContent: proposed runtime contract

Status: **design only**, not implemented. This is not a claim of complete MCE
conformance. `rocx` is an editor as well as a potential markup consumer: the
logical content a consumer selects and the XML an editor retains are different
contracts.

## Sources and evidence

- [ECMA-376 Part 3, fifth edition (December 2015)](https://ecma-international.org/wp-content/uploads/ECMA-376-3_5th_edition_december_2015.zip),
  verified against the publisher's PDF in the linked archive (archive SHA-256
  `42294159fbbbe9393ccadac95b859d7729cc68d908898bcbe31034dda059daa8`).
  See §§7.1 and 7.4–7.7 for wrapper syntax, §8 for application-defined
  extension boundaries, and §§9.1–9.4 for the reference processing model.
- [`tools/validate_ooxml.py`](../tools/validate_ooxml.py) `prefix_uris` and
  `preprocess` implement a **development-only**, destructive schema-validation
  projection. It selects using `BASE`, splices children, and errors when no
  fallback exists. It is neither runtime support nor the editor contract; in
  particular, its no-fallback error is not the consumer behavior described below.
- [`src/ooxml/compatibility.rs`](../src/ooxml/compatibility.rs) currently checks
  `MustUnderstand` on editable WordprocessingML roots and descendants, stops
  at foreign subtrees, and does not select alternatives. The runtime fixture
  `test_preserves_an_opaque_alternative_without_processing_it` in
  [`tests/python_bindings/test_mandatory_namespaces.py`](../tests/python_bindings/test_mandatory_namespaces.py)
  checks retained choice, binding and nested requirement, not branch selection
  or byte preservation.
- [OOXML.md](OOXML.md#xml-and-compatibility) documents that untouched part
  payloads are retained, while parsed and rewritten XML is normalized; neither
  implies whole-DOCX input/output byte identity.

## Consumer selection, versus editor policy

P3 §9.3 selects the first `mc:Choice` for which **every** `Requires` prefix
denotes a namespace in the application configuration. If none is eligible,
it selects `mc:Fallback` if present. Under §9.4(3), with neither an eligible
choice nor fallback, the wrapper contributes no content; the lack of a
selected branch is not itself a mismatch. The processor checks
`MustUnderstand` on the `AlternateContent` wrapper even if no branch is
selected. It also checks it on the selected `Choice` or `Fallback`, but not
the inactive branch wrappers, and recurses only into the selected payload.
The reference model can mark a nested branch selected under an inactive
ancestor (§9.3 example); that nested branch still cannot contribute to the
output (§9.4). MCE processing is suspended inside application-defined
extension elements (§8); rocx's opaque boundaries are a separate editor policy.

P3 §§7.1 and 7.4–7.7 require at least one choice and allow at most one fallback
after the choices. `Requires` is an unqualified, nonempty, whitespace-delimited
list of bound prefixes and cannot designate the MCE namespace. The wrapper and
fallback have no unqualified attributes, and `Choice` has only `Requires` as
an unqualified attribute. Qualified attributes are allowed in MCE or ignorable
namespaces; MCE elements cannot carry `xml:*` attributes. Foreign children
declared ignorable can occur before, between, or after branches (§7.5).
The reference model signals a mismatch if such a child is **not** marked
ignored, for example if its namespace is understood (§§9.2 and 9.4(3)(a)).
P3 §9.1 distinguishes signaling a mismatch from stopping processing.

For a proposed runtime selector, resolve **each token** at its own `Choice`
using the original in-scope namespace bindings, including declarations on that
choice. Compare namespace **URIs** against an explicit capability allowlist,
never conventional prefix spellings. Aliases for the same URI are equivalent;
local shadowing changes the URI for that choice only, and scope must be restored
for siblings. The implicit `xml` binding is handled as XML namespace rules
require. Do not infer missing source bindings from serializer defaults. A known
URI counts as understood only when the selector's declared profile actually
supports the semantics needed there; an XML parser knowing a name or the
existing `MustUnderstand` guard accepting a URI is not enough. Publish and
test the profile; do not automatically reuse validator `BASE` or expand it just
because markup can be preserved.

The editor should retain the complete `AlternateContent` wrapper, all choices,
fallbacks, ordering, attributes, namespace meaning and inactive content when
rewriting an enclosing part. A logical selected view must not physically splice
or drop inactive alternatives into the stored tree. Untouched package part
payloads remain byte-identical; rewritten XML is only promised semantic/structural
preservation, including bindings used by prefix-valued attributes, not lexical
byte identity. Full DOCX bytes can change due to package serialization. Exact
Rust/Python DOCX parity is a separate cross-language output requirement.

An edit to only the selected branch can leave another consumer displaying a
**stale inactive alternative**, even if every branch is faithfully retained.
Never silently update all branches: their structures and meanings can differ.
For the first slice, reject any operation that targets, traverses, deletes or
rewrites content inside an alternative, or replaces an ancestor in a way that
would discard it. Permit unrelated edits only when preservation can be proven;
otherwise reject before publishing output. A later explicit branch-edit policy
must decide synchronization, diagnostics and whether callers accept stale
alternatives; selection alone does not solve this problem.

## Bounded first runtime slice

Recommend an internal **selection/inspection and safety-guard** slice for
`AlternateContent` encountered in editable WordprocessingML parts, not a
general MCE processor or an editing API for selected content. Preserve the
original tree and evaluate a logical branch without exposing its content to
existing mutators. Apply the guard to every relevant edit path, including
ancestor replacement and broad text/revision operations, not merely direct
branch edits. Out-of-scope opaque parts remain opaque. Nested alternatives are
selected recursively only along the selected logical path, with their own
namespace scopes; unselected payloads remain retained. Distinguish wrapper
syntax validation from processing `MustUnderstand`, which applies to the
outer wrapper and the selected branch wrapper only. Do not consume inactive
payloads.

Before selection, validate encountered wrapper structure and prefix lists:
reject missing/empty or syntactically invalid `Requires`, unbound tokens or
tokens bound to MCE, missing choices and duplicate/misordered fallbacks, with
part/location diagnostics. Until ignore processing is implemented, also
reject encountered foreign wrapper children as unsupported, even when
§7.5 allows them. Check XML parse failures as usual.
Reject unsupported MCE constructs encountered in the selected view unless
their semantics are implemented; do not pretend selection alone implements
`Ignorable`, `ProcessContent` or nested `MustUnderstand`. This conservative
error policy is a **proposed rocx safety profile**, not a claim that every
malformation has that exact P3-mandated outcome. On a valid no-choice/no-fallback
case, the inspection view is empty and the original wrapper is retained.
Do not scan untouched opaque parts merely to enforce this new profile.

## Acceptance matrix for implementation

Tests should use independently constructed template DOCX fixtures, inspect
expanded names and in-scope URI bindings, and exercise actual Rust and Python
entry points, not only a helper or validator projection.

| Case | Expected logical result / editor result |
| --- | --- |
| Two supported choices and fallback | First choice wins; all branches retained in order. |
| First choice has supported and unsupported tokens; second all supported | Second wins; every token matters. |
| Same URI under different prefixes; same prefix shadowed locally to unsupported URI | Aliases select equivalently; shadowed choice fails eligibility; sibling scope restores. Include declarations on the choice and `xml`. |
| No eligible choice, fallback exists | Fallback selected; inactive choice retained. |
| No eligible choice, no fallback | Empty logical content; wrapper retained, not validator's error. |
| Unknown `MustUnderstand` on outer wrapper with no selected branch; on selected branch; on inactive branch | First two signal mismatch (proposed editor diagnostic); the inactive attribute is retained and does not itself cause a mismatch. |
| Foreign ignorable child alongside branches | Valid wrapper syntax; a full processor ignores it only when §9.2 marks it ignored. First slice reports an unsupported construct. |
| Nested alternative in selected branch; another in inactive branch | Nested selected path resolves in local scope; inactive tree retained, not consumed. |
| Unbound, empty, missing or invalid `Requires`; no choice; duplicate or early fallback; malformed XML | Diagnostic failure before output for encountered edited-part structures; no destination/source damage. |
| Unsupported selected-view MCE semantics or selected nested mandatory namespace | Fail closed with diagnostic until implemented; do not inspect inactive payload as selected. |
| Unrelated edit adjacent to alternative | Edit succeeds only if all branches and prefix-valued bindings survive; independently inspect output. |
| Edit within selected or inactive branch, or destructive ancestor edit | Reject atomically, never silently stale or discard a branch. |
| Untouched opaque part containing alternatives | Exact part payload bytes preserved, without preprocessing. |
| Paired Rust/Python fixture and existing final/template catalogs | Compare complete DOCX bytes exactly across languages for each equivalent output, not normalized XML; separately validate applicable schemas and read back all branches. |

## Decision points before implementation

1. Approve the exact URI capability allowlist and what “understood” guarantees
   for each editable story; do not equate it with existing guard constants.
2. Inventory all mutation/traversal paths and define proof of safe unrelated
   edits; if any cannot enforce the guard, narrow the slice further.
3. Decide whether and how a future explicit selected-branch edit may accept or
   resolve stale alternatives. No synchronization or byte-lossless edited XML
   is promised by this proposal.
