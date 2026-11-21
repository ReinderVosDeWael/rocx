# Formatting coverage

## Rust chainable authoring setters

Public authoring property records support consuming `with_<field>` setters next
to their definitions. These are an additive alternative to struct literals:

```rust
use rocx::{ParagraphFormat, RunFormat, TextRun};

let run = TextRun::new("Participant (REDCap ID): ")
    .with_format(RunFormat::default().with_bold(true));
let paragraph_format = ParagraphFormat::default()
    .with_spacing_after(6.)
    .with_keep_with_next(false);
```

- Optional properties accept the plain value and store `Some(value)`. No call
  leaves the original value unchanged, including `None`; `with_bold(false)` is
  explicitly false, not unspecified. Repeating a setter replaces the prior value.
- Strings accept `impl Into<String>` (borrowed or owned). Other value properties
  accept their existing field type. Collection setters replace the whole
  collection, rather than appending or merging.
- Nested records compose normally, for example
  `TableCellProperties::default().with_margins(CellMargins::default().with_top(3.))`.
- Existing public fields, struct literals and defaults remain available. Clear
  optional properties with field assignment (`format.bold = None`). Setters do
  not clear conflicting properties or validate early: existing validation still
  runs at its existing boundary. For example, setting both `line_spacing` and
  `line_height` remains an error.

Coverage includes `RunFormat`, `RunLanguage`, `ParagraphFormat`, `TabStop`,
`DocumentDefaults`, page size/margins/numbering, columns and section properties;
paragraph/character/table style definitions and conditional table formatting;
table/row/cell properties, cell margins, table-look flags and border records;
numbering definitions/levels/start overrides/references; image transformations
and alt text; document/comment metadata, note targets, hyperlink and field
properties. `with_type` sets fields named `r#type`.

Authoring components also expose setters for their configurable fields alongside
their existing constructors, `with_format`, `with_properties`, `with_children`
and `when` methods. Deferred fields accept `impl Into<Value<T>>`, preserving
ready values, property references and owned futures. When `T` is optional, use
`Some(value)` or an existing `Value<Option<T>>`, for example
`Section::default().with_properties(Some(SectionProperties::default().with_bidi(false)))`.
Conditions continue to use `when`; paragraph content continues to use `with_text`
or `with_children`, preserving its exclusive representation.

Immutable resolved snapshots, checked identifiers/measurements and infrastructure
records are not mutable authoring configuration and gain no setters. Python keeps
its existing keyword arguments; no Python builder API or wire format is added.
Tests compare setters to literals, cover all value fields and deferred component
inputs, and require exact DOCX bytes for equivalent RunFormat authoring. Topic 02
of both visual catalogs labels omitted, enabled and explicitly disabled bold.

## R6a: paragraph line height

Paragraphs and paragraph styles share `LineHeight` and the same spacing codec.
This authors direct properties; it does not calculate effective style values,
font metrics, pagination or rendered line boxes.

| Input | Rust | Python | Wire value / rule |
| --- | --- | --- | --- |
| Multiple | `LineHeight::Multiple(1.5)` | `LineHeight.multiple(1.5)` | `360` / `auto` |
| Exact points | `LineHeight::Exact(14.25)` | `LineHeight.exact(14.25)` | `285` / `exact` |
| Minimum points | `LineHeight::AtLeast(18.)` | `LineHeight.at_least(18)` | `360` / `atLeast` |
| Legacy multiplier | `line_spacing: Some(1.5)` | `line_spacing=1.5` | `360` / `auto` (unchanged) |
| Absent | `line_height: None` | `line_height=None` | No line attributes authored; inheritance remains consumer-side |

Rust example:

```rust
use rocx::{LineHeight, Paragraph, ParagraphFormat};
let paragraph = Paragraph::new("Measured lines").with_format(ParagraphFormat {
    line_height: Some(LineHeight::Exact(14.25)),
    spacing_after: Some(6.),
    ..Default::default()
});
```

Python example:

```python
from rocx import declarative as d
paragraph = d.Paragraph(text="Measured lines", line_height=d.LineHeight.exact(14.25))
style = d.ParagraphStyleDefinition(name="Minimum", line_height=d.LineHeight.at_least(18))
```

