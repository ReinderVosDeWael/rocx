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
Instances, overrides, paragraph references and IDs are retained. This API
does not evaluate counters, renumber lists, change links, or delete definitions.
Other metadata fields (including dates/custom properties) are outside this edit surface.

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
