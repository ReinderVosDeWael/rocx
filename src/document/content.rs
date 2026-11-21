//! Immutable document content; no serialization or execution effects.
use super::*;
use std::collections::BTreeMap;
/// Immutable resolved content. Rendering never invokes conditions or producers.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedDocument {
    pub(crate) sections: Vec<ResolvedSection>,
    pub(crate) notes: Vec<ResolvedNote>,
    pub(crate) metadata: Metadata,
    pub(crate) comment_author: Option<String>,
    pub(crate) styles: Vec<StyleDefinition>,
    pub(crate) numbering: Vec<NumberingDefinition>,
    pub(crate) defaults: Option<DocumentDefaults>,
    pub(crate) timestamp: String,
}

impl ResolvedDocument {
    pub fn defaults(&self) -> Option<&DocumentDefaults> {
        self.defaults.as_ref()
    }
    pub fn notes(&self) -> &[ResolvedNote] {
        &self.notes
    }
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }
    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }
    pub fn comment_author(&self) -> Option<&str> {
        self.comment_author.as_deref()
    }
    pub fn styles(&self) -> &[StyleDefinition] {
        &self.styles
    }
    pub fn numbering(&self) -> &[NumberingDefinition] {
        &self.numbering
    }
    pub fn sections(&self) -> &[ResolvedSection] {
        &self.sections
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSection {
    pub(crate) blocks: Vec<super::ResolvedBlock>,
    pub(crate) properties: Option<SectionProperties>,
    pub(crate) headers: BTreeMap<HeaderFooterType, super::ResolvedHeaderFooter>,
    pub(crate) footers: BTreeMap<HeaderFooterType, super::ResolvedHeaderFooter>,
}
impl ResolvedSection {
    pub fn properties(&self) -> Option<&SectionProperties> {
        self.properties.as_ref()
    }
    pub fn headers(&self) -> &BTreeMap<HeaderFooterType, ResolvedHeaderFooter> {
        &self.headers
    }
    pub fn footers(&self) -> &BTreeMap<HeaderFooterType, ResolvedHeaderFooter> {
        &self.footers
    }
    pub fn paragraphs(&self) -> Vec<&ResolvedParagraph> {
        self.blocks
            .iter()
            .filter_map(|b| {
                if let super::ResolvedBlock::Paragraph(p) = b {
                    Some(p)
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn blocks(&self) -> &[super::ResolvedBlock] {
        &self.blocks
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedParagraph {
    pub(crate) text: String,
    pub(crate) runs: Vec<ResolvedInline>,
    pub(crate) format: ParagraphFormat,
    pub(crate) comment: Option<super::ResolvedComment>,
    pub(crate) numbering: Option<NumberingReference>,
}
impl ResolvedParagraph {
    pub fn numbering(&self) -> Option<&NumberingReference> {
        self.numbering.as_ref()
    }
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn format(&self) -> &ParagraphFormat {
        &self.format
    }
    pub fn comment(&self) -> Option<&super::ResolvedComment> {
        self.comment.as_ref()
    }
    pub fn runs(&self) -> &[ResolvedInline] {
        &self.runs
    }
}

/// A resolved leaf run or inline container, without spare content tags.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ResolvedInline {
    Shape(ShapeProperties),
    ContentControl(super::ResolvedInlineContentControl),
    Field(ResolvedField),
    NoteReference(NoteTarget),
    RangeStart {
        id: RangeId,
        kind: RangeKind,
    },
    RangeEnd(RangeId),
    Hyperlink(ResolvedHyperlink),
    Text {
        text: String,
        format: RunFormat,
        comment: Option<ResolvedComment>,
        revision: Option<super::TextRevision>,
    },
    Tab {
        format: RunFormat,
        comment: Option<ResolvedComment>,
    },
    Break {
        kind: BreakType,
        format: RunFormat,
        comment: Option<ResolvedComment>,
    },
    Image {
        image: ResolvedImage,
        format: RunFormat,
        comment: Option<ResolvedComment>,
    },
}
impl ResolvedInline {
    /// Containers have no run format; format their display children instead.
    pub fn format(&self) -> Option<&RunFormat> {
        match self {
            Self::Text { format, .. }
            | Self::Tab { format, .. }
            | Self::Break { format, .. }
            | Self::Image { format, .. } => Some(format),
            Self::Field(_)
            | Self::Shape(_)
            | Self::ContentControl(_)
            | Self::Hyperlink(_)
            | Self::RangeStart { .. }
            | Self::RangeEnd(_)
            | Self::NoteReference(_) => None,
        }
    }
    pub fn comment(&self) -> Option<&ResolvedComment> {
        match self {
            Self::Text { comment, .. }
            | Self::Tab { comment, .. }
            | Self::Break { comment, .. }
            | Self::Image { comment, .. } => comment.as_ref(),
            Self::Field(_)
            | Self::Shape(_)
            | Self::ContentControl(_)
            | Self::Hyperlink(_)
            | Self::RangeStart { .. }
            | Self::RangeEnd(_)
            | Self::NoteReference(_) => None,
        }
    }
    pub fn image(&self) -> Option<&ResolvedImage> {
        if let Self::Image { image, .. } = self {
            Some(image)
        } else {
            None
        }
    }
    pub fn revision(&self) -> Option<&super::TextRevision> {
        match self {
            Self::Text { revision, .. } => revision.as_ref(),
            _ => None,
        }
    }
    /// Display text only: borrowed for leaves, projected for containers.
    pub fn text(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Self::Field(field) => return field.text().into(),
            Self::ContentControl(control) => return control.text().into(),
            Self::Hyperlink(link) => return link.text().into(),
            Self::Text { text, revision, .. }
                if !revision.as_ref().is_some_and(|r| {
                    matches!(
                        r.kind,
                        super::RevisionKind::Deletion | super::RevisionKind::MoveSource(_)
                    )
                }) =>
            {
                text
            }
            Self::Tab { .. } => "\t",
            Self::Break {
                kind: BreakType::Line | BreakType::TextWrapping,
                ..
            } => "\n",
            _ => "",
        }
        .into()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Metadata {
    pub creator: Option<String>,
    pub title: Option<String>,
    pub subject: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub category: Option<String>,
    pub comments: Option<String>,
    pub version: Option<String>,
}
setters!(Metadata {
    with_creator => creator: optional_into<String>,
    with_title => title: optional_into<String>,
    with_subject => subject: optional_into<String>,
    with_description => description: optional_into<String>,
    with_keywords => keywords: optional_into<String>,
    with_category => category: optional_into<String>,
    with_comments => comments: optional_into<String>,
    with_version => version: optional_into<String>,
});