The property is named `line_height` on `ParagraphFormat` and
`ParagraphStyleDefinition` in Rust, and on `Paragraph` and
`ParagraphStyleDefinition` in Python. Rust defers the format or styles aggregate
through `Value`/`PropertyRef`; Python uses its existing generic deferred-field
interpreter, including `paragraph.deferred("line_height")`. Conditions skip
producers normally. `LineHeight` is a value, not a new component or resolver.

### Validation and preservation

- Setting both `line_spacing` and `line_height` is `invalid_content`, including
  on styles. No precedence rule silently discards either input.
- All values must be finite, nonnegative and fit the existing checked signed
  64-bit wire conversion after rounding. Negative rejection is library policy,
  not a claim that the schema's signed measure forbids negative numbers. Zero is
  accepted, including values rounding to zero; positive visual height is not
  guaranteed. Exact heights can clip content and minimum heights may expand.
- Multiple values round to the nearest 1/240 line; exact/minimum points round to
  the nearest twip (1/20 point), using the existing checked measurement helper.
- Space before/after the paragraph remains independent and shares `w:spacing`.
  Direct properties do not materialize inherited line-height values. When a style
  is updated, only supplied spacing attributes are merged; omitted attributes are
  retained. There is no new typed import or template text-edit operation here.
- Rust exhaustive struct literals need the new `line_height` member, usually
  avoided with `..Default::default()`. Python's new optional argument is appended
  so existing positional arguments keep their meanings.

Evidence: `tests/line_height.rs`, `tests/python_bindings/test_line_height.py`,
official-schema checks in `test_standards.py`, and Rust-generated independent
python-docx readback in `tests/python_docx.rs`. Readback mutations change units,
rules and style values and must fail after loading. Invalid rule mutations fail
official schemas in both document and styles parts. These are serialization and
consumer-object checks, not manual Word rendering or visual-equivalence evidence.

Normative source: ECMA-376 P1 (2016) §17.3.1.33 (`spacing`, `line`, `lineRule`)
and Annex A.1. Other R6 formatting/style/list/table work remains on the roadmap.

## R6b: custom tab stops

`ParagraphFormat.tab_stops` and `ParagraphStyleDefinition.tab_stops` take
`Vec<TabStop>` in Rust. Python's `Paragraph` and `ParagraphStyleDefinition` append
an optional `tab_stops` list, defaulting to empty. The format/styles aggregate in
Rust and per-field references in Python defer these values through existing
resolution, without a tab-specific interpreter.

```rust
use rocx::{ParagraphFormat, TabAlignment, TabLeader, TabStop};
let format = ParagraphFormat {
    tab_stops: vec![TabStop {
        position: 108., alignment: TabAlignment::Right, leader: Some(TabLeader::Dot),
    }],
    ..Default::default()
};
```

```python
paragraph = d.Paragraph(text="Label\tValue", tab_stops=[d.TabStop(108, "right", "dot")])
style = d.ParagraphStyleDefinition(name="Tabbed", tab_stops=[d.TabStop(72)])
```

| Property | Supported values / meaning |
| --- | --- |
| Position | Signed points, rounded to nearest twip, relative to the paragraph text margin; negative values can extend into the margin |
| Alignment | Rust `TabAlignment::{Left,Center,Right,Decimal,Bar,Clear,Start,End}`; Python `left/center/right/decimal/bar/clear/start/end` |
| Leader | Absent or Rust `TabLeader::{None,Dot,Hyphen,Underscore,Heavy,MiddleDot}`; Python `none/dot/hyphen/underscore/heavy/middle_dot` |
| Empty list | No local overrides authored; **not** a request to clear inherited/default stops |
| Explicit clear | `TabAlignment::Clear` / `"clear"` at a known position requests removal of that inherited stop |

The writer emits `w:pPr/w:tabs/w:tab` in ascending position order. Positions must
be finite and fit the existing signed 64-bit checked twip conversion. Duplicate
positions after rounding fail with `invalid_content`, including clear/set pairs
in one local list. This is a conservative library policy, not an XSD uniqueness
constraint. Absent and explicit `none` leaders remain distinct in XML. Leaders
on clear/bar entries are retained but do not create leader-filled tab spacing.
`start`/`end` are direction-relative; `left`/`right` use the Transitional vocabulary.
The backwards-compatible `num` tab type is not exposed.

