//! Structured rule failures shared by languages and serialization boundaries.
use std::fmt;

/// A structural declaration location, separate from package-part identities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclarationPath(String);
impl DeclarationPath {
    pub fn root(field: &'static str) -> Self {
        Self(field.into())
    }
    pub fn field(&self, field: &'static str) -> Self {
        Self(format!("{}.{field}", self.0))
    }
    pub fn index(&self, index: usize) -> Self {
        Self(format!("{}[{index}]", self.0))
    }
}
impl fmt::Display for DeclarationPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
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
    ProducerError,
    Io,
    InvalidXml,
}
impl Rule {
    /// Stable language-independent diagnostic code (not a display message).
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidMeasurement => "invalid_measurement",
            Self::UnknownStyle => "unknown_style",
            Self::InvalidPackage => "invalid_package",
            Self::InvalidRelationship => "invalid_relationship",
            Self::UnsupportedProfile => "unsupported_profile",
            Self::InvalidContent => "invalid_content",
            Self::DependencyCycle => "dependency_cycle",
            Self::ExcludedDependency => "excluded_dependency",
            Self::SchemaOrder => "schema_order",
            Self::UnsupportedEdit => "unsupported_edit",
            Self::ProducerError => "producer_error",
            Self::Io => "io_error",
            Self::InvalidXml => "invalid_xml",
        }
    }
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
