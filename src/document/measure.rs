//! Checked physical measurements; P1 §§17.18.42, 22.9.2.14–22.9.2.15.
use super::{Diagnostic, Rule};
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Points(f64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Twips(i64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Emu(i64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HalfPoints(i64);
impl Points {
    pub fn new(value: f64) -> Result<Self, Diagnostic> {
        if !value.is_finite() {
            return Err(Diagnostic::new(
                Rule::InvalidMeasurement,
                "length must be finite",
            ));
        }
        Ok(Self(value))
    }
    pub fn get(self) -> f64 {
        self.0
    }
    pub fn scaled(self, factor: f64) -> Result<i64, Diagnostic> {
        let value = (self.0 * factor).round();
        if !factor.is_finite() || value < i64::MIN as f64 || value >= i64::MAX as f64 {
            return Err(Diagnostic::new(
                Rule::InvalidMeasurement,
                "length cannot be represented in the selected wire unit",
            ));
        }
        Ok(value as i64)
    }
    pub fn twips(self) -> Result<Twips, Diagnostic> {
        self.scaled(20.).map(Twips)
    }
    pub fn emu(self) -> Result<Emu, Diagnostic> {
        self.scaled(12700.).map(Emu)
    }
    pub fn half_points(self) -> Result<HalfPoints, Diagnostic> {
        self.scaled(2.).map(HalfPoints)
    }
}
macro_rules! integer_measure {
    ($name:ident, $factor:literal) => {
        impl $name {
            pub const fn new(value: i64) -> Self {
                Self(value)
            }
            pub const fn get(self) -> i64 {
                self.0
            }
            pub fn points(self) -> Points {
                Points(self.0 as f64 / $factor)
            }
        }
    };
}
integer_measure!(Twips, 20.);
integer_measure!(Emu, 12700.);
integer_measure!(HalfPoints, 2.);

impl Emu {
    /// Round EMU to twips without lossy floating-point intermediate values.
    pub fn twips(self) -> Twips {
        let value = self.0 as i128;
        let rounded = if value < 0 {
            -((-value + 317) / 635)
        } else {
            (value + 317) / 635
        };
        Twips::new(rounded as i64)
    }
}