Tab definitions do not insert text. Use an existing inline `Tab` component or a
tab character in text to advance to a stop. Bar tabs request a vertical bar,
not an advance. Effective tab sets remain additive across the style hierarchy;
the library does not compute their resolution, default tab intervals, decimal
separator behavior, bidi layout or leader rendering. A nonempty style update
replaces the **local** tab collection, while an empty list retains an imported
collection. Other style properties remain unchanged. Removing every inherited
or default stop is not represented by an empty list or a new global setting.

Evidence: `tests/tab_stops.rs`, `tests/python_bindings/test_tab_stops.py`,
independent Rust-generated python-docx readback, official-schema positive and
invalid alignment/leader/position mutations on document/styles parts. Changed
position/alignment/leader negative controls fail after successful loading. No
manual rendering or complete typed tab import is claimed. Source: P1 (2016)
§§17.3.1.37–38, §§17.18.84–85 and the Transitional schema's left/right values.

## R6e: language and complex-script formatting

Direct `RunFormat` and paragraph style definitions support independent script
properties; Python exposes the same fields on `TextRun`/`ParagraphStyleDefinition`.
Rust defers the run format or styles aggregate, and Python uses generic per-field
references. Neither computes effective values or transforms the stored text.

| Property | Encoding / meaning |
| --- | --- |
| `language: RunLanguage` | Optional `latin`, `east_asia`, `bidi` strings map to `lang` attributes `val`, `eastAsia`, `bidi` |
| `bold_complex_script`, `italic_complex_script` | Optional `bCs`/`iCs`, independent from Latin `b`/`i` |
| `size_complex_script` | Nonnegative finite points to checked half-points in `szCs`, independent from `sz` |
| `rtl` | Optional run `rtl` flag, not a text-reversal operation |
| `complex_script` | Optional `cs` flag requesting complex-script formatting |
| `font_east_asia`, `font_complex_script` | Literal `rFonts/eastAsia` and `rFonts/cs` names |
| Existing `font` | Still sets only `ascii` and `hAnsi`; it does not overwrite the other slots |

```python
run = d.TextRun("العربية 日本語", language=d.RunLanguage(east_asia="ja-JP", bidi="ar-SA"),
                font_east_asia="East Asian Font", font_complex_script="Arabic Font",
                size_complex_script=14.5, bold_complex_script=True, rtl=True)
```

Flag absence/false/true remains distinct. At style level, bold/italic (including
complex-script variants) are **toggle properties** under P1 §17.7.3: false does
not mean a computed disabling override across the whole style hierarchy. The
library serializes these values, not effective toggle inheritance. Direct false
values remain explicit direct-formatting overrides. Size values round to nearest
half-point with the existing signed conversion bound; zero is accepted.

Language slots follow the schema's unconstrained string type, not a BCP 47
registry parser. Strings (including empty/custom ones) are preserved and XML
escaped; invalid XML characters still fail serialization. They are hints for a
consumer's language/proofing behavior, not library-side proofing or shaping.
An all-absent record emits no `lang`. Partial style updates retain omitted slots.
Fonts are names only: no availability check, embedding, download, fallback,
script classification, font metrics or bidi rendering is performed. Style font
updates remove only the corresponding theme-font attributes, retaining other
literal/theme slots. Theme-font authoring remains outside this slice.
Conditional table-style script properties and document defaults use the same
codec; see [STYLES.md](STYLES.md) for their scopes and merge contracts.

Evidence: `tests/script_formatting.rs`, `test_script_formatting.py`, independent
Rust-generated python-docx API/XML readback and changed language/font/size/flag
negative controls, official schemas and invalid size/flag mutations in document/
styles parts, deferred references, hidden guards, and style-update preservation.
No visual/shaping equivalence is claimed. Sources: P1 (2016) §§17.3.2.2, 17.3.2.7,
17.3.2.17, 17.3.2.20, 17.3.2.26, 17.3.2.30, 17.3.2.39 and 17.7.3.

## R6i: paragraph bidi direction

`ParagraphFormat.bidi` and paragraph style `bidi` are optional booleans in Rust;
Python appends `bidi` to `Paragraph` and `ParagraphStyleDefinition`. The shared
paragraph codec emits `pPr/bidi` with explicit `val=1/0`; omission emits no request.
Generic deferred format/style aggregates and Python field references work as
for existing properties. Style and direct values remain distinct; no effective
inheritance is materialized.

