//! DTOs shared by table declarations, validation, and resolved projections.
use crate::{
    boundary::{self as b, Failure, Operation, Pass, Record},
    host::Wire,
};
use js_sys::Array;
use rocx::{
    BorderLine, BorderSide, CellBorder, CellBorderSide, CellMargins, CellTextDirection,
    ResolvedTable, ResolvedTableCell, Result, RowHeight, TableAlignment, TableBorder,
    TableCellProperties, TableLayout, TableLook, TableProperties, TableRowProperties,
    VerticalAlignment, VerticalMerge,
};
use wasm_bindgen::JsValue;

macro_rules! tags {
    ($decode:ident, $view:ident, $type:ident { $($tag:literal => $variant:ident),+ $(,)? }) => {
        fn $decode(wire: &Wire, path: &str) -> Result<$type> {
            match b::string(wire, path)?.as_str() {
                $($tag => Ok($type::$variant),)+
                _ => Err(Failure::invalid(path, concat!("unknown ", stringify!($type), " tag"))),
            }
        }
        fn $view(value: $type) -> JsValue {
            match value {
                $($type::$variant => $tag.into(),)+
            }
        }
    };
}

tags!(layout, layout_view, TableLayout { "fixed" => Fixed, "autofit" => Autofit });
tags!(alignment, alignment_view, TableAlignment {
    "left" => Left, "center" => Center, "right" => Right, "start" => Start, "end" => End,
});
tags!(merge, merge_view, VerticalMerge { "restart" => Restart, "continue" => Continue });
tags!(vertical_alignment, vertical_alignment_view, VerticalAlignment {
    "top" => Top, "center" => Center, "bottom" => Bottom, "both" => Both,
});
tags!(direction, direction_view, CellTextDirection {
    "leftToRight" => LeftToRight,
    "topToBottomRightToLeft" => TopToBottomRightToLeft,
    "bottomToTopLeftToRight" => BottomToTopLeftToRight,
    "leftToRightVertical" => LeftToRightVertical,
    "topToBottomRightToLeftVertical" => TopToBottomRightToLeftVertical,
    "topToBottomLeftToRightVertical" => TopToBottomLeftToRightVertical,
});
tags!(border_line, border_line_view, BorderLine { "single" => Single, "dashed" => Dashed });
tags!(border_side, border_side_view, BorderSide {
    "top" => Top, "left" => Left, "bottom" => Bottom, "right" => Right,
    "insideH" => InsideH, "insideV" => InsideV,
});
tags!(cell_border_side, cell_border_side_view, CellBorderSide {
    "top" => Top, "bottom" => Bottom, "start" => Start, "end" => End,
});

fn get<T>(record: &Record, key: &str, decode: impl FnOnce(&Wire, &str) -> Result<T>) -> Result<T> {
    decode(&record.get(key)?, &record.path(key))
}

fn optional<T>(
    record: &Record,
    key: &str,
    decode: impl FnOnce(&Wire, &str) -> Result<T>,
) -> Result<Option<T>> {
    let wire = record.get(key)?;
    if wire.js().is_undefined() {
        Ok(None)
    } else {
        decode(&wire, &record.path(key)).map(Some)
    }
}

fn list<T>(wire: &Wire, path: &str, decode: impl Fn(&Wire, &str) -> Result<T>) -> Result<Vec<T>> {
    b::list(wire, path)?
        .iter()
        .enumerate()
        .map(|(index, wire)| decode(wire, &format!("{path}[{index}]")))
        .collect()
}

fn boolean(wire: &Wire, path: &str) -> Result<bool> {
    wire.js()
        .as_bool()
        .ok_or_else(|| Failure::invalid(path, "expected a boolean"))
}

fn points(wire: &Wire, path: &str) -> Result<f64> {
    // Keep non-finite numbers for Rust's measurement validation.
    wire.js()
        .as_f64()
        .ok_or_else(|| Failure::invalid(path, "expected a number"))
}

fn unsigned(wire: &Wire, path: &str) -> Result<u32> {
    u32::try_from(b::u64_number(wire, path)?)
        .map_err(|_| Failure::invalid(path, "integer is outside u32 bounds"))
}

fn rgb(wire: &Wire, path: &str) -> Result<[u8; 3]> {
    let channels = b::list(wire, path)?;
    let [r, g, b] = channels.as_slice() else {
        return Err(Failure::invalid(path, "expected three RGB channels"));
    };
    let channel = |wire, index| {
        let path = format!("{path}[{index}]");
        u8::try_from(b::u64_number(wire, &path)?)
            .map_err(|_| Failure::invalid(&path, "RGB channel is outside u8 bounds"))
    };
    Ok([channel(r, 0)?, channel(g, 1)?, channel(b, 2)?])
}

