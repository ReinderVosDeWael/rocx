# Numbering coverage

## Concrete list start overrides

`NumberingDefinition` keeps its explicit concrete ID and abstract level list,
and adds `start_overrides: Vec<NumberingStartOverride>` in Rust or a Python list.
Each `NumberingStartOverride` has `level` and `start`. Python usage:

```python
definition = d.NumberingDefinition(42, [d.NumberingLevel(start=1)],
    start_overrides=[d.NumberingStartOverride(level=0, start=7)])
paragraph = d.Paragraph(text="Restarted list", numbering=d.NumberingReference(42))
```

The compiler emits `num/lvlOverride/startOverride` on the concrete list instance;
the abstract level's original `start` remains unchanged. Override output is
sorted by level for determinism. To restart a later group, author a distinct
concrete definition/ID and refer the relevant paragraphs to it. The library does
not insert labels into text, calculate counters, or decide when a list restarts.

Abstract levels must be unique and in 0..8. Overrides must uniquely target levels
defined by that definition. Duplicate or undefined override levels are rejected
with `invalid_content`; this is a conservative public-subset policy rather than
full schema support for replacement level definitions. Starts retain the existing
nonnegative 32-bit authoring range, including zero. These are authoring bounds,
not the full arbitrary-precision integer range of every imported XML value.

Both frontends use existing generic deferred inputs: Rust defers the numbering
collection; Python resolves nested definition/override fields normally. No new
dependency engine exists. Snapshots preserve instructions and are repeatable.
Existing imported-ID collisions still fail instead of rewriting references.

Each definition creates its own abstract resource unless an abstract ID is
explicitly shared. Imported numbering is preserved but is not exposed as a
complete editable numbering model.
Readback/schema validity is not evidence of rendered counter/indent equivalence.

Evidence: `tests/numbering_overrides.rs`, `test_numbering_overrides.py`, independent
Rust-generated python-docx loaded-numbering XML readback with changed-start
negative controls, official XSD positive/bad-start checks, generic deferred fields,
zero/sorted levels and definition/override validation. python-docx has no full
high-level list-counter evaluator; plain paragraph text intentionally omits labels.
Source: P1 (2016) §§17.9.8 (`lvlOverride`), 17.9.26 (`startOverride`) and Annex A.1.

## Concrete level replacements

`NumberingDefinition.level_overrides` accepts existing `NumberingLevel` values
in both APIs. Each replacement uses the same supported subset as an abstract
level: `level`, `start`, `format`, `text`, optional `is_legal`, `suffix`,
`restart` and `alignment`. Its `level` supplies both the outer
`lvlOverride/@ilvl` and inner `lvl/@ilvl`, so they cannot disagree in authoring.

```python
definition = d.NumberingDefinition(42, [d.NumberingLevel(0, 1, "decimal", "%1.")],
    start_overrides=[d.NumberingStartOverride(0, 7)],
    level_overrides=[d.NumberingLevel(0, 4, "upperRoman", "(%1)")])
```

Replacements write `num/lvlOverride/lvl` without modifying any abstract level.
Targets must be unique levels defined by this definition, in 0..8, just as for
start overrides. This defined-target restriction is the library's public subset,
not a claim that the complete OOXML grammar forbids additional forms. Duplicate,
undefined and out-of-range targets fail with `invalid_content`, attributed to
`level_overrides[index].level` within the numbering definition.

A start override and a replacement may coexist at the same level. They produce
one `lvlOverride`, with `startOverride` followed by `lvl`; the union is sorted by
level. Neither request is folded into the other: the explicit starting values
remain distinct in XML. Consumer numbering semantics determine the resulting
label; this library does not calculate effective starts, labels or counters.
Empty replacements preserve existing start-only output.

