//! External hyperlinks; P1 (2016) §17.16.22 and P2 (2021) §§6.4–6.5.
use super::*;

/// An absolute external URI/IRI, preserved verbatim rather than normalized/fetched.
/// Relative and bookmark-only targets are not supported in R2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyperlinkTarget(String);
impl HyperlinkTarget {
    pub fn external(value: impl Into<String>) -> Result<Self, Diagnostic> {
        let value = value.into();
        let scheme = value.split_once(':').map(|(scheme, _)| scheme);
        let valid_scheme = scheme.is_some_and(|scheme| {
            scheme.starts_with(|c: char| c.is_ascii_alphabetic())
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        });
        let bytes = value.as_bytes();
        let valid_escapes = bytes.iter().enumerate().all(|(i, b)| {
            *b != b'%'
                || bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit)
                    && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit)
        });
        if !valid_scheme
            || !valid_escapes
            || value.chars().any(|c| {
                c.is_control()
                    || c.is_whitespace()
                    || matches!(c, '<' | '>' | '"' | '{' | '}' | '|' | '\\' | '^' | '`')
            })
        {
            return Err(Diagnostic::new(
                Rule::InvalidRelationship,
                "hyperlink target must be an absolute external URI/IRI with valid percent escapes",
            )
            .at("target"));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Optional hyperlink attributes, separate from the formatting of display runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HyperlinkProperties {
    pub tooltip: Option<String>,
    pub history: Option<bool>,
}

/// Immutable display content; text is projected rather than stored twice.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedHyperlink {
    target: HyperlinkTarget,
    children: Vec<ResolvedInline>,
    properties: HyperlinkProperties,
}
impl ResolvedHyperlink {
    pub fn new(
        target: HyperlinkTarget,
        children: Vec<ResolvedInline>,
        properties: HyperlinkProperties,
    ) -> Result<Self, Diagnostic> {
        for (index, child) in children.iter().enumerate() {
            if matches!(child, ResolvedInline::Hyperlink(_)) {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "nested hyperlinks are not supported",
                )
                .at(format!("children[{index}]")));
            }
            child.validate().map_err(|mut error| {
                error.location = Some(format!(
                    "children[{index}].format.{}",
                    error.location.unwrap_or_default()
                ));
                error
            })?;
        }
        Ok(Self {
            target,
            children,
            properties,
        })
    }
    pub fn target(&self) -> &HyperlinkTarget {
        &self.target
    }
    pub fn children(&self) -> &[ResolvedInline] {
        &self.children
    }
    pub fn properties(&self) -> &HyperlinkProperties {
        &self.properties
    }
    pub fn text(&self) -> String {
        inline_text(&self.children)
    }
}

/// Display text projection, including containers; images/page breaks add no text.
pub fn inline_text(children: &[ResolvedInline]) -> String {
    let mut text = String::new();
    for child in children {
        text.push_str(&child.text());
    }
    text
}
