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
            style.set(
                "w:val",
                self.styles
                    .get_typed(style.get("w:val").unwrap(), "paragraph")
                    .map_err(|error| error.at(context.source.as_str()))?,
            );
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
            runs.extend(self.inline(run, context)?);
        }
        for n in self.comment(runs, &p.comment, context)? {
            node.push(n);
        }
        Ok(node)
    }
    /// P1 §17.16.22: hyperlink owns runs, not a run's child element.
    pub(super) fn inline(
        &mut self,
        run: &ResolvedInline,
        context: &crate::ooxml::PartContext,
    ) -> Result<Vec<Node>> {
        match run {
            ResolvedInline::Field(field) => return self.field(field, context),
            ResolvedInline::NoteReference(target) => {
                return Ok(vec![self.note_reference(*target, context)?]);
            }
            ResolvedInline::RangeStart { id, kind } => {
                return Ok(vec![self.range_start(id, kind, context)?]);
            }
            ResolvedInline::RangeEnd(id) => return self.range_end(id, context),
            _ => (),
        }
        if let ResolvedInline::Hyperlink(link) = run {
            let mut node = if let Some(name) = link.target().bookmark_name() {
                Node::new("w:hyperlink").attr("w:anchor", name.as_str())
            } else {
                let id = self.package.add_relationship(
                context.source.as_str(),
                &crate::opc::RelationshipType::new(
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink",
                )?,
                crate::opc::Target::External(link.target().as_str().into()),
                )?;
                Node::new("w:hyperlink").attr("r:id", id)
            };
            if let Some(tooltip) = &link.properties().tooltip {
                node.set("w:tooltip", tooltip);
            }
            if let Some(history) = link.properties().history {
                node.set("w:history", if history { "1" } else { "0" });
            }
            for child in link.children() {
                for node_child in self.inline(child, context)? {
                    node.push(node_child);
                }
            }
            return Ok(vec![node]);
        }
        let mut n = match run {
            ResolvedInline::Text { text, .. } => text_run(text),
            ResolvedInline::Tab { .. } => Node::new("w:r").with(Node::new("w:tab")),
            ResolvedInline::Break { kind, .. } => {
                Node::new("w:r").with(Node::new("w:br").attr("w:type", kind.xml()))
            }
            ResolvedInline::Image { image, .. } => self.image(image, context)?,
            ResolvedInline::Field(_)
            | ResolvedInline::Hyperlink(_)
            | ResolvedInline::RangeStart { .. }
            | ResolvedInline::RangeEnd(_) => unreachable!("handled structural inline above"),
            ResolvedInline::NoteReference(_) => unreachable!("handled note reference above"),
        };
        let mut rpr = crate::ooxml::wordprocessing::format::run(run.format().expect("leaf run"))?;
        if let Some(style) = rpr.child_mut("w:rStyle") {
            let id = self
                .styles
                .get_typed(style.get("w:val").unwrap(), "character")
                .map_err(|error| error.at(context.source.as_str()))?;
            style.set("w:val", id);
        }
        if !rpr.children.is_empty() {
            n.children.insert(0, Content::Element(rpr));
        }
        self.comment(vec![n], &run.comment().cloned(), context)
    }
}

use super::text::text_run;
