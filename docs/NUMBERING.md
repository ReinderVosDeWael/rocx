# Numbered lists

Paragraphs reference a **concrete list ID**. Its **abstract definition** supplies the level patterns. To restart a group independently, give it a new concrete ID. The viewer calculates counters and labels; paragraph text contains neither.

```python
from rocx import declarative as d

levels = [d.NumberingLevel(0, 1, "decimal", "%1.")]
first = d.NumberingDefinition(42, levels, abstract_id=100)
second = d.NumberingDefinition(43, levels, abstract_id=100,
    start_overrides=[d.NumberingStartOverride(0, 7)])
paragraph = d.Paragraph(text="Restarted list", numbering=d.NumberingReference(43))
```

## Shared abstract numbering definitions

Definitions sharing an explicit `abstract_id` must supply identical ordered levels. Omit it for a private definition: allocation starts above imported IDs and skips reserved IDs. Explicit abstract or concrete ID collisions with template numbering fail. Imported numbering is preserved, not remapped or exposed as a complete editable model.

## Concrete list start overrides

`NumberingStartOverride(level, start)` changes the concrete start without changing the abstract level. Levels are unique, zero-based (`0`–`8`); starts are unsigned 32-bit integers, including zero.

## Concrete level replacements

```python
definition = d.NumberingDefinition(42, levels,
    start_overrides=[d.NumberingStartOverride(0, 7)],
    level_overrides=[d.NumberingLevel(0, 4, "upperRoman", "(%1)")])
```

Each override list may target a defined level only once. A start override and a level replacement can target the same level. Neither changes the shared definition.

## Level options

| Property | Meaning |
| --- | --- |
| `is_legal=True` | Display references in this label as decimal; retain each level's stored format. Omission and explicit false differ. |
| `suffix` | `"tab"` (default when omitted), `"space"`, or `"nothing"` after the label. Explicit tab differs from omission in XML. |
| `restart` | **One-based** trigger: `level=2, restart=1` restarts after level 0. A positive trigger includes earlier levels; `0` means never. Omit to restart after the preceding level or earlier. Inapplicable unsigned 32-bit values pass through for the viewer to ignore. |
| `alignment` | `"left"`, `"center"`, `"right"`, `"start"`, `"end"` relative to the paragraph text margin. Default: left for LTR, right for RTL. Independent of paragraph alignment; `"justify"` is invalid. |
| `label_format` | Rust `RunFormat` / Python `RunDefaults`: direct label formatting, not paragraph text formatting. No run style references. |
| `paragraph_format` | Rust `ParagraphFormat` / Python `ParagraphDefaults`: lengths in points, including signed tab positions and first-line indents (negative = hanging). No heading or style reference. |
| `paragraph_style` | Name or ID of an authored or imported paragraph style. Missing targets and other kinds fail. |

Rust uses `NumberingSuffix` and `NumberingAlignment` variants corresponding to the Python tokens. Omitted properties emit no local override; explicit zero and false survive. Rust can defer the numbering collection; Python can also defer nested fields. Use `..Default::default()` for remaining Rust struct fields.

## Numbering styles

`NumberingStyleDefinition(name, numbering_id)` targets an authored, nonzero concrete ID. Each abstract definition can own at most one style, even when shared. Use a new style name and ID.

## Inheriting numbering styles

Set `NumberingDefinition.inherit_style` to an authored numbering style name or ID. Omit local levels and overrides on the inheriting definition. Its target must define levels; inheritance chains are unsupported.