Paragraph bidi is not run `rtl`, cell text flow, a string reversal, or automatic
alignment/indent conversion. The writer retains stored text and explicitly
supplied left/right alignment/indent values; the consumer interprets layout.
Tests exercise absent/false/true, generic references, style/direct separation,
unchanged text/indent values, Rust-generated independent loaded-XML readback and
bad-bool schema/semantic mutations. The installed wheel probe also checks the
request. No bidi shaping or visual-layout equivalence is claimed. Source: P1
(2016) §17.3.1.6 (`bidi`) and Annex A.1.

## R6j: section page direction

`SectionProperties.bidi` is an optional boolean in both APIs. It emits a
section-level `sectPr/bidi` request with explicit `val=1/0`, separate from
paragraph `pPr/bidi`, run `rtl` and cell text flow. Omission retains the previous
or template section property under the existing section inheritance contract;
on a fresh document it authors no bidi element. False explicitly overrides an
inherited true value. Python appends the field, preserving positional arguments.
Rust uses the existing deferred section-properties aggregate; Python supports
generic nested deferred fields and section property references.

This requests right-to-left **page** direction (for example, column population
order in a consumer), not text direction. The library does not reverse stored
text, mirror physical margins, change column counts, or calculate page layout.
Paragraph and run direction remain independently authored. See P1 (2016)
§17.6.1 and Annex A.1.

Evidence: `tests/section_direction.rs`, `test_section_direction.py`, independent
Rust-generated python-docx loaded-XML readback with a changed section flag
negative control, required-schema validation with an invalid boolean mutation,
template preservation, inherited/explicit values, deferred references, hidden
sections and repeatable snapshots. Installed-wheel probes cover the section flag.
No manual Office rendering or visual-layout equivalence is claimed.

## R6o: section gutter placement

`SectionProperties.rtl_gutter` is an optional boolean in both APIs. It writes
`sectPr/rtlGutter` with explicit `val=1/0`. Omission preserves previous/template
section settings and authors no override in a fresh document; false explicitly
disables a previous right-side request. It uses the existing deferred aggregate
in Rust and generic nested fields/references in Python.

This requests right-side gutter placement when a consumer uses a side gutter;
the document-level `gutterAtTop` setting can instead select a top gutter. The
library does not change that setting, author a gutter measurement, mirror physical
margins, adjust width-allocation metrics or calculate binding/page layout. Section
`bidi`, paragraph `bidi`, run `rtl` and cell text flow remain independent. Existing
template gutter amounts are retained. Python appends the field; Rust exhaustive
literals need it or `..Default::default()`.

Evidence: `tests/section_gutter.rs`, `test_section_gutter.py`, independent
Rust-generated loaded-XML readback with changed-flag negative control, official
schema validation with invalid boolean mutation, and clean-wheel probes. Tests
cover absence/false/true, inheritance, template preservation, deferred/hidden
inputs, repeated snapshots and unchanged text/margins/gutter amounts/top settings/
width metrics. No visual-rendering claim is made. Source: P1 (2016) §17.6.16.

### Section line numbering

Set `SectionProperties.line_numbering` to `LineNumbering`. The record supports
`count_by` (1–100), `start` (0–32767), `distance` (unsigned twips), and
`restart` (`newPage`, `newSection`, or `continuous` in Python). The starting
value is stored as supplied. Word interprets it as the number of lines to skip
before starting or restarting. Rust uses `LineNumberRestart` for the restart
mode. Python resolves deferred records and nested fields.

Explicit settings merge into `sectPr/lnNumType` with the same preceding-section
and template behavior as other section settings. An omitted record preserves
the existing node, and omitted fields preserve existing attributes. The
consumer lays out and displays line numbers. Source: P1 (2016) §17.6.8 and
Annex A.1 CT_LineNumber. The count and start bounds follow
[MS-OI29500 §2.1.215](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/c4252e13-48d2-4ac9-9fb5-3b276d02be30).

Set `suppress_line_numbers` on a paragraph format, paragraph style, or
paragraph defaults to request `pPr/suppressLineNumbers` (P1 2016 Annex A.1
CT_PPrBase). True suppresses line numbering, and explicit false requests
that the section's line numbering apply. Omission leaves the local setting
unspecified. The numbering level paragraph codec also accepts this property.

