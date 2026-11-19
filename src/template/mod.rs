//! Structural text edits. P1 §§17.3, 17.5, 17.13, 17.16; boundaries are library policy.
use crate::xml::{Content, Node, W};
use crate::{Diagnostic, Error, Result, Rule};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditScope {
    Body,
    #[default]
    BodyHeadersFooters,
    /// Also visit related comments, footnotes and endnotes. Unknown parts stay opaque.
    SupportedStories,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemplateEdits {
    pub replacements: Vec<(String, String)>,
    pub paragraph_index: Option<usize>,
    pub scope: EditScope,
}
#[derive(Clone)]
struct Token {
    path: Vec<usize>,
    before: String,
    after: String,
    group: u64,
    protected: bool,
}
#[derive(Default)]
struct Position {
    group: u64,
    field_depth: usize,
}
fn collect(
    node: &Node,
    path: &mut Vec<usize>,
    position: &mut Position,
    protected: bool,
    tokens: &mut Vec<Token>,
) {
    if node.name.namespace() != Some(W) {
        position.group += 1;
        return;
    }
    if node.name == "w:p" && !path.is_empty() {
        position.group += 1;
        return;
    }
    if node.name == "w:fldChar" {
        position.group += 1;
        match node.get("w:fldCharType") {
            Some("begin") => position.field_depth += 1,
            Some("end") => position.field_depth = position.field_depth.saturating_sub(1),
            _ => (),
        }
        return;
    }
    if ["w:instrText"].contains(&node.name.as_str()) {
        position.group += 1;
        return;
    }
    let container =
        ["w:hyperlink", "w:fldSimple", "w:sdt", "w:ins", "w:del"].contains(&node.name.as_str());
    let protected =
        protected || ["w:fldSimple", "w:sdt", "w:ins", "w:del"].contains(&node.name.as_str());
    if container {
        position.group += 1;
    }
    let value: Option<String> = match node.name.as_str() {
        "w:t" => Some(
            node.children
                .iter()
                .filter_map(|c| {
                    if let Content::Text(t) = c {
                        Some(t.as_str())
                    } else {
                        None
                    }
                })
                .collect(),
        ),
        "w:tab" => Some("\t".into()),
        "w:br" if node.get("w:type").is_none_or(|kind| kind == "textWrapping") => Some("\n".into()),
        "w:cr" => Some("\n".into()),
        "w:br" => {
            position.group += 1;
            None
        }
        _ => None,
    };
    if let Some(value) = value {
        tokens.push(Token {
            path: path.clone(),
            before: value.clone(),
            after: value,
            group: position.group,
            protected: protected || position.field_depth > 0,
        });
    } else {
        for (index, child) in node.children.iter().enumerate() {
            if let Content::Element(child) = child {
                path.push(index);
                collect(child, path, position, protected, tokens);
                path.pop();
            }
        }
    }
    if container {
        position.group += 1;
    }
}
fn parent<'a>(mut node: &'a mut Node, path: &[usize]) -> &'a mut Node {
    for &index in path {
        node = match &mut node.children[index] {
            Content::Element(n) => n,
            _ => unreachable!(),
        };
    }
    node
}
fn edit_paragraph(
    node: &mut Node,
    needle: &str,
    replacement: &str,
    position: &mut Position,
) -> Result<()> {
    let mut tokens = Vec::new();
    collect(node, &mut Vec::new(), position, false, &mut tokens);
    let joined: String = tokens.iter().map(|t| t.before.as_str()).collect();
    for start in joined
        .match_indices(needle)
        .map(|(i, _)| i)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        let end = start + needle.len();
        let mut offset = 0;
        let mut group = None;
        let mut inserted = false;
        // Validate the entire match before mutating any token.
        for token in &tokens {
            let finish = offset + token.before.len();
            if start < finish && end > offset {
                if token.protected || group.is_some_and(|g| g != token.group) {
                    return Err(Diagnostic::new(Rule::UnsupportedEdit,"text replacement crosses a structured boundary or edits a field, revision or content control").into());
                }
                group = Some(token.group);
            }
            offset = finish;
        }
        let mut offset = 0;
        for token in &mut tokens {
            let finish = offset + token.before.len();
            if start < finish && end > offset {
                let a = start.saturating_sub(offset);
                let b = (end - offset).min(token.before.len());
                token.after.replace_range(
                    a..b,
                    if inserted {
                        ""
                    } else {
                        inserted = true;
                        replacement
                    },
                );
            }
            offset = finish;
        }
    }
    for token in tokens.into_iter().rev().filter(|t| t.before != t.after) {
        let (index, path) = token.path.split_last().unwrap();
        let parent = parent(node, path);
        let original = match &parent.children[*index] {
            Content::Element(n) => n.clone(),
            _ => unreachable!(),
        };
        let mut replacements = crate::ooxml::wordprocessing::text::text_run(&token.after).children;
        if original.name == "w:t" {
            if replacements.is_empty() {
                let mut original = original.clone();
                original.children.clear();
                replacements.push(Content::Element(original));
            }
            for child in &mut replacements {
                if let Content::Element(text) = child
                    && text.name == "w:t"
                {
                    let children = std::mem::take(&mut text.children);
                    *text = original.clone();
                    text.children = children;
                    text.set("xml:space", "preserve");
                }
            }
        }
        parent.children.splice(*index..=*index, replacements);
    }
    Ok(())
}
fn visit(node: &mut Node, needle: &str, replacement: &str, position: &mut Position) -> Result<()> {
    if node.name.namespace() != Some(W) {
        return Ok(());
    }
    // A part may contain many independent note/comment stories. An unclosed
    // imported field must not protect text in the next entry of the same part.
    let mut entry_position = Position::default();
    let position = if matches!(node.name.as_str(), "w:footnote" | "w:endnote" | "w:comment") {
        &mut entry_position
    } else {
        position
    };
    if node.name == "w:p" {
        edit_paragraph(node, needle, replacement, position)?;
    }
    for child in node.elements_mut() {
        visit(child, needle, replacement, position)?;
    }
    Ok(())
}
pub(crate) fn replacements(root: &mut Node, replacements: &[(String, String)]) -> Result<()> {
    for (needle, replacement) in replacements {
        if needle.is_empty() {
            return Err(Error::Validation(
                "template replacement needle cannot be empty".into(),
            ));
        }
        crate::xml::validate_text(needle)?;
        crate::xml::validate_text(replacement)?;
        visit(root, needle, replacement, &mut Position::default())?;
    }
    Ok(())
}
