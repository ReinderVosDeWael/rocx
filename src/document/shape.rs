//! Checked geometry for a basic DrawingML shape.
use super::{Diagnostic, Points, Rule};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShapeGeometry {
    #[default]
    Rectangle,
    Ellipse,
}
impl ShapeGeometry {
    pub(crate) const fn xml(self) -> &'static str {
        match self {
            Self::Rectangle => "rect",
            Self::Ellipse => "ellipse",
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
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeProperties {
    pub width: f64,
    pub height: f64,
    pub geometry: ShapeGeometry,
    pub fill: Option<[u8; 3]>,
    pub text: Option<String>,
    pub outline: Option<ShapeOutline>,
}
setters!(ShapeProperties {
    with_width => width: value<f64>,
    with_height => height: value<f64>,
    with_geometry => geometry: value<ShapeGeometry>,
    with_fill => fill: optional<[u8; 3]>,
    with_text => text: optional_into<String>,
    with_outline => outline: optional<ShapeOutline>,
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
        }
    }
}
impl ShapeProperties {
    pub(crate) fn validate(&self) -> Result<(), Diagnostic> {
        self.dimensions_emu()?;
        if let Some(outline) = &self.outline {
            outline.width_emu()?;
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
