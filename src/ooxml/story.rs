//! Logical story identity is independent of physical part ownership.
use crate::opc::PartName;
use crate::{Diagnostic, StoryId, StoryKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryContent {
    Blocks,
    CommentText,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryDescriptor {
    pub id: StoryId,
    pub kind: StoryKind,
    pub owner: PartName,
    pub content: StoryContent,
}
#[derive(Debug, Default)]
pub struct StoryRegistry {
    stories: Vec<StoryDescriptor>,
}
impl StoryRegistry {
    /// Always register a logical story, even if another story has the same owner/kind.
    pub fn register(
        &mut self,
        kind: StoryKind,
        owner: PartName,
        content: StoryContent,
    ) -> Result<StoryId, Diagnostic> {
        let id = self
            .stories
            .last()
            .map(|s| s.id.next())
            .transpose()?
            .unwrap_or_default();
        self.stories.push(StoryDescriptor {
            id,
            kind,
            owner,
            content,
        });
        Ok(id)
    }
    pub fn get(&self, id: StoryId) -> Option<&StoryDescriptor> {
        self.stories.get(id.get() as usize)
    }
    pub fn stories(&self) -> &[StoryDescriptor] {
        &self.stories
    }
}
