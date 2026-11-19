//! Simple and complex fields; P1 (2016) §§17.16.18–19, 17.16.23.
use crate::ooxml::{Compiler, PartContext};
use crate::xml::Node;
use crate::{FieldForm, ResolvedField, Result};

impl Compiler<'_> {
    pub(super) fn field(
        &mut self,
        field: &ResolvedField,
        context: &PartContext,
    ) -> Result<Vec<Node>> {
        let props = field.properties();
        let mut start = match props.form {
            FieldForm::Simple => {
                Node::new("w:fldSimple").attr("w:instr", field.instruction().text())
            }
            FieldForm::Complex => Node::new("w:fldChar").attr("w:fldCharType", "begin"),
        };
        for (name, value) in [("w:dirty", props.dirty), ("w:fldLock", props.locked)] {
            if let Some(value) = value {
                start.set(name, if value { "1" } else { "0" });
            }
        }
        let mut result = Vec::new();
        for child in field.children() {
            result.extend(self.inline(child, context)?);
        }
        Ok(match props.form {
            FieldForm::Simple => {
                for child in result {
                    start.push(child);
                }
                vec![start]
            }
            FieldForm::Complex => {
                let mut nodes = vec![
                    Node::new("w:r").with(start),
                    Node::new("w:r").with(
                        Node::new("w:instrText")
                            .attr("xml:space", "preserve")
                            .text(field.instruction().text()),
                    ),
                    Node::new("w:r").with(Node::new("w:fldChar").attr("w:fldCharType", "separate")),
                ];
                nodes.extend(result);
                nodes.push(
                    Node::new("w:r").with(Node::new("w:fldChar").attr("w:fldCharType", "end")),
                );
                nodes
            }
        })
    }
}