Starts keep the existing nonnegative 32-bit bound, including zero. Format/text
keep the existing abstract-level contract, including XML escaping; this does not
introduce a general level-XML escape hatch. Rust defers the numbering collection;
Python supports nested deferred replacement fields through the generic resolver.
Existing concrete-ID/template collision rejection is unchanged. Python appends
the new field, preserving prior positional arguments. Rust exhaustive literals
need `level_overrides` or `..Default::default()`.

Evidence: `tests/numbering_replacements.rs`, `test_numbering_replacements.py`,
independent Rust-generated loaded-XML readback with changed replacement-text
negative control, official schema validation with bad order/format/missing-ilvl
mutations, clean-wheel probes and both visual catalogs (topic 06, numId=60).
Tests cover mixed start-only/replacement-only/combined requests, sorted output,
unchanged abstract levels, zero starts, invalid targets, nested deferred values,
snapshots and imported ID collisions. Sources: P1 (2016) §§17.9.7–8, 17.9.26 and
Annex A.1 CT_NumLvl. No shared-abstract identity API or visual-counter guarantee.

## Legal numbering display

`NumberingLevel.is_legal: Option<bool>` in Rust and `is_legal: bool | None`
in Python apply to both abstract levels and concrete replacements. Omission
emits no `isLgl`; explicit false/true emit `isLgl/@val=0/1`, before `lvlText`.
For example, `d.NumberingLevel(1, 1, "lowerRoman", "%1.%2", is_legal=True)`
requests decimal display of all level references in this level's label pattern.
It does not replace the stored `numFmt` or change another level's definition.
The consumer calculates displayed labels; this library does not evaluate counters
or materialize labels in paragraph text. Replacements preserve abstract requests.

Rust uses the deferred numbering collection; Python also supports nested deferred
`is_legal` fields through the existing generic resolver. Snapshots are repeatable.
The Python argument is appended; exhaustive Rust literals need `is_legal` or
`..Default::default()`. Omitted values preserve previous output.

Evidence: `tests/legal_numbering.rs`, `test_legal_numbering.py`, official schema
acceptance and invalid-boolean mutation, independent catalog XML checks with a
changed-request negative control, and exact Rust/Python catalog byte parity.
Topic 06 labels enabled abstract/replacement requests and an explicitly disabled
replacement. No manual Office rendering is claimed. Source: P1 (2016) §17.9.4
and Annex A.1 CT_Lvl.

## Numbering suffix

`NumberingLevel.suffix: Option<NumberingSuffix>` in Rust accepts `Tab`, `Space`
or `Nothing`. Python uses `suffix="tab"`, `"space"` or `"nothing"`; other tokens
fail with `invalid_content`. This applies independently to abstract levels and
concrete replacements through the same level serializer.

Omission emits no `suff`, preserving previous output; the normative omitted
default is tab. Explicit tab remains distinct from omission in the request/XML.
The codec writes `suff/@val` after `isLgl` and before `lvlText`. Nothing means
no separator, not an omitted request. For example:

```python
level = d.NumberingLevel(0, 1, "decimal", "%1)", suffix="space")
```

The consumer creates the generated label and its separator. The library does not
append tabs/spaces to paragraph text, determine tab widths/indentation, or calculate
labels. A replacement's suffix does not modify its abstract level. No imported
typed-edit or effective-numbering model is added. Rust uses the deferred numbering
collection, and Python supports nested deferred suffix fields; snapshots remain
repeatable. Python appends the argument; exhaustive Rust literals need `suffix`
or `..Default::default()`.

Evidence: `tests/numbering_suffix.rs`, `test_numbering_suffix.py`, independent
Rust-generated python-docx readback with changed-suffix negative control, official
schema acceptance for all tokens and invalid-token rejection, installed wheel
probes, independent catalog mutation checks, and exact Rust/Python catalog parity.
Topic 06 labels all three tokens and omitted defaults. Schema/readback checks
do not claim manual rendering or layout equivalence. Source: P1 (2016) §17.9.28
and Annex A.1 CT_Lvl.

## Numbering level restart

