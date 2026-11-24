//! Wire DTOs only. Measurements, geometry relations and text validity belong to Rust.
use crate::{
    boundary::{self as b, Failure, Pass, Record},
    host::Wire,
};
use js_sys::Array;
use rocx::{
    AltText, BreakType, ColumnChartProperties, ImageAnchor, ImageAnchorHorizontalAlignment,
    ImageAnchorReference, ImageAnchorVerticalAlignment, ImageCrop, ImageTransformation, ImageWrap,
    ImageWrapSide, Result, ShapeGeometry, ShapeOutline, ShapeProperties, ShapeTextAlignment,
    ShapeTextFormat, ShapeTextVerticalAlignment,
};
use wasm_bindgen::JsValue;

type Read<T> = fn(&Wire, &str) -> Result<T>;

pub(super) fn get<T>(record: &Record, key: &str, read: Read<T>) -> Result<Option<T>> {
    let wire = record.get(key)?;
    if wire.js().is_undefined() {
        Ok(None)
    } else {
        read(&wire, &record.path(key)).map(Some)
    }
}

fn number(wire: &Wire, path: &str) -> Result<f64> {
    // Let Rust assign the domain rule for NaN, infinity and out-of-range values.
    wire.js()
        .as_f64()
        .ok_or_else(|| Failure::invalid(path, "expected a number"))
}

fn boolean(wire: &Wire, path: &str) -> Result<bool> {
    wire.js()
        .as_bool()
        .ok_or_else(|| Failure::invalid(path, "expected a boolean"))
}

fn integer<T: TryFrom<i64>>(wire: &Wire, path: &str) -> Result<T> {
    let value = number(wire, path)?;
    if value.fract() != 0.0 || value.abs() > u32::MAX as f64 {
        return Err(Failure::invalid(path, "expected an integer in range"));
    }
    T::try_from(value as i64).map_err(|_| Failure::invalid(path, "expected an integer in range"))
}

fn exact<const N: usize, T>(wire: &Wire, path: &str, read: Read<T>) -> Result<[T; N]> {
    sequence(wire, path, read)?
        .try_into()
        .map_err(|_| Failure::invalid(path, format!("expected an array of length {N}")))
}

fn sequence<T>(wire: &Wire, path: &str, read: Read<T>) -> Result<Vec<T>> {
    b::list(wire, path)?
        .iter()
        .enumerate()
        .map(|(i, wire)| read(wire, &format!("{path}[{i}]")))
        .collect()
}

fn rgb(wire: &Wire, path: &str) -> Result<[u8; 3]> {
    exact(wire, path, integer)
}

fn vertex(wire: &Wire, path: &str) -> Result<(i32, i32)> {
    let [x, y] = exact(wire, path, integer)?;
    Ok((x, y))
}

macro_rules! tokens {
    ($read:ident, $token:ident, $ty:ident, { $($literal:literal => $arm:ident),+ $(,)? }) => {
        pub(super) fn $read(wire: &Wire, path: &str) -> Result<$ty> {
            match b::string(wire, path)?.as_str() {
                $($literal => Ok($ty::$arm),)+
                actual => Err(Failure::invalid(path, format!("unknown {} `{actual}`", stringify!($ty)))),
            }
        }
        pub(super) fn $token(value: $ty) -> &'static str {
            match value { $($ty::$arm => $literal,)+ }
        }
    };
}

tokens!(reference, reference_token, ImageAnchorReference, {
    "page" => Page, "margin" => Margin, "column" => Column, "character" => Character,
    "leftMargin" => LeftMargin, "rightMargin" => RightMargin, "paragraph" => Paragraph,
    "line" => Line, "topMargin" => TopMargin, "bottomMargin" => BottomMargin,
    "insideMargin" => InsideMargin, "outsideMargin" => OutsideMargin,
});
tokens!(horizontal, horizontal_token, ImageAnchorHorizontalAlignment, {
    "left" => Left, "center" => Center, "right" => Right, "inside" => Inside, "outside" => Outside,
});
tokens!(vertical, vertical_token, ImageAnchorVerticalAlignment, {
    "top" => Top, "center" => Center, "bottom" => Bottom, "inside" => Inside, "outside" => Outside,
});
tokens!(wrap, wrap_token, ImageWrap, {
    "none" => None, "square" => Square, "topAndBottom" => TopAndBottom, "tight" => Tight, "through" => Through,
});
tokens!(side, side_token, ImageWrapSide, {
    "bothSides" => BothSides, "left" => Left, "right" => Right, "largest" => Largest,
});
tokens!(geometry, geometry_token, ShapeGeometry, {
    "rectangle" => Rectangle, "ellipse" => Ellipse, "textBox" => TextBox,
});
tokens!(text_align, text_align_token, ShapeTextAlignment, {
    "left" => Left, "center" => Center, "right" => Right,
});
tokens!(text_vertical, text_vertical_token, ShapeTextVerticalAlignment, {
    "top" => Top, "center" => Center, "bottom" => Bottom,
});
tokens!(break_type, break_type_token, BreakType, {
    "line" => Line, "textWrapping" => TextWrapping, "page" => Page, "column" => Column,
});

