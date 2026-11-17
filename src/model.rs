//! Current declarative document model and resolved snapshots.
//!
//! Relevant domain vocabulary: ECMA-376 Part 1 (2016), §§17.2 (main story),
//! 17.3 (paragraphs/runs), and 17.6 (sections).
//! [Official Part 1](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip).
//! The API is a selected authoring model, not a complete schema representation.
use std::path::Path;

use futures_util::future::try_join_all;

use crate::types::*;
use crate::{Condition, Error, Result, Value, render};
use crate::{Footer, Header};
use std::collections::BTreeMap;

/// Top-level declaration. Use `resolve` before rendering or saving.
#[derive(Debug, Default)]
pub struct Document {
    pub sections: Vec<Value<Section>>,
    pub creator: Option<String>,
    pub title: Option<String>,
    pub subject: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub category: Option<String>,
    pub comments: Option<String>,
    pub version: Option<String>,
    pub comment_author: Option<String>,
    pub styles: Vec<StyleDefinition>,
    pub numbering: Vec<NumberingDefinition>,
}

impl Document {
    pub fn new(sections: Vec<Value<Section>>) -> Self {
        Self {
            sections,
            ..Self::default()
        }
    }

    /// Resolve independent branches concurrently, retaining their declaration order.
    /// The first observed error cancels remaining work by dropping its futures.
    pub async fn resolve(self) -> Result<ResolvedDocument> {
        let sections = try_join_all(self.sections.into_iter().enumerate().map(
            |(index, section)| async move {
                let path = format!("sections[{index}]");
                section
                    .resolve()
                    .await
                    .map_err(|e| e.at(&path))?
                    .resolve_at(path)
                    .await
            },
        ))
        .await?;
        Ok(ResolvedDocument {
            sections: sections.into_iter().flatten().collect(),
            metadata: Metadata {
                creator: self.creator,
                title: self.title,
                subject: self.subject,
                description: self.description,
                keywords: self.keywords,
                category: self.category,
                comments: self.comments,
                version: self.version,
            },
            comment_author: self.comment_author,
            styles: self.styles,
            numbering: self.numbering,
            timestamp: crate::render::timestamp(),
        })
    }
}

/// Block children can be immediate components or deferred producers.
pub type BlockChildren = Vec<Value<Block>>;

/// A document section with default layout and a conditional list of blocks.
#[derive(Debug, Default)]
pub struct Section {
    pub properties: Option<SectionProperties>,
    pub headers: BTreeMap<HeaderFooterType, Value<Header>>,
    pub footers: BTreeMap<HeaderFooterType, Value<Footer>>,
    pub children: Value<BlockChildren>,
    pub condition: Condition,
}

impl Section {
    pub fn new(children: BlockChildren) -> Self {
        Self {
            children: children.into(),
            ..Self::default()
        }
    }

    pub fn with_children(mut self, children: impl Into<Value<BlockChildren>>) -> Self {
        self.children = children.into();
        self
    }

    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }

    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedSection>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let content = async {
            let child_path = format!("{path}.children");
            let children = self
                .children
                .resolve()
                .await
                .map_err(|e| e.at(&child_path))?;
            crate::table::resolve_blocks(children, child_path).await
        };
        let header_work = async {
            Ok::<_, Error>(
                try_join_all(self.headers.into_iter().map(|(kind, header)| {
                    let path = format!("{path}.headers.{}", kind.xml());
                    async move {
                        Ok::<_, Error>((
                            kind,
                            header
                                .resolve()
                                .await
                                .map_err(|e| e.at(&path))?
                                .resolve_at(path)
                                .await?,
                        ))
                    }
                }))
                .await?
                .into_iter()
                .filter_map(|(kind, h)| h.map(|h| (kind, h)))
                .collect(),
            )
        };
        let footer_work = async {
            Ok::<_, Error>(
                try_join_all(self.footers.into_iter().map(|(kind, footer)| {
                    let path = format!("{path}.footers.{}", kind.xml());
                    async move {
                        Ok::<_, Error>((
                            kind,
                            footer
                                .resolve()
                                .await
                                .map_err(|e| e.at(&path))?
                                .resolve_at(path)
                                .await?,
                        ))
                    }
                }))
                .await?
                .into_iter()
                .filter_map(|(kind, h)| h.map(|h| (kind, h)))
                .collect(),
            )
        };
        let (blocks, headers, footers) =
            futures_util::future::try_join3(content, header_work, footer_work).await?;
        Ok(Some(ResolvedSection {
            blocks,
            properties: self.properties,
            headers,
            footers,
        }))
    }
}

