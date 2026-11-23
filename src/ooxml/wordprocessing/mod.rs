//! ECMA-376 Part 1 (2016): P1 §§17.2–17.4, story block content.
use crate::ooxml::Compiler;
use crate::xml::Node;
use crate::{ResolvedBlock, Result};
impl Compiler<'_> {
    pub(crate) fn blocks(
        &mut self,
        blocks: &[ResolvedBlock],
        context: &crate::ooxml::PartContext,
    ) -> Result<Vec<Node>> {
        blocks
            .iter()
            .map(|b| match b {
                ResolvedBlock::ContentControl(control) => {
                    self.block_content_control(control, context)
                }
                ResolvedBlock::Paragraph(p) => self.paragraph(p, context),
                ResolvedBlock::Table(t) => self.table(t, context),
            })
            .collect()
    }
}
mod comments;
mod content_control;
mod fields;
pub(crate) mod format;
mod paragraph;
pub(crate) mod run_child;
pub(crate) mod sections;
mod stories;
pub(crate) mod styles;
mod table;
pub(crate) mod text;