fn rgb_view(color: [u8; 3]) -> JsValue {
    color
        .into_iter()
        .map(JsValue::from)
        .collect::<Array>()
        .into()
}

fn border<T>(
    wire: &Wire,
    path: &str,
    decode_side: fn(&Wire, &str) -> Result<T>,
) -> Result<(T, u32, [u8; 3], BorderLine)> {
    let record = Record::new(wire, path, &["side", "sz", "color", "val"])?;
    Ok((
        get(&record, "side", decode_side)?,
        get(&record, "sz", unsigned)?,
        get(&record, "color", rgb)?,
        get(&record, "val", border_line)?,
    ))
}

fn border_view(side: JsValue, sz: u32, color: [u8; 3], val: BorderLine) -> JsValue {
    b::record(&[
        ("side", side),
        ("sz", sz.into()),
        ("color", rgb_view(color)),
        ("val", border_line_view(val)),
    ])
}

fn put<T>(object: &JsValue, key: &str, value: Option<T>, project: impl FnOnce(T) -> JsValue) {
    if let Some(value) = value {
        b::set(object, key, &project(value));
    }
}

fn look(wire: &Wire, path: &str) -> Result<TableLook> {
    let record = Record::new(
        wire,
        path,
        &[
            "firstRow",
            "lastRow",
            "firstColumn",
            "lastColumn",
            "horizontalBanding",
            "verticalBanding",
        ],
    )?;
    Ok(TableLook {
        first_row: optional(&record, "firstRow", boolean)?,
        last_row: optional(&record, "lastRow", boolean)?,
        first_column: optional(&record, "firstColumn", boolean)?,
        last_column: optional(&record, "lastColumn", boolean)?,
        horizontal_banding: optional(&record, "horizontalBanding", boolean)?,
        vertical_banding: optional(&record, "verticalBanding", boolean)?,
    })
}

fn look_view(look: &TableLook) -> JsValue {
    let view = b::record(&[]);
    for (key, value) in [
        ("firstRow", look.first_row),
        ("lastRow", look.last_row),
        ("firstColumn", look.first_column),
        ("lastColumn", look.last_column),
        ("horizontalBanding", look.horizontal_banding),
        ("verticalBanding", look.vertical_banding),
    ] {
        put(&view, key, value, JsValue::from);
    }
    view
}

pub(super) fn table(wire: &Wire, _: &Pass, path: &str) -> Result<TableProperties> {
    let record = Record::new(
        wire,
        path,
        &[
            "columnWidths",
            "layout",
            "style",
            "borders",
            "look",
            "alignment",
        ],
    )?;
    Ok(TableProperties {
        column_widths: optional(&record, "columnWidths", |wire, path| {
            list(wire, path, unsigned)
        })?,
        layout: optional(&record, "layout", layout)?,
        style: optional(&record, "style", b::string)?,
        borders: optional(&record, "borders", |wire, path| {
            list(wire, path, |wire, path| {
                let (side, sz, color, val) = border(wire, path, border_side)?;
                Ok(TableBorder {
                    side,
                    sz,
                    color,
                    val,
                })
            })
        })?
        .unwrap_or_default(),
        look: optional(&record, "look", look)?,
        alignment: optional(&record, "alignment", alignment)?,
    })
}

pub(super) fn table_view(table: &ResolvedTable) -> JsValue {
    let borders = table
        .borders()
        .iter()
        .map(|b| border_view(border_side_view(b.side), b.sz, b.color, b.val));
    let view = b::record(&[("borders", borders.collect::<Array>().into())]);
    put(&view, "columnWidths", table.column_widths(), |widths| {
        widths
            .iter()
            .copied()
            .map(JsValue::from)
            .collect::<Array>()
            .into()
    });
    put(&view, "layout", table.layout(), layout_view);
    put(&view, "style", table.style(), JsValue::from);
    put(&view, "look", table.look(), look_view);
    put(&view, "alignment", table.alignment(), alignment_view);
    view
}

fn height(wire: &Wire, path: &str) -> Result<RowHeight> {
    let record = Record::new(wire, path, &["kind", "points"])?;
    let kind = get(&record, "kind", b::string)?;
    let points = get(&record, "points", points)?;
    match kind.as_str() {
        "exact" => Ok(RowHeight::Exact(points)),
        "atLeast" => Ok(RowHeight::AtLeast(points)),
        _ => Err(Failure::invalid(
            &record.path("kind"),
            "unknown RowHeight tag",
        )),
    }
}

fn height_view(height: RowHeight) -> JsValue {
    let (kind, points) = match height {
        RowHeight::Exact(points) => ("exact", points),
        RowHeight::AtLeast(points) => ("atLeast", points),
    };
    b::record(&[("kind", kind.into()), ("points", points.into())])
}

