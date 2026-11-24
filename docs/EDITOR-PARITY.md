# Editor declarative parity inventory

## Existing imported addresses

Anchors belong to one open snapshot, not the DOCX file: opening identical bytes
or reopening a preview issues a new snapshot. Obtain new anchors after reopening;
the original snapshot and its anchors remain usable after preview. Rust
`inline_at`, Python `inline_at`, and JavaScript `inlineAt` reject foreign inline
anchors. They are not persistent IDs and must not be serialized for later use.

Inline anchors distinguish runs and hyperlinks by physical location, not text,
relationship ID, or entry ID. Aliased section stories for the same physical part
share inline identities; different physical parts do not. Header/footer image
targets select a section, kind and variant, even when slots share a part. Duplicate
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

For point insertion, use Rust `EditBatch::text_insertions` with
`TextInsertion { target, position, text }`, Python
`preview([], text_insertions=[TextInsertion(target, position, text)])`, or
JavaScript `preview({textInsertions: [{target, position, text}]})`.
`position` uses the same original scalar coordinates. Inserted text must be
nonempty single-line XML text without tabs. At a run boundary, the next scalar's
run supplies the formatting; at paragraph end the final run supplies it. A
runless paragraph accepts position zero and gets one unformatted run. The
ordinary run-text report records its previous text as `""`. Points at either
endpoint of a replacement are allowed and stay on that side of the replacement.
Two points at one position, a point strictly inside a selection, or a conflicting
whole-run or destructive block edit reject the batch.

## Supported imported targets

