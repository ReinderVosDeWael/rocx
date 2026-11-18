//! Resolved image, comment and story content values.
use super::*;
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedImage {
    pub(crate) data: Vec<u8>,
    pub(crate) extension: String,
    pub(crate) content_type: String,
    pub(crate) width: i64,
    pub(crate) height: i64,
    pub(crate) name: String,
    pub(crate) alt_text: Option<AltText>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedComment {
    pub text: String,
    pub author: Option<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedHeaderFooter {
    pub(crate) blocks: Vec<ResolvedBlock>,
}
impl ResolvedImage {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn alt_text(&self) -> Option<&AltText> {
        self.alt_text.as_ref()
    }
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    pub fn dimensions_emu(&self) -> (i64, i64) {
        (self.width, self.height)
    }
    pub fn content_type(&self) -> &str {
        &self.content_type
    }
}
impl ResolvedHeaderFooter {
    pub fn blocks(&self) -> &[ResolvedBlock] {
        &self.blocks
    }
}
