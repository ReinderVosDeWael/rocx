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
| Run, paragraph and settings defaults | `document_defaults()` / `Default(DefaultField, Option<DefaultValue>)` | `document_defaults` / `document_default(field, value)` | `documentDefaults()` / `"documentDefault"` with `field` |
| Shared abstract level start | `abstract_starts()` / `AbstractStart(AbstractStart { abstract_id, level, start })` | `abstract_starts` / `abstract_start(abstract_id, level, start)` | `abstractStarts()` / `"abstractStart"` with `abstractId`, `level` |

Pass these edits to Rust `preview_with_resources` (last slice), Python
`preview([], resources=[...])`, or JavaScript `preview({resources: [...]})`.
JavaScript edits require `value`. An omitted edit retains its property;
`None`/`null` clears the direct element. Setting the title to `""` creates
or retains a present empty title. Clearing default alignment leaves unrelated
defaults and their containers in place. Inspection returns `None`/`undefined`
for an absent scalar; absent numbering yields an empty list.

The title follows the **package** core-properties relationship. Defaults and
numbering use the main document's relationships. A set can create an absent
styles/settings part; a clear alone never creates it. Duplicate targets,
duplicate IDs/levels, ambiguous property paths,
and annotated or nested property leaves are rejected. Titles accept at most
255 XML characters; justification accepts the existing alignment enum.
Numbering inspection lists direct physical levels (0–8) and optional unsigned
32-bit starts, in source order. An abstract ID and level identify a shared
definition; setting its start affects consumers without an overriding start.
The abstract start edit retains instances, overrides, paragraph references and IDs.

### Editing document defaults

The defaults view is keyed by `DefaultField` in Rust, or by the strings below
in Python and JavaScript. Only directly present properties are returned. A
toggle element without `w:val` means true. An explicit false survives readback.
For example, to turn off default bold without touching any direct run formatting:

| Rust `resources` entry | Python `resources` entry | JavaScript `resources` entry |
| --- | --- | --- |
| `ResourceEdit::Default(DefaultField::Bold, Some(DefaultValue::Bool(false)))` | `ResourceEdit.document_default("bold", False)` | `{kind: "documentDefault", field: "bold", value: false}` |

Clear the field with `None`/`None`/`null`; omit the entry to leave it alone.
Rust variants use the key in PascalCase (e.g. `SizeCsHalfPoints`). Values are
`DefaultValue::{Bool,Integer,Text,Tabs}`. Fonts, language tags, theme attributes,
spacing and indentation are physical properties: setting a font does not
silently remove its theme, nor does a twip indent remove character-unit indents.

| Direct properties | Field keys |
| --- | --- |
| Run switches | `bold`, `italic`, `boldCs`, `italicCs`, `runRtl`, `complexScript`, `caps`, `smallCaps`, `strike`, `runSnapToGrid` |
| Fonts and script language | `fontAscii`, `fontHAnsi`, `fontEastAsia`, `fontCs`, `fontAsciiTheme`, `fontHAnsiTheme`, `fontEastAsiaTheme`, `fontCsTheme`, `language`, `languageEastAsia`, `languageBidi` |
| Run size, decoration and color | `sizeHalfPoints`, `sizeCsHalfPoints`, `underline`, `verticalAlign`, `runColor`, `colorTheme`, `colorTint`, `colorShade` |
| Paragraph switches | `paragraphBidi`, `keepTogether`, `keepWithNext`, `pageBreakBefore`, `widowControl`, `suppressLineNumbers`, `contextualSpacing`, `mirrorIndents`, `paragraphSnapToGrid` |
| Paragraph spacing | `beforeTwips`, `afterTwips`, `beforeLines`, `afterLines`, `beforeAutoSpacing`, `afterAutoSpacing`, `line`, `lineRule` |
| Paragraph indents | `leftTwips`, `rightTwips`, `startTwips`, `endTwips`, `firstLineTwips`, `hangingTwips`, `leftChars`, `rightChars`, `startChars`, `endChars`, `firstLineChars`, `hangingChars` |
| Other paragraph properties | `justification`, `tabStops` |
| Settings switches | `mirrorMargins`, `gutterAtTop`, `trackRevisions` |
| Note settings | `footnotePosition`, `endnotePosition`, `footnoteFormat`, `endnoteFormat`, `footnoteStart`, `endnoteStart`, `footnoteRestart`, `endnoteRestart` |

Text enums take OOXML spellings such as `superscript`, `both`, `eachSect` or
`lowerRoman`. The older alignment API maps `justify` to `both`; do not edit
both aliases in one batch. Sizes are integer half-points. `*Twips` are 1/20 pt;
`*Chars` and `*Lines` use hundredths of a character or line. `line` is in
240ths of a line for `auto` or twips for `exact`/`atLeast`. `runColor` accepts
a six-digit RGB string or `auto`; theme tint/shade are two hex digits. A theme
color still requires a fallback `runColor`.

Replace all direct tabs with a nonempty list of `DefaultTab { position_twips,
alignment, leader }` in Rust, `(position_twips, alignment, leader_or_None)`
tuples in Python, or `{positionTwips, alignment, leader?}` in JavaScript.
Positions are signed twips and must be unique; clear the list with `None`/`null`.
Changing a tab list with opaque annotations is rejected. Scalar edits preserve
unrelated attributes, settings, note references and styles. Destructive clears
that would discard annotations are rejected. Arbitrary OOXML defaults,
noninteger measurement lexemes and computed effective formatting
are unsupported. `trackRevisions` changes the setting, not existing revisions.

### Package metadata

| Target | Rust inspection / edit | Python inspection / edit | JavaScript inspection / edit |
| --- | --- | --- | --- |
| Core or extended scalar | `doc.metadata_field(MetadataField::Creator)` / `ResourceEdit::Metadata(MetadataField::Creator, Some("Reviewer".into()))` | `doc.metadata_field("creator")` / `ResourceEdit.metadata("creator", "Reviewer")` | `doc.metadataField("creator")` / `{kind: "metadata", field: "creator", value: "Reviewer"}` |
| Named custom property | `doc.custom_property("Approved")` / `ResourceEdit::CustomProperty("Approved".into(), Some(CustomValue::Boolean(false)))` | `doc.custom_property("Approved")` / `ResourceEdit.custom_property("Approved", False)` | `doc.customProperty("Approved")` / `{kind: "customProperty", name: "Approved", value: false}` |

The shared field keys are `creator`, `description`, `keywords`, `category`,
`version`, `created`, `modified`, `lastPrinted`, `lastModifiedBy`, `revision`,
`identifier`, `language`, `contentStatus`, `company` and `manager`. Rust enum
variants capitalize the first letter. All are string-valued. Word's core
“Comments” field is `description` (`dc:description`); comment-entry authors are
story data. Dates are read as source text; set them with a valid UTC timestamp
in `YYYY-MM-DDTHH:MM:SSZ` form. Timestamp edits never update another date.

The package relationships select the parts, including relocated targets.
Company and manager use extended properties. Setting an extended or custom
property creates its part if missing; clearing never creates a part.
Custom names use exact spelling. Supported values are a string (`CustomValue::Text`)
or boolean (`CustomValue::Boolean`). Replacing a supported value retains its
property ID, including across a type change. Clear with `None`/`null`;
`""` and `false` remain present. Duplicate targets and ambiguous or annotated
leaves fail the whole batch. Other extended properties, linked custom values
and other variant types are retained when a supported sibling changes.

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
