//! Transform imported direct paragraph text revisions. P1 (2016) §17.13.5.
use super::TextRevisionAction;
use crate::xml::{Content, Node, W};
use crate::{Diagnostic, Error, Result, Rule};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn edit_text_revisions(root: &mut Node, action: TextRevisionAction) -> Result<()> {
    let mut moves = Moves {
        body_story: root.name == "w:document",
        ..Default::default()
    };
    visit(root, action, false, &mut 0, &mut moves)?;
    moves.finish()
}

fn visit(
    node: &mut Node,
    action: TextRevisionAction,
    protected: bool,
    field_depth: &mut usize,
    moves: &mut Moves,
) -> Result<()> {
    if node.name.namespace() != Some(W) {
        return Ok(());
    }
    let mut local_depth = 0;
    let field_depth = if matches!(node.name.as_str(), "w:comment" | "w:footnote" | "w:endnote") {
        &mut local_depth
    } else {
        field_depth
    };
    if node.name == "w:fldChar" {
        match node.get("w:fldCharType") {
            Some("begin") => *field_depth += 1,
            Some("end") => *field_depth = field_depth.saturating_sub(1),
            _ => (),
        }
        return Ok(());
    }
    let protected = protected
        || matches!(
            node.name.as_str(),
            "w:sdt" | "w:fldSimple" | "w:hyperlink" | "w:moveFrom" | "w:moveTo"
        );
    let mut index = 0;
    while index < node.children.len() {
        let Content::Element(child) = &mut node.children[index] else {
            index += 1;
            continue;
        };
        if let Some(source) = move_start(&child.name) {
            if node.name != "w:p" || protected || *field_depth != 0 || !moves.body_story {
                return Err(unsupported(
                    "text move action requires direct body paragraph ranges",
                ));
            }
            index = moves.edit_leg(node, index, source, action)?;
            continue;
        }
        if matches!(
            child.name.as_str(),
            "w:moveFrom" | "w:moveTo" | "w:moveFromRangeEnd" | "w:moveToRangeEnd"
        ) {
            return Err(unsupported(
                "text move action requires adjacent start, run and end markers",
            ));
        }
        if matches!(child.name.as_str(), "w:ins" | "w:del") {
            if node.name != "w:p" || protected || *field_depth != 0 {
                return Err(Diagnostic::new(
                    Rule::UnsupportedEdit,
                    "text revision action requires direct paragraph revisions outside protected structures",
                ).into());
            }
            match edit_run(child, action)? {
                Some(run) => node.children[index] = Content::Element(run),
                None => {
                    node.children.remove(index);
                    continue;
                }
            }
        } else {
            visit(child, action, protected, field_depth, moves)?;
        }
        index += 1;
    }
    Ok(())
}

fn unsupported(message: impl Into<String>) -> Error {
    Diagnostic::new(Rule::UnsupportedEdit, message).into()
}

fn move_start(name: &crate::xml::Name) -> Option<bool> {
    match name.as_str() {
        "w:moveFromRangeStart" => Some(true),
        "w:moveToRangeStart" => Some(false),
        _ => None,
    }
}

fn text_run(wrapper: &Node, text_names: &[&str]) -> Result<Node> {
    let mut children = wrapper.elements();
    let Some(run) = children
        .next()
        .filter(|run| {
            run.name == "w:r"
                && run.elements().all(|node| {
                    text_names.contains(&node.name.as_str())
                        || matches!(node.name.as_str(), "w:rPr" | "w:tab" | "w:br" | "w:cr")
                })
        })
        .filter(|_| children.next().is_none())
    else {
        return Err(unsupported(
            "text revision action requires one text run per revision",
        ));
    };
    Ok(run.clone())
}

fn restore_deleted_text(run: &mut Node) {
    for text in run.elements_mut().filter(|node| node.name == "w:delText") {
        text.name = "w:t".into();
    }
}

