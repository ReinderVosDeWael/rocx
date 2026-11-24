# Formatting and drawings

The examples use `from rocx import declarative as d`. Optional flags have three states: omit to leave a setting unspecified, or explicitly set true or false. The viewer applies styles, lays out pages and chooses fonts.

## Rust chainable authoring setters

```rust
use rocx::{ParagraphFormat, RunFormat, TextRun};
let title = TextRun::new("Participant").with_format(RunFormat::default().with_bold(true));
let format = ParagraphFormat::default().with_spacing_after(6.).with_keep_with_next(false);
```

`with_<field>` consumes an authoring record and replaces the field (including whole collections). Strings accept `Into<String>`; optional record properties take the plain value. Clear an option by assigning `None`. Component setters for deferred fields take `Into<Value<T>>`; for an optional `T`, pass `Some(value)`. Use `when` for conditions, `with_text` **or** `with_children` for paragraph content, and `with_type` for `r#type`. Setters and struct literals use the same validation. Python uses keyword arguments.

## Paragraph line height

```rust
use rocx::{LineHeight, Paragraph, ParagraphFormat};
let p = Paragraph::new("Measured lines")
    .with_format(ParagraphFormat::default().with_line_height(LineHeight::Exact(14.25)));
```

```python
p = d.Paragraph(text="Measured lines", line_height=d.LineHeight.exact(14.25))
style = d.ParagraphStyleDefinition(name="Minimum", line_height=d.LineHeight.at_least(18))
```

| Line height | Rust / Python | Unit |
| --- | --- | --- |
| Multiple | `Multiple` / `.multiple` | Lines, rounded to 1/240 line |
| Exact | `Exact` / `.exact` | Points, rounded to 1/20 pt; may clip |
| Minimum | `AtLeast` / `.at_least` | Points, rounded to 1/20 pt; may expand |

### Validation and preservation

Use either `line_height` or the `line_spacing` multiplier, never both. Values must be finite, nonnegative and fit signed 64-bit units after rounding; zero is valid. Before/after paragraph spacing is independent. Style updates preserve omitted spacing attributes.

## Custom tab stops

```rust
use rocx::{ParagraphFormat, TabAlignment, TabLeader, TabStop};
let format = ParagraphFormat::default().with_tab_stops(vec![TabStop {
    position: 108., alignment: TabAlignment::Right, leader: Some(TabLeader::Dot),
}]);
```

```python
p = d.Paragraph(text="Label\tValue", tab_stops=[d.TabStop(108, "right", "dot")])
```

- Positions are signed points from the text margin, rounded to signed 64-bit twips and sorted. Nonfinite values or duplicate rounded positions (even clear/set pairs) fail.
- Alignments: `left`, `center`, `right`, `decimal`, `bar`, `clear`, `start`, `end`; start/end are direction-relative. `clear` removes an inherited stop at that position. `bar` draws a bar rather than advancing text.
- Leaders: omitted, `none`, `dot`, `hyphen`, `underscore`, `heavy`, `middle_dot`. Explicit `none` differs from omission. Clear/bar stops do not fill space with leaders.
- Insert a literal tab or `Tab` to advance. A nonempty style update replaces local stops; an empty list preserves them and does not clear inherited/default stops.

## Language and complex-script formatting

```python
run = d.TextRun("العربية 日本語", language=d.RunLanguage(east_asia="ja-JP", bidi="ar-SA"),
                font_east_asia="East Asian Font", font_complex_script="Arabic Font",
                size_complex_script=14.5, bold_complex_script=True, rtl=True)
```

| Run or style property | Meaning |
| --- | --- |
| `language` | `RunLanguage(latin=..., east_asia=..., bidi=...)`: XML strings, not validated language tags. An empty record emits nothing; omitted slots survive style updates. |
| `bold_complex_script`, `italic_complex_script` | Independent of Latin bold/italic. Direct false disables; style bold/italic are toggles, so false does not cancel an inherited style toggle. |
| `size_complex_script` | Nonnegative finite points, rounded to checked half-points; independent of `size`. |
| `rtl`, `complex_script` | Request right-to-left run direction or complex-script formatting. Neither reverses stored text. |
| `font`, `font_east_asia`, `font_complex_script` | Literal font names. `font` sets ASCII and high ANSI only. Updating a slot clears its matching theme-font attribute. No font embedding or theme-font authoring. |

See also [conditional styles and document defaults](STYLES.md).

## Paragraph text direction

Set `bidi` on a paragraph format or style for right-to-left paragraph layout. This does not change run `rtl`, cell flow, text order or explicit physical indents/alignment.

## Section page direction

`SectionProperties.bidi` requests right-to-left **page** direction (e.g. column order). It does not mirror margins or change paragraph direction. Omitted section settings retain preceding-section or template values; false explicitly disables a flag.

## Right-side gutter placement

`SectionProperties.rtl_gutter` requests a right-side gutter without changing its width. Document-level `gutter_at_top` can select a top gutter instead.

## Section line numbering

```python
numbering = d.LineNumbering(count_by=5, start=0, restart="newPage")
section = d.SectionProperties(line_numbering=numbering)
```

`count_by`: 1–100; `start`: 0–32767 lines to skip; `distance`: unsigned twips. `restart` accepts `newPage`, `newSection`, `continuous` (Rust `LineNumberRestart`). Omitted fields preserve existing settings. Remove numbering with `LineNumbering(clear=True)` or `LineNumbering::default().with_clear(true)`, with no other fields. Later sections may enable it again.

Set `suppress_line_numbers` on paragraph formats, styles, defaults or numbering-level paragraph formats. False explicitly lets section numbering apply.