pub(super) fn row(wire: &Wire, pass: &Pass, path: &str) -> Result<TableRowProperties> {
    let record = Record::new(
        wire,
        path,
        &["height", "repeatHeader", "cantSplit", "contentControl"],
    )?;
    Ok(TableRowProperties {
        height: optional(&record, "height", height)?,
        repeat_header: optional(&record, "repeatHeader", boolean)?,
        cant_split: optional(&record, "cantSplit", boolean)?,
        content_control: optional(&record, "contentControl", |wire, path| {
            crate::structured::content_control_properties(wire, pass, path)
        })?,
    })
}

pub(super) fn row_view(row: &TableRowProperties) -> JsValue {
    let view = b::record(&[]);
    put(&view, "height", row.height, height_view);
    put(&view, "repeatHeader", row.repeat_header, JsValue::from);
    put(&view, "cantSplit", row.cant_split, JsValue::from);
    put(
        &view,
        "contentControl",
        row.content_control.as_ref(),
        crate::structured::content_control_properties_view,
    );
    view
}

fn margins(wire: &Wire, path: &str) -> Result<CellMargins> {
    let record = Record::new(wire, path, &["top", "start", "bottom", "end"])?;
    Ok(CellMargins {
        top: optional(&record, "top", points)?,
        start: optional(&record, "start", points)?,
        bottom: optional(&record, "bottom", points)?,
        end: optional(&record, "end", points)?,
    })
}

fn margins_view(margins: &CellMargins) -> JsValue {
    let view = b::record(&[]);
    for (key, value) in [
        ("top", margins.top),
        ("start", margins.start),
        ("bottom", margins.bottom),
        ("end", margins.end),
    ] {
        put(&view, key, value, JsValue::from);
    }
    view
}

pub(super) fn cell(wire: &Wire, pass: &Pass, path: &str) -> Result<TableCellProperties> {
    let record = Record::new(
        wire,
        path,
        &[
            "gridSpan",
            "vmerge",
            "borders",
            "backgroundColor",
            "verticalAlignment",
            "margins",
            "textDirection",
            "contentControl",
        ],
    )?;
    Ok(TableCellProperties {
        grid_span: optional(&record, "gridSpan", unsigned)?,
        vmerge: optional(&record, "vmerge", merge)?,
        borders: optional(&record, "borders", |wire, path| {
            list(wire, path, |wire, path| {
                let (side, sz, color, val) = border(wire, path, cell_border_side)?;
                Ok(CellBorder {
                    side,
                    sz,
                    color,
                    val,
                })
            })
        })?
        .unwrap_or_default(),
        background_color: optional(&record, "backgroundColor", rgb)?,
        vertical_alignment: optional(&record, "verticalAlignment", vertical_alignment)?,
        margins: optional(&record, "margins", margins)?,
        text_direction: optional(&record, "textDirection", direction)?,
        content_control: optional(&record, "contentControl", |wire, path| {
            crate::structured::content_control_properties(wire, pass, path)
        })?,
    })
}

pub(super) fn cell_view(cell: &ResolvedTableCell) -> JsValue {
    let borders = cell
        .borders()
        .iter()
        .map(|b| border_view(cell_border_side_view(b.side), b.sz, b.color, b.val));
    let view = b::record(&[
        ("gridSpan", cell.grid_span().into()),
        ("borders", borders.collect::<Array>().into()),
    ]);
    put(&view, "vmerge", cell.vmerge(), merge_view);
    put(&view, "backgroundColor", cell.background_color(), rgb_view);
    put(
        &view,
        "verticalAlignment",
        cell.vertical_alignment(),
        vertical_alignment_view,
    );
    put(&view, "margins", cell.margins(), margins_view);
    put(
        &view,
        "textDirection",
        cell.text_direction(),
        direction_view,
    );
    put(
        &view,
        "contentControl",
        cell.content_control(),
        crate::structured::content_control_properties_view,
    );
    view
}

pub(super) fn validate_row(wire: &Wire) -> Result<JsValue> {
    let input = Record::new(wire, "", &["kind", "value"])?;
    row(&input.get("value")?, &Operation::new().pass(), "value")?.validate()?;
    Ok(JsValue::UNDEFINED)
}

pub(super) fn validate_cell(wire: &Wire) -> Result<JsValue> {
    let input = Record::new(wire, "", &["kind", "value"])?;
    cell(&input.get("value")?, &Operation::new().pass(), "value")?.validate()?;
    Ok(JsValue::UNDEFINED)
}

pub(super) fn validate_margins(wire: &Wire) -> Result<JsValue> {
    let input = Record::new(wire, "", &["kind", "value"])?;
    get(&input, "value", margins)?.validate()?;
    Ok(JsValue::UNDEFINED)
}
