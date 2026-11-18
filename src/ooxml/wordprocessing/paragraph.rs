//! ECMA-376 Part 1 (2016): P1 §17.3, paragraph/run structure.
use crate::ooxml::Compiler;
use crate::xml::{Content, Node, val};
use crate::{ResolvedInline, ResolvedParagraph, Result};
impl Compiler<'_> {
    pub(crate) fn paragraph(
        &mut self,
        p: &ResolvedParagraph,
        context: &crate::ooxml::PartContext,
    ) -> Result<Node> {
        let mut node = Node::new("w:p");
        let mut props = crate::ooxml::wordprocessing::format::paragraph(&p.format, 20.)?;
        if let Some(style) = props.child_mut("w:pStyle") {
            style.set("w:val", self.styles.get(style.get("w:val").unwrap())?);
        }
        if let Some(num) = &p.numbering {
            props.push(
                Node::new("w:numPr")
                    .with(val("ilvl", num.level))
                    .with(val("numId", num.id)),
            );
        }
        if !props.children.is_empty() {
            node.push(props);
        }
        let mut runs = Vec::new();
        for run in &p.runs {
            let mut n = match run {
                ResolvedInline::Text { text, .. } => text_run(text),
                ResolvedInline::Tab { .. } => Node::new("w:r").with(Node::new("w:tab")),
                ResolvedInline::Break { kind, .. } => {
                    Node::new("w:r").with(Node::new("w:br").attr("w:type", kind.xml()))
                }
                ResolvedInline::Image { image, .. } => self.image(image, context)?,
            };
            let rpr = crate::ooxml::wordprocessing::format::run(run.format())?;
            if !rpr.children.is_empty() {
                n.children.insert(0, Content::Element(rpr));
            }
            runs.extend(self.comment(vec![n], &run.comment().cloned(), context)?);
        }
        for n in self.comment(runs, &p.comment, context)? {
            node.push(n);
        }
        Ok(node)
    }
}

use super::text::text_run;