fn read_anchor(wire: &Wire, path: &str) -> Result<ImageAnchor> {
    let record = Record::new(
        wire,
        path,
        &[
            "horizontalReference",
            "verticalReference",
            "horizontalAlignment",
            "verticalAlignment",
            "x",
            "y",
            "wrap",
            "wrapSide",
            "wrapPolygon",
            "distanceTop",
            "distanceBottom",
            "distanceLeft",
            "distanceRight",
            "relativeHeight",
            "behindText",
            "locked",
            "layoutInCell",
            "allowOverlap",
        ],
    )?;
    let default = ImageAnchor::default();
    Ok(ImageAnchor {
        horizontal_reference: get(&record, "horizontalReference", reference)?
            .unwrap_or(default.horizontal_reference),
        vertical_reference: get(&record, "verticalReference", reference)?
            .unwrap_or(default.vertical_reference),
        horizontal_alignment: get(&record, "horizontalAlignment", horizontal)?,
        vertical_alignment: get(&record, "verticalAlignment", vertical)?,
        x: get(&record, "x", number)?.unwrap_or(default.x),
        y: get(&record, "y", number)?.unwrap_or(default.y),
        wrap: get(&record, "wrap", wrap)?.unwrap_or(default.wrap),
        wrap_side: get(&record, "wrapSide", side)?,
        wrap_polygon: get(&record, "wrapPolygon", |wire, path| {
            sequence(wire, path, vertex)
        })?
        .unwrap_or(default.wrap_polygon),
        distance_top: get(&record, "distanceTop", number)?,
        distance_bottom: get(&record, "distanceBottom", number)?,
        distance_left: get(&record, "distanceLeft", number)?,
        distance_right: get(&record, "distanceRight", number)?,
        relative_height: get(&record, "relativeHeight", integer)?
            .unwrap_or(default.relative_height),
        behind_text: get(&record, "behindText", boolean)?.unwrap_or(default.behind_text),
        locked: get(&record, "locked", boolean)?,
        layout_in_cell: get(&record, "layoutInCell", boolean)?,
        allow_overlap: get(&record, "allowOverlap", boolean)?,
    })
}

fn crop(wire: &Wire, path: &str) -> Result<ImageCrop> {
    let record = Record::new(wire, path, &["left", "top", "right", "bottom"])?;
    let default = ImageCrop::default();
    Ok(ImageCrop {
        left: get(&record, "left", number)?.unwrap_or(default.left),
        top: get(&record, "top", number)?.unwrap_or(default.top),
        right: get(&record, "right", number)?.unwrap_or(default.right),
        bottom: get(&record, "bottom", number)?.unwrap_or(default.bottom),
    })
}

pub(super) fn transformation(wire: &Wire, _: &Pass, path: &str) -> Result<ImageTransformation> {
    let record = Record::new(
        wire,
        path,
        &[
            "anchor",
            "crop",
            "rotation",
            "flipHorizontal",
            "flipVertical",
            "width",
            "height",
        ],
    )?;
    Ok(ImageTransformation {
        anchor: get(&record, "anchor", read_anchor)?,
        crop: get(&record, "crop", crop)?,
        rotation: get(&record, "rotation", number)?,
        flip_horizontal: get(&record, "flipHorizontal", boolean)?,
        flip_vertical: get(&record, "flipVertical", boolean)?,
        width: get(&record, "width", number)?,
        height: get(&record, "height", number)?,
    })
}

pub(super) fn alt_text(wire: &Wire, _: &Pass, path: &str) -> Result<AltText> {
    let record = Record::new(wire, path, &["name", "title", "description"])?;
    Ok(AltText {
        name: get(&record, "name", b::string)?,
        title: get(&record, "title", b::string)?,
        description: get(&record, "description", b::string)?,
    })
}

fn required<T>(record: &Record, key: &str, read: Read<T>) -> Result<T> {
    read(&record.get(key)?, &record.path(key))
}

fn outline(wire: &Wire, path: &str) -> Result<ShapeOutline> {
    let record = Record::new(wire, path, &["width", "color"])?;
    Ok(ShapeOutline {
        width: required(&record, "width", number)?,
        color: required(&record, "color", rgb)?,
    })
}

fn text_format(wire: &Wire, path: &str) -> Result<ShapeTextFormat> {
    let record = Record::new(
        wire,
        path,
        &[
            "fontSize",
            "bold",
            "color",
            "alignment",
            "verticalAlignment",
            "italic",
            "font",
        ],
    )?;
    Ok(ShapeTextFormat {
        font_size: get(&record, "fontSize", number)?,
        bold: get(&record, "bold", boolean)?,
        color: get(&record, "color", rgb)?,
        alignment: get(&record, "alignment", text_align)?,
        vertical_alignment: get(&record, "verticalAlignment", text_vertical)?,
        italic: get(&record, "italic", boolean)?,
        font: get(&record, "font", b::string)?,
    })
}