`NumberingLevel.restart: Option<u32>` in Rust and `restart: int | None` in Python
request when this level's numbering restarts. The value is a **one-based** trigger
index, not a zero-based `level` field. Zero requests never restarting; omission
emits no `lvlRestart` (the normative default restarts after use of the previous
level or any earlier level). A positive request restarts after the specified
earlier level or any level preceding it. For example:

```python
level = d.NumberingLevel(level=2, restart=1)  # restart after the first level
never = d.NumberingLevel(level=2, restart=0)
```

Requests retain the existing nonnegative 32-bit authoring bound (0..4294967295).
This is a public-subset bound, not the full schema integer range. Values that
are inapplicable to this level are preserved for the consumer to ignore; they
are not clamped, renumbered or rejected based on the authored level list. Existing
level uniqueness, reference and replacement-target checks remain unchanged.

The shared abstract/replacement codec writes `lvlRestart` after `numFmt` and
before `isLgl`, `suff` and `lvlText`. Replacements leave abstract requests unchanged.
This is separate from `start`, concrete `start_overrides`, and allocating another
concrete list identity. No effective restart policy or counters are computed,
and no paragraph text is rewritten. Omission preserves prior output. Rust defers
the numbering collection; Python also supports nested deferred restart values.
Snapshots remain repeatable. Python appends the argument; exhaustive Rust level
literals need `restart` or `..Default::default()`.

Evidence: `tests/numbering_restart.rs`, `test_numbering_restart.py`, independent
Rust-generated loaded-numbering readback with a changed-trigger negative control,
official schema acceptance including zero/positive/large requests and invalid
integer rejection, wheel probes, independent catalog mutation assertions and
exact Rust/Python catalog byte parity. Tests cover omission, zero, one-based
values, bounds, both level locations, unchanged starts, combined start overrides,
deferred inputs and snapshots. Topic 06 labels never/first-level/omitted requests.
No manual rendering or counter equivalence is claimed. Source: P1 (2016) §17.9.10
and Annex A.1 CT_Lvl.

## Numbering label alignment

`NumberingLevel.alignment: Option<NumberingAlignment>` in Rust accepts `Left`,
`Center`, `Right`, `Start` and `End`. Python uses `"left"`, `"center"`, `"right"`,
`"start"` and `"end"`. These align the generated label relative to the paragraph
text margin, independently of the paragraph's text alignment. They are a supported
subset of the broader `CT_Jc` grammar; other Python tokens are rejected with
`invalid_content`, including paragraph-style aliases such as `"justify"`.

```python
level = d.NumberingLevel(0, 1, "decimal", "%1.", alignment="end")
```

Omission emits no `lvlJc`, preserving prior output and the consumer's default:
left in left-to-right paragraphs and right in right-to-left paragraphs. Explicit
physical left/right remain distinct from direction-relative start/end. The library
does not infer direction, translate these requests, calculate label widths or
positions, or set paragraph `jc` from a level's `lvlJc`.

Abstract and replacement levels share the same codec: `lvlJc` follows `lvlText`.
A replacement leaves abstract alignment unchanged. Rust defers the numbering
collection; Python also supports nested deferred alignment fields. Snapshots are
repeatable. The Python argument is appended; exhaustive Rust level literals need
`alignment` or `..Default::default()`.

Evidence: `tests/numbering_alignment.rs`, `test_numbering_alignment.py`, independent
Rust-generated python-docx loaded-numbering readback with a changed-alignment
negative control, official schema acceptance/invalid-token mutation, installed
wheel probes, independent catalog assertions/mutations and exact Rust/Python
catalog parity. All five tokens and omission are checked in both level locations;
paragraph text/alignment stay separate. Catalog topic 06 labels all five abstract
requests and start/end replacements. Existing template numbering alignments are
not rewritten; checks select authored abstract identities rather than assuming
the template contains no `lvlJc`. No manual rendering is claimed. Source: P1
(2016) §17.9.7 and Annex A.1 CT_Lvl.

## Shared abstract numbering definitions

