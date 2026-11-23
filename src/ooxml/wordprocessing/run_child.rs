//! Recognized direct run children (ECMA-376 Part 1 §17.3.3).

use crate::BreakType;
use crate::xml::{Content, Node};

pub(crate) enum RunChild {
    Properties,
    Text(String),
    Tab,
    Break(BreakType),
    Opaque,
}

impl RunChild {
    pub(crate) fn classify(node: &Node) -> Self {
        match node.name.as_str() {
            "w:rPr" => Self::Properties,
            "w:t" => Self::Text(
                node.children
                    .iter()
                    .filter_map(|child| match child {
                        Content::Text(text) => Some(text.as_str()),
                        _ => None,
                    })
                    .collect(),
            ),
            "w:tab" => Self::Tab,
            "w:cr" => Self::Break(BreakType::Line),
            "w:br" => match node.get("w:type") {
                None | Some("textWrapping") => Self::Break(BreakType::TextWrapping),
                Some("page") => Self::Break(BreakType::Page),
                Some("column") => Self::Break(BreakType::Column),
                Some(_) => Self::Opaque,
            },
            _ => Self::Opaque,
        }
    }
}