Use `LineNumbering(clear=True)` in Python or
`LineNumbering::default().with_clear(true)` in Rust to remove inherited or
template section line numbering. The clear request must omit other line
numbering fields. A later section can configure line numbering again.

### Header and footer edge distances

Set `PageMargins.header` and `PageMargins.footer` in Rust or use the
`header` and `footer` keys in Python's `SectionProperties.page_margins`.
Each accepts a nonnegative EMU value, including zero. The codec converts each
distance to twips and writes it to the corresponding `sectPr/pgMar` attribute.
Omitted values retain the template or preceding section's distances. Editing
these fields leaves body margins and story references intact. Source: P1
(2016) §17.6.11 and Annex A.1 CT_PageMar.

### Page borders

Set `SectionProperties.page_borders` to a `PageBorders` record with optional
`top`, `left`, `bottom`, and `right` borders. Each physical edge accepts a
`PageBorder` with line style `single` or `dashed`, optional RGB color,
width (2–96 eighth-points), and offset (0–31 points). These are the supported
line-border authoring bounds. The codec writes supplied edges in schema order.
Replacing one edge preserves other edges and the parent options. Omitted
records preserve template or previous-section borders. Source: P1 (2016)
§17.6.10 and Annex A.1 CT_PageBorders.

The `PageBorders` record also accepts `offset_from` (`page` or `text`),
`display` (`allPages`, `firstPage`, or `notFirstPage`), and `z_order`
(`front` or `back`). Rust exposes these as `PageBorderOffset`,
`PageBorderDisplay`, and `PageBorderZOrder`. An omitted option preserves the
existing setting. The consumer positions and displays the borders.

### Section document grids

`SectionProperties.document_grid` accepts a `DocumentGrid` record. Modes are
`default`, `lines`, `linesAndChars`, and `snapToChars` in Python and the
corresponding `DocumentGridMode` variants in Rust. The line pitch is positive
and uses twips. The signed character-pitch delta uses units of 1/4096 point
relative to the Normal-font pitch. The compiler writes `sectPr/docGrid`.
Supplied attributes merge with the existing template or preceding section
settings. The consumer determines effective text layout. Source: P1 (2016)
§17.6.5 and Annex A.1 CT_DocGrid.

`snap_to_grid` requests line-grid snapping on paragraph formats and
character-grid snapping on run formats, including styles and defaults.
True and false emit explicit requests. Omission leaves the current level of
formatting unspecified. The compiler writes `snapToGrid` in the paragraph
or run properties. See P1 (2016) Annex A.1 CT_PPrBase and CT_RPr.

### Contextual spacing and mirrored paragraph indents

`contextual_spacing` requests that the consumer suppress spacing between
paragraphs using the same paragraph style. `mirror_indents` requests that
paragraph left and right indent values act as inside and outside indents.
They are independent optional flags on paragraph formats, paragraph styles
and paragraph defaults. Omitted flags keep the current property level
unspecified. Explicit false emits `0`. The codec retains the original indent
values and writes `contextualSpacing` and `mirrorIndents` (P1 2016 Annex A.1
CT_PPrBase).

### Document page-layout requests

`DocumentDefaults.mirror_margins` and `gutter_at_top` write
`settings/mirrorMargins` and `settings/gutterAtTop`. These flags let the
consumer interpret mirrored margins and top gutter placement. Omitted
values retain settings from the template. False writes an explicit off
value. These requests leave section geometry and unrelated settings intact.
Source: P1 (2016) §§17.15.1.50 and 17.15.1.57.

### Picture geometry and floating placement

Set `ImageTransformation.crop` to an `ImageCrop` with physical-edge insets
in percent, and set `rotation` in clockwise degrees. Python uses the
corresponding `transformation` dictionary keys. Crop insets must leave a
positive source rectangle after rounding. Both requests preserve the target
extents and stored image bytes.

An optional `ImageAnchor` requests floating placement from the page or
margins independently on each axis. Signed offsets use points and must fit
a signed DrawingML coordinate after conversion to EMUs. The anchor supports
none, square, or top-and-bottom wrapping, a relative stacking height, and
placement behind text. Square wrapping defaults to both sides and accepts
left, right, or the largest side. Square wrap distances use nonnegative
points on each edge. Top-and-bottom wrapping accepts top and bottom distances.
Optional anchor locking, in-cell placement, and overlap flags default to
unlocked, layout in cell, and overlap allowed. The consumer performs page layout.

