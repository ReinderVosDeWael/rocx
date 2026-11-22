//! Checked dimensions and properties for geometric shapes and plain text boxes.
use super::{Diagnostic, ImageAnchor, Points, Rule};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShapeGeometry {
    #[default]
    Rectangle,
    Ellipse,
    /// Standalone DrawingML text rectangle, without geometric fill or outline.
    TextBox,
}
impl ShapeGeometry {
    pub(crate) const fn xml(self) -> Option<&'static str> {
        match self {
            Self::Rectangle => Some("rect"),
            Self::Ellipse => Some("ellipse"),
            Self::TextBox => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeOutline {
    /// Line width in points.
    pub width: f64,
    pub color: [u8; 3],
}
setters!(ShapeOutline {
    with_width => width: value<f64>,
    with_color => color: value<[u8; 3]>,
});
impl ShapeOutline {
    pub(crate) fn width_emu(&self) -> Result<i64, Diagnostic> {
        let invalid = || {
            Diagnostic::new(
                Rule::InvalidMeasurement,
                "shape outline width must be between 0 and 1584 points",
            )
            .at("shape.outline.width")
        };
        if !(0. ..=1584.).contains(&self.width) {
            return Err(invalid());
        }
        Points::new(self.width)
            .and_then(Points::emu)
            .map(|value| value.get())
            .map_err(|_| invalid())
    }
}
/// Supported DrawingML paragraph alignments, independent of WordprocessingML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeTextAlignment {
    Left,
    Center,
    Right,
}
impl ShapeTextAlignment {
    pub(crate) const fn xml(self) -> &'static str {
        match self {
            Self::Left => "l",
            Self::Center => "ctr",
            Self::Right => "r",
        }
    }
}
/// Supported DrawingML text-body anchors; this does not position the shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeTextVerticalAlignment {
    Top,
    Center,
    Bottom,
}
impl ShapeTextVerticalAlignment {
    pub(crate) const fn xml(self) -> &'static str {
        match self {
            Self::Top => "t",
            Self::Center => "ctr",
            Self::Bottom => "b",
        }
    }
}
/// Uniform requests for the shape's plain text, not computed effective styling.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShapeTextFormat {
    /// Font size in points (1..=4000), rounded to hundredths of a point.
    pub font_size: Option<f64>,
    pub bold: Option<bool>,
    pub color: Option<[u8; 3]>,
    pub alignment: Option<ShapeTextAlignment>,
    pub vertical_alignment: Option<ShapeTextVerticalAlignment>,
    pub italic: Option<bool>,
    /// DrawingML Latin typeface request; no installed-font lookup is performed.
    pub font: Option<String>,
}
setters!(ShapeTextFormat {
    with_font_size => font_size: optional<f64>,
    with_bold => bold: optional<bool>,
    with_color => color: optional<[u8; 3]>,
    with_alignment => alignment: optional<ShapeTextAlignment>,
    with_vertical_alignment => vertical_alignment: optional<ShapeTextVerticalAlignment>,
    with_italic => italic: optional<bool>,
    with_font => font: optional_into<String>,
});
impl ShapeTextFormat {
    pub(crate) fn font_size_hundredths(&self) -> Result<Option<i64>, Diagnostic> {
        self.font_size
            .map(|size| {
                if !(1. ..=4000.).contains(&size) {
                    return Err(Diagnostic::new(
                        Rule::InvalidMeasurement,
                        "shape text font size must be between 1 and 4000 points",
                    )
                    .at("shape.text_format.font_size"));
                }
                Points::new(size)?.scaled(100.)
            })
            .transpose()
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeProperties {
    pub width: f64,
    pub height: f64,
    pub geometry: ShapeGeometry,
    pub fill: Option<[u8; 3]>,
    pub text: Option<String>,
    pub outline: Option<ShapeOutline>,
    pub text_format: Option<ShapeTextFormat>,
    /// Floating placement; omission keeps the canvas inline.
    pub anchor: Option<ImageAnchor>,
}
setters!(ShapeProperties {
    with_width => width: value<f64>,
    with_height => height: value<f64>,
    with_geometry => geometry: value<ShapeGeometry>,
    with_fill => fill: optional<[u8; 3]>,
    with_text => text: optional_into<String>,
    with_outline => outline: optional<ShapeOutline>,
    with_text_format => text_format: optional<ShapeTextFormat>,
    with_anchor => anchor: optional<ImageAnchor>,
});
impl Default for ShapeProperties {
    fn default() -> Self {
        Self {
            width: 72.,
            height: 72.,
            geometry: ShapeGeometry::Rectangle,
            fill: None,
            text: None,
            outline: None,
            text_format: None,
            anchor: None,
        }
    }
}
impl ShapeProperties {
    pub(crate) fn validate(&self) -> Result<(), Diagnostic> {
        self.dimensions_emu()?;
        if let Some(anchor) = &self.anchor {
            anchor.validate()?;
        }
        if self.geometry == ShapeGeometry::TextBox
            && (self.fill.is_some() || self.outline.is_some())
        {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "standalone text boxes do not support geometric fill or outline",
            )
            .at("shape.geometry"));
        }
        if let Some(outline) = &self.outline {
            outline.width_emu()?;
        }
        if let Some(format) = &self.text_format {
            format.font_size_hundredths()?;
        }
        Ok(())
    }

    pub(crate) fn dimensions_emu(&self) -> Result<(i64, i64), Diagnostic> {
        let dimension = |value, name| {
            let emu = Points::new(value)
                .and_then(Points::emu)
                .map_err(|error| error.at(name))?
                .get();
            if !(1..=i64::from(i32::MAX)).contains(&emu) {
                return Err(Diagnostic::new(
                    Rule::InvalidMeasurement,
                    "shape dimensions must be positive and fit DrawingML coordinates",
                )
                .at(name));
            }
            Ok(emu)
        };
        Ok((
            dimension(self.width, "shape.width")?,
            dimension(self.height, "shape.height")?,
        ))
    }
}
