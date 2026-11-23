//! ECMA-376 Part 1 (2016): P1 §17.13.4, comment anchors and content.
use crate::ooxml::Compiler;
use crate::xml::{Node, val};
use crate::{CommentId, Diagnostic, Error, ResolvedComment, Result, Rule, StoryKind};
impl Compiler<'_> {
    pub(crate) fn comment(
        &mut self,
        nodes: Vec<Node>,
        comment: &Option<ResolvedComment>,
        context: &crate::ooxml::PartContext,
    ) -> Result<Vec<Node>> {
        let Some(comment) = comment else {
            return Ok(nodes);
        };
        if context.kind != StoryKind::Body {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "comment generation is currently supported only in the main body story",
            )
            .at(context.source.as_str())
            .into());
        }
        if nodes.is_empty() {
            return Err(Error::Validation(
                "comments must be anchored to at least one run".into(),
            ));
        }
        let anchor = self.allocate_range(&crate::RangeKind::Comment(comment.clone()), context)?;
        let mut out = vec![anchor.start()];
        out.extend(nodes);
        out.extend(anchor.end());
        Ok(out)
    }
    pub(crate) fn add_comment(
        &mut self,
        comment: &ResolvedComment,
        context: &crate::ooxml::PartContext,
    ) -> Result<CommentId> {
        let metadata = crate::CommentMetadata {
            author: comment.author.clone(),
            date: None,
            initials: Some(String::new()),
        };
        let (path, mut node, id, _) =
            self.comment_definition(&metadata, context, crate::ooxml::StoryContent::CommentText)?;
        for line in comment.text.split('\n') {
            node.push(
                Node::new("w:p")
                    .with(Node::new("w:pPr").with(val("pStyle", "CommentText")))
                    .with(text_run(line)),
            );
        }
        self.store_comment(&path, node)?;
        Ok(id)
    }
    pub(crate) fn add_structured_comment(
        &mut self,
        body: &crate::ResolvedCommentBody,
        context: &crate::ooxml::PartContext,
    ) -> Result<CommentId> {
        let (path, mut node, id, story) =
            self.comment_definition(body.metadata(), context, crate::ooxml::StoryContent::Blocks)?;
        let mut blocks = self.blocks(body.blocks(), &story)?;
        if blocks.is_empty() {
            blocks.push(Node::new("w:p"));
        }
        for block in blocks {
            node.push(block);
        }
        self.store_comment(&path, node)?;
        Ok(id)
    }
    fn comment_definition(
        &mut self,
        metadata: &crate::CommentMetadata,
        context: &crate::ooxml::PartContext,
        content: crate::ooxml::StoryContent,
    ) -> Result<(String, Node, CommentId, crate::ooxml::PartContext)> {
        if context.kind != StoryKind::Body {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "comment generation is currently supported only in the main body story",
            )
            .at(context.source.as_str())
            .into());
        }
        let defaults = self.comment_defaults.as_ref().ok_or_else(|| {
            Diagnostic::new(
                Rule::UnsupportedEdit,
                "fragment rendering cannot add comments",
            )
            .at(context.source.as_str())
        })?;
        let comment_path = self
            .package
            .related(&self.main, "comments")?
            .map(|(_, path)| path)
            .unwrap_or_else(|| {
                if self
                    .package
                    .parts
                    .keys()
                    .any(|p| p.eq_ignore_ascii_case("word/comments.xml"))
                {
                    self.package.unique("word/comments", "xml")
                } else {
                    "word/comments.xml".into()
                }
            });
        metadata.validate()?;
        let path = comment_path.as_str();
        let id = self.ranges.comment_id()?;
        let story = self.stories.register(
            StoryKind::Comment,
            crate::opc::PartName::new(path)?,
            content,
        )?;
        let author = metadata
            .author
            .as_deref()
            .filter(|s| !s.is_empty())
            .or(defaults.author)
            .unwrap_or("");
        let mut node = Node::new("w:comment")
            .attr("w:id", id)
            .attr("w:author", author)
            .attr(
                "w:date",
                metadata.date.as_deref().unwrap_or(defaults.timestamp),
            );
        if let Some(initials) = &metadata.initials {
            node.set("w:initials", initials);
        }
        Ok((
            comment_path,
            node,
            id,
            self.context(story, context.width.get()),
        ))
    }
    fn store_comment(&mut self, path: &str, node: Node) -> Result<()> {
        let mut comments = if self.package.parts.contains_key(path) {
            self.package.word(path)?
        } else {
            crate::ooxml::root("w:comments")
        };
        if comments.name != "w:comments" {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "incorrect comments part root/profile",
            )
            .at(path)
            .into());
        }
        comments.push(node);
        self.package.put_word(path, comments)?;
        self.package.content_type(
            path,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml",
        )?;
        self.package.link(&self.main, "comments", path)?;
        Ok(())
    }
}

use super::text::text_run;

use crate::ooxml::WordParts;