pub(super) fn shape(wire: &Wire, _: &Pass, path: &str) -> Result<ShapeProperties> {
    let record = Record::new(
        wire,
        path,
        &[
            "width",
            "height",
            "geometry",
            "fill",
            "text",
            "outline",
            "textFormat",
            "anchor",
        ],
    )?;
    let default = ShapeProperties::default();
    Ok(ShapeProperties {
        width: get(&record, "width", number)?.unwrap_or(default.width),
        height: get(&record, "height", number)?.unwrap_or(default.height),
        geometry: get(&record, "geometry", geometry)?.unwrap_or(default.geometry),
        fill: get(&record, "fill", rgb)?,
        text: get(&record, "text", b::string)?,
        outline: get(&record, "outline", outline)?,
        text_format: get(&record, "textFormat", text_format)?,
        anchor: get(&record, "anchor", read_anchor)?,
    })
}

pub(super) fn chart(wire: &Wire, _: &Pass, path: &str) -> Result<ColumnChartProperties> {
    let record = Record::new(wire, path, &["width", "height", "categories", "values"])?;
    Ok(ColumnChartProperties {
        width: required(&record, "width", number)?,
        height: required(&record, "height", number)?,
        categories: required(&record, "categories", |wire, path| {
            sequence(wire, path, b::string)
        })?,
        values: required(&record, "values", |wire, path| sequence(wire, path, number))?,
    })
}

fn opt<V: Into<JsValue>>(view: &JsValue, key: &str, value: Option<V>) {
    if let Some(value) = value {
        b::set(view, key, &value.into());
    }
}

fn array(items: impl IntoIterator<Item = impl Into<JsValue>>) -> JsValue {
    items.into_iter().map(Into::into).collect::<Array>().into()
}

pub(super) fn alt_text_view(alt: &AltText) -> JsValue {
    let view = b::record(&[]);
    opt(&view, "title", alt.title.as_deref());
    opt(&view, "description", alt.description.as_deref());
    opt(&view, "name", alt.name.as_deref());
    view
}

fn anchor_view(anchor: &ImageAnchor) -> JsValue {
    let view = b::record(&[
        (
            "horizontalReference",
            reference_token(anchor.horizontal_reference).into(),
        ),
        (
            "verticalReference",
            reference_token(anchor.vertical_reference).into(),
        ),
        ("x", anchor.x.into()),
        ("y", anchor.y.into()),
        ("wrap", wrap_token(anchor.wrap).into()),
        (
            "wrapPolygon",
            array(anchor.wrap_polygon.iter().map(|&(x, y)| array([x, y]))),
        ),
        ("relativeHeight", anchor.relative_height.into()),
        ("behindText", anchor.behind_text.into()),
    ]);
    opt(
        &view,
        "horizontalAlignment",
        anchor.horizontal_alignment.map(horizontal_token),
    );
    opt(
        &view,
        "verticalAlignment",
        anchor.vertical_alignment.map(vertical_token),
    );
    opt(&view, "wrapSide", anchor.wrap_side.map(side_token));
    opt(&view, "distanceTop", anchor.distance_top);
    opt(&view, "distanceBottom", anchor.distance_bottom);
    opt(&view, "distanceLeft", anchor.distance_left);
    opt(&view, "distanceRight", anchor.distance_right);
    opt(&view, "locked", anchor.locked);
    opt(&view, "layoutInCell", anchor.layout_in_cell);
    opt(&view, "allowOverlap", anchor.allow_overlap);
    view
}

fn text_format_view(format: &ShapeTextFormat) -> JsValue {
    let view = b::record(&[]);
    opt(&view, "fontSize", format.font_size);
    opt(&view, "bold", format.bold);
    opt(&view, "color", format.color.map(array));
    opt(&view, "alignment", format.alignment.map(text_align_token));
    opt(
        &view,
        "verticalAlignment",
        format.vertical_alignment.map(text_vertical_token),
    );
    opt(&view, "italic", format.italic);
    opt(&view, "font", format.font.as_deref());
    view
}

pub(super) fn shape_view(shape: &ShapeProperties) -> JsValue {
    let view = b::record(&[
        ("width", shape.width.into()),
        ("height", shape.height.into()),
        ("geometry", geometry_token(shape.geometry).into()),
    ]);
    opt(&view, "fill", shape.fill.map(array));
    opt(&view, "text", shape.text.as_deref());
    opt(&view, "anchor", shape.anchor.as_ref().map(anchor_view));
    opt(
        &view,
        "textFormat",
        shape.text_format.as_ref().map(text_format_view),
    );
    opt(
        &view,
        "outline",
        shape.outline.as_ref().map(|outline| {
            b::record(&[
                ("width", outline.width.into()),
                ("color", array(outline.color)),
            ])
        }),
    );
    view
}

pub(super) fn chart_view(chart: &ColumnChartProperties) -> JsValue {
    b::record(&[
        ("width", chart.width.into()),
        ("height", chart.height.into()),
        (
            "categories",
            array(chart.categories.iter().map(String::as_str)),
        ),
        ("values", array(chart.values.iter().copied())),
    ])
}
