# Table layout

```python
from rocx import declarative as d

cell = d.TableCell([d.Paragraph(text="Header")],
    margins=d.CellMargins(top=3, start=0, bottom=6, end=9), text_direction="tbRl")
row = d.TableRow([cell], height=d.RowHeight.at_least(18), repeat_header=True, cant_split=True)
table = d.Table([row], column_widths=[1440], alignment="center")
```

## Inline table alignment

`TableProperties.alignment` (Rust `TableAlignment`) accepts left, center, right, start or end, including on nested and header/footer tables. Python uses lowercase strings; other tokens fail with `invalid_content`. Start/end are direction-relative. Omission leaves placement to the style, otherwise to the leading margin with zero indent. `ResolvedTable.alignment()` reports the request, not the effective value. Table alignment does not change the grid, width, paragraph alignment or floating position.

## Row heights and pagination requests

```rust
use rocx::{RowHeight, TableRow, TableRowProperties};
let row = TableRow::new(vec![]).with_properties(TableRowProperties {
    height: Some(RowHeight::AtLeast(18.)),
    repeat_header: Some(true),
    cant_split: Some(true),
    ..Default::default()
});
```

| Request | Effect |
| --- | --- |
| `RowHeight::Exact` / `RowHeight.exact` | Fixed points; content can clip. |
| `AtLeast` / `.at_least` | Minimum points; row can grow. |
| `repeat_header` | Repeat a contiguous group of leading header rows. Later requests are stored but do not move rows into that group. |
| `cant_split` | Keep the row on one page **if it fits**. |

Heights and cell margins must be finite, nonnegative points that round to signed 64-bit twips (1/20 pt); zero is valid. Omitted height leaves sizing to the viewer; no explicit `auto` override. Omitted flags differ from false. With no requests, no row properties element is emitted.

## Cell margins and text flow

Set `TableCellProperties.margins` in Rust or pass `margins=d.CellMargins(...)` to a Python cell. Set an edge to zero to remove its margin, or omit it to use the default. Margins do not recalculate column widths or image sizes.

| Rust `CellTextDirection` | Python `text_direction` |
| --- | --- |
| `LeftToRight` | `"lrTb"` |
| `TopToBottomRightToLeft` | `"tbRl"` |
| `BottomToTopLeftToRight` | `"btLr"` |
| `LeftToRightVertical` | `"lrTbV"` |
| `TopToBottomRightToLeftVertical` | `"tbRlV"` |
| `TopToBottomLeftToRightVertical` | `"tbLrV"` |

Only these Transitional tokens are accepted. Direction changes viewer text flow, not stored order or paragraph bidi. Omit for no override.

## Conditional table-style regions

```python
table = d.Table([row], style="My Table Style",
                look=d.TableLook(first_row=True, horizontal_banding=False))
```

`TableProperties.look` selects which [conditional style regions](STYLES.md#conditional-table-paragraph-formatting) the viewer applies. Whole-table formatting is unconditional. Nothing is copied into cells.

New tables enable `first_row`, `first_column` and `horizontal_banding` by default. `last_row`, `last_column` and `vertical_banding` default to false. These defaults do not change existing template tables.
