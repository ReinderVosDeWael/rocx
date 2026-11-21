# Table coverage

## R6v: inline table alignment

Rust `TableProperties.alignment: Option<TableAlignment>` supports `Left`,
`Center`, `Right`, `Start` and `End`; Python `Table(..., alignment="center")`
supports the corresponding lowercase tokens. Other tokens fail with
`invalid_content`; paragraph-style justification tokens are not table alignment.
These requests write `tblPr/jc/@val`, ordered after `tblW`. The codec is shared
by both frontends, including nested and header/footer tables.

```python
table = d.Table(rows, column_widths=[1440, 1440], alignment="center")
```

Omission emits no `jc`, preserving prior output and style/consumer defaults.
The specification defers omitted alignment to table styles and otherwise uses
left justification with zero indentation from the leading margin; table direction
affects interpretation. The library preserves the token rather than resolving
style inheritance or direction. Rust `ResolvedTable.alignment()` returns the
source optional enum, not an inferred effective alignment.

These are inline-table alignment requests, not floating-table positioning or
page coordinates. They do not change grids, widths, merges, cell text alignment,
paragraph alignment, indentation or `bidiVisual`. A narrow table makes the request
easier to inspect; a full-width table may look unchanged. The viewer calculates
placement. Nested-table alignment remains independent of its parent, and adding
a newly aligned table does not rewrite imported template tables.

Rust uses the existing deferred TableProperties aggregate. Python supports
deferred alignment fields/references; hidden tables skip producers. Snapshots
remain repeatable. Python appends the argument; exhaustive Rust TableProperties
literals need `alignment` or `..Default::default()`.

Evidence: `tests/table_alignment.rs`, `test_table_alignment.py`, independent
Rust-generated python-docx readback with a changed-alignment negative control,
official body/header schema validation and invalid-token rejection, wheel probes,
independent catalog mutation checks, and exact Rust/Python catalog byte parity.
All five tokens/omission, widths/text alignment, source getters, deferred/hidden
fields, nested tables and template boundaries are checked. Topic 04 labels five
narrow fixed-width alignment tables. No manual rendering, effective alignment or
floating-table support is claimed. Source: P1 (2016) §17.4.28 and Annex A.1
CT_TblPrBase/CT_JcTable, ST_JcTable.

## R6c: row heights and pagination requests

Rust uses `TableRow.properties: Value<TableRowProperties>` and
`.with_properties(...)`. Resolved rows expose read-only `.properties()`; their
cells stay independent immutable content. Python appends `height`, `repeat_header`
and `cant_split` properties to `TableRow`. Both frontends reuse generic deferred
resolution, conditions and property references. No row-specific evaluator exists.

```rust
use rocx::{RowHeight, TableRow, TableRowProperties};
let row = TableRow::new(vec![]).with_properties(TableRowProperties {
    height: Some(RowHeight::AtLeast(18.)),
    repeat_header: Some(true),
    cant_split: Some(true),
});
```

```python
from rocx import declarative as d
row = d.TableRow([d.TableCell([d.Paragraph(text="Header")])],
                 height=d.RowHeight.at_least(18), repeat_header=True, cant_split=True)
```

| Property | Supported values | Wire encoding |
| --- | --- | --- |
| Height | Rust `RowHeight::Exact(points)` / `AtLeast(points)`; Python `RowHeight.exact(points)` / `.at_least(points)` | `trPr/trHeight` with twip `val` and `hRule=exact/atLeast` |
| Repeat header | Optional bool (`repeat_header`) | `trPr/tblHeader` with `val=1/0` |
| Cannot split | Optional bool (`cant_split`) | `trPr/cantSplit` with `val=1/0` |
| Absent | Default `None` for all properties | No corresponding element; no empty `trPr` when all absent |

Height values must be finite, nonnegative points and fit the existing checked
signed 64-bit twip conversion. They round to the nearest 1/20 point. Zero and
values rounding to zero are accepted. The wire height is an unsigned measure;
the checked helper's signed limit is a library bound, not the full schema range.
Omission leaves sizing to the consumer and applicable defaults; this slice does
not expose an explicit `auto` override for inherited heights. Exact heights may
clip content; minimum heights may expand. No font metrics are measured.

Absent, false and true flags remain distinct. Header repetition is only effective
for a contiguous header group at the beginning of a table. The writer preserves
requests on later rows without reordering rows, marking earlier rows or claiming
the later row will repeat. `cant_split` requests one-page placement **if the row
fits**; a row taller than a page may still flow across pages. The library does not
calculate pagination, page heights, repeated copies or effective style values.

Rust resolves the row's children list and properties concurrently and skips both
when its condition is false. Share cloneable row properties through a
`PropertyRef`, not owned authoring cell futures. Python supports per-field
references such as `source.deferred("height")`; snapshots can render repeatedly.

There is no new row template-edit/import model or table-style row authoring here.
Existing grid widths, spans, merges, padding and nested-cell paragraph handling
remain unchanged; this slice is not completion of the remaining R6 table work.

Evidence: `tests/row_properties.rs`, `tests/async_fields.rs`,
`tests/python_bindings/test_row_properties.py`, Rust-generated independent
python-docx readback, official-schema checks including nested/header tables,
and clean-wheel probes. Changed heights/rules/header/split flags fail readback
after loading; invalid height/rule/bool mutations fail schemas. Row flags use
loaded XML where python-docx has no high-level getter. These checks establish
serialization, not visual pagination equivalence or manual Word rendering.

