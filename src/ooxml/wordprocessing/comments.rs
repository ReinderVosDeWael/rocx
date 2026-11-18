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
            .into());
        }
        if nodes.is_empty() {
            return Err(Error::Validation(
                "comments must be anchored to at least one run".into(),
            ));
        }
        let comment_path = self
            .package
            .related(&self.main, "comments")?
            .map(|(_, path)| path)
            .unwrap_or("word/comments.xml".into());
        let path = comment_path.as_str();
        let mut comments = if self.package.parts.contains_key(path) {
            self.package.word(path)?
        } else {
            crate::ooxml::root("w:comments")
        };
        let id = comments
            .elements()
            .filter_map(|n| n.get("w:id"))
            .filter_map(|s| s.parse::<u32>().ok())
            .max()
            .map(|n| CommentId::new(n).next())
            .transpose()?
            .unwrap_or_default();
        let author = comment
            .author
            .as_deref()
            .filter(|s| !s.is_empty())
            .or(self.document.comment_author.as_deref())
            .unwrap_or("");
        let mut node = Node::new("w:comment")
            .attr("w:id", id)
            .attr("w:author", author)
            .attr("w:initials", "")
            .attr("w:date", &self.document.timestamp);
        for line in comment.text.split('\n') {
            node.push(
                Node::new("w:p")
                    .with(Node::new("w:pPr").with(val("pStyle", "CommentText")))
                    .with(text_run(line)),
            );
        }
        comments.push(node);
        self.package.put_word(path, comments)?;
        self.package.content_type(
            path,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml",
        )?;
        self.package.link(&self.main, "comments", path)?;
        let mut out = vec![Node::new("w:commentRangeStart").attr("w:id", id)];
        out.extend(nodes);
        out.push(Node::new("w:commentRangeEnd").attr("w:id", id));
        out.push(
            Node::new("w:r")
                .with(Node::new("w:rPr").with(val("rStyle", "CommentReference")))
                .with(Node::new("w:commentReference").attr("w:id", id)),
        );
        Ok(out)
    }
}

use super::text::text_run;

use crate::ooxml::WordParts;