/// A paragraph with either shorthand text or inline children, never both.
#[derive(Debug)]
pub struct Paragraph {
    pub heading: Option<u8>,
    pub style: Option<String>,
    pub alignment: Option<Alignment>,
    pub spacing_before: Option<f64>,
    pub spacing_after: Option<f64>,
    pub line_spacing: Option<f64>,
    pub left_indent: Option<f64>,
    pub right_indent: Option<f64>,
    pub first_line_indent: Option<f64>,
    pub keep_together: Option<bool>,
    pub keep_with_next: Option<bool>,
    pub page_break_before: Option<bool>,
    pub widow_control: Option<bool>,
    pub numbering: Option<NumberingReference>,
    pub comment_text: Option<Value<String>>,
    pub comment_author: Option<Value<String>>,
    pub text: Option<Value<String>>,
    pub children: Option<Value<InlineChildren>>,
    pub condition: Condition,
}

impl Default for Paragraph {
    fn default() -> Self {
        Self {
            text: Some(Value::default()),
            children: None,
            condition: Condition::default(),
            heading: None,
            style: None,
            alignment: None,
            spacing_before: None,
            spacing_after: None,
            line_spacing: None,
            left_indent: None,
            right_indent: None,
            first_line_indent: None,
            keep_together: None,
            keep_with_next: None,
            page_break_before: None,
            widow_control: None,
            numbering: None,
            comment_text: None,
            comment_author: None,
        }
    }
}

impl Paragraph {
    pub fn new(text: impl Into<Value<String>>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }

    pub fn with_text(mut self, text: impl Into<Value<String>>) -> Self {
        self.text = Some(text.into());
        self.children = None;
        self
    }

    pub fn from_children(children: InlineChildren) -> Self {
        Self::default().with_children(children)
    }

    /// Replace shorthand text with inline children, which may themselves be deferred.
    pub fn with_children(mut self, children: impl Into<Value<InlineChildren>>) -> Self {
        self.text = None;
        self.children = Some(children.into());
        self
    }

    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }

    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedParagraph>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let content = async {
            Ok(match (self.text, self.children) {
                (Some(text), None) => {
                    let path = format!("{path}.text");
                    let text = text.resolve().await.map_err(|e| e.at(&path))?;
                    render::validate_text(&text).map_err(|e| e.at(&path))?;
                    if text.is_empty() {
                        Vec::new()
                    } else {
                        vec![ResolvedTextRun {
                            text,
                            ..ResolvedTextRun::default()
                        }]
                    }
                }
                (None, Some(children)) => {
                    let path = format!("{path}.children");
                    let children = children.resolve().await.map_err(|e| e.at(&path))?;
                    try_join_all(children.into_iter().enumerate().map(|(index, child)| {
                        let path = format!("{path}[{index}]");
                        async move {
                            match child.resolve().await.map_err(|e| e.at(&path))? {
                                Inline::TextRun(run) => run.resolve_at(path).await,
                                Inline::Tab(tab) => tab.resolve_at(path).await,
                                Inline::Break(br) => br.resolve_at(path).await,
                                Inline::ImageRun(image) => image.resolve_at(path).await,
                            }
                        }
                    }))
                    .await?
                    .into_iter()
                    .flatten()
                    .collect()
                }
                _ => return Err(Error::InvalidParagraphContent.at(&path)),
            })
        };
        let (runs, comment): (Vec<ResolvedTextRun>, _) = futures_util::future::try_join(
            content,
            crate::components::resolve_comment(self.comment_text, self.comment_author, &path),
        )
        .await?;
        let text = runs.iter().map(|run| run.text.as_str()).collect();
        Ok(Some(ResolvedParagraph {
            text,
            runs,
            comment,
            numbering: self.numbering,
            format: ParagraphFormat {
                heading: self.heading,
                style: self.style,
                alignment: self.alignment,
                spacing_before: self.spacing_before,
                spacing_after: self.spacing_after,
                line_spacing: self.line_spacing,
                left_indent: self.left_indent,
                right_indent: self.right_indent,
                first_line_indent: self.first_line_indent,
                keep_together: self.keep_together,
                keep_with_next: self.keep_with_next,
                page_break_before: self.page_break_before,
                widow_control: self.widow_control,
            },
        }))
    }
}

pub type InlineChildren = Vec<Value<Inline>>;

/// Inline content; additional CMI inline components can be added later.
#[derive(Debug)]
#[non_exhaustive]
pub enum Inline {
    TextRun(TextRun),
    Tab(crate::Tab),
    Break(crate::Break),
    ImageRun(crate::ImageRun),
}

/// A formatted text run with independently deferred text, comments and a condition.
#[derive(Debug, Default)]
pub struct TextRun {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub font: Option<String>,
    pub size: Option<f64>,
    pub color: Option<[u8; 3]>,
    pub superscript: Option<bool>,
    pub subscript: Option<bool>,
    pub strike: Option<bool>,
    pub all_caps: Option<bool>,
    pub small_caps: Option<bool>,
    pub comment_text: Option<Value<String>>,
    pub comment_author: Option<Value<String>>,
    pub text: Value<String>,
    pub condition: Condition,
}

