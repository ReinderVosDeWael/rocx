use std::path::Path;

use futures_util::future::try_join_all;

use crate::{Condition, Result, Value, render};

/// Top-level declaration. Use `resolve` before rendering or saving.
#[derive(Debug, Default)]
pub struct Document {
    pub sections: Vec<Value<Section>>,
}

impl Document {
    pub fn new(sections: Vec<Value<Section>>) -> Self {
        Self { sections }
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
                    .resolve(path)
                    .await
            },
        ))
        .await?;
        Ok(ResolvedDocument {
            sections: sections.into_iter().flatten().collect(),
        })
    }
}

/// Block children can be immediate components or deferred producers.
pub type BlockChildren = Vec<Value<Block>>;

/// A document section with default layout and a conditional list of blocks.
#[derive(Debug, Default)]
pub struct Section {
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

    async fn resolve(self, path: String) -> Result<Option<ResolvedSection>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let child_path = format!("{path}.children");
        let children = self
            .children
            .resolve()
            .await
            .map_err(|e| e.at(&child_path))?;
        let paragraphs = try_join_all(children.into_iter().enumerate().map(|(index, child)| {
            let path = format!("{child_path}[{index}]");
            async move {
                match child.resolve().await.map_err(|e| e.at(&path))? {
                    Block::Paragraph(paragraph) => paragraph.resolve(path).await,
                }
            }
        }))
        .await?;
        Ok(Some(ResolvedSection {
            paragraphs: paragraphs.into_iter().flatten().collect(),
        }))
    }
}

/// A plain-text paragraph. An empty string is an explicit empty paragraph.
#[derive(Debug, Default)]
pub struct Paragraph {
    pub text: Value<String>,
    pub condition: Condition,
}

impl Paragraph {
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

    async fn resolve(self, path: String) -> Result<Option<ResolvedParagraph>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let path = format!("{path}.text");
        let text = self.text.resolve().await.map_err(|e| e.at(&path))?;
        render::validate_text(&text).map_err(|e| e.at(path))?;
        Ok(Some(ResolvedParagraph { text }))
    }
}

/// Mixed block content; more CMI component variants will be added in later milestones.
#[derive(Debug)]
#[non_exhaustive]
pub enum Block {
    Paragraph(Paragraph),
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

/// Immutable, validated content. Rendering never invokes conditions or producers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDocument {
    pub(crate) sections: Vec<ResolvedSection>,
}

impl ResolvedDocument {
    pub fn sections(&self) -> &[ResolvedSection] {
        &self.sections
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSection {
    pub(crate) paragraphs: Vec<ResolvedParagraph>,
}
impl ResolvedSection {
    pub fn paragraphs(&self) -> &[ResolvedParagraph] {
        &self.paragraphs
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedParagraph {
    pub(crate) text: String,
}
impl ResolvedParagraph {
    pub fn text(&self) -> &str {
        &self.text
    }
}
