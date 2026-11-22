//! Current declarative document model and resolved snapshots.
//!
//! Relevant domain vocabulary: ECMA-376 Part 1 (2016), §§17.2 (main story),
//! 17.3 (paragraphs/runs), and 17.6 (sections).
//! [Official Part 1](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip).
//! The API is a selected authoring model, not a complete schema representation.

use crate::document::*;
use crate::{Condition, Value};
use std::collections::BTreeMap;

/// Top-level declaration. Use `resolve` before rendering or saving.
#[derive(Debug, Default)]
pub struct Document {
    pub sections: Vec<Value<Section>>,
    pub notes: Value<Vec<Value<Note>>>,
    pub metadata: Value<Metadata>,
    pub comment_author: Value<Option<String>>,
    pub timestamp: Value<Option<String>>,
    pub styles: Value<Vec<StyleDefinition>>,
    pub numbering: Value<Vec<NumberingDefinition>>,
    pub defaults: Value<Option<DocumentDefaults>>,
}
setters!(Document {
    with_sections => sections: value<Vec<Value<Section>>>,
    with_notes => notes: into<Value<Vec<Value<Note>>>>,
    with_metadata => metadata: into<Value<Metadata>>,
    with_comment_author => comment_author: into<Value<Option<String>>>,
    with_timestamp => timestamp: into<Value<Option<String>>>,
    with_styles => styles: into<Value<Vec<StyleDefinition>>>,
    with_numbering => numbering: into<Value<Vec<NumberingDefinition>>>,
    with_defaults => defaults: into<Value<Option<DocumentDefaults>>>,
});

impl Document {
    pub fn new(sections: Vec<Value<Section>>) -> Self {
        Self {
            sections,
            ..Self::default()
        }
    }
}

/// Block children can be immediate components or deferred producers.
pub type BlockChildren = Vec<Value<Block>>;

/// A document section with default layout and a conditional list of blocks.
#[derive(Debug, Default)]
pub struct Section {
    pub properties: Value<Option<SectionProperties>>,
    pub headers: BTreeMap<HeaderFooterType, Value<Header>>,
    pub footers: BTreeMap<HeaderFooterType, Value<Footer>>,
    pub children: Value<BlockChildren>,
    pub condition: Condition,
}
setters!(Section {
    with_properties => properties: into<Value<Option<SectionProperties>>>,
    with_headers => headers: value<BTreeMap<HeaderFooterType, Value<Header>>>,
    with_footers => footers: value<BTreeMap<HeaderFooterType, Value<Footer>>>,
});

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
}

/// A paragraph with either shorthand text or inline children, never both.
#[derive(Debug)]
pub struct Paragraph {
    pub format: Value<ParagraphFormat>,
    pub numbering: Value<Option<NumberingReference>>,
    pub comment_text: Option<Value<String>>,
    pub comment_author: Option<Value<String>>,
    pub content: ParagraphContent,
    pub condition: Condition,
}
setters!(Paragraph {
    with_numbering => numbering: into<Value<Option<NumberingReference>>>,
    with_comment_text => comment_text: optional_into<Value<String>>,
    with_comment_author => comment_author: optional_into<Value<String>>,
});

impl Default for Paragraph {
    fn default() -> Self {
        Self {
            format: Value::default(),
            content: ParagraphContent::Text(Value::default()),
            condition: Condition::default(),
            numbering: Value::default(),
            comment_text: None,
            comment_author: None,
        }
    }
}

impl Paragraph {
    pub fn with_format(mut self, format: impl Into<Value<ParagraphFormat>>) -> Self {
        self.format = format.into();
        self
    }
    pub fn new(text: impl Into<Value<String>>) -> Self {
        Self {
            content: ParagraphContent::Text(text.into()),
            ..Self::default()
        }
    }

    pub fn with_text(mut self, text: impl Into<Value<String>>) -> Self {
        self.content = ParagraphContent::Text(text.into());
        self
    }

    pub fn from_children(children: InlineChildren) -> Self {
        Self::default().with_children(children)
    }

    /// Replace shorthand text with inline children, which may themselves be deferred.
    pub fn with_children(mut self, children: impl Into<Value<InlineChildren>>) -> Self {
        self.content = ParagraphContent::Inlines(children.into());
        self
    }

    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }
}

pub type InlineChildren = Vec<Value<Inline>>;

/// Inline content; additional variants can be added as features are implemented.
#[derive(Debug)]
#[non_exhaustive]
pub enum Inline {
    Field(Field),
    NoteReference(NoteReference),
    RangeStart(RangeStart),
    RangeEnd(RangeEnd),
    Hyperlink(Hyperlink),
    ContentControl(InlineContentControl),
    /// Reuse immutable content without cloning authoring futures or guards.
    Resolved(ResolvedInline),
    TextRun(TextRun),
    Tab(crate::Tab),
    Break(crate::Break),
    ImageRun(crate::ImageRun),
    ShapeRun(ShapeRun),
}

/// A formatted text run with independently deferred text, comments and a condition.
#[derive(Debug, Default)]
pub struct TextRun {
    pub revision: Value<Option<crate::TextRevision>>,
    pub format: Value<RunFormat>,
    pub comment_text: Option<Value<String>>,
    pub comment_author: Option<Value<String>>,
    pub text: Value<String>,
    pub condition: Condition,
}
setters!(TextRun {
    with_revision => revision: into<Value<Option<crate::TextRevision>>>,
    with_comment_text => comment_text: optional_into<Value<String>>,
    with_comment_author => comment_author: optional_into<Value<String>>,
});

impl TextRun {
    pub fn with_format(mut self, format: impl Into<Value<RunFormat>>) -> Self {
        self.format = format.into();
        self
    }
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

/// Mixed block content; additional variants can be added as features are implemented.
#[derive(Debug)]
#[non_exhaustive]
pub enum Block {
    ContentControl(BlockContentControl),
    /// Reuse immutable content without cloning authoring futures or guards.
    Resolved(ResolvedBlock),
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

impl From<ResolvedBlock> for Block {
    fn from(value: ResolvedBlock) -> Self {
        Self::Resolved(value)
    }
}
impl From<ResolvedBlock> for Value<Block> {
    fn from(value: ResolvedBlock) -> Self {
        Self::ready(Block::Resolved(value))
    }
}
impl From<ResolvedInline> for Inline {
    fn from(value: ResolvedInline) -> Self {
        Self::Resolved(value)
    }
}
impl From<ResolvedInline> for Value<Inline> {
    fn from(value: ResolvedInline) -> Self {
        Self::ready(Inline::Resolved(value))
    }
}

mod components;
mod shape;
pub use shape::ShapeRun;
mod content_control;
pub use content_control::{BlockContentControl, InlineContentControl};
mod field;
mod hyperlink;
pub use field::Field;
mod notes;
mod range;
pub use hyperlink::Hyperlink;
pub use notes::{Comment, Note, NoteReference};
pub use range::{RangeEnd, RangeStart};
pub(crate) mod table;
pub use components::{Break, Footer, Header, ImageData, ImageRun, Tab};
pub use table::{Table, TableCell, TableRow};

/// A paragraph has exactly one content source (P1 §17.3).
#[derive(Debug)]
pub enum ParagraphContent {
    Text(Value<String>),
    Inlines(Value<InlineChildren>),
}