impl TextRun {
    pub fn new(text: impl Into<Value<String>>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    pub fn with_text(mut self, text: impl Into<Value<String>>) -> Self {
        self.text = text.into();
        self
    }

    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }

    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedTextRun>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let text_path = format!("{path}.text");
        let text_work = async {
            let text = self.text.resolve().await.map_err(|e| e.at(&text_path))?;
            render::validate_text(&text).map_err(|e| e.at(&text_path))?;
            Ok::<_, Error>(text)
        };
        let (text, comment) = futures_util::future::try_join(
            text_work,
            crate::components::resolve_comment(self.comment_text, self.comment_author, &path),
        )
        .await?;
        Ok(Some(ResolvedTextRun {
            text,
            comment,
            kind: ResolvedInlineKind::Text,
            format: RunFormat {
                bold: self.bold,
                italic: self.italic,
                underline: self.underline,
                font: self.font.filter(|s| !s.is_empty()),
                size: self.size.filter(|v| *v != 0.),
                color: self.color,
                superscript: self.superscript,
                subscript: self.subscript,
                strike: self.strike,
                all_caps: self.all_caps,
                small_caps: self.small_caps,
            },
        }))
    }
}

impl From<TextRun> for Inline {
    fn from(value: TextRun) -> Self {
        Self::TextRun(value)
    }
}
impl From<TextRun> for Value<Inline> {
    fn from(value: TextRun) -> Self {
        Self::Ready(Inline::TextRun(value))
    }
}

/// Mixed block content; more CMI component variants will be added in later milestones.
#[derive(Debug)]
#[non_exhaustive]
pub enum Block {
    Paragraph(Paragraph),
    Table(crate::Table),
}

impl From<Paragraph> for Block {
    fn from(value: Paragraph) -> Self {
        Self::Paragraph(value)
    }
}
impl From<Paragraph> for Value<Block> {
    fn from(value: Paragraph) -> Self {
        Self::Ready(Block::Paragraph(value))
    }
}

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
    pub fn sections(&self) -> &[ResolvedSection] {
        &self.sections
    }

    pub fn to_docx_with_template(&self, template: &DocumentTemplate) -> Result<Vec<u8>> {
        render::render_with_template(self, Some(template))
    }
    pub fn save_with_template(
        &self,
        path: impl AsRef<Path>,
        template: &DocumentTemplate,
    ) -> Result<()> {
        let bytes = self.to_docx_with_template(template)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }
    pub fn to_docx(&self) -> Result<Vec<u8>> {
        render::render(self)
    }

    /// Build the entire package before opening the destination. File I/O is synchronous.
    /// Filesystem errors during the final write can still leave a partial file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let bytes = self.to_docx()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSection {
    pub(crate) blocks: Vec<crate::ResolvedBlock>,
    pub(crate) properties: Option<SectionProperties>,
    pub(crate) headers: BTreeMap<HeaderFooterType, crate::ResolvedHeaderFooter>,
    pub(crate) footers: BTreeMap<HeaderFooterType, crate::ResolvedHeaderFooter>,
}
impl ResolvedSection {
    pub fn paragraphs(&self) -> Vec<&ResolvedParagraph> {
        self.blocks
            .iter()
            .filter_map(|b| {
                if let crate::ResolvedBlock::Paragraph(p) = b {
                    Some(p)
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn blocks(&self) -> &[crate::ResolvedBlock] {
        &self.blocks
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedParagraph {
    pub(crate) text: String,
    pub(crate) runs: Vec<ResolvedTextRun>,
    pub(crate) format: ParagraphFormat,
    pub(crate) comment: Option<crate::ResolvedComment>,
    pub(crate) numbering: Option<NumberingReference>,
}
impl ResolvedParagraph {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn format(&self) -> &ParagraphFormat {
        &self.format
    }
    pub fn comment(&self) -> Option<&crate::ResolvedComment> {
        self.comment.as_ref()
    }
    pub fn runs(&self) -> &[ResolvedTextRun] {
        &self.runs
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResolvedTextRun {
    pub(crate) text: String,
    pub(crate) format: RunFormat,
    pub(crate) comment: Option<crate::ResolvedComment>,
    pub(crate) kind: ResolvedInlineKind,
}
impl ResolvedTextRun {
    pub fn format(&self) -> &RunFormat {
        &self.format
    }
    pub fn comment(&self) -> Option<&crate::ResolvedComment> {
        self.comment.as_ref()
    }
    pub fn image(&self) -> Option<&crate::ResolvedImage> {
        if let ResolvedInlineKind::Image(i) = &self.kind {
            Some(i)
        } else {
            None
        }
    }
    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Metadata {
    pub creator: Option<String>,
    pub title: Option<String>,
    pub subject: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub category: Option<String>,
    pub comments: Option<String>,
    pub version: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum ResolvedInlineKind {
    #[default]
    Text,
    Tab,
    Break(BreakType),
    Image(crate::ResolvedImage),
}
