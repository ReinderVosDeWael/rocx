//! Inline structured document tags, P1 (2016) §17.5.2.
use super::{Diagnostic, ResolvedBlock, ResolvedInline, Rule, inline_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentControlLock {
    Unlocked,
    ControlLocked,
    ContentLocked,
    ControlAndContentLocked,
}
impl ContentControlLock {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::Unlocked => "unlocked",
            Self::ControlLocked => "sdtLocked",
            Self::ContentLocked => "contentLocked",
            Self::ControlAndContentLocked => "sdtContentLocked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentControlListItem {
    pub display_text: String,
    pub value: String,
}
setters!(ContentControlListItem {
    with_display_text => display_text: into<String>,
    with_value => value: into<String>,
});

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ContentControlText {
    #[default]
    Rich,
    Plain {
        multi_line: Option<bool>,
    },
    DropDown {
        items: Vec<ContentControlListItem>,
        selected: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContentControlProperties {
    pub text: ContentControlText,
    pub alias: Option<String>,
    pub tag: Option<String>,
    pub lock: Option<ContentControlLock>,
    pub id: Option<u32>,
}
setters!(ContentControlProperties {
    with_text => text: value<ContentControlText>,
    with_alias => alias: optional_into<String>,
    with_tag => tag: optional_into<String>,
    with_lock => lock: optional<ContentControlLock>,
    with_id => id: optional<u32>,
});

impl ContentControlProperties {
    pub(super) fn validate_table_context(&self) -> Result<(), Diagnostic> {
        self.validate()?;
        if self.text != ContentControlText::Rich {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "row and cell content controls require rich text",
            )
            .at("content_control.text"));
        }
        Ok(())
    }

    fn validate_text_children(&self, children: &[ResolvedInline]) -> Result<(), Diagnostic> {
        let multi_line = match &self.text {
            ContentControlText::Rich => return Ok(()),
            ContentControlText::Plain { multi_line } => *multi_line,
            ContentControlText::DropDown { .. } => None,
        };
        let [
            ResolvedInline::Text {
                text,
                comment: None,
                revision: None,
                ..
            },
        ] = children
        else {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "plain-text and drop-down controls require exactly one text run without comments or revisions",
            )
            .at("children"));
        };
        if text.contains('\t') || (multi_line != Some(true) && text.contains(['\r', '\n'])) {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "plain-text control tabs are unsupported and line breaks require multi_line",
            )
            .at("children[0].text"));
        }
        if let ContentControlText::DropDown { items, selected } = &self.text
            && !items
                .iter()
                .any(|item| item.value == *selected && item.display_text == *text)
        {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "drop-down cached text must match the selected item display text",
            )
            .at("children[0].text"));
        }
        Ok(())
    }

    pub(crate) fn validate(&self) -> Result<(), Diagnostic> {
        if self
            .id
            .is_some_and(|id| !(1..=i32::MAX as u32).contains(&id))
        {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "content control ID must be in 1..=2147483647",
            )
            .at("properties.id"));
        }
        if let ContentControlText::DropDown { items, selected } = &self.text {
            let mut values = std::collections::BTreeSet::new();
            for (index, item) in items.iter().enumerate() {
                if item.value.is_empty()
                    || item.display_text.is_empty()
                    || item.display_text.contains(['\t', '\r', '\n'])
                    || !values.insert(&item.value)
                {
                    return Err(Diagnostic::new(Rule::InvalidContent,
                        "drop-down items require unique nonempty values and nonempty single-line display text")
                        .at(format!("properties.items[{index}]")));
                }
            }
            if !values.contains(selected) {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "drop-down selected value must identify an item",
                )
                .at("properties.selected"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedInlineContentControl {
    children: Vec<ResolvedInline>,
    properties: ContentControlProperties,
}
impl ResolvedInlineContentControl {
    pub fn new(
        children: Vec<ResolvedInline>,
        properties: ContentControlProperties,
    ) -> Result<Self, Diagnostic> {
        properties.validate()?;
        properties.validate_text_children(&children)?;
        for (index, child) in children.iter().enumerate() {
            if child
                .revision()
                .is_some_and(|revision| revision.kind.move_name().is_some())
            {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "text moves within inline containers are unsupported",
                )
                .at(format!("children[{index}]")));
            }

            if !matches!(
                child,
                ResolvedInline::Text { .. }
                    | ResolvedInline::Tab { .. }
                    | ResolvedInline::Break { .. }
                    | ResolvedInline::Image { .. }
            ) {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "inline content control children must be text, tabs, breaks or images",
                )
                .at(format!("children[{index}]")));
            }
            child
                .validate()
                .map_err(|error| error.at(format!("children[{index}]")))?;
        }
        Ok(Self {
            children,
            properties,
        })
    }

    pub fn children(&self) -> &[ResolvedInline] {
        &self.children
    }

    pub fn properties(&self) -> &ContentControlProperties {
        &self.properties
    }

    pub fn text(&self) -> String {
        inline_text(&self.children)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedBlockContentControl {
    children: Vec<ResolvedBlock>,
    properties: ContentControlProperties,
}
impl ResolvedBlockContentControl {
    pub fn new(
        children: Vec<ResolvedBlock>,
        properties: ContentControlProperties,
    ) -> Result<Self, Diagnostic> {
        properties.validate()?;
        if properties.text != ContentControlText::Rich {
            let [ResolvedBlock::Paragraph(paragraph)] = children.as_slice() else {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "plain-text and drop-down block controls require exactly one paragraph",
                )
                .at("children"));
            };
            if paragraph.comment().is_some() {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "plain-text and drop-down block controls cannot contain paragraph comments",
                )
                .at("children[0]"));
            }
            properties
                .validate_text_children(paragraph.runs())
                .map_err(|error| error.at("children[0]"))?;
        }

        if let Some(index) = children
            .iter()
            .position(|child| matches!(child, ResolvedBlock::ContentControl(_)))
        {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "nested block content controls are unsupported",
            )
            .at(format!("children[{index}]")));
        }
        Ok(Self {
            children,
            properties,
        })
    }

    pub fn children(&self) -> &[ResolvedBlock] {
        &self.children
    }

    pub fn properties(&self) -> &ContentControlProperties {
        &self.properties
    }
}
