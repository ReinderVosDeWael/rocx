# Editor declarative parity inventory

## Existing imported addresses

Anchors belong to one open snapshot, not the DOCX file: opening identical bytes
or reopening a preview issues a new snapshot. Obtain new anchors after reopening;
the original snapshot and its anchors remain usable after preview. Rust
`inline_at`, Python `inline_at`, and JavaScript `inlineAt` reject foreign inline
anchors. They are not persistent IDs and must not be serialized for later use.

Inline anchors distinguish runs and hyperlinks by physical location, not text,
relationship ID, or entry ID. Aliased section stories for the same physical part
share inline identities; different physical parts do not. Header image
targets instead select a section and variant, even when slots share a header. Duplicate
edits to one target and conflicting shared-header edits are rejected atomically.
This is not a general contract for resolving duplicate source IDs.
Rust and Python compare inline anchors by identity; JavaScript reuses the same
opaque handle for an inline identity in a snapshot. Other anchor types do not
promise cross-binding value equality.

Section body ranges are zero-based, half-open block indices, not editable text
ranges. For text offsets, use a snapshot paragraph anchor plus the scalar range below. Opaque nodes, run
items, rows, cells and nested tables lack independent addresses. Paragraph or table insertion
uses a direct-body paragraph and inserts after it; active or ambiguous supported
fields and ranges prevent insertion there. Single-text direct-body run replacement or removal
also rejects targets inside active or ambiguous supported ranges. These guards
do not imply all OOXML range types are inspected or editable.

## Character ranges in plain body paragraphs

Use Rust `TextRangeEdit { target, start, end, text }` with `preview_with_ranges`,
Python `preview([], text_ranges=[TextRangeEdit(target, start, end, text)])`, or
JavaScript `preview({textRanges: [{target, start, end, text}]})`. `target` is the
original direct-body paragraph anchor. `[start, end)` counts Unicode scalar values
across consecutive direct runs. In `a😀é`, the emoji occupies one position and
`e` plus the combining accent occupy two. Use `Array.from(text)` for scalar
positions in JavaScript; string indexing there counts UTF-16 code units.

The whole paragraph must consist of direct runs with one `w:t` each (plus optional
properties). A field, hyperlink, control, revision, bookmark, tab, break, raw
markup or other inline object makes its offsets unsupported, even for selections
on just one side. A paragraph inside an active or ambiguous field/range is also
ineligible. Offsets never span paragraphs or stories. There are no persistent
range IDs: reopening even the same bytes requires fresh anchors.

Selections must be nonempty and in bounds; empty replacement text deletes.
Replacement text must be single-line XML text without tabs. The first selected
scalar's run receives the replacement. Unselected prefixes and suffixes keep
their formatting; all run containers and their properties survive. Replacing
text with itself preserves the original distribution across runs. The run change
report lists only changed runs, in source order. All ranges address the original
snapshot: adjacent ranges may share a run, but overlapping/duplicate selections
or a whole-run edit/removal on a touched run reject the entire batch.

## Supported imported targets

