# Style references

## R6f: reference allocation and inheritance checks

Both authoring APIs use the shared native style compiler. Paragraph and table
style identities are reserved before resolving `base_style` and paragraph
`next_paragraph_style` references. They can reference definitions later in the
same list, including names containing spaces; generated wire IDs follow the
existing name-to-ID policy. Imported targets remain available through actual
styles relationships and the existing name/ID lookup.

- A new style's base must exist and have the same kind as the new style.
- A paragraph's next style must be a paragraph. Self-next links are legal and
  are not mistaken for inheritance cycles.
- Based-on cycles reachable from any authored definition fail with
  `invalid_content`. This conservative authoring policy is not an XSD constraint
  or a claim about consumer recovery of malformed styles. Traversal is iterative.
- Missing direct targets retain `unknown_style`; references are not silently
  renamed. Kind mismatches fail with `invalid_content`.
- Existing paragraph definitions update in place. As before, `base_style` is
  applied **only when creating** a paragraph style; it is not consulted when
  updating one. `next_paragraph_style` can be updated. Table style duplicates
  continue to fail, including a table duplicate earlier in the same list.
- An imported style with omitted `w:type` is treated as paragraph per P1
  §17.7.4.17; its imported attribute is not materialized just to satisfy a reader.
- Unrelated imported cycles are retained. Only reachable cycles are checked;
  this is not a complete imported-style graph validator. Unknown indirect
  imported ancestors and effective formatting are not resolved by this feature.

This slice does not add theme-font authoring or computed/effective style
inspection. Document defaults and linked styles are described below.
The deferred style collection still uses ordinary Rust `Value`/`PropertyRef` and
Python generic resolution; there is no new dependency engine.

Evidence: Rust XML acceptance plus Rust-generated python-docx readback with
retargeted base/next negative controls, Python forward/self-next/type/cycle tests,
imported omission/cycle boundaries and official schema validation. The omitted
type test uses python-docx's loaded XML: its high-level `StyleFactory` does not
implement the normative omitted-type paragraph default. This reader limitation
does not justify altering imported data. Schema validation alone cannot detect
all graph cycles or semantic kind mismatches. No layout equivalence is claimed.

Sources: P1 (2016) §17.7, particularly §§17.7.4.3/17.7.4.10 (based-on/next) and
§17.7.4.17 (`style`), plus Annex A.1.

## R6g: character styles and direct run references

Rust adds `CharacterStyleDefinition { name, base_style, format: RunFormat }` to
`StyleDefinition`. Put it in the ordinary deferred `Document.styles` collection.
Use `RunFormat.style: Some(name_or_id)` on direct runs. Character-style formatting
cannot itself contain a run style reference: use `base_style` for inheritance.

Python adds `CharacterStyleDefinition(name, base_style=None, ...)` with the same
flattened formatting properties as `TextRun`, excluding text/comments/run style.
Its size property is `size` (points), not paragraph styles' `font_size`.
`TextRun(..., style="My Character")` uses generic per-field deferred resolution.

```python
document = d.Document([d.Section(children=[d.Paragraph(children=[
    d.TextRun("styled", style="My Character", bold=False),
])])], styles=[d.CharacterStyleDefinition(name="My Character", italic=True)])
```

The shared run codec supports the existing colors, underline, script/language,
font and size properties. Direct formatting remains separate from the referenced
style; no effective inheritance is materialized in resolved run properties.
Style toggles keep the semantics described in FORMATTING.md. Character parents
must also be character styles; forward bases and reachable cycle checks use the
same two-pass style compiler. Existing character styles update in place with
creation-only base links, matching paragraph style updates. Direct run references
must resolve to character styles, including imported built-ins; missing names or
paragraph/table targets fail. Names resolve to existing/allocated wire IDs.

Character style references work through the shared leaf-run codec, including
hyperlink display and cached field runs. Linked paragraph/character style
references are described below; automatic effective-style calculation is unsupported.
No font embedding or visual-equivalence claim is made.