Set `NumberingDefinition.abstract_id` (Rust `Option<u32>`, Python `int | None`)
to reuse one authored `abstractNum` across multiple concrete instances. All
definitions using the same explicit ID must supply identical ordered levels.
Concrete starts and level replacements remain per instance.

```python
levels = [d.NumberingLevel()]
first = d.NumberingDefinition(42, levels, abstract_id=100)
second = d.NumberingDefinition(43, levels, abstract_id=100,
    start_overrides=[d.NumberingStartOverride(0, 7)])
```

An omitted ID allocates a private abstract definition. Automatic allocation
skips all requested explicit IDs and starts above the largest imported ID, if
present. Explicit collisions with imported abstract IDs fail. Imported
numbering is preserved when adding new definitions through the template's
numbering relationship. Rust accepts a deferred numbering collection and Python
also accepts deferred IDs through the generic resolver. Python appends the
argument. Exhaustive Rust literals need `abstract_id` or `..Default::default()`.

Source: ECMA-376 Part 1 (2016) §§17.9.1–2 and Annex A.1. Topic 06 of both
catalogs shows shared IDs with separate concrete start requests.

## Numbering label formatting

Set a level's `label_format` to a Rust `RunFormat` or Python `RunDefaults`
record to request direct run properties on the generated label. This is
available on abstract and replacement levels independently. Omission leaves
out `rPr`; explicit false values survive. Existing measurement checks apply,
and Rust style references are rejected in this context. Python accepts nested
deferred values. Exhaustive Rust level literals need the new field or a
default update.

The codec writes `lvl/rPr` after level alignment and paragraph properties
when present (P1 2016 Annex A.1 CT_Lvl). Paragraph text runs use their own
formatting.

## Numbering level paragraph properties

Set `paragraph_format` to Rust `ParagraphFormat` or Python `ParagraphDefaults`
on an abstract or replacement level. Lengths use points, including signed
first-line indents and tab positions. Negative first-line indents produce
hanging indents. The existing paragraph validator checks measurements and
duplicates. Rust style references and headings are rejected. Python accepts
deferred records and nested properties.

The codec writes `lvl/pPr` before `lvl/rPr` (P1 2016 Annex A.1 CT_Lvl).
Omission leaves out `pPr`. Paragraphs retain their direct formatting.
The library leaves effective formatting to the consumer.

## Numbering level paragraph style links

Set `paragraph_style` on an abstract or replacement level to associate a
registered paragraph style. The compiler resolves style names and IDs using
the same registry as other style references. It accepts authored and imported
paragraph styles and rejects missing targets and other style kinds. Python
supports deferred style names. The codec writes the resolved style ID in
`lvl/pStyle` before `isLgl` (P1 2016 Annex A.1 CT_Lvl). Omission leaves
out `pStyle`. Abstract-level numbering-style associations remain separate.

## Numbering styles

Add `NumberingStyleDefinition { name, numbering_id }` to the document's styles
or pass `d.NumberingStyleDefinition(name, numbering_id)`. It creates a style of
type `numbering` with `pPr/numPr/numId`. The concrete target must be an authored,
nonzero definition. Its abstract definition receives the matching `styleLink`.
One abstract definition can own one style, even when multiple concrete IDs
share it. Existing style names and IDs cannot be replaced by a numbering style.
Python supports deferred fields.

The relationship follows P1 2016 Annex A.1 CT_AbstractNum and CT_Style. The
catalog's `Catalog Bullets` style targets concrete ID 61 of abstract ID 100.

## Inheriting numbering styles

Set `NumberingDefinition.inherit_style` to an authored numbering style name or
ID. A level-free abstract definition contains `numStyleLink`, and its concrete
instance references that definition. The target numbering style points to a
definition with local levels. This subset rejects local levels and overrides on
an inheriting definition and disallows chaining through another inheriting
definition. Imported numbering styles are preserved. Python accepts deferred
style names. See P1 2016 Annex A.1 CT_AbstractNum.