fn edit_run(wrapper: &Node, action: TextRevisionAction) -> Result<Option<Node>> {
    let deletion = wrapper.name == "w:del";
    let text_name = if deletion { "w:delText" } else { "w:t" };
    let mut run = text_run(wrapper, &[text_name])?;
    if deletion != (action == TextRevisionAction::Reject) {
        return Ok(None);
    }
    if deletion {
        restore_deleted_text(&mut run);
    }
    Ok(Some(run))
}

#[derive(Default)]
struct Moves {
    body_story: bool,
    ids: BTreeSet<u32>,
    pairs: BTreeMap<String, (Option<MoveContent>, Option<MoveContent>)>,
}

#[derive(PartialEq, Eq)]
struct MoveContent {
    text: String,
    author: String,
    date: String,
}

impl Moves {
    fn edit_leg(
        &mut self,
        paragraph: &mut Node,
        start_index: usize,
        source: bool,
        action: TextRevisionAction,
    ) -> Result<usize> {
        let unsupported_shape =
            || unsupported("text move action requires adjacent start, run and end markers");
        let mut elements = paragraph
            .children
            .iter()
            .enumerate()
            .skip(start_index)
            .filter_map(|(index, item)| match item {
                Content::Element(element) => Some((index, element)),
                _ => None,
            });
        let (_, start) = elements.next().ok_or_else(unsupported_shape)?;
        let (wrapper_index, wrapper) = elements.next().ok_or_else(unsupported_shape)?;
        let (end_index, end) = elements.next().ok_or_else(unsupported_shape)?;
        let (wrapper_name, end_name) = if source {
            ("w:moveFrom", "w:moveFromRangeEnd")
        } else {
            ("w:moveTo", "w:moveToRangeEnd")
        };
        let parse_id = |node: &Node| node.get("w:id")?.trim().parse::<u32>().ok();
        let id = parse_id(start).ok_or_else(unsupported_shape)?;
        if wrapper.name != wrapper_name
            || end.name != end_name
            || parse_id(end) != Some(id)
            || !self.ids.insert(id)
        {
            return Err(unsupported_shape());
        }
        let name = start
            .get("w:name")
            .filter(|name| !name.is_empty())
            .ok_or_else(unsupported_shape)?;
        let author = start.get("w:author").ok_or_else(unsupported_shape)?;
        let date = start.get("w:date").ok_or_else(unsupported_shape)?;
        let expected_text = if source {
            &["w:t", "w:delText"][..]
        } else {
            &["w:t"][..]
        };
        let mut run = text_run(wrapper, expected_text)?;
        let mut text = String::new();
        for child in run.elements() {
            match child.name.as_str() {
                "w:t" | "w:delText" => {
                    for segment in &child.children {
                        if let Content::Text(value) = segment {
                            text.push_str(value);
                        }
                    }
                }
                "w:tab" => text.push('\t'),
                "w:br" | "w:cr" => text.push('\n'),
                _ => (),
            }
        }
        let (from, to) = self.pairs.entry(name.into()).or_default();
        let slot = if source { from } else { to };
        if slot
            .replace(MoveContent {
                text,
                author: author.into(),
                date: date.into(),
            })
            .is_some()
        {
            return Err(unsupported(
                "text move name has more than one source or destination",
            ));
        }

        paragraph.children.remove(end_index);
        paragraph.children.remove(wrapper_index);
        if source == (action == TextRevisionAction::Reject) {
            restore_deleted_text(&mut run);
            paragraph.children[start_index] = Content::Element(run);
            Ok(start_index + 1)
        } else {
            paragraph.children.remove(start_index);
            Ok(start_index)
        }
    }

    fn finish(self) -> Result<()> {
        for (name, (source, destination)) in self.pairs {
            if source.is_none() || source != destination {
                return Err(unsupported(format!(
                    "text move {name:?} requires matching source and destination text and metadata"
                )));
            }
        }
        Ok(())
    }
}