Evidence: `tests/character_styles.rs`, `test_character_styles.py`, official schemas
for resource/run links in regular/hyperlink/cached-field content, Rust-generated
python-docx API readback with retargeted-link negative controls, generic references,
existing-style updates and invalid measurements/kinds/cycles. Source: P1 (2016)
§§17.7.3, 17.7.4.17 and 17.3.2.29 (`rStyle`).

## R6k: conditional table paragraph formatting

`TableSectionFormat` adds `line_height`, `tab_stops` and `bidi` in both APIs.
They use the same checked paragraph codec as direct formatting and paragraph
styles: typed multiple/exact/minimum line heights, signed point tab positions
with checked rounding/collisions and explicit clears, and optional paragraph
direction. Existing before/after spacing, alignment, run properties and shading
remain supported. Python appends the fields to preserve positional arguments.
Rust exhaustive literals need the fields or `..Default::default()`.

All existing regions support these properties. `whole_table` writes base table
style `pPr`; named conditional regions write `tblStylePr/pPr`. No properties are
copied into cell paragraphs. Style definitions do not themselves change table-look
flags; use the table's explicit `TableLook` controls described in
[TABLES.md](TABLES.md). Conditional precedence/effective formatting or line layout
is not calculated. An empty tab
list does not clear inherited stops. Table style duplicate rejection is unchanged.
Generic deferred style aggregates and Python nested fields continue to work.

Evidence: `tests/conditional_paragraph_formatting.rs`,
`test_conditional_paragraph_formatting.py` (all 13 region inputs, invalid
measurements/collisions, deferred nested values and unchanged direct paragraphs),
independent Rust-generated python-docx loaded-style XML readback and changed-height
negative control, plus official schema checks with bad line-rule/tab/bidi values.
See [FORMATTING.md](FORMATTING.md) for units and inheritance limits. Source:
P1 (2016) §17.7.6.1 (`pPr`), §17.7.6.6 (`tblStylePr`) and Annex A.1.
Cell/table properties and full effective-style inspection remain outside this slice.

## R6l: conditional table run script formatting

`TableSectionFormat` also supports `language`, `bold_complex_script`,
`italic_complex_script`, `size_complex_script`, `rtl`, `complex_script`,
`font_east_asia` and `font_complex_script` in both APIs. These append optional
fields without changing existing Python positional arguments. Rust exhaustive
literals need the fields or `..Default::default()`.

All 13 region inputs use the shared run codec: base `whole_table` properties
write style `rPr`, while named regions write `tblStylePr/rPr`. Omitted fields
author no override; false and zero remain explicit. Sizes are nonnegative finite
points converted to checked half-points, including zero as for direct runs.
Language hints are strings without registry validation. Literal fonts request
names, not embedded fonts, fallback selection or shaping. `rtl` is independent
of paragraph `bidi` and section direction. Style-toggle semantics still apply.
No conditional properties are materialized in direct cell runs, and effective
conditional formatting or visual layout is not calculated.

Evidence: `tests/conditional_script_formatting.rs`,
`test_conditional_script_formatting.py` (all regions, omission/zero/false,
checked sizes, nested deferred inputs, unchanged direct text/runs and repeatable
snapshots), Rust-generated independent loaded-XML readback with changed RTL
negative control, official schema validation with invalid toggle/size mutations,
and clean installed-wheel probes. Source: P1 §17.7.6.2 and §17.3.2.

## R6m: document defaults

`Document.defaults` accepts optional `DocumentDefaults` through the ordinary
deferred field mechanism. Rust reuses `RunFormat` and `ParagraphFormat` in its
optional `run` and `paragraph` members. Python provides `RunDefaults` and
`ParagraphDefaults` records with generic nested deferred fields and references.
Resolved Rust snapshots expose `defaults()`; neither API expands defaults into
resolved or serialized paragraphs/runs, and neither calculates effective styles.

