use super::{InlineAnchor, InlineLocation, insertion::BoundaryState, unsupported};
use crate::Result;
use crate::xml::{Content, Node};
use std::collections::BTreeMap;

/// Change the text of one direct body run, preserving its formatting and attributes.
#[derive(Clone, Debug)]
pub struct RunTextEdit {
    pub target: InlineAnchor,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct RunTextChange {
    pub body_index: usize,
    pub inline_index: usize,
    pub before: String,
    pub after: String,
}

pub(super) fn apply(body: &mut Node, edits: &[RunTextEdit]) -> Result<Vec<RunTextChange>> {
    let mut targets = BTreeMap::new();
    for edit in edits {
        let (InlineLocation::Body(body_index), [inline_index]) =
            (&edit.target.location, edit.target.path.as_slice())
        else {
            return Err(unsupported("text target must be a direct body run"));
        };
        if edit.text.contains(['\t', '\r', '\n'])
            || crate::document::invalid_xml_character(&edit.text).is_some()
        {
            return Err(unsupported(
                "replacement must be single-line XML text without tabs",
            ));
        }
        if targets
            .insert((*body_index, *inline_index), &edit.text)
            .is_some()
        {
            return Err(unsupported("duplicate run text target"));
        }
    }
    let mut changes = Vec::new();
    let mut boundaries = BoundaryState::default();
    for (body_index, block) in body
        .elements_mut()
        .filter(|n| n.name != "w:sectPr")
        .enumerate()
    {
        if targets.is_empty() {
            break;
        }
        if block.name != "w:p" {
            boundaries.advance(block);
            continue;
        }
        for (inline_index, child) in block
            .elements_mut()
            .filter(|n| n.name != "w:pPr")
            .enumerate()
        {
            if let Some(after) = targets.remove(&(body_index, inline_index)) {
                if !boundaries.safe()
                    || child.name != "w:r"
                    || child
                        .elements()
                        .filter(|n| n.name != "w:rPr")
                        .map(|n| n.name.as_str())
                        .ne(["w:t"])
                {
                    return Err(unsupported(
                        "target must be a single-text run outside fields and ranges",
                    ));
                }
                let t = child.child_mut("w:t").unwrap();
                let mut before = String::new();
                for c in &t.children {
                    let Content::Text(s) = c else {
                        return Err(unsupported("target text has opaque content"));
                    };
                    before.push_str(s);
                }
                if &before != after {
                    t.children = vec![Content::Text(after.clone())];
                    if after.starts_with(' ') || after.ends_with(' ') {
                        t.set("xml:space", "preserve");
                    }
                    changes.push(RunTextChange {
                        body_index,
                        inline_index,
                        before,
                        after: after.clone(),
                    });
                }
            }
            boundaries.advance(child);
        }
    }
    if !targets.is_empty() {
        return Err(unsupported("text target not found"));
    }
    Ok(changes)
}