## Header and footer edge distances

Rust `PageMargins.header` / `.footer` and Python `SectionProperties.page_margins` keys `header` / `footer` take nonnegative EMUs, including zero. They change edge distances, not body margins or story references.

## Page borders

`SectionProperties.page_borders` accepts `PageBorders`. Set any of `top`, `left`, `bottom`, `right` to a `PageBorder`: `single` or `dashed`, optional RGB, width 2–96 eighth-points, offset 0–31 points. Replacing an edge preserves other edges. Omitted options preserve their values:

| Option | Values |
| --- | --- |
| `offset_from` | `page`, `text` |
| `display` | `allPages`, `firstPage`, `notFirstPage` |
| `z_order` | `front`, `back` |

## Section document grids

`SectionProperties.document_grid` takes `DocumentGrid`: mode `default`, `lines`, `linesAndChars` or `snapToChars`; positive line pitch in twips; signed character-pitch delta in 1/4096 pt relative to the Normal font. Only supplied attributes change. Set `snap_to_grid` on paragraphs for line snapping or on runs for character snapping, including styles and defaults.

## Contextual spacing and mirrored paragraph indents

On paragraph formats, styles and defaults, `contextual_spacing` suppresses spacing between paragraphs of the same style; `mirror_indents` interprets left/right as inside/outside without changing their values.

## Document page-layout requests

`DocumentDefaults.mirror_margins` and `.gutter_at_top` request mirrored margins and a top gutter. Omission retains template settings. Neither changes section geometry.

## Picture geometry and floating placement

`ImageTransformation` (Python: `transformation` dictionary) supports crop insets as percentages on physical edges, clockwise `rotation` in degrees, and `flip_horizontal` / `flip_vertical`. Cropping must leave a positive rectangle after rounding. These settings preserve image bytes and target dimensions.

Add an `ImageAnchor` for floating placement. Each axis accepts alignment **or** a signed point offset fitting the DrawingML coordinate range after EMU conversion. Both axes support page, margins, inside margin and outside margin. Horizontal also supports column, character and left/right margins; vertical also supports paragraph, line and top/bottom margins.

| Wrap | Options |
| --- | --- |
| None | No wrapping |
| Square | Both sides (default), left, right or largest side; nonnegative point distances on each edge |
| Top and bottom | Top and bottom distances |
| Tight / through | Same side choices as square, left/right distances; supply at least three ordered wrap-path vertices with nonzero signed area. The viewer closes the polygon. |

Anchors also support relative stacking height, behind-text placement, locking (default off), in-cell layout (on) and overlap (on).

## Inline geometric shapes

`ShapeRun` creates a rectangle or ellipse in a body paragraph. Supply positive point dimensions, optional RGB fill and `ShapeOutline` with RGB color and point width (0–20,116,800 EMUs after conversion). Shapes use DrawingML, not image media. Plain text uses one paragraph per line, including blank lines. Absent text omits the text body; `""` produces an empty paragraph.

## Shape text styling

```python
shape = d.ShapeRun(text="Label\n\nValue", text_format=d.ShapeTextFormat(
    font_size=14.25, bold=False, italic=True, font="Arial", color=(255, 255, 255),
    alignment="center", vertical_alignment="bottom"))
```

`ShapeTextFormat` styles all lines, including empty ones, independently of surrounding runs. Horizontal alignment: left/center/right; vertical: top/center/bottom. Font size: finite 1–4000 pt **before** rounding to 0.01 pt, even if text is absent. `font` selects the Latin typeface only (any XML string, including empty); color applies to text, not fill. Omit a property to leave it unspecified.

## Standalone inline text boxes

```rust
use rocx::{ShapeGeometry, ShapeProperties, ShapeRun};
let label = ShapeRun::new(ShapeProperties::default().with_geometry(ShapeGeometry::TextBox)
    .with_width(144.).with_height(48.).with_text("Standalone\nText box"));
```

```python
label = d.ShapeRun(144, 48, geometry="text_box", text="Standalone\nText box")
```

A text box uses plain DrawingML text, not rich blocks. Omitted text produces one empty paragraph. Fill and outline are invalid. Shapes and text boxes are body-paragraph content, not header/footer, note, hyperlink or field-result content. No text-flow linking, padding or autofit.

## Floating shapes and text boxes

Pass `ShapeProperties::with_anchor(ImageAnchor)` or `ShapeRun(anchor=...)` to float a rectangle, ellipse or text box. The same [placement and wrapping rules](#picture-geometry-and-floating-placement) apply; supply your own polygon for tight/through wrapping. Omit the anchor for inline placement.

## Inline column charts

```python
chart = d.ColumnChartRun(288, 180, ["North", "South", "West"], [8, 13.5, 5])
p = d.Paragraph(children=[chart])
```

Use `ColumnChartRun` in a body paragraph, including body table cells. Dimensions are in points. Supply at least one category and one finite value per category (zero and negatives allowed). Data is stored in the chart; no embedded workbook. Only one inline series; no chart styles, floating placement, imported chart edits, or charts in hyperlinks, field results or inline controls.

## Inline fractions

```rust
use rocx::{InlineFraction, Paragraph};
let p = Paragraph::from_children(vec![InlineFraction::new("1", "2").into()]);
```

```python
p = d.Paragraph(children=[d.InlineFraction("1", "2")])
```

Both operands are nonempty, single-line XML text; spaces are preserved and operands can be deferred independently. Use a fraction directly in a body paragraph or body table-cell paragraph. No display equations, nested expressions, equation styling or imported equation edits.