```python
defaults = d.DocumentDefaults(
    run=d.RunDefaults(font="Aptos", size=11, language=d.RunLanguage(latin="en-US")),
    paragraph=d.ParagraphDefaults(spacing_after=6, line_height=d.LineHeight.multiple(1.15)),
)
document = d.Document([], defaults=defaults)
```

The compiler merges explicitly supplied properties into the related styles
part's `docDefaults/rPrDefault/rPr` and `docDefaults/pPrDefault/pPr`. Missing
containers are created in schema order, including templates without docDefaults.
Omitted defaults, omitted members and omitted properties retain template values.
False and zero author explicit overrides. Compound property attributes (for
example language slots and before/after spacing) merge without dropping unrelated
attributes. Literal fonts clear only their corresponding theme-font slots.

All paragraph lengths here use **points**, including indents, matching direct
paragraph formatting rather than the legacy paragraph-style inch convention.
Run sizes use points converted to checked half-points. The shared codecs retain
their existing measurement, tab-collision and mutually exclusive line-height
checks. A nonempty tab list replaces the template defaults' local tab list;
empty lists preserve it. This is not a command to clear inherited/default tabs.
Explicit first-line/hanging values remove both competing point and character-unit
attributes; explicit left/right values remove their corresponding character-unit
attributes so the authored point values are not ignored by a consumer.

Defaults deliberately do not accept run/paragraph style references or heading
selection (checked in Rust; not exposed by Python defaults records). Named styles,
direct formatting and consumer interpretation remain separate. No shaping,
pagination, font fallback or visual-equivalence claim is made. Other imported
properties retain the existing edited-XML preservation limits.

Evidence: `tests/document_defaults.rs`, `test_document_defaults.py`, independent
Rust-generated loaded-XML readback with changed default-size negative control,
official schemas with invalid size/order mutations and creation in a template
without defaults, and clean-wheel probes. Tests cover template merging,
absent/false/zero, point units, style-reference rejection, deferred references,
detached snapshots and unchanged direct formatting. Source: P1 (2016) §§17.7.5.1–5.

## R6n: explicit linked-style references

Paragraph and character definitions accept optional `linked_style` names/IDs in
both APIs. Forward targets use the existing two-pass style allocation. An authored
paragraph link must target a character style, and a character link must target a
paragraph style. Unknown targets fail with `unknown_style`; wrong kinds fail with
`invalid_content`, attributed to `styles[index].linked_style`. Omitted imported
style types retain the normative paragraph interpretation.

```python
styles = [
    d.ParagraphStyleDefinition(name="Pair Paragraph", linked_style="Pair Character"),
    d.CharacterStyleDefinition(name="Pair Character", linked_style="Pair Paragraph"),
]
```

Each supplied edge writes that definition's `w:link` only. To request a reciprocal
pair, author both edges. One-sided edges are allowed; no target, backlink or
formatting is silently created or modified. These are pairing references, not
based-on inheritance, so reciprocal links are not inheritance cycles. Omission
preserves an existing link; explicit values update new or existing definitions.
There is no link-clearing operation in this slice. Unrelated or omitted malformed
imported links remain preserved rather than globally repaired or validated.

Both halves remain independent resources. Direct paragraph and run references
must target paragraph and character styles respectively; the compiler does not
automatically substitute a linked half or copy formatting between halves. The
paragraph-kind check closes a previously unchecked malformed-authoring case.
Application UI selection/merging, effective formatting and visual layout are not
computed. The normal deferred styles aggregate and Python nested field references
continue to apply. Python appends the new fields; Rust exhaustive literals need
`linked_style` or `..Default::default()`.

Evidence: `tests/linked_styles.rs`, `test_linked_styles.py`, independent
Rust-generated readback with retargeted-link negative control, official schemas
with a missing required link value mutation, and clean-wheel probes. XSD alone
does not verify target existence/kind or semantic pairing. Tests cover forward
reciprocal/one-sided links, invalid targets, updates/omission, imported boundaries,
deferred inputs and unchanged direct formatting. Source: P1 (2016) §17.7.4.6.
