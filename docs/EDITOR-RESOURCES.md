# Imported document resources

Rust `ImportedDocument::styles()`, Python `ImportedDocument.styles`, and
JavaScript `ImportedDocument.styles()` inspect the relationship-selected
styles part. Absence of the relationship returns `Ok(None)`/`None`/`undefined`;
an ambiguous, invalid, or unsupported part produces an error. The part name
identifies the physical source, and styles appear in source order.

Each style exposes its optional source ID, direct name and `basedOn` ID.
Rust and Python expose `next_style`, `linked_style`, and `property_xml`;
JavaScript exposes `nextStyle`, `linkedStyle`, and `propertyXml`. The first
two are the IDs stored in `w:next` and `w:link`. The last is a source-ordered
list of standalone XML strings for the direct `pPr`, `rPr`, `tblPr`, `trPr`,
`tcPr` and `tblStylePr` elements, including their attributes and descendants.
Parse by namespace URI; serialization does not promise original lexical bytes.
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
| Core subject | `core_subject()` / `CoreSubject` | `core_subject` / `core_subject(value)` | `coreSubject()` / `"coreSubject"` |
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

Title and subject independently follow the **package** core-properties relationship. Defaults and
numbering use the main document's relationships. A set can create an absent
styles/settings part; a clear alone never creates it. Duplicate targets,
duplicate IDs/levels, ambiguous property paths,
and annotated or nested property leaves are rejected. Titles and subjects accept at most
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

The shared field keys are `title`, `subject`, `creator`, `description`, `keywords`, `category`,
`version`, `created`, `modified`, `lastPrinted`, `lastModifiedBy`, `revision`,
`identifier`, `language`, `contentStatus`, `company` and `manager`. Rust enum
variants capitalize the first letter. All are string-valued. Word's core
“Comments” is the same `dc:description` as `description`. Use
`MetadataField::Comments` in Rust or the `comments` key in Python and JavaScript;
edits using both spellings in one batch conflict. Comment-entry authors are
story data. Dates are read as source text; set them with a valid UTC timestamp
in `YYYY-MM-DDTHH:MM:SSZ` form. Timestamp edits never update another date.

The package relationships select the parts, including relocated targets.
Company and manager use extended properties. Setting a core, extended or custom
property creates its part if missing; clearing never creates a part.
The `title` and `subject` metadata keys alias their dedicated core-property
edits. Supplying both spellings for a property, even to clear it, is a duplicate target.
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
Adding an abstract can create a missing numbering part. The editor does not
compute displayed labels.

| Operation | Rust variant | Python factory | JavaScript `kind` and fields |
| --- | --- | --- | --- |
| Add abstract levels | `AddAbstract { abstract_id, levels }` | `add_abstract(abstract_id, levels)` | `addAbstract`: `abstractId`, `levels` |
| Replace the levels of an existing abstract | `ReplaceAbstract { abstract_id, levels }` | `replace_abstract(abstract_id, levels)` | `replaceAbstract`: `abstractId`, `levels` |
| Set/clear one physical abstract level | `AbstractLevel { abstract_id, level, value }` | `abstract_level(abstract_id, level, value)` | `abstractLevel`: `abstractId`, `level`, `value` |
| Remove unused abstract | `RemoveAbstract { abstract_id }` | `remove_abstract(abstract_id)` | `removeAbstract`: `abstractId` |
| Add concrete instance of an abstract | `AddInstance { num_id, abstract_id }` | `add_instance(num_id, abstract_id)` | `addInstance`: `numId`, `abstractId` |
| Relink an existing concrete instance | `RelinkInstance { num_id, abstract_id }` | `relink_instance(num_id, abstract_id)` | `relinkInstance`: `numId`, `abstractId` |
| Replace a concrete link and all its overrides | `ReplaceInstance(Box<NumberingDefinition>)` | `replace_instance(definition)` | `replaceInstance`: `definition` |
| Remove unused instance | `RemoveInstance { num_id }` | `remove_instance(num_id)` | `removeInstance`: `numId` |
| Set/clear concrete start | `InstanceStart { num_id, level, start }` | `instance_start(num_id, level, start)` | `instanceStart`: `numId`, `level`, `value` |
| Set/clear concrete level replacement | `InstanceLevel { num_id, level, value }` | `instance_level(num_id, level, value)` | `instanceLevel`: `numId`, `level`, `value` |
| Assign/clear a direct body paragraph's list | `Paragraph { target, value }` | `paragraph_numbering(target, value)` | `paragraphNumbering`: `target`, `value` |
| Assign/clear a direct cell or story paragraph's list | `StoryParagraph { target, value }` | `story_paragraph_numbering(target, value)` | `storyParagraphNumbering`: `target`, `value` |

