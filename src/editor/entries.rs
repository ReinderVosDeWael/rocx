//! Read logical note and comment stories from their relationship-selected parts.
use super::{
    ImportedDocument, ImportedInlineNode, ImportedStoryBlock, InlineLocation, header::StoryPart,
    unsupported,
};
use crate::Result;
use crate::ooxml::WordParts;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    Footnote,
    Endnote,
    Comment,
}

impl EntryKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Footnote => "footnote",
            Self::Endnote => "endnote",
            Self::Comment => "comment",
        }
    }
}

/// One read-only source entry. IDs retain their original spelling; they are
/// metadata, not edit anchors, and may coincide across parts.
#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedEntry {
    pub kind: EntryKind,
    pub part_name: String,
    pub id: String,
    pub note_type: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub initials: Option<String>,
    pub(super) element_index: usize,
    content: StoryPart,
}

impl ImportedEntry {
    pub fn blocks(&self) -> &[ImportedStoryBlock] {
        &self.content.blocks
    }
    pub fn inline_structure(&self, index: usize) -> Option<&[ImportedInlineNode]> {
        self.content.inline_structure(index)
    }
}

impl ImportedDocument {
    /// Footnotes, endnotes, then comments in source order, including special
    /// note entries. Missing parts produce no entries.
    pub fn entry_stories(&self) -> Result<&[ImportedEntry]> {
        if let Some(entries) = self.entries.get() {
            return Ok(entries);
        }
        let mut entries = Vec::new();
        for kind in [EntryKind::Footnote, EntryKind::Endnote, EntryKind::Comment] {
            let name = kind.as_str();
            let relation = format!("{name}s");
            let links = self.package.relationships(&self.main, &relation)?;
            if links.is_empty() {
                continue;
            }
            if links.len() != 1 {
                return Err(unsupported(&format!("ambiguous {relation} part")));
            }
            let (_, path) = self
                .package
                .related(&self.main, &relation)?
                .ok_or_else(|| unsupported("missing entry story part"))?;
            let mime = format!(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.{relation}+xml"
            );
            if self.package.content_type_for(&path)?.as_deref() != Some(&mime) {
                return Err(unsupported(&format!("invalid {relation} content type")));
            }
            let root = self.package.word(&path)?;
            if root.name != format!("w:{relation}").as_str() {
                return Err(unsupported(&format!("invalid {relation} root")));
            }
            let tag = format!("w:{name}");
            for (element_index, node) in root
                .elements()
                .enumerate()
                .filter(|(_, n)| n.name == tag.as_str())
            {
                let index = entries.len();
                let attribute = |key| node.get(key).map(str::to_owned);
                entries.push(ImportedEntry {
                    element_index,
                    kind,
                    part_name: path.clone(),
                    id: attribute("w:id").ok_or_else(|| unsupported("missing entry story ID"))?,
                    note_type: attribute("w:type"),
                    author: attribute("w:author"),
                    date: attribute("w:date"),
                    initials: attribute("w:initials"),
                    content: self.read_story_blocks(node, |block_index| InlineLocation::Entry {
                        index,
                        block_index,
                    }),
                });
            }
        }
        Ok(self.entries.get_or_init(|| entries))
    }
}