The editor exposes direct body inspection with section ranges, explicit section
types and ordered paragraph content. Default, first and even headers and footers
expose section references, inherited status, physical part names and ordered
direct paragraphs; other direct children are opaque. Only default and first-page
header slots supply image-edit anchors. The [operation contract](#operation-contract)
below describes the broader goal, not the currently supported surface.

Direct body controls expose source tag, alias and lock: Rust
`BodyBlock::ContentControl`, Python `BodyContentControl`, JavaScript
`kind: "contentControl"`. Missing attributes are absent, empty strings remain
empty and unknown lock values stay literal. There is no edit anchor or traversal
of the contents. Controls in other contexts remain opaque.

Paragraph insertion adds a resolved paragraph immediately after a direct body paragraph.
Rust uses `ParagraphInsertion` with `preview_with_paragraphs`, Python uses
`ParagraphInsertion` with `paragraph_insertions=`, and JavaScript uses
`paragraphInsertions` in the preview batch. Payloads are resolved declarations;
their supported inline content and referenced styles use the table-insertion
guards. An anchor cannot receive both a paragraph and a table insertion in one
batch. Paragraph replacement and removal, and paragraph insertion in other
stories, are unsupported.

Whole-inline edits address a direct child of a direct body paragraph. Insert after
or replace a single text, break or tab run, or a hyperlink containing only such
runs. Payloads use the imported-paragraph codecs for resolved text, breaks, tabs,
fractions and external hyperlinks. New external links allocate relationships on
the actual document part. Replacement refuses to discard unrecognized properties
or markup; unused existing relationships are retained. A direct bold patch on a
supported run sets only `w:rPr/w:b`, preserving other properties (including
unmodeled markup). Duplicate targets, nested targets, tracked or ambiguous
properties and active supported field/range boundaries reject the batch.

| Rust: `preview_with_inlines`, `InlineEdit { target, action }` | Python: `preview([], inline_edits=[…])` | JavaScript: `preview({inlineEdits: […]})` |
| --- | --- | --- |
| `InlineAction::InsertAfter(inline)` | `InlineEdit.insert_after(anchor, inline)` | `{kind: "insertAfter", target, inline}` |
| `InlineAction::Replace(inline)` | `InlineEdit.replace(anchor, inline)` | `{kind: "replace", target, inline}` |
| `InlineAction::SetBold(false)` | `InlineEdit.set_bold(anchor, False)` | `{kind: "setBold", target, bold: false}` |

`inline_changes()` / `inline_changes` / `inlineChanges()` return original
(body, inline) index pairs in document order; an already-matching direct bold
value is omitted. There are no character-range addresses in these operations.

Direct body tables expose a snapshot-scoped table identity and ordered physical
rows, cells, and cell paragraphs with flat and structured inline projections in
Rust, Python and JavaScript. The table anchor plus physical row and cell indices
can target a vertical-alignment patch, but cannot be used as a paragraph edit
target. Rows and cells have no independent edit identities yet. The patch
validates the explicit grid, spans and vertical merge chains; it rejects omitted
grid columns, horizontal merges, tracked cell-property changes and opaque row or
cell wrappers. It edits the selected physical cell, including a merge continuation,
not all cells in a merged region. Other cell properties remain untouched.
Physical cells are not expanded into a logical grid. Unsupported
children, including nested tables and controls, are retained as opaque element
names in their original order; their markup stays in the source package.
Tables in other stories are not inspected by this slice.

In Rust use `CellAlignmentEdit` with `preview_with_cells`; in Python use
`CellAlignmentEdit` with `preview(..., cell_alignments=[...])`; in JavaScript
use `preview({ cellAlignments: [...] })`. Indices refer to the original
snapshot's inspected children, including opaque entries. Duplicate, missing,
foreign-snapshot and structurally ambiguous targets reject the entire batch.

Direct body, header, and footer paragraphs retain run and hyperlink boundaries through
`inline_structure`. Hyperlinks expose their explicit owner-local relationship
ID and bookmark reference, without resolving either. Runs expose ordered text,
tabs, breaks and opaque run children, but not run or hyperlink formatting.
Unsupported inline elements are opaque names (not traversed) and stay in the
package. The existing `text` and `inline_items` projections are unchanged.
Runs (including empty ones) and hyperlinks have snapshot-scoped `InlineAnchor`
values; `ImportedDocument::inline_at` in Rust and
`ImportedDocument.inline_at` in Python reject anchors from another open,
even when both snapshots came from identical bytes. Individual run items and
opaque nodes are not addressable. Direct body run anchors support a single-text
replacement via `RunTextEdit`. Whole-run removal is stricter: the selected
run must have recognized local formatting and no unsafe or unknown markup.
Replacement and removal of the same run conflict. Direct-body whole-inline edits
are described above; inline addresses in other stories and cells remain read-only.

Inline `w:ins` and `w:del` wrappers are also inspectable without applying them:
Rust `ImportedInlineNode::Revision`, Python `ImportedRevision`, and JavaScript
`kind: "revision"` expose insertion/deletion kind, literal optional ID, author
and date, and ordered children. Deleted `w:delText` is represented as text
inside the deletion; unsupported children and nested revisions remain opaque.
Wrapper and child anchors resolve in their original snapshot but cannot be used
for run-text edits. Flat projections continue to show an opaque wrapper;
move revisions and revision editing are outside this slice.

Rust keeps the flat `ImportedCellBlock::Paragraph` payload and adds
`ImportedTableCell::inline_structure(block_index)`. The index is the position in
`blocks`, including opaque blocks. An empty paragraph returns an empty slice;
an opaque or missing block returns `None`. Python exposes
`ImportedCellParagraph.inline_structure` alongside `inline_items`. Inline
anchors distinguish physical tables, rows, cells, and paragraphs, even when
the nodes contain identical text. A cell hyperlink's relationship ID is local
to the main document part.

Use `ImportedDocument::stories()` in Rust or `ImportedDocument.stories` in Python
to read effective story references in section order: headers before footers,
then default, first, and even. Each slot inherits independently across sections.
Slots without a reference are omitted. The view does not simulate page selection
or filter based on first-page or odd/even settings. Footnote, endnote, and comment
stories are not included. Rust uses `SectionStoryKind` and `HeaderFooterType`;
Python exposes `kind` and `variant` as lowercase strings.

`entry_stories()` (Python: `entry_stories`; JavaScript: `entryStories()`) separately inspects footnotes,
endnotes and comments in that order, including separator entries. It follows
main-document relationships; entries preserve source order within each part.
IDs are source strings, not addresses. Run/link anchors distinguish entries
even with identical IDs across part kinds. Tables and other non-paragraph
children remain opaque. Inspecting an ambiguous relationship, wrong root,
wrong content type or missing entry ID fails without changing the snapshot.

[Style resource inspection](EDITOR-RESOURCES.md) covers relationship-selected
part identity and direct style ID, kind, name, and base reference in all three
bindings. Other style properties and resource editing remain outstanding.

Rust's `ImportedStory::inline_structure(block_index)` follows the original block
order (including opaque blocks); `ImportedStoryBlock` re-exports the existing
`ImportedHeaderBlock` type. In Python, `ImportedStoryParagraph` is an alias of
`ImportedHeaderParagraph` and exposes `inline_items` and `inline_structure`.
Story parsing is lazy and deduplicated by physical part within each view.
Aliases of the same part produce equal inline anchors even across the two views,
but distinct parts do not, even when their contents and relationship IDs match.
`default_headers` retains its existing inspection scope and section-scoped
`HeaderAnchor` for image edits; unrelated footer failures don't affect it.
The broader story view also supplies an image-edit anchor for default and
first-page headers. Other story slots have no such anchor. Local edits restore
inheritance only for the selected variant; shared edits include all aliases.

To combine these seven edit kinds in one call, Rust uses
`preview_mixed(alignments, tables, header_images, run_text, paragraphs, cells, run_removals)`.
Python uses `preview(alignments, table_insertions=..., header_images=..., run_text=...,
paragraph_insertions=..., cell_alignments=..., run_removals=...)`; JavaScript uses
`preview({alignments, tableInsertions, headerImages, runText, paragraphInsertions,
cellAlignments, runRemovals})`. Every target comes from the original snapshot.
A shared edit conflicts with any other image edit on that
physical header. Distinct sections may each fork the same original header
with section-local edits. A rejected operation produces no partial preview.

The shared Rust/Python surface is inventoried at field level in [`examples/showcase/coverage.json`](../examples/showcase/coverage.json); the [visual catalog](../examples/ALL_FEATURES.md#inspection-guide-and-coverage-inventory) describes exercised values and boundaries. The table below groups those declarations by their *resolved document targets*, rather than repeating every field. It includes defaults, resources and metadata, not just visible content. The catalog covers supported fields, not every combination, story, open string or raw OOXML feature.

For public entry points and conversion boundaries see the [Rust facade](../src/lib.rs), [Python declarations](../python/rocx/declarative/__init__.py) and [module map](MODULES.md). The [OOXML domain map](OOXML.md#wordprocessingml-domain-map) records current generation limits: for example, no nested hyperlinks or custom XML control binding; imported field edits are currently rejected. Those limits are not expanded merely by listing editor goals here.

## Operation contract

In the table, **inspect** means a typed, snapshot-anchored view of the authored result and relevant resource identities; **add** means insert at a valid parent/position or register a resource; **patch** means change selected modeled properties while preserving unspecified and unmodeled properties on that same target; **replace** means explicitly replace the selected target; **remove** means delete the selected target or property when valid. Cells list intended targets, not existing API names. For scalar properties, add/replace/remove mean set/change/clear; for ordered children they mean insert/replace/delete. Where properties are grouped below, patch and clear apply to individual supported properties as well as the group. Omitted, explicit false and clear must remain distinguishable where the declaration permits them.

All operations are subject to structural validity and reference integrity: removal or replacement cannot leave dangling references outside the selected target. Whole-node replacement may discard opaque content *inside* it, not break external references. Imported unsupported markup must remain opaque and anchored rather than be silently treated as absent. Edits apply to the original package, preserving untouched part payloads byte for byte; edited XML and ZIP serialization may change. Batches must reject stale anchors, conflicts and unsafe edits atomically. Shared-part edits require explicit shared versus section-local intent and preview of effects. Existing signature and compatibility guards remain in force.

| Declarative feature group | Inspect target | Add / patch / replace / remove targets and constraints |
| --- | --- | --- |
| Document and sections: `Document`, `Section`, `SectionProperties`, `LineNumbering`, `PageBorders`, `DocumentGrid` | Document, ordered sections and section properties: size/orientation, margins, columns, break type, page numbering, vertical alignment, title page, bidi/gutter, line numbering, borders and grid. | Insert/replace/remove sections at valid boundaries; patch or clear individual section properties and border edges. Preserve section-boundary semantics and header/footer references; line-number clear differs from omission. |
| Stories: `Header`, `Footer`, `Note`, `Comment` | Body and default/first/even header/footer stories; footnote, endnote and comment definitions and their identities. | Add/replace/remove story definitions and their block content; patch story metadata where modeled. Respect section inheritance, shared part identities and owner-local relationships; shared versus section-local header/footer edits must be explicit. Note kind plus ID and comment/range references must remain valid. |
| Blocks and paragraphs: `Paragraph`, `BlockContentControl` | Ordered blocks in each valid story, table cell and block control; paragraph text/children, heading, style, numbering and direct properties. | Insert/replace/remove blocks at valid anchors; patch/clear paragraph alignment, spacing, line height, indents, tabs, pagination, bidi, grid, contextual spacing and mirrored indents independently. Preserve unrelated paragraph markup on property patches. Respect required trailing cell paragraphs. |
| Text and inline formatting: `TextRun`, `RunLanguage`, `Tab`, `Break` | Ordered inline nodes and text, formatting, script/language slots, style and revision metadata; explicit tab and break variants. | Insert/replace/remove inline nodes or text; patch/clear individual run properties without discarding unrelated markup. Preserve whitespace and distinguish literal tab/newline from explicit nodes. |
| Tables: `Table`, `TableRow`, `TableCell`, `TableLook`, borders, margins, `RowHeight` | Table grid, layout, alignment, style, look and borders; rows, cells, spans/merges, shading, margins, alignment, direction and nested blocks. | Insert/replace/remove tables, rows, cells and nested blocks where valid; patch/clear their individual properties and border sides. Maintain valid grids, merges, cell contents and required trailing paragraphs; row/cell control wrappers are separate targets. |
| Styles: paragraph, character, table and numbering style definitions; `TableSectionFormat` | Named style definitions, kind, base/next/linked references, direct properties and 13 table regions. | Add named styles, patch/clear selected fields or regions, replace/remove definitions subject to references. Preserve unspecified properties, including on template-origin styles; do not claim computed effective formatting or viewer conditional precedence. |
| Numbering: `NumberingDefinition`, `NumberingLevel`, overrides, `NumberingReference`, numbering styles | Abstract and concrete identities, levels, style links, starts and level overrides, and paragraph references. | Add/replace/remove definitions, instances, levels and overrides where valid; patch/clear their fields and paragraph references. Preserve shared abstracts and other consumers; validate IDs and links. Labels and counters are viewer-computed, not stored evaluated text. |
| Defaults: `DocumentDefaults`, `RunDefaults`, `ParagraphDefaults`, `NoteNumbering` | Run/paragraph defaults and document-wide note numbering/placement, mirror margins, top gutter and revision tracking. | Add/set, patch, replace or clear individual default properties independently of direct formatting. Preserve unspecified values and distinctions between omitted and explicit false. |
| Metadata: document core, extended and custom properties | Creator, title, subject, description, keywords, category, comments, version, timestamps, company, manager, comment author and typed custom properties. | Add/set, patch, replace or remove supported properties by name/type; retain unrelated properties and shared package metadata. Snapshot/render timestamps are inputs, not necessarily reconstructible authoring intent. |
| Links and ranges: `Hyperlink`, `BookmarkTarget`, `RangeStart`, `RangeEnd`, `Comment` | Link targets, children, tooltip/history; bookmark and comment ranges, including cross-paragraph/overlapping endpoints and comment story/metadata. | Insert/replace/remove links and paired ranges, patch link/comment properties and content. Keep endpoints and references consistent; external links use owner-local relationships and are not fetched. Removing a range must not leave orphaned endpoints or comment references. |
| Notes: `NoteReference`, `Note`, `NoteNumbering` | Reference kind/ID, corresponding rich note story and document-wide settings. | Add/replace/remove references and definitions, patch/clear settings and note content; preserve valid references and infrastructure entries. Footnote and endnote IDs have separate namespaces. |
| Fields: `Field`, `FieldInstruction` | Simple/complex instructions, cached children, dirty/locked flags and nesting. | Insert/replace/remove fields and cached content; patch/clear instruction and flags where valid. Preserve structural pairing; cached values are not evaluated by rocx. |
| Controls and revisions: inline/block controls, row/cell `ContentControlMetadata`, list items, `TextRevision` | Control wrapper, alias/tag/lock/ID, plain/rich/drop-down settings and selection; insertion/deletion and paired move metadata. | Insert/replace/remove controls at their supported inline/block/row/cell locations; patch/clear metadata and choices subject to validity. Insert/replace/remove revisions while preserving revision and move pairing and reference integrity. Existing template acceptance and tag-targeted cached text editing are narrower operations, not proof of general editor parity. |
| Images: `ImageRun`, crop, transformation, alt text, `ImageAnchor` | Image occurrence, asset/relationship identity, size, crop, rotation/flips, accessibility metadata and inline/floating placement and wrapping. | Insert/replace/remove occurrences; patch/clear modeled occurrence properties; add or replace image assets without inadvertently changing other users of shared bytes. Maintain owner-local relationships, asset references and valid image formats; type hint is not conversion. |
| Drawings, charts and math: `ShapeRun`, `ShapeTextFormat`, `ShapeOutline`, `ColumnChartRun`, `InlineFraction` | Supported inline/floating shape and plain text properties; workbook-free single-series column chart; inline stacked fraction. | Insert/replace/remove each supported inline object; patch/clear its modeled properties where valid. Preserve relationships and unsupported drawing markup on patches. Do not infer rich text-box stories, chart workbooks or general math editing from these declarations. |

## Inputs versus imported content

Conditions, components, property references, lazy/async/deferred values, reuse and template instructions (`DocumentTemplate`, replacements, insertion index, scope, content-control replacements and revision action) describe how content is produced or an existing template is processed. They are not necessarily recoverable from a DOCX. Inspect the resolved result, not invented callbacks, conditions or inputs; author new content using existing declarations. Package profile/read budgets, path/bytes saving and Rust package-plan helpers are operational APIs rather than editable document nodes.

## Acceptance cross-check

Milestone 2 uses fixtures generated independently with python-docx to prove body-anchor table insertion, a paragraph-property patch preserving unrelated markup, and image insertion into a header shared by two sections in both shared and section-local modes. Milestone 3 requires typed readback of the all-features DOCX against expected content, opaque-markup and unchanged-part preservation, unsafe-batch rejection, updated examples and coverage checks, and exact Rust/Python/JavaScript DOCX byte parity for equivalent edit batches. XML normalization is not a substitute for byte parity. See the [active plan](PLAN.md) for verification status.

## Body block edits

Use an original direct body paragraph anchor for these operations:

| Operation | Rust `BlockEdit` | Python `BlockEdit` | JavaScript `blockEdits` item |
|---|---|---|---|
| Replace one paragraph | `Replace(anchor, Box::new(p))` | `replace(anchor, p)` | `{kind: "replace", target, paragraph}` |
| Remove one paragraph | `Remove(anchor)` | `remove(anchor)` | `{kind: "remove", target}` |
| Replace section content | `ReplaceSection(anchor, blocks)` | `replace_section(anchor, blocks)` | `{kind: "replaceSection", target, blocks}` |
| Set direct keep-with-next | `KeepWithNext(anchor, bool)` | `keep_with_next(anchor, value)` | `{kind: "keepWithNext", target, value}` |

Section payloads can mix resolved paragraphs and tables, subject to the same
style, inline and geometry restrictions as insertion. Automatic table widths
use the owning section. A section target must be its first body block,
and that block must be a paragraph. A nonfinal section retains its original
break paragraph, emptied of supported runs, **after** the declarations;
terminal section properties stay in the body. An empty declaration list clears
supported content. Source tables, controls, fields, relationships, anchors,
unrecognized attributes and other opaque markup cannot be discarded. Section
header/footer references are validated; the original owning properties remain
in place, preserving inheritance. Direct replacement/removal of a break
paragraph is refused. Keep-with-next patches the single direct `w:keepNext`
property, retaining other properties, attributes, and inline content.

Use Rust `preview_blocks` or `preview_with_blocks` for a mixed batch, Python
`preview([], block_edits=[...])` or JavaScript `preview({blockEdits: [...]})`.
Structural block operations reserve their entire original range against other body edits.
Keep-with-next can accompany an alignment patch on the same paragraph.
Preview `block_changes()` / `.block_changes` / `.blockChanges()` reports the
original start index per operation (unchanged direct keep-with-next omitted).
