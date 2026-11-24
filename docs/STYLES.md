# Styles and defaults

```python
from rocx import declarative as d

styles = [
    d.ParagraphStyleDefinition(name="Lead", linked_style="Emphasis"),
    d.CharacterStyleDefinition(name="Emphasis", linked_style="Lead", italic=True),
]
run = d.TextRun("Introduction", style="Emphasis", bold=False)
document = d.Document([d.Section(children=[d.Paragraph(children=[run])])], styles=styles)
```

## Style references and inheritance checks

References accept names (including spaces) or IDs of authored or imported styles. Forward references work. Unknown direct targets fail with `unknown_style`; wrong kinds or reachable `base_style` cycles fail with `invalid_content`.

| Reference | Target | Update behavior |
| --- | --- | --- |
| `base_style` | Same kind as the new style | Creation only for paragraph and character styles. Duplicate table styles fail. |
| `next_paragraph_style` | Paragraph style, including self | May update an existing paragraph style. |
| `linked_style` | Opposite paragraph/character kind | Sets a one-way link; omit to preserve. No clear operation. |

Template styles without an explicit kind count as paragraph styles. The viewer determines effective formatting.

## Character styles and direct run references

Use Rust `CharacterStyleDefinition { name, base_style, format, .. }` and `RunFormat.style`, or Python `CharacterStyleDefinition` and `TextRun(style=...)`. A character style takes run formatting (`size` in points in Python, versus paragraph style `font_size`); use `base_style`, not a run style reference, to inherit. Direct runs, hyperlink text and cached field runs can reference character styles and add direct overrides. See [script formatting and style toggles](FORMATTING.md#language-and-complex-script-formatting).

## Linked styles

For a reciprocal pair, declare **both** links as above. A one-way link creates no backlink. Pairing is not inheritance: reciprocal links do not form a base-style cycle, copy formatting or substitute the linked style for a direct paragraph/run reference.

## Conditional table paragraph formatting

`TableSectionFormat` supports paragraph alignment, before/after spacing, `line_height`, `tab_stops` and `bidi` alongside run formatting and shading. All 13 region inputs use the [paragraph units and validation rules](FORMATTING.md#paragraph-line-height). `whole_table` writes base style properties; named regions write `tblStylePr`. An empty tab list does not clear inherited stops. Use [TableLook](TABLES.md#conditional-table-style-regions) on the table to activate regions.

## Conditional table run script formatting

The same regions accept `language`, `bold_complex_script`, `italic_complex_script`, `size_complex_script`, `rtl`, `complex_script`, `font_east_asia` and `font_complex_script` under the [run formatting rules](FORMATTING.md#language-and-complex-script-formatting). Neither paragraph nor run properties are copied into cell content.

## Document defaults

```python
defaults = d.DocumentDefaults(
    run=d.RunDefaults(font="Aptos", size=11, language=d.RunLanguage(latin="en-US")),
    paragraph=d.ParagraphDefaults(spacing_after=6, line_height=d.LineHeight.multiple(1.15)),
)
document = d.Document([], defaults=defaults)
```

Rust `DocumentDefaults` takes optional `RunFormat` and `ParagraphFormat`; snapshots expose `defaults()`. Defaults and styles can be deferred; Python supports nested fields. No run/paragraph style references or headings in defaults.

- Omitted defaults retain their template values; explicit false and zero override them.
- Paragraph lengths, **including indents**, use points (legacy paragraph-style indents use inches). Run sizes round to half-points. Explicit indents clear competing character-unit attributes; first-line and hanging indents replace one another.
- A nonempty tab list replaces the local list; an empty list preserves it. Literal fonts clear only matching theme-font slots. Theme-font authoring is unsupported.
