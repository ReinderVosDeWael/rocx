//! ECMA-376 Part 1 (2016): P1 §17.10, header/footer stories.
use crate::ooxml::Compiler;
use crate::xml::{Node, val};
use crate::{ResolvedBlock, Result, StoryKind};
impl Compiler<'_> {
    pub(crate) fn header_footer(
        &mut self,
        blocks: &[ResolvedBlock],
        kind: &str,
        width: i64,
        existing: Option<&str>,
    ) -> Result<String> {
        let related = existing
            .map(|id| self.package.related(&self.main, &format!("#{id}")))
            .transpose()?
            .flatten();
        let path = related
            .as_ref()
            .map(|(_, path)| path.clone())
            .unwrap_or_else(|| self.package.unique(&format!("word/{kind}"), "xml"));
        let mut root = if related.is_some() {
            self.package.word(&path)?
        } else {
            crate::ooxml::root(if kind == "header" { "w:hdr" } else { "w:ftr" })
        };
        if related.is_none() {
            root.push(Node::new("w:p").with(Node::new("w:pPr").with(val(
                "pStyle",
                if kind == "header" { "Header" } else { "Footer" },
            ))));
        }
        let context = self.context(
            &path,
            if kind == "header" {
                StoryKind::Header
            } else {
                StoryKind::Footer
            },
            width,
        );
        for block in self.blocks(blocks, &context)? {
            root.push(block);
        }
        self.package.put_word(&path, root)?;
        self.package.content_type(
            &path,
            &format!("application/vnd.openxmlformats-officedocument.wordprocessingml.{kind}+xml"),
        )?;
        if let Some((id, _)) = related {
            return Ok(id);
        }
        self.package.link(&self.main, kind, &path)
    }
}

use crate::ooxml::WordParts;
