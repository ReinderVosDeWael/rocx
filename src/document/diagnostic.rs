//! Structured rule failures shared by languages and serialization boundaries.
use std::fmt;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Rule {
    InvalidMeasurement,
    UnknownStyle,
    InvalidPackage,
    InvalidRelationship,
    UnsupportedProfile,
    InvalidContent,
    DependencyCycle,
    ExcludedDependency,
    SchemaOrder,
    UnsupportedEdit,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub rule: Rule,
    pub location: Option<String>,
    pub message: String,
}
impl Diagnostic {
    pub fn new(rule: Rule, message: impl Into<String>) -> Self {
        Self {
            rule,
            location: None,
            message: message.into(),
        }
    }
    pub fn at(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(location) = &self.location {
            write!(f, "{location}: ")?;
        }
        f.write_str(&self.message)
    }
}
impl std::error::Error for Diagnostic {}
