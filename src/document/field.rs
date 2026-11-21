//! Field instructions and cached results; P1 (2016) §17.16. No evaluation.
use super::*;

/// Common instructions or an uninterpreted instruction preserved verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldInstruction {
    Page,
    NumPages,
    Reference(BookmarkName),
    /// A heading-level 1–3 TOC request, not a computed table of contents.
    TableOfContents,
    Raw(RawFieldInstruction),
}
/// Checked XML text, not parsed field syntax. Construct with `FieldInstruction::raw`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFieldInstruction(String);
impl FieldInstruction {
    pub fn raw(value: impl Into<String>) -> Result<Self, Diagnostic> {
        let value = value.into();
        if value.trim().is_empty() || value.chars().any(|c| !matches!(c,
            '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')) {
            return Err(Diagnostic::new(Rule::InvalidContent, "field instruction must be nonempty XML text").at("instruction"));
        }
        Ok(Self::Raw(RawFieldInstruction(value)))
    }
    pub fn text(&self) -> String {
        match self {
            Self::Page => " PAGE ".into(),
            Self::NumPages => " NUMPAGES ".into(),
            Self::Reference(name) => format!(" REF {} ", name.as_str()),
            Self::TableOfContents => " TOC \\o \"1-3\" \\h \\z \\u ".into(),
            Self::Raw(value) => value.0.clone(),
        }
    }
    pub fn bookmark_name(&self) -> Option<&BookmarkName> {
        if let Self::Reference(name) = self {
            Some(name)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FieldForm {
    #[default]
    Simple,
    Complex,
}
/// Absent flags remain absent; dirty requests consumer update, lock inhibits it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldProperties {
    pub form: FieldForm,
    pub dirty: Option<bool>,
    pub locked: Option<bool>,
}
setters!(FieldProperties {
    with_form => form: value<FieldForm>,
    with_dirty => dirty: optional<bool>,
    with_locked => locked: optional<bool>,
});

/// A paragraph-local, balanced field with immutable cached display inlines.
/// Nested fields occur only in results; markers cannot escape into other stories.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedField {
    instruction: FieldInstruction,
    children: Vec<ResolvedInline>,
    properties: FieldProperties,
}
impl ResolvedField {
    pub fn new(
        instruction: FieldInstruction,
        children: Vec<ResolvedInline>,
        properties: FieldProperties,
    ) -> Result<Self, Diagnostic> {
        for (index, child) in children.iter().enumerate() {
            if matches!(
                child,
                ResolvedInline::RangeStart { .. } | ResolvedInline::RangeEnd(_)
            ) {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "range markers within cached field results are unsupported",
                )
                .at(format!("children[{index}]")));
            }
            child.validate().map_err(|mut e| {
                e.location = Some(format!(
                    "children[{index}]{}",
                    e.location.map(|p| format!(".{p}")).unwrap_or_default()
                ));
                e
            })?;
        }
        Ok(Self {
            instruction,
            children,
            properties,
        })
    }
    pub fn instruction(&self) -> &FieldInstruction {
        &self.instruction
    }
    pub fn children(&self) -> &[ResolvedInline] {
        &self.children
    }
    pub fn properties(&self) -> &FieldProperties {
        &self.properties
    }
    pub fn text(&self) -> String {
        inline_text(&self.children)
    }
}
