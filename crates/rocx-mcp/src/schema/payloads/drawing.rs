use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;

macro_rules! wire_enum {
    ($name:ident => $native:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Deserialize, JsonSchema)]
        #[serde(rename_all = "kebab-case")]
        pub(crate) enum $name { $($variant),+ }

        impl From<$name> for rocx::$native {
            fn from(value: $name) -> Self {
                match value { $($name::$variant => Self::$variant),+ }
            }
        }
    };
}

wire_enum!(Reference => ImageAnchorReference {
    Page, Margin, Column, Character, LeftMargin, RightMargin, Paragraph, Line,
    TopMargin, BottomMargin, InsideMargin, OutsideMargin,
});
wire_enum!(Horizontal => ImageAnchorHorizontalAlignment { Left, Center, Right, Inside, Outside });
wire_enum!(Vertical => ImageAnchorVerticalAlignment { Top, Center, Bottom, Inside, Outside });
wire_enum!(Wrap => ImageWrap { None, Square, TopAndBottom, Tight, Through });
wire_enum!(WrapSide => ImageWrapSide { BothSides, Left, Right, Largest });
wire_enum!(Geometry => ShapeGeometry { Rectangle, Ellipse, TextBox });
wire_enum!(TextAlignment => ShapeTextAlignment { Left, Center, Right });
wire_enum!(TextVerticalAlignment => ShapeTextVerticalAlignment { Top, Center, Bottom });

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Anchor {
    horizontal_reference: Option<Reference>,
    vertical_reference: Option<Reference>,
    horizontal_alignment: Option<Horizontal>,
    vertical_alignment: Option<Vertical>,
    /// Horizontal and vertical offsets in points.
    x: Option<f64>,
    y: Option<f64>,
    wrap: Option<Wrap>,
    wrap_side: Option<WrapSide>,
    /// Ordered [x, y] vertices in DrawingML wrap-path coordinates.
    wrap_polygon: Option<Vec<(i32, i32)>>,
    distance_top: Option<f64>,
    distance_bottom: Option<f64>,
    distance_left: Option<f64>,
    distance_right: Option<f64>,
    relative_height: Option<u32>,
    behind_text: Option<bool>,
    locked: Option<bool>,
    layout_in_cell: Option<bool>,
    allow_overlap: Option<bool>,
}

impl From<Anchor> for rocx::ImageAnchor {
    fn from(value: Anchor) -> Self {
        Self {
            horizontal_reference: value
                .horizontal_reference
                .map(Into::into)
                .unwrap_or_default(),
            vertical_reference: value.vertical_reference.map(Into::into).unwrap_or_default(),
            horizontal_alignment: value.horizontal_alignment.map(Into::into),
            vertical_alignment: value.vertical_alignment.map(Into::into),
            x: value.x.unwrap_or_default(),
            y: value.y.unwrap_or_default(),
            wrap: value.wrap.map(Into::into).unwrap_or_default(),
            wrap_side: value.wrap_side.map(Into::into),
            wrap_polygon: value.wrap_polygon.unwrap_or_default(),
            distance_top: value.distance_top,
            distance_bottom: value.distance_bottom,
            distance_left: value.distance_left,
            distance_right: value.distance_right,
            relative_height: value.relative_height.unwrap_or_default(),
            behind_text: value.behind_text.unwrap_or_default(),
            locked: value.locked,
            layout_in_cell: value.layout_in_cell,
            allow_overlap: value.allow_overlap,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Crop {
    /// Insets as percentages, with an omitted edge treated as zero.
    left: Option<f64>,
    top: Option<f64>,
    right: Option<f64>,
    bottom: Option<f64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Transform {
    anchor: Option<Anchor>,
    crop: Option<Crop>,
    /// Clockwise rotation in degrees.
    rotation: Option<f64>,
    flip_horizontal: Option<bool>,
    flip_vertical: Option<bool>,
    /// Dimensions in points.
    width: Option<f64>,
    height: Option<f64>,
}

impl From<Transform> for rocx::ImageTransformation {
    fn from(value: Transform) -> Self {
        Self {
            anchor: value.anchor.map(Into::into),
            crop: value.crop.map(|crop| rocx::ImageCrop {
                left: crop.left.unwrap_or_default(),
                top: crop.top.unwrap_or_default(),
                right: crop.right.unwrap_or_default(),
                bottom: crop.bottom.unwrap_or_default(),
            }),
            rotation: value.rotation,
            flip_horizontal: value.flip_horizontal,
            flip_vertical: value.flip_vertical,
            width: value.width,
            height: value.height,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AltText {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
}

impl From<AltText> for rocx::AltText {
    fn from(value: AltText) -> Self {
        Self {
            name: value.name,
            title: value.title,
            description: value.description,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Outline {
    /// Line width in points.
    width: f64,
    color: [u8; 3],
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TextFormat {
    font_size: Option<f64>,
    bold: Option<bool>,
    color: Option<[u8; 3]>,
    alignment: Option<TextAlignment>,
    vertical_alignment: Option<TextVerticalAlignment>,
    italic: Option<bool>,
    font: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Shape {
    /// Dimensions in points; omitted dimensions use the native defaults.
    width: Option<f64>,
    height: Option<f64>,
    geometry: Option<Geometry>,
    fill: Option<[u8; 3]>,
    text: Option<String>,
    outline: Option<Outline>,
    text_format: Option<TextFormat>,
    anchor: Option<Anchor>,
}

impl From<Shape> for rocx::ShapeProperties {
    fn from(value: Shape) -> Self {
        let default = Self::default();
        Self {
            width: value.width.unwrap_or(default.width),
            height: value.height.unwrap_or(default.height),
            geometry: value.geometry.map(Into::into).unwrap_or_default(),
            fill: value.fill,
            text: value.text,
            outline: value.outline.map(|outline| rocx::ShapeOutline {
                width: outline.width,
                color: outline.color,
            }),
            text_format: value.text_format.map(|format| rocx::ShapeTextFormat {
                font_size: format.font_size,
                bold: format.bold,
                color: format.color,
                alignment: format.alignment.map(Into::into),
                vertical_alignment: format.vertical_alignment.map(Into::into),
                italic: format.italic,
                font: format.font,
            }),
            anchor: value.anchor.map(Into::into),
        }
    }
}
