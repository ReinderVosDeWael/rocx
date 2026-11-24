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
both. Use the separate operations below for wrapper and choice edits.

| Operation | Rust | Python | JavaScript |
| --- | --- | --- | --- |
| Inspect eligible wrapper sites | `doc.control_sites()` | `doc.control_sites` | `doc.controlSites()` |
| Inspect controls by physical story part | `doc.story_controls()` | `doc.story_controls` | `doc.storyControls()` |
| Wrap the original payload in a tagged rich-text control | `doc.preview_wrap_control(site, tag)` | `doc.preview_wrap_control(site, tag)` | `doc.previewWrapControl({target: site.anchor, tag})` |
| Replace the complete wrapper definition | `doc.preview_replace_control(anchor, &properties)` | `doc.preview_replace_control(anchor, properties)` | `doc.previewReplaceControl({target, properties})` |
| Select a drop-down value; optionally replace the ordered choices | `doc.preview_control_choice(anchor, value, items)` | `doc.preview_control_choice(anchor, value, choices=items)` | `doc.previewControlChoice({target, selected, choices})` |
| Remove the wrapper, retaining its payload | `doc.preview_unwrap_control(anchor)` | `doc.preview_unwrap_control(anchor)` | `doc.previewUnwrapControl(anchor)` |

`control_sites` identifies physical targets by snapshot; its `context` and
`text` label are for inspection, not unique addresses. Each standalone operation
returns a preview. Reopen its output before taking anchors for the next edit.
The original control inspection also returns `kind` (e.g. `richText`, `text`,
`dropDownList`), literal `multi_line` / `multiLine`, `selected` (the stored
`lastValue`) and ordered `choices`. Display labels and stored values are
separate; selection updates the cached text and preserves run formatting.
Rust choices are `ContentControlListItem`, Python choice edits use
`(display_text, value)` pairs, and JS uses `{displayText, value}`.

