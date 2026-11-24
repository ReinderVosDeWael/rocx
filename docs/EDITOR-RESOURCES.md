# Imported document resources

Rust `ImportedDocument::styles()`, Python `ImportedDocument.styles`, and
JavaScript `ImportedDocument.styles()` inspect the relationship-selected
styles part. Absence of the relationship returns `Ok(None)`/`None`/`undefined`;
an ambiguous, invalid, or unsupported part produces an error. The part name
identifies the physical source, and styles appear in source order.

Each style exposes its optional source ID, direct name and `basedOn` ID.
Kinds are paragraph, character, table or numbering; a missing kind is distinct
from an unsupported source value. Python and JavaScript expose the original
kind as `kind_xml` / `kindXml` (absent for a missing kind); Rust retains an unknown value in
`StyleKind::Unsupported`. References are not resolved or validated, and no
effective formatting is inferred. Unmodeled properties remain in the original
package, which is the source for subsequent previews.

For authoring definitions, patching selected properties and conservatively
replacing or removing unused styles, see [imported style edits](EDITOR-PARITY.md#imported-style-edits).

## Direct editable properties

| Property | Rust getter / `ResourceEdit` variant | Python property / `ResourceEdit` factory | JavaScript getter / resource `kind` |
| --- | --- | --- | --- |
| Core title | `core_title()` / `CoreTitle` | `core_title` / `core_title(value)` | `coreTitle()` / `"coreTitle"` |
| Default paragraph justification | `default_alignment()` / `DefaultAlignment` | `default_alignment` / `default_alignment(value)` | `defaultAlignment()` / `"defaultAlignment"` |
| Shared abstract level start | `abstract_starts()` / `AbstractStart(AbstractStart { abstract_id, level, start })` | `abstract_starts` / `abstract_start(abstract_id, level, start)` | `abstractStarts()` / `"abstractStart"` with `abstractId`, `level` |

Pass these edits to Rust `preview_with_resources` (last slice), Python
`preview([], resources=[...])`, or JavaScript `preview({resources: [...]})`.
JavaScript edits require `value`. An omitted edit retains its property;
`None`/`null` clears the direct element. Setting the title to `""` creates
or retains a present empty title. Clearing default alignment leaves unrelated
defaults and their containers in place. Inspection returns `None`/`undefined`
for an absent scalar; absent numbering yields an empty list.

The title follows the **package** core-properties relationship. Defaults and
numbering use the main document's relationships. Missing parts cannot be
created. Duplicate targets, duplicate IDs/levels, ambiguous property paths,
and annotated or nested property leaves are rejected. Titles accept at most
255 XML characters; justification accepts the existing alignment enum.
Numbering inspection lists direct physical levels (0–8) and optional unsigned
32-bit starts, in source order. An abstract ID and level identify a shared
definition; setting its start affects consumers without an overriding start.
The abstract start edit retains instances, overrides, paragraph references and IDs.
Other metadata fields (including dates/custom properties), run defaults, and
default paragraph properties other than alignment are outside this edit surface.

## Numbering definitions, instances and paragraph references

Use Rust `ResourceEdit::Numbering(NumberingEdit::…)`, Python `ResourceEdit`
factories, or JavaScript `resources` records. IDs are explicit unsigned physical
IDs in the selected numbering part; 0 is reserved for concrete instances.
No edit computes displayed labels or allocates a missing part.

| Operation | Rust variant | Python factory | JavaScript `kind` and fields |
| --- | --- | --- | --- |
| Add abstract levels | `AddAbstract { abstract_id, levels }` | `add_abstract(abstract_id, levels)` | `addAbstract`: `abstractId`, `levels` |
| Remove unused abstract | `RemoveAbstract { abstract_id }` | `remove_abstract(abstract_id)` | `removeAbstract`: `abstractId` |
| Add concrete instance of an abstract | `AddInstance { num_id, abstract_id }` | `add_instance(num_id, abstract_id)` | `addInstance`: `numId`, `abstractId` |
| Remove unused instance | `RemoveInstance { num_id }` | `remove_instance(num_id)` | `removeInstance`: `numId` |
| Set/clear concrete start | `InstanceStart { num_id, level, start }` | `instance_start(num_id, level, start)` | `instanceStart`: `numId`, `level`, `value` |
| Set/clear concrete level replacement | `InstanceLevel { num_id, level, value }` | `instance_level(num_id, level, value)` | `instanceLevel`: `numId`, `level`, `value` |
| Assign/clear a direct body paragraph's list | `Paragraph { target, value }` | `paragraph_numbering(target, value)` | `paragraphNumbering`: `target`, `value` |

Add a nonempty list of distinct `NumberingLevel` declarations (indices 0–8).
Supported `format` tokens are `decimal`, `decimalZero`, `upperRoman`, `lowerRoman`,
`upperLetter`, `lowerLetter`, `bullet` and `none`. A concrete replacement's index
must match its target. Level declarations cannot link paragraph styles. A
replacement and a start override may coexist; clearing either retains the other.
Pass `None`/`null` to clear. Omit an edit to retain that property.

Rust `numbering_instances()`, Python `numbering_instances`, and JavaScript
`numberingInstances()` list the physical instance ID, abstract ID, direct
start overrides and replacement-level indices. Python starts are `(level, start)`
pairs. Rust/Python `paragraph_numbering(anchor)` and JavaScript
`paragraphNumbering(anchor)` inspect the direct reference or return absence.
The value is Rust `NumberingReference`, Python `(num_id, level)` or JavaScript
`{id, level}`. A partial, annotated or revision-bearing `numPr` is unsupported;
inherited numbering is not returned. Paragraph anchors expire on reopen.

Adds, overrides and paragraph assignments can share a batch, including forward
references. A reference or override must point to a direct level of a surviving
instance's abstract. Removal never retargets other users. Clear the paragraph
reference and remove the instance and abstract together if they have no remaining
users. Destructive edits reject definitions with formatting/opaque markup they
cannot safely discard. Removal checks the staged numbering part and other Word
parts, including styles, note stories and headers; fields, opaque extensions or
uninspectable parts can prevent removal even without a known direct reference.
Duplicate edits and dangling targets fail the entire preview.

There is no general whole-definition replacement, instance relinking, in-place
abstract level insertion/removal, numbering-style linking, or direct reference
editing in other stories.
