//! P1 (2016) §17.5.2 and Annex A.1 CT_SdtRun, CT_SdtBlock and CT_SdtPr.
use crate::ooxml::{Compiler, PartContext};
use crate::xml::{Node, val, validate_text};
use crate::{
    ContentControlProperties, ResolvedBlockContentControl, ResolvedInlineContentControl, Result,
};

fn control_node(properties: &ContentControlProperties, children: Vec<Node>) -> Result<Node> {
    let mut props = Node::new("w:sdtPr");
    for (tag, value) in [("alias", &properties.alias), ("tag", &properties.tag)] {
        if let Some(value) = value {
            validate_text(value).map_err(|error| error.at(format!("properties.{tag}")))?;
            props.push(val(tag, value));
        }
    }
    if let Some(id) = properties.id {
        props.push(val("id", id));
    }
    if let Some(lock) = properties.lock {
        props.push(val("lock", lock.xml()));
    }
    match &properties.text {
        crate::ContentControlText::Rich => props.push(Node::new("w:richText")),
        crate::ContentControlText::Plain { multi_line } => {
            let mut text = Node::new("w:text");
            if let Some(multi_line) = multi_line {
                text.set("w:multiLine", u8::from(*multi_line));
            }
            props.push(text);
        }
        crate::ContentControlText::DropDown { items, selected } => {
            let mut list = Node::new("w:dropDownList").attr("w:lastValue", selected);
            for (index, item) in items.iter().enumerate() {
                validate_text(&item.display_text)
                    .map_err(|error| error.at(format!("properties.items[{index}].display_text")))?;
                validate_text(&item.value)
                    .map_err(|error| error.at(format!("properties.items[{index}].value")))?;
                list.push(
                    Node::new("w:listItem")
                        .attr("w:displayText", &item.display_text)
                        .attr("w:value", &item.value),
                );
            }
            props.push(list);
        }
    }
    let mut content = Node::new("w:sdtContent");
    for child in children {
        content.push(child);
    }
    Ok(Node::new("w:sdt").with(props).with(content))
}

impl Compiler<'_> {
    fn reserve_content_control_id(&mut self, properties: &ContentControlProperties) -> Result<()> {
        if let Some(id) = properties.id {
            let ids = self.content_control_ids.get_or_insert_with(|| {
                crate::ooxml::defaults::imported_word_values(&self.package, |node| {
                    if node.name == "w:sdtPr" {
                        node.child("w:id")
                            .and_then(|id| id.get("w:val")?.trim().parse().ok())
                    } else {
                        None
                    }
                })
            });
            if !ids.insert(id) {
                return Err(crate::Diagnostic::new(
                    crate::Rule::InvalidContent,
                    "duplicate content control ID",
                )
                .at("properties.id")
                .into());
            }
        }
        Ok(())
    }
    // P1 (2016) Annex A.1 CT_SdtRow and CT_SdtCell.
    pub(super) fn table_content_control(
        &mut self,
        node: Node,
        properties: Option<&ContentControlProperties>,
    ) -> Result<Node> {
        match properties {
            Some(properties) => {
                self.reserve_content_control_id(properties)?;
                control_node(properties, vec![node])
            }
            None => Ok(node),
        }
    }

    pub(super) fn inline_content_control(
        &mut self,
        control: &ResolvedInlineContentControl,
        context: &PartContext,
    ) -> Result<Node> {
        self.reserve_content_control_id(control.properties())?;
        let mut children = Vec::new();
        for child in control.children() {
            children.extend(self.inline(child, context)?);
        }
        control_node(control.properties(), children)
    }

    pub(super) fn block_content_control(
        &mut self,
        control: &ResolvedBlockContentControl,
        context: &PartContext,
    ) -> Result<Node> {
        self.reserve_content_control_id(control.properties())?;
        control_node(
            control.properties(),
            self.blocks(control.children(), context)?,
        )
    }
}