Use Rust `ContentControlProperties`, Python `rocx.editor.ControlProperties`,
or JS `ContentControlProperties` for a complete replacement, clearing omitted
metadata. An explicit ID must be unique across the package. Rows and cells
require rich text. Block and inline sites also support plain text and a
drop-down whose selected label matches the existing text. Replacing a
definition requires an unlocked control; either content or wrapper lock
prevents replacing the choice list. A wrapper-only lock still allows selection,
and a content-only lock permits removing an understood wrapper without
changing its payload. Imported opaque properties, ambiguous layouts,
protected ranges, nested controls and custom XML bindings are refused.
Compatible siblings, including paragraphs and tables, can be wrapped together
without re-encoding their contents. Rich block and inline controls can receive
new supported payloads. Use `WrapperEdit` in Rust `EditBatch::wrappers`,
Python `preview([], wrappers=[...])`, or JavaScript `preview({wrappers: [...]})`.
Payload replacement uses `ReplaceBlocks` / `ReplaceInlines`,
`replace_blocks` / `replace_inlines`, or `replaceBlocks` / `replaceInlines`.
Wrapped or unwrapped blocks cannot share a target with another structural edit
or cross a staged range. Controls in headers, footers, notes and comments are
inspectable by physical owner but read-only.

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
runs, or target a [supported object occurrence](#drawing-and-fraction-occurrences).
Payloads accept resolved text, breaks, tabs, fractions, supported fields, external
or paired-body-bookmark hyperlinks, pictures, shapes and column charts.
See [fields and links](FIELDS.md#editing-an-imported-field-or-link) for field
inspection, replacement, flag and instruction patches, and link metadata edits;
see [paired ranges](RANGES.md#imported-body-ranges) for cross-paragraph bookmark
and comment-range insertion, relocation and removal. New external links allocate relationships on
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

**Property-only edits across physical owners.** For an anchored run in a body
paragraph or a direct body-table cell paragraph, use Rust
`doc.edit_run_format(None, &[edit])`, Python `doc.edit_run_format([edit])`,
or JavaScript `doc.editRunFormat({inlineEdits: [edit]})`. A run can be a
direct paragraph child or a direct child of its hyperlink. For a section
header/footer or an ordinary note/comment entry, pass the inspected physical
part name: `Some(&part)` in Rust, `owner_part=part` in Python, `ownerPart:
part` in JavaScript. Each call returns new DOCX bytes. Section aliases share
the edit; reopen before targeting another part or using the new snapshot.
Disjoint patches to one physical run coalesce, even through section aliases.

This operation changes only `rPr`. Text, tabs, breaks, inline/anchored
drawings, symbols and supported hyphens are retained, as are the hyperlink
wrapper and the owner's relationships. It rejects field/reference runs, active
ranges, revision or control wrappers, reviewed ancestor properties, enforced
document protection, non-format actions, stale anchors and owner mismatches.
Entry identities must be unique and ordinary; separator/continuation entries
are protected. Story tables, nested body tables, textbox stories and creating
a local fork of a shared section part remain outside this operation.

`ImportedRun.bold`, `.italic`, `.strike`, and `.hidden` inspect the same direct
properties wherever structured runs are exposed. An unset or ambiguous toggle
is `None` in Rust/Python and absent in JavaScript. `has_opaque_formatting`
(`hasOpaqueFormatting` in JavaScript) signals retained properties or markup
outside this projection. These values do not resolve styles, complex-script
variants, or effective visual formatting.

`inline_changes()` / `inline_changes` / `inlineChanges()` return original
(body, inline) index pairs in document order; an already-matching direct toggle
value is omitted. There are no character-range addresses in these operations.

### Drawing and fraction occurrences

Use the direct body paragraph's structured inline view. A run's optional
`drawing` describes the occurrence: `kind` (`image`, `shape`, `chart`),
`placement` (`inline`, `floating`), explicit extent in EMUs, accessibility
name/title/description, and owner-local relationship ID for embedded pictures
and charts. Simple shapes expose preset geometry and plain text. Image crop
is `[left, top, right, bottom]` in thousandths of a percent; rotation is in
1/60000 degree units. Flips preserve explicit false versus absence. Rust and
Python use `width_emu`, `relationship_id`, `shape_text`, `flip_horizontal`;
JavaScript uses `widthEmu`, `relationshipId`, `shapeText`, `flipHorizontal`.
The legacy flat projection still reports opaque XML at these locations.
A simple stacked fraction exposes an inline anchor, numerator and denominator
(`ImportedInlineNode::Fraction`, `ImportedFraction`, `kind: "fraction"`).
Other math stays opaque.

| Rust `InlineAction` | Python `InlineEdit` | JavaScript `inlineEdits` element |
| --- | --- | --- |
| `InsertAfter(inline)` / `Replace(inline)` | `insert_after(target, inline)` / `replace(target, inline)` | `{kind: "insertAfter" \| "replace", target, inline}` |
| `RemoveObject` | `remove_object(target)` | `{kind: "removeObject", target}` |
| `SetDrawingDescription(Some(text))` / `None` | `set_drawing_description(target, text)` / `None` | `{kind: "setDrawingDescription", target, value: text}` / `null` |
| `SetImageAsset(image)` | `set_image_asset(target, resolved_image_run)` | `{kind: "setImageAsset", target, image: resolvedImageRun.view.image}` |
| `PatchDrawing(DrawingProperty::Size, inline)` | `patch_drawing(target, "size", inline)` | `{kind: "patchDrawing", target, property: "size", inline}` |

`PatchDrawing` copies **one property group** from a resolved declaration of
the same object kind. Other declaration properties, drawing IDs and source
relationships stay in place. A missing optional property in the declaration
clears that group. Rust enum variants / Python and JavaScript selectors:

* `Size` / `size`: update the outer and modeled inner extents.
* `Placement` / `placement`: set inline/floating placement, including wrapping,
  for a picture or shape. `Accessibility` / `accessibility`: copy occurrence
  name/title/description. For a literal attribute removal use the separate
  description action.
* `Crop` / `crop` and `Transform` / `transform`: picture crop, or rotation and
  flips. Asset replacement redirects only one `r:embed`; neither the peer
  occurrence nor its bytes are changed.
* `ShapeFill` / `shapeFill`, `ShapeOutline` / `shapeOutline`, `ShapeText` /
  `shapeText`: geometric shape fill/line or simple text and its format.
  Shape text requires compatible geometric-shape or standalone-text-box
  declarations. `ShapeGeometry` / `shapeGeometry` switches an unadjusted
  rectangle or ellipse in place, retaining its extent, text and appearance.
  Replace the shape to convert to or from a standalone text box.
* `ChartSeries` / `chartSeries`: set categories and values in an existing
  workbook-free, literal single column series.
  A changed series forks the chart part and redirects this occurrence only;
  axis settings and unrelated chart XML are retained. Read the series via
  `doc.chart_series(&anchor)` (Rust tuple), `doc.chart_series(anchor)` (Python
  tuple), or `doc.chartSeries(anchor)` (`{categories, values}` in JavaScript).
  Point counts can change on the plain chart profile emitted by the column
  chart declaration. Imported charts with indexed formatting, annotations or
  other extensions require the same count, or explicit whole-chart replacement.

One edit per original occurrence per preview. To patch another group on the
same occurrence, reopen the preview bytes and obtain a fresh anchor. Whole
replacements/removals refuse unrecognized markup in the occurrence being
discarded. Patches preserve unrelated opaque drawing content and reject
ambiguous targets or unsupported markup on a subtree they would discard.
Unused relationships and old parts remain in the package. Complex equations,
general charts/workbooks, rich text-box stories and VML are outside this
interface. Entry-story pictures use the separate owner-scoped operation below.

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
Direct body runs can also carry [reviewable bold-only property changes](REVISIONS.md#propose-a-formatting-change).
Supported direct body paragraphs can carry [paired mark and text revisions](REVISIONS.md#propose-or-review-a-whole-paragraph).

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
bindings. See [imported style edits](#imported-style-edits) for definition
and property edits. The resource guide also covers direct [core title, default alignment
and abstract level start](EDITOR-RESOURCES.md#direct-editable-properties).
Effective formatting and arbitrary resource editing remain outstanding.
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
direct run formatting. Format-only batches also accept direct hyperlink child
runs, using the same guards as `edit_run_format`. Use original inline anchors from ordinary entries with unique
decimal IDs. Replacement uses the body editor's discard checks; note/reference
marks, field/range interiors, nested splice targets and reserved notes are ineligible.
External links allocate relationships in the entry part, including when its
path is nonstandard. Existing relationships are retained. No entry or reference
is created, deleted or renumbered. Reopen the returned package for new anchors.
The standalone field/range operations and direct hyperlink property/removal
operations above are scoped to the main body, not these entry batches.
Embedded pictures in direct entry paragraphs also support occurrence insertion,
replacement, removal, asset redirection and image property groups. The actual
entry part owns its image relationships; shared bytes elsewhere remain intact.
Shape, chart and fraction edits require a direct body paragraph.

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

For public entry points and conversion boundaries see the [Rust facade](../src/lib.rs), [Python declarations](../python/rocx/declarative/__init__.py) and [module map](MODULES.md). The [OOXML domain map](OOXML.md#wordprocessingml-domain-map) records current generation limits: for example, no nested hyperlinks or custom XML control binding. Those limits are not expanded merely by listing editor goals here.

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

## Imported style edits

| Operation | Rust | Python (`rocx.editor`) | JavaScript |
| --- | --- | --- | --- |
| Address an exact style ID | `doc.style_anchor(id)?` | `doc.style_anchor(id)` | `doc.styleAnchor(id)` |
| Add a declaration | `StyleEdit::Add(definition)` | `StyleEdit.add(definition)` | `{op: "add", definition}` |
| Patch just bold / italic | `StyleEdit::Patch {target, bold, italic}` | `StyleEdit.patch(target, italic=True)` | `{op: "patch", target, italic: true}` |
| Replace an unused definition | `StyleEdit::Replace {target, definition}` | `StyleEdit.replace(target, definition)` | `{op: "replace", target, definition}` |
| Remove an unused definition | `StyleEdit::Remove(target)` | `StyleEdit.remove(target)` | `{op: "remove", target}` |
| Stage with content edits | `doc.preview_edits(EditBatch { styles, ..Default::default() })?` | `doc.preview([], style_edits=styles)` | `doc.preview({styleEdits: styles})` |

Add paragraph, character or table declarations to an existing relationship-selected
styles part. Names, source aliases, and generated IDs must be unambiguous
across styles under ASCII case-insensitive comparison. The authoring codec
resolves forward references and checks their types and inheritance cycles.
Patches address direct paragraph/character run properties, including explicit
false; omitted properties, unknown siblings and attributes are preserved.

Replace and remove require a nondefault custom style with a simple definition
(name, base, empty paragraph properties and/or bold/italic). Replacement retains
its type, ID, display name and source slot. Both operations reject potential
references elsewhere in the package, including other styles, settings, stories,
numbering and fields, and reject unsupported markup or parts when usage cannot
be established. There is no automatic retargeting, numbering-style creation or
synthesis of a missing styles part.