The editor exposes direct body inspection with section ranges, explicit section
types and ordered paragraph content. Default, first and even headers and footers
expose section references, inherited status, physical part names and ordered
direct paragraphs; other direct children are opaque. All six kinds of section slots supply
image-edit anchors when an effective reference exists. The [operation contract](#operation-contract)
below describes the broader goal, not the currently supported surface.

Direct body controls expose source tag, alias and lock: Rust
`BodyBlock::ContentControl`, Python `BodyContentControl`, JavaScript
`kind: "contentControl"`. Missing attributes are absent, empty strings remain
empty and unknown lock values stay literal.

For physical control targets, use Rust `ImportedDocument::controls()`, Python
`ImportedDocument.controls`, or JavaScript `ImportedDocument.controls()`. This
ordered main-body view distinguishes `block`, `inline`, `row` and `cell`.
It reports literal `id`, `tag`, `alias`, `lock`, a `bound` flag, and an opaque
snapshot anchor. Neither `id` nor `tag` needs to be unique. The existing body,
table and inline projections keep their original shape.

Use Rust `ControlEdit` with `preview_with_controls`, Python `ControlEdit` with
`preview([], controls=[...])`, or JavaScript `preview({controls: [...]})`.
An edit sets the supplied tag, alias, or single-line text; omitted fields stay
unchanged. The wrapper must contain a single run/text leaf, with the ordinary
paragraph, row and cell containers for its context. Text edits preserve run
formatting. Metadata edits preserve unrecognized extension properties; a text
edit refuses unknown control properties. Content locks prohibit text edits,
control locks prohibit metadata edits, and combined or unknown locks prohibit
both. Bound controls, nested controls, protected ranges, ambiguous wrappers,
and specialized controls (e.g. lists) are not editable. This does not insert
or remove wrappers or edit controls in headers, footers or entry stories.

Paragraph insertion adds a resolved paragraph immediately after a direct body paragraph.
Rust uses `ParagraphInsertion` with `preview_with_paragraphs`, Python uses
`ParagraphInsertion` with `paragraph_insertions=`, and JavaScript uses
`paragraphInsertions` in the preview batch. Payloads are resolved declarations;
their supported inline content and referenced styles use the table-insertion
guards. An anchor cannot receive both a paragraph and a table insertion in one
batch. For guarded paragraph replacement/removal, see [body block edits](#body-block-edits).
Paragraph insertion in other stories is unsupported.

Whole-inline edits address a direct child of a direct body paragraph. Insert after
or replace a single text, break or tab run, or a hyperlink containing only such
runs. Payloads use the imported-paragraph codecs for resolved text, breaks, tabs,
fractions and external hyperlinks. New external links allocate relationships on
the actual document part. Replacement refuses to discard unrecognized properties
or markup; unused existing relationships are retained. Property-only patches also
accept runs containing multiple ordinary text, tab, and break leaves. They preserve
the leaves and unrelated run properties. Overlapping edits, nested targets, tracked or ambiguous
properties and active supported field/range boundaries reject the batch.

| Rust: `preview_with_inlines`, `InlineEdit { target, action }` | Python: `preview([], inline_edits=[…])` | JavaScript: `preview({inlineEdits: […]})` |
| --- | --- | --- |
| `InlineAction::InsertAfter(inline)` | `InlineEdit.insert_after(anchor, inline)` | `{kind: "insertAfter", target, inline}` |
| `InlineAction::Replace(inline)` | `InlineEdit.replace(anchor, inline)` | `{kind: "replace", target, inline}` |
| `InlineAction::SetBold(false)` | `InlineEdit.set_bold(anchor, False)` | `{kind: "setBold", target, bold: false}` |
| `InlineAction::SetToggle(RunToggle::Italic, None)` | `InlineEdit.set_toggle(anchor, "italic", None)` | `{kind: "setToggle", target, property: "italic", value: null}` |
| `InlineAction::PatchFormat { set: Box::new(RunFormat::default().with_size(12.5)), clear: vec![RunFormatField::FontEastAsia] }` | `InlineEdit.patch_format(anchor, size=12.5, clear=["font_east_asia"])` | `{kind: "patchFormat", target, format: {size: 12.5}, clear: ["fontEastAsia"]}` |

Use `Bold`, `Italic`, `Strike`, or `Hidden` in Rust and `bold`, `italic`,
`strike`, or `hidden` in Python and JavaScript. `true` and `false` set
explicit direct values; `None`/`null` removes the property so style inheritance
applies. A clear refuses to discard attributes or children outside the modeled
value. Disjoint format edits on one run compose in any order and report one
changed site. A repeated property (even twice with the same value), or mixing
a format patch with text replacement/removal or a whole-inline edit on that
run, rejects the batch.

Read `ImportedRun.format` in all three bindings: a Rust `RunFormat`, a fresh
Python dictionary of present keyword values, or a JavaScript `ResolvedRunFormat`.
`PatchFormat` accepts the modeled run-format fields: character style, literal
Latin/East Asian/complex-script fonts, point sizes, supported underline styles,
RGB/auto/theme color (including tint and shade), superscript/subscript, bold,
italic, strike, capitals, small capitals, snap-to-grid, script-specific bold and
italic, right-to-left, complex-script selection and language hints. Character
style input is an existing character-style ID or unambiguous display name in the
staged catalog; inspection returns the ID. Sizes round to half-points.

Omitted values retain the original property. To restore inheritance, use the
corresponding `RunFormatField` variant in Rust, snake-case name in Python, or
camel-case name in JavaScript. `VerticalAlignment` / `vertical_alignment` /
`verticalAlignment` clears the *one* slot shared by superscript and subscript;
requesting either as false without a true counterpart writes explicit baseline.
Language subfields have independent clear slots: `LanguageLatin`, `LanguageEastAsia`,
`LanguageBidi` (Python: `language_latin`, `language_east_asia`, `language_bidi`;
JavaScript: `languageLatin`, `languageEastAsia`, `languageBidi`).
Latin font owns the `ascii`/`hAnsi` pair. Font and language patches preserve
sibling attributes on their shared elements. Updating an underline preserves its
color; clearing that underline refuses to discard the color. A direct color edit
owns both the fallback and theme attributes. A semantic no-op preserves the
existing spelling and fallback. Unsupported direct selections, including
themed fonts and unequal or incomplete Latin font pairs, are opaque and cannot
be patched in the affected slot.

`ImportedRun.bold`, `.italic`, `.strike`, and `.hidden` inspect the same direct
properties wherever structured runs are exposed. An unset or ambiguous toggle
is `None` in Rust/Python and absent in JavaScript. `has_opaque_formatting`
(`hasOpaqueFormatting` in JavaScript) signals retained properties or markup
outside this projection. These values do not resolve styles, complex-script
variants, or effective visual formatting.

`inline_changes()` / `inline_changes` / `inlineChanges()` return original
(body, inline) index pairs in document order; an already-matching direct toggle
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

Read `layout` in all three APIs for the recognized direct `fixed` or `autofit`
setting (a `TableLayout` in Rust). `grid_widths` (Rust/Python) or `gridWidths`
(JavaScript) lists direct grid columns in source order, in twips; a missing
or unparseable unsigned width gives `None`/`null` at that position. On each
physical cell, `grid_span`/`gridSpan` exposes a direct positive span, and
`vertical_merge`/`verticalMerge` exposes the marker value (`continue` for an
empty `w:vMerge`). Absent properties stay absent. These are source properties,
not effective formatting or a computed viewer grid.

In Rust use `CellAlignmentEdit` with `preview_with_cells`; in Python use
`CellAlignmentEdit` with `preview(..., cell_alignments=[...])`; in JavaScript
use `preview({ cellAlignments: [...] })`. Indices refer to the original
snapshot's inspected children, including opaque entries. Duplicate, missing,
foreign-snapshot and structurally ambiguous targets reject the entire batch.

### Structural edits to a direct body table

Use the original table anchor with Rust `TableStructureEdit { target, operation }`
(the last slice of `preview_with_table_structure`), Python
`preview([], table_structure=[TableStructureEdit.insert_row(target, index)])`, or
JavaScript `preview({tableStructure: [{target, kind: "insertRow", index}]})`.

| Rust `TableOperation` | Python `TableStructureEdit` factory | JavaScript `kind` | Coordinate |
| --- | --- | --- | --- |
| `InsertRow` / `RemoveRow` | `insert_row` / `remove_row` | `insertRow` / `removeRow` | `index`: physical row |
| `InsertCell` / `RemoveCell` | `insert_cell` / `remove_cell` | `insertCell` / `removeCell` | `row`, `cell`: physical row and cell |
| `InsertColumn` / `RemoveColumn` | `insert_column` / `remove_column` | `insertColumn` / `removeColumn` | `index`: logical grid column, across all rows |
| `ReplaceCell` | `replace_cell` | `replaceCell` | `row`, `cell`, `blocks`: physical target and ordered resolved paragraphs/tables |
| `Layout(Some(TableLayout::Autofit))` | `layout(target, "autofit")` | `layout` | Whole table; JS `value: "autofit"`. Also accepts `fixed`; pass `None`/`None`/`null` to clear. |

The layout patch only changes `w:tblLayout`; existing cell and grid widths
are retained. It uses the same range and grid guards as structural edits and
requires exclusive use of the table within the table edit batch. Duplicate
layout elements, tracked table properties, and clears that would discard
extension attributes or children on the property are rejected.

Indices are zero-based; insert before an index or append at the count. A cell
spanning three grid columns still counts as **one physical cell**. A new row
contains blank cells except where it must continue an existing vertical merge;
those continuations inherit the exact logical interval, including horizontal span.
A row-local cell insertion shifts the row's suffix right, consuming one unused
trailing grid slot or appending a grid column and increasing `gridAfter` on
other rows. Row-local deletion shifts the suffix left and adds a trailing gap;
it does not delete a grid column. Whole-column edits require full rows and a
common cell boundary. They change `tblGrid` and all physical rows together.

Content replacement keeps `tcPr` and accepts resolved paragraphs and nested tables
using the existing insertion guards, including external hyperlink support. Supply
explicit nested table widths if the parent cell has no absolute `dxa` width.
A trailing table gets a required empty paragraph after it. An empty list clears
the contents to a required empty paragraph. Replace the origin of a merged cell,
not its continuation. Destructive edits only discard plain paragraphs/runs and
basic geometry; foreign markup, formatting outside the whitelist, revisions,
fields, protected ranges, and merges that would be split or vertically misaligned cause
rejection. Structural edits reject a target table containing spelling or grammar
range markers, including ranges that span cells. Nested tables are preserved outside a destructive target. Removing
the final row or final physical cell of a row is unsupported. A geometry edit
needs exclusive access to its table for the batch; distinct content replacements
can be batched on the same table. Other original body targets can be edited in
the same transaction. Reopen the result for coordinates in the new geometry.

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
are described above. Cell run anchors are inspection-only; selected runs in
other stories support the physical-owner operation below.

Inline `w:ins` and `w:del` wrappers are also inspectable without applying them:
Rust `ImportedInlineNode::Revision`, Python `ImportedRevision`, and JavaScript
`kind: "revision"` expose insertion/deletion kind, literal optional ID, author
and date, and ordered children. Deleted `w:delText` is represented as text
inside the deletion; unsupported children and nested revisions remain opaque.
Wrapper and child anchors resolve in their original snapshot but cannot be used
for run-text edits. Flat projections continue to show an opaque wrapper;
named move legs expose `moveSource` / `moveDestination` when the wrapper has an
author. Direct body single-text runs support [explicit tracked text and move
previews](REVISIONS.md#review-a-single-change-in-an-existing-docx), and supported
wrapper anchors allow individual accept/reject (both anchors for a move).
Child anchors and revisions in other locations remain inspection-only.

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
part identity, direct style ID, kind, name, base/next/linked references, and property groups in all three
bindings. See [imported style edits](#imported-style-edits) for definition
and property edits. The resource guide also covers direct [core title, default alignment
and abstract level start](EDITOR-RESOURCES.md#direct-editable-properties).
For definition and instance creation/removal, concrete overrides and body
paragraph list assignments, see [numbering operations](EDITOR-RESOURCES.md#numbering-definitions-instances-and-paragraph-references).
Effective formatting and arbitrary resource editing remain outstanding.
The [defaults field inventory](EDITOR-RESOURCES.md#editing-document-defaults)
covers direct run/paragraph properties, tabs, footnote/endnote settings, page
switches and revision tracking with independent set/clear semantics.

For timestamps, other core fields, extended company/manager and named string
or boolean custom properties, see [package metadata](EDITOR-RESOURCES.md#package-metadata).

Rust's `ImportedStory::inline_structure(block_index)` follows the original block
order (including opaque blocks); `ImportedStoryBlock` re-exports the existing
`ImportedHeaderBlock` type. In Python, `ImportedStoryParagraph` is an alias of
`ImportedHeaderParagraph` and exposes `inline_items` and `inline_structure`.
Story parsing is lazy and deduplicated by physical part within each view.
Aliases of the same part produce equal inline anchors even across the two views,
but distinct parts do not, even when their contents and relationship IDs match.
`default_headers` retains its existing inspection scope and section-scoped
`HeaderAnchor` for image edits; unrelated footer failures don't affect it.
Every effective header/footer slot supplies an image-edit anchor. Local edits fork
only the selected kind and variant and restore downstream inheritance. Shared
edits include all aliases of the physical part.

For text edits use `edit_story_text(owner_part, edits)` in Rust and Python, or
`editStoryText({ownerPart, runText})` in JavaScript. Pass the exact `part_name` /
`partName` from inspection and snapshot `RunTextEdit` targets. The result is DOCX
bytes. This separate atomic batch updates direct, single-text paragraph runs in
one physical header, footer, or note/comment part. Inherited slots see the same
text. References, range markers, note IDs and comment metadata stay intact.
Nested, mixed-content, reserved note and mismatched-owner targets are rejected.

`edit_entry_inlines(owner_part, edits)` (Rust/Python) and
`editEntryInlines({ownerPart, inlineEdits})` (JavaScript) return bytes for an
atomic batch of `InlineEdit` operations in one footnotes, endnotes or comments
part. Insert after or replace an eligible direct run or hyperlink, or patch
direct run bold. Use original inline anchors from ordinary entries with unique
decimal IDs. Replacement uses the body editor's discard checks; note/reference
marks, field/range interiors, nested targets and reserved notes are ineligible.
External links allocate relationships in the entry part, including when its
path is nonstandard. Existing relationships are retained. No entry or reference
is created, deleted or renumbered. Reopen the returned package for new anchors.

### Note references and entry definitions

`doc.edit_note_reference(owner, &target, NoteReferenceEdit::…)` (Rust),
`doc.edit_note_reference(owner, target, kind, **fields)` (Python) and
`doc.editNoteReference({ownerPart, target, kind, ...fields})` (JavaScript)
return a separate atomic DOCX copy. A simple reference item exposes its
source ID as a string: Rust `ImportedInline::NoteReference {kind,id}`,
Python `kind` (`footnoteReference`/`endnoteReference`) and `value`, JavaScript
`kind` and `id`. The anchor belongs to the enclosing run.

| Rust variant | Python/JS `kind` and fields | Effect |
| --- | --- | --- |
| `Create {id,text}` | `"create"`, `id`, `text` | After a supported direct body run, insert a reference and add its one-paragraph definition. |
| `Attach(id)` | `"attach"`, `id` | Insert a reference after the target run to an existing unused ordinary definition. |
| `Detach` | `"detach"` | Remove the reference run, retaining its definition. |
| `Retarget(id)` | `"retarget"`, `id` | Change the reference to an existing unused ordinary definition of the same kind. Both definitions remain. |
| `Renumber(id)` | `"renumber"`, `id` | Change a definition and its sole reference together to a free ID. |
| `Delete` | `"delete"` | Remove the reference and its sole discardable definition. |

For `Create` and `Attach`, the anchor must identify a supported direct run in
a body paragraph, outside fields and ranges. For the remaining operations it
must identify the simple reference run itself. The owner is the physical
footnotes or endnotes part. A first `Create` can initialize `word/footnotes.xml`
or `word/endnotes.xml` when that kind has no relationship, the paths are free
and there are no existing note references, definitions, or special-entry
settings of that kind. The new part, content type and relationship are created
together; automatic Word numbering is independent of the supplied ID.
New IDs are unsigned 32-bit values. Imported IDs are compared as arbitrary-
precision decimal integers, separately by kind. Allocation and destruction
check all declared XML payloads, including settings and opaque descendants.
Ambiguous/shared references, custom marks, reserved entries, protected ranges,
and deletion of nodes with unmodeled content are rejected.

`doc.edit_entry(owner, id, EntryEdit::…)` (Rust),
`doc.edit_entry(owner, id, kind, **fields)` (Python), and
`doc.editEntry({ownerPart, id, kind, ...fields})` (JavaScript) address an
ordinary definition with its physical owner and decimal **string** ID.

| Rust variant | Python/JS `kind` and fields | Effect |
| --- | --- | --- |
| `DeleteUnusedNote` | `"deleteUnusedNote"` | Remove a discardable ordinary definition only if it has no reference anywhere in the declared XML graph. |
| `InsertParagraph {index,text}` / `ReplaceParagraph {index,text}` | `"insertParagraph"` / `"replaceParagraph"`, `index`, `value` | Insert before the original physical block (count to append), or replace a discardable paragraph, with single-line text. |
| `RemoveParagraph(index)` | `"removeParagraph"`, `index` | Remove a simple paragraph while retaining an entry block. |
| `CommentAuthor(s)` | `"author"`, `value` | Set a base comment's author. |
| `CommentInitials(Some(s))` / `CommentInitials(None)` | `"initials"`, `value` / `"clearInitials"` | Set or clear initials. |
| `CommentDate(Some(s))` / `CommentDate(None)` | `"date"`, `value` / `"clearDate"` | Set a validated `YYYY-MM-DDTHH:MM:SSZ` timestamp or clear it. |

Block indices include opaque blocks. The first note paragraph cannot be moved
or removed; eligible replacement preserves its reference-mark run. The same
guard applies to the standard first comment annotation mark. Tables and rich
or opaque paragraphs cannot be discarded. Extended/threaded comment metadata
is unsupported. These APIs do not create or delete comment ranges, their
main-story reference runs or comment definitions. A range operation and a
definition edit compose by reopening between calls; see the
[fields/stories ownership contract](https://github.com/ReinderVosDeWael/rocx/issues/3#issuecomment-7145560023).

### Combine ordinary edits

Rust accepts `doc.preview_edits(EditBatch { /* slices */, ..Default::default() })`;
existing `preview_*` methods remain available. In Python use `doc.preview(alignments, ...)`;
in JavaScript use `doc.preview({...})`. The corresponding batch fields are:

| Rust slice | Python argument | JavaScript field |
| --- | --- | --- |
| `alignments` | `edits` (first argument; pass `[]` for none) | `alignments` |
| `tables` | `table_insertions` | `tableInsertions` |
| `header_images` | `header_images` | `headerImages` |
| `run_text` | `run_text` | `runText` |
| `paragraphs` | `paragraph_insertions` | `paragraphInsertions` |
| `cells` | `cell_alignments` | `cellAlignments` |
| `run_removals` | `run_removals` | `runRemovals` |
| `ranges` | `text_ranges` | `textRanges` |
| `text_insertions` | `text_insertions` | `textInsertions` |
| `blocks` | `block_edits` | `blockEdits` |
| `inline_edits` | `inline_edits` | `inlineEdits` |
| `structural` | `table_structure` | `tableStructure` |
| `styles` | `style_edits` | `styleEdits` |
| `resources` | `resources` | `resources` |
| `controls` | `controls` | `controls` |

Every target comes from the original snapshot. New content does not supply
anchors until you reopen the result. Physical-owner story text edits and
tracked/reviewed revisions have their own calls.
A shared edit conflicts with any other image edit on that
physical part. Distinct sections may each fork the same original story
with section-local edits. A rejected operation produces no partial preview.

For example:

- Two disjoint scalar selections may share a run. A whole-inline action
  (including insert-after or set-bold), run replacement or removal may not
  target a run touched by a selection, even if the selected text is unchanged.
- Paragraph replacement/removal reserves that entire original paragraph.
  Section replacement also reserves the insertion slot immediately after
  the preceding section break. Keep-with-next and direct alignment can
  share a paragraph because they patch different properties.
- A geometry change conflicts with another structural operation or a cell
  alignment in the same table. Distinct cell content replacements may
  share a table: replacing cell content preserves its cell properties.
- A style edit and a default-alignment edit can share the styles part:
  changing a definition and changing the document default are distinct
  targets. New definitions are available to inserted content. A style
  removal/replacement must be safe in both the original and staged content:
  removing the last user in this batch does not free a previously used
  style, and introducing a new user of the target also rejects the batch.

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
| Numbering: `NumberingDefinition`, `NumberingLevel`, overrides, `NumberingReference`, numbering styles | Physical abstract level starts, concrete IDs and override indices/starts, and direct body, cell and story paragraph references. Full typed level bodies and abstract style links are not yet exposed. | [Add/replace/remove definitions, instances, levels and overrides](EDITOR-RESOURCES.md#numbering-definitions-instances-and-paragraph-references) where valid; patch/clear direct starts and paragraph references. Retain level addresses of surviving instances and reject opaque destructive edits or style-linked abstract replacements/relinks. Labels and counters are viewer-computed. |
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
| Patch one paragraph flag | `ParagraphFlag(anchor, flag, value)` | `paragraph_flag(anchor, flag, value)` | `{kind: "paragraphFlag", target, flag, value}` |
| Patch one section flag | `SectionFlag(anchor, flag, value)` | `section_flag(anchor, flag, value)` | `{kind: "sectionFlag", target, flag, value}` |
| Patch section break type | `SectionType(anchor, value)` | `section_type(anchor, value)` | `{kind: "sectionType", target, value}` |

For these last three operations, pass `None` / `None` / `null` to clear the
direct property. Explicit false and an absent flag are distinct. Read direct
flags from the paragraph or section's `flags` map. It contains recognized
unambiguous on/off values only; it does not calculate effective formatting.
Read the break type from `section_type` / `section_type` / `sectionType`.
The section target is its first direct body paragraph, including when the
properties are owned by a later break paragraph or by the terminal body.

| Rust flag (`ParagraphFlag` unless indicated) | Python flag key | JavaScript flag key |
|---|---|---|
| `KeepWithNext` | `keep_with_next` | `keepWithNext` |
| `KeepTogether` | `keep_together` | `keepTogether` |
| `PageBreakBefore` | `page_break_before` | `pageBreakBefore` |
| `WidowControl` | `widow_control` | `widowControl` |
| `SuppressLineNumbers` | `suppress_line_numbers` | `suppressLineNumbers` |
| `SnapToGrid` | `snap_to_grid` | `snapToGrid` |
| `ContextualSpacing` | `contextual_spacing` | `contextualSpacing` |
| `MirrorIndents` | `mirror_indents` | `mirrorIndents` |
| `Bidi` (paragraph or section) | `bidi` | `bidi` |
| `SectionFlag::TitlePage` | `title_page` | `titlePage` |
| `SectionFlag::RtlGutter` | `rtl_gutter` | `rtlGutter` |

Setting a flag preserves attributes besides its value. Clearing a flag or type
refuses to discard extension attributes, children or comments. A duplicate
request for the same physical property (including the keep-with-next alias),
an ambiguous property owner or a protected target rejects the batch.

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
original start index per changed operation, sorted in body order. Distinct
property patches can therefore report the same index; no-op patches are omitted.

## Imported style edits

| Operation | Rust | Python (`rocx.editor`) | JavaScript |
| --- | --- | --- | --- |
| Address an exact style ID | `doc.style_anchor(id)?` | `doc.style_anchor(id)` | `doc.styleAnchor(id)` |
| Add a declaration | `StyleEdit::Add(definition)` | `StyleEdit.add(definition)` | `{op: "add", definition}` |
| Patch just bold / italic | `StyleEdit::Patch {target, bold, italic}` | `StyleEdit.patch(target, italic=True)` | `{op: "patch", target, italic: true}` |
| Set or clear a direct scalar | `StyleEdit::Field {target, field, value, region}` | `StyleEdit.field(target, field, value, region=region)` | `{op: "field", target, field, value, region}` |
| Set or clear an entire physical conditional region | `StyleEdit::Region {target, region, format}` | `StyleEdit.region(target, region, format)` | `{op: "region", target, region, format}` |
| Replace an unused definition | `StyleEdit::Replace {target, definition}` | `StyleEdit.replace(target, definition)` | `{op: "replace", target, definition}` |
| Remove an unused definition | `StyleEdit::Remove(target)` | `StyleEdit.remove(target)` | `{op: "remove", target}` |
| Migrate stored references and remove source | `StyleEdit::Retarget {target, replacement}` | `StyleEdit.retarget(target, replacement)` | `{op: "retarget", target, replacement}` |
| Stage with content edits | `doc.preview_edits(EditBatch { styles, ..Default::default() })?` | `doc.preview([], style_edits=styles)` | `doc.preview({styleEdits: styles})` |

Add paragraph, character, table or numbering declarations to the relationship-selected
styles part. If its relationship is absent, an add-only batch creates a
minimal `w:styles` part beside the actual main document, registers its content
type, and adds the relationship. An orphan at `styles.xml` (even with a
different name case) is retained; the new part receives an available numbered
name. No default style definitions are invented, so bases must exist in the
new batch. Names, source aliases, and generated IDs must be unambiguous
across styles under ASCII case-insensitive comparison. The authoring codec
resolves forward references and checks their types and inheritance cycles.
Patches address direct paragraph/character run properties, including explicit
false; omitted properties, unknown siblings and attributes are preserved.

`Field` uses the same OOXML local names and string values in all three bindings:
`basedOn`, `next`, `link` take an exact style ID; `jc` takes supported
paragraph justification tokens (e.g. `center`, `both`). The run flags
`b`, `i`, `bCs`, `iCs`, `rtl`, `cs`, `strike`, `caps`, `smallCaps`, `vanish`
and paragraph flags `keepNext`, `keepLines`, `pageBreakBefore`, `widowControl`,
`bidi`, `contextualSpacing`, `mirrorIndents`, `snapToGrid` take OOXML on/off
strings. `None`/`null` removes the field; clearing an annotated leaf is refused.
A character style cannot carry paragraph fields. Multiple distinct `Field`
edits may address the same style, in list order. A duplicate field/region
or mixing a field edit with a whole-definition operation on that ID is rejected.

Composite fields use `element.attribute` in all three bindings, with OOXML
wire units and exact physical slots. For example, `spacing.before="120"` sets
six points without disturbing `spacing.after`, `lineRule`, or extension
attributes. `None`/`null` removes only that attribute. Required `val` fields
cannot be discarded if doing so would lose annotations or sibling properties.
These edits do not clear theme or character-unit settings implicitly.

| Group | Fields |
| --- | --- |
| Run | `rFonts.{ascii,hAnsi,eastAsia,cs,asciiTheme,hAnsiTheme,eastAsiaTheme,cstheme}`, `lang.{val,eastAsia,bidi}`, `sz.val`, `szCs.val` (half-points), `vertAlign.val`, `runSnapToGrid.val` |
| Color/underline | `color.{val,themeColor,themeTint,themeShade}`, `u.{val,color,themeColor,themeTint,themeShade}` |
| Paragraph | `spacing.{before,after,line,lineRule,beforeLines,afterLines,beforeAutospacing,afterAutospacing}`, `ind.{left,right,start,end,firstLine,hanging,leftChars,rightChars,startChars,endChars,firstLineChars,hangingChars}`, `suppressLineNumbers` |
| Tab stop | `tab.<signed-twips>.val` (alignment) or `.leader`; clearing `.val` removes that single physical stop, rejecting opaque discarded markup |
| Table cell shading | `shd.{val,color,fill,themeColor,themeTint,themeShade,themeFill,themeFillTint,themeFillShade}` |

Where the schema requires a companion value, creating a color, shading, or
tab with an optional attribute supplies `auto`, `clear`, or `left`, respectively.
The same formatting selectors work in conditional regions.

Omitting the region addresses the style's direct properties, including the
base formatting of a table style. A region name addresses an actual
`w:tblStylePr` wrapper: `wholeTable`, `firstRow`, `lastRow`, `firstCol`,
`lastCol`, `band1Horz`, `band2Horz`, `band1Vert`, `band2Vert`, `nwCell`,
`neCell`, `swCell`, or `seCell`. `Region` takes a `TableSectionFormat` (boxed
in Rust) or `None`/`null` to clear the wrapper. Replacement discards **all**
formatting inside that wrapper; use `Field` to retain its siblings. Duplicate
wrappers or opaque markup that would be discarded reject the operation.

Adding `NumberingStyleDefinition` requires a nonzero concrete ID present
in the staged numbering part (including an instance added by the same batch)
whose abstract has local levels and no existing style owner. The edit writes
the style's `numId` and the abstract's reciprocal `styleLink`. It allocates
no numbering IDs and leaves levels, overrides and paragraph references intact.

Replace and remove require a nondefault custom style with recognized markup
(including supported direct formatting groups and conditional regions). Replacement retains
its type, ID, display name and source slot. Both operations reject potential
references elsewhere in the package, including other styles, settings, stories,
numbering and fields, and reject unsupported markup or parts when usage cannot
be established. `Retarget` explicitly migrates a recognized custom paragraph,
character or table style to a different, same-kind snapshot anchor. It rewrites
exact stored style IDs in paragraphs, runs, tables, numbering levels and other
style definitions (including base, next and link), checks changed inheritance
chains and the complete package for residual users, then removes the source.
Fields, protected wrappers and ambiguous/opaque references reject the transaction.
Retarget is the only style operation in its batch; other supported edit
families stage first. There is no rewriting of field instructions, display
names or linked numbering-style owners.