Each axis accepts an offset or alignment. Common reference frames include
the page, margins, inside margin and outside margin. Horizontal positioning
also accepts column, character, left margin and right margin. Vertical
positioning also accepts paragraph, line, top margin and bottom margin.

Tight and through wrapping take at least three ordered polygon vertices in
DrawingML wrap-path coordinates and require nonzero signed area. The codec
writes the first vertex as the start and the rest as line segments. The
consumer closes the polygon if needed. Both modes support left and right
wrap distances and the same side choices as square wrapping. Source:
[P1 wrapping polygon contract](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.wordprocessing.wrappolygon?view=openxml-3.0.1).

Picture transformations also accept optional `flip_horizontal` and
`flip_vertical` flags. Explicit false is preserved and the image bytes
and target extents remain the same.

### Inline geometric shapes

`ShapeRun` creates an inline rectangle or ellipse with positive dimensions
in points and an optional RGB fill. Rust takes a `ShapeProperties` record
and Python takes keyword arguments. Shapes are supported in body paragraphs.
The codec emits a standard DrawingML locked canvas and creates no image
media part. Geometry is serialized independently of the consuming app's
drawing behavior. Source: ECMA-376 P1 (2016), §§20.1 and 20.3.

An optional shape `text` value is stored in a separate DrawingML text
body. Line separators create DrawingML paragraphs, including empty lines.
The text stays inside the shape and uses its geometry as the text rectangle.

An optional `ShapeOutline` supplies a line width in points and an RGB
color. The width is checked against DrawingML's 0–20,116,800 EMU range.
Shape and canvas identifiers are reserved alongside existing drawing IDs.

### Shape text styling (R8l)

Optional `ShapeTextFormat` applies uniform styling to the shape's plain text:

```rust
use rocx::{ShapeProperties, ShapeRun, ShapeTextAlignment, ShapeTextFormat,
    ShapeTextVerticalAlignment};
let shape = ShapeRun::new(ShapeProperties::default().with_text("Label\n\nValue")
    .with_text_format(ShapeTextFormat::default().with_font_size(14.25)
        .with_bold(false).with_color([255, 255, 255])
        .with_alignment(ShapeTextAlignment::Center)
        .with_vertical_alignment(ShapeTextVerticalAlignment::Bottom)));
```

```python
shape = d.ShapeRun(text="Label\n\nValue", text_format=d.ShapeTextFormat(
    font_size=14.25, bold=False, color=(255, 255, 255),
    alignment="center", vertical_alignment="bottom"))
```

The supported horizontal subset is left/center/right; vertical alignment is
top/center/bottom. Rust uses dedicated DrawingML enums, not WordprocessingML
paragraph alignment. Python uses the lowercase tokens above and supports nested
deferred property values. Unsupported tokens are rejected.

Font size is in points and must be finite and within **1–4000 inclusive before
rounding**. It rounds to the nearest hundredth of a point for `ST_TextFontSize`
(100–400000). RGB color applies to text, independently of shape fill/outline.
`bold=None` omits the request; `False` explicitly disables bold.

The codec emits `bodyPr/@anchor`, per-line `pPr/@algn`, and run `rPr` with `sz`,
`b` and `solidFill/srgbClr`. Character requests also go on `endParaRPr`, so empty
lines retain the requested styling. Paragraph and run properties stay in schema
order. Missing individual values are not materialized; an omitted or empty
format preserves prior output bytes. If text is absent, no text body is emitted,
but invalid format values are still rejected during validation. Empty text is
distinct from absent text and produces one empty DrawingML paragraph.

This does not change surrounding WordprocessingML text, calculate line layout,
select fonts, fit text, paginate or position the shape. No manual Office rendering
was verified. Sources: P1 (2016) §20.1 text-body, paragraph and character
properties; Annex A.4 `CT_TextBodyProperties`, `CT_TextParagraph`,
`CT_TextCharacterProperties`, `ST_TextFontSize`, `ST_TextAlignType` and
`ST_TextAnchoringType`. Tests: `shape_text_format.rs`, `test_shape_text_format.py`,
the paired catalogs and schema/content negative controls.
