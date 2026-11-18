//! Immutable document content; no serialization or execution effects.
use super::*;
use std::collections::BTreeMap;
/// Immutable resolved content. Rendering never invokes conditions or producers.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedDocument {
    pub(crate) sections: Vec<ResolvedSection>,
    pub(crate) metadata: Metadata,
    pub(crate) comment_author: Option<String>,
    pub(crate) styles: Vec<StyleDefinition>,
    pub(crate) numbering: Vec<NumberingDefinition>,
    pub(crate) timestamp: String,
}

impl ResolvedDocument {
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

/// A resolved inline value. Every variant has its own content, without spare tags.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ResolvedInline {
    Text {
        text: String,
        format: RunFormat,
        comment: Option<ResolvedComment>,
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
    pub fn format(&self) -> &RunFormat {
        match self {
            Self::Text { format, .. }
            | Self::Tab { format, .. }
            | Self::Break { format, .. }
            | Self::Image { format, .. } => format,
        }
    }
    pub fn comment(&self) -> Option<&ResolvedComment> {
        match self {
            Self::Text { comment, .. }
            | Self::Tab { comment, .. }
            | Self::Break { comment, .. }
            | Self::Image { comment, .. } => comment.as_ref(),
        }
    }
    pub fn image(&self) -> Option<&ResolvedImage> {
        if let Self::Image { image, .. } = self {
            Some(image)
        } else {
            None
        }
    }
    pub fn text(&self) -> &str {
        match self {
            Self::Text { text, .. } => text,
            Self::Tab { .. } => "\t",
            Self::Break {
                kind: BreakType::Line | BreakType::TextWrapping,
                ..
            } => "\n",
            _ => "",
        }
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