Sources: ECMA-376 P1 (2016) §§17.4.6 (`cantSplit`), 17.4.49 (`tblHeader`),
17.4.80 (`trHeight`) and Annex A.1.

## R6d: cell margins and text flow

`TableCellProperties` adds `margins: Option<CellMargins>` and
`text_direction: Option<CellTextDirection>`. Resolved cells expose read-only
getters. Python appends `margins` and `text_direction` to `TableCell`:

```python
cell = d.TableCell([d.Paragraph(text="Vertical")],
                   margins=d.CellMargins(top=3, start=0, bottom=6, end=9),
                   text_direction="tbRl")
```

Margins are optional nonnegative point values per top/start/bottom/end edge,
rounded to twips with checked signed-64-bit conversion. Omitted sides remain
omitted; explicit zero authors a zero-width override. `None` and an all-absent
margin record produce no `tcMar`. The codec emits only fixed `dxa` widths, not
percentage/auto/nil cell margins. Defaults and effective margin inheritance are
consumer behavior. Cell margins do not shrink the library's existing image-width
context or recalculate the grid; no content-fit guarantee follows.

Text direction uses these checked values:

| Rust `CellTextDirection` | Python / Transitional wire value |
| --- | --- |
| `LeftToRight` | `lrTb` |
| `TopToBottomRightToLeft` | `tbRl` |
| `BottomToTopLeftToRight` | `btLr` |
| `LeftToRightVertical` | `lrTbV` |
| `TopToBottomRightToLeftVertical` | `tbRlV` |
| `TopToBottomLeftToRightVertical` | `tbLrV` |

Direction is independent of paragraph bidi and stored text order. It requests
consumer text flow without rotating content or implementing vertical shaping.
The alternative short/Strict spellings are not public inputs. Omission authors
no direction override. Generic deferred cell properties/field references and
hidden-cell guards remain unchanged; there is no special layout evaluator.

Evidence: `tests/cell_layout.rs`, `test_cell_layout.py`, independent Rust-generated
python-docx loaded-XML readback and negative margin/direction mutations, official
schema controls (bad direction, margin type/width), and installed-wheel probes.
All six directions and partial/zero/invalid margins are covered. python-docx has
no complete high-level margin/direction API, so readback uses its loaded cell XML.
No manual rendering or effective-layout equivalence is claimed. Sources: P1
(2016) §§17.4.68 (`tcMar`), 17.4.72 (`textDirection`), 17.18.93 and Annex A.1.

## R6q: table-look activation

`TableProperties.look: Option<TableLook>` in Rust and `Table(..., look=TableLook(...))`
in Python request which conditional regions of a table style the consumer should
apply. The six optional booleans are `first_row`, `last_row`, `first_column`,
`last_column`, `horizontal_banding` and `vertical_banding`. Python example:

```python
table = d.Table(rows, style="My Table Style", look=d.TableLook(
    first_row=True, last_row=True, first_column=True, last_column=True,
    horizontal_banding=True, vertical_banding=True))
```

Omitted records/fields retain the existing **library defaults**: first row,
first column and horizontal banding enabled; last row, last column and vertical
banding disabled. These are not the normative defaults for omitted `tblLook`.
The library has historically emitted an explicit look record; that behavior and
its `04A0` mask remain byte-compatible when the new property is omitted or empty.
False disables an enabled-by-default request. This applies to newly authored
tables, not to typed editing or inference from an imported table's look.

The codec writes all six explicit `tblLook` attributes plus a consistent four-digit
uppercase `val` mask. Banding uses positive API names, so false maps to
`noHBand=1` or `noVBand=1`. First/last row/column bits are 0020/0040/0080/0100;
the two no-banding bits are 0200/0400. All-enabled emits `01E0`; all-disabled
emits `0600`. Snapshot getters preserve source omission instead of materializing
fallback values: Rust `ResolvedTable.look()` returns the optional record.

These are activation requests only. The library does not compute conditional
precedence/effective styles, copy properties into cells or alter rows, grids,
widths, merges or pagination. Whole-table formatting is unconditional. Applicable
style definitions and consumer behavior still determine visible results. Newly
authored defaults do not rewrite imported tables in a template.

Rust uses the existing deferred table-properties aggregate. Python supports
nested deferred look fields and per-table look references; hidden tables skip
their producers and resolved snapshots remain repeatable. Python appends the
Table argument; Rust exhaustive property literals need `look` or
`..Default::default()`.

Evidence: `tests/table_look.rs`, `test_table_look.py` (all 64 flag combinations,
mask consistency, default byte compatibility, partial/deferred/hidden inputs,
template preservation and unchanged direct properties), independent Rust-generated
loaded-XML readback with changed-flag negative control, official schema checks
for body/header tables and invalid bool/mask mutations, wheel probes and both
visual catalogs. Topic 05 explicitly enables all six flags for all 13 region
tables; topic 04 shows all-disabled behavior. No manual rendering or
effective-style guarantee is claimed. Source: P1 (2016) §17.4.55 and Annex A.1.