Add a nonempty list of distinct `NumberingLevel` declarations (indices 0–8).
Use a standard `numFmt` token, e.g. `decimal`, `ordinal`, `lowerRoman` or `bullet`.
A concrete replacement's index must match its target. A level can identify
an existing, unambiguous paragraph style using its ID or name. A
replacement and a start override may coexist; clearing either retains the other.
Pass `None`/`null` to clear. Omit an edit to retain that property.
Replace or clear only modeled level content. Whole abstract replacement
retains the abstract ID, metadata and unrelated siblings; reapplying the same
modeled level payload is a no-op. A surviving instance must keep its old level
addresses. An owned abstract may change its local levels if its `styleLink`
and the owning style's `numId` are reciprocal. A level-free `numStyleLink`
alias has no direct editable levels; paragraph assignments through the alias
validate against the proven owner. An owning instance cannot move to another
abstract and strand the link. Non-owner instances can relink when old level
addresses and overrides remain valid. Dangling, cyclic, ambiguous or malformed
links block edits to the affected graph. Concrete replacement takes an explicit abstract ID and may
specify start and level overrides, but cannot declare abstract levels or
inherited styles. It conflicts with separate instance changes at the same ID.

Rust `numbering_instances()`, Python `numbering_instances`, and JavaScript
`numberingInstances()` list the physical instance ID, abstract ID, direct
start overrides and replacement-level indices. Python starts are `(level, start)`
pairs. Rust/Python `paragraph_numbering(anchor)` and JavaScript
`paragraphNumbering(anchor)` inspect the direct reference or return absence.
The value is Rust `NumberingReference`, Python `(num_id, level)` or JavaScript
`{id, level}`. A partial, annotated or revision-bearing `numPr` is unsupported;
inherited numbering is not returned. Paragraph anchors expire on reopen.

`story_paragraph_anchor` / `storyParagraphAnchor` selects a direct physical
header/footer paragraph or an ordinary note/comment paragraph by owner, original
block index and, for an entry, its decimal ID. Two slots sharing a part target
the same paragraph. `cell_paragraph_anchor` / `cellParagraphAnchor` selects a
direct paragraph in a direct body table by table anchor and physical row, cell
and block indices. `story_paragraph_numbering` / `storyParagraphNumbering`
reads either anchor. Nested, protected, opaque and merge-continuation targets
are not supported.

Adds, overrides and paragraph assignments can share a batch, including forward
references. A reference or override must point to a level of a surviving
instance's local or proven inherited abstract. Removal never retargets other users. Clear the paragraph
reference and remove the instance and abstract together if they have no remaining
users. Destructive edits reject definitions with formatting/opaque markup they
cannot safely discard. Removal checks the staged numbering part and other Word
parts, including styles, note stories and headers; fields, opaque extensions or
uninspectable parts can prevent removal even without a known direct reference.
Duplicate edits and dangling targets fail the entire preview.

`numbering_catalog()` (Rust), `numbering_catalog` (Python) and
`numberingCatalog()` (JavaScript) expose physical abstract and concrete entries
in source order. Abstracts include `style_link` / `styleLink` and
`num_style_link` / `numStyleLink`. Concrete entries include direct `style_ids`
/ `styleIds`, start overrides and complete replacement-level readback. Each
level reports the directly stored start, format, text, restart, legal flag,
suffix, alignment and paragraph style ID, plus modeled label and paragraph
formatting in authoring units. Rust uses `ImportedNumberingLevel` with typed
`RunFormat` and `ParagraphFormat`; Python returns a dictionary with snake-case
keys (the same as CLI `numbering_catalog`), and JavaScript uses typed camel-case
records. Missing values are not synthesized, and aliases do not flatten their
provider's levels. Legacy and typed line spacing use the same stored XML and
read back as typed line height.

`has_opaque_content` / `hasOpaqueContent` marks fields that cannot be fully
represented. Destructive updates refuse such content, including legacy
numbering tabs, independent theme-color fallback, malformed values and
foreign markup. The editor does not compute effective numbering or rewrite
ownership.

A new numbering style may bind a concrete instance created in the same
preview. The style stores its `numId`, and the abstract gets a reciprocal
`styleLink`. A simultaneous defaults patch uses the same styles part, including
when it must be created. Removals check the final staged references; clearing a
paragraph reference never permits removing an instance still used by a style.

### Modeled acceptance boundary

Editing covers the declarative `DocumentDefaults` run and paragraph fields
and document-wide note numbering/placement, margin/gutter switches and
revision tracking. Metadata covers every stored `Metadata` field (including
the description/Comments alias, Company and Manager), both declarative custom
types, and independent core timestamps. These are physical values, not a
reconstruction of authoring precedence or render options. Other Word
settings, extended properties, linked or other custom-property types and
unmodeled numbering markup are preserved when changing supported siblings,
not treated as writable declarations.
