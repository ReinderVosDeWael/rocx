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
| Core subject | `core_subject()` / `CoreSubject` | `core_subject` / `core_subject(value)` | `coreSubject()` / `"coreSubject"` |
| Default paragraph justification | `default_alignment()` / `DefaultAlignment` | `default_alignment` / `default_alignment(value)` | `defaultAlignment()` / `"defaultAlignment"` |
| Shared abstract level start | `abstract_starts()` / `AbstractStart(AbstractStart { abstract_id, level, start })` | `abstract_starts` / `abstract_start(abstract_id, level, start)` | `abstractStarts()` / `"abstractStart"` with `abstractId`, `level` |

Pass these edits to Rust `preview_with_resources` (last slice), Python
`preview([], resources=[...])`, or JavaScript `preview({resources: [...]})`.
JavaScript edits require `value`. An omitted edit retains its property;
`None`/`null` clears the direct element. Setting the title to `""` creates
or retains a present empty title. Clearing default alignment leaves unrelated
defaults and their containers in place. Inspection returns `None`/`undefined`
for an absent scalar; absent numbering yields an empty list.

Title and subject independently follow the **package** core-properties relationship.
An empty subject, like an empty title, is distinct from an absent property. Defaults and
numbering use the main document's relationships. Missing parts cannot be
created. Duplicate targets, duplicate IDs/levels, ambiguous property paths,
and annotated or nested property leaves are rejected. Titles and subjects accept at most
255 XML characters; justification accepts the existing alignment enum.
Numbering inspection lists direct physical levels (0–8) and optional unsigned
32-bit starts, in source order. An abstract ID and level identify a shared
definition; setting its start affects consumers without an overriding start.
Instances, overrides, paragraph references and IDs are retained. This API
does not evaluate counters, renumber lists, change links, or delete definitions.
Run defaults and default paragraph properties other than alignment are outside this slice.

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
