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

/// Removal of a direct body run, addressed in the original snapshot.
#[derive(Clone, Debug)]
pub struct RunRemovalChange {
    pub body_index: usize,
    pub inline_index: usize,
    pub before: String,
}

pub(super) fn apply(
    body: &mut Node,
    edits: &[RunTextEdit],
    removals: &[InlineAnchor],
) -> Result<(Vec<RunTextChange>, Vec<RunRemovalChange>)> {
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
            .insert((*body_index, *inline_index), Some(&edit.text))
            .is_some()
        {
            return Err(unsupported("duplicate run text target"));
        }
    }
    for target in removals {
        let (InlineLocation::Body(body_index), [inline_index]) =
            (&target.location, target.path.as_slice())
        else {
            return Err(unsupported("removal target must be a direct body run"));
        };
        if targets.insert((*body_index, *inline_index), None).is_some() {
            return Err(unsupported("duplicate or conflicting run target"));
        }
    }
    let mut changes = Vec::new();
    let mut removed = Vec::new();
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
        let mut remove_indices = Vec::new();
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
                if let Some(after) = after {
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
                } else {
                    // Only discard markup whose semantics are known to be local to
                    // this run. Unknown attributes and nested nodes may carry IDs,
                    // relationships, revision data, or other opaque semantics.
                    if child.attrs.iter().any(|(name, _)| name.declaration().is_none())
                        || child.children.iter().any(|content| match content {
                            Content::Element(_) => false,
                            Content::Text(s) => !s.trim().is_empty(),
                            Content::Raw(_) => true,
                        })
                        || child.child("w:t").is_some_and(|text| {
                            text.attrs.iter().any(|(name, _)| name.declaration().is_none() && name != "xml:space")
                        })
                        || before.contains(['\t', '\r', '\n'])
                        || child.elements().filter(|n| n.name == "w:rPr").count() > 1
                        || child.child("w:rPr").is_some_and(|properties| {
                        properties.attrs.iter().any(|(name, _)| name.declaration().is_none()) || properties.children.iter().any(|content| {
                            let Content::Element(property) = content else {
                                return !matches!(content, Content::Text(s) if s.trim().is_empty());
                            };
                            !matches!(
                            property.name.as_str(),
                            "w:rStyle" | "w:b" | "w:bCs" | "w:i" | "w:iCs"
                                | "w:caps" | "w:smallCaps" | "w:strike" | "w:dstrike"
                                | "w:outline" | "w:shadow" | "w:emboss" | "w:imprint"
                                | "w:noProof" | "w:snapToGrid" | "w:vanish" | "w:webHidden"
                                | "w:color" | "w:spacing" | "w:w" | "w:kern" | "w:position"
                                | "w:sz" | "w:szCs" | "w:highlight" | "w:u" | "w:effect"
                                | "w:vertAlign" | "w:rtl" | "w:cs" | "w:em"
                                | "w:specVanish" | "w:oMath"
                            ) || property.attrs.iter().any(|(name, _)| name.declaration().is_none() && name != "w:val")
                                || property.children.iter().any(|c| !matches!(c, Content::Text(s) if s.trim().is_empty()))
                        })
                    }) {
                        return Err(unsupported("run has unsupported properties"));
                    }
                    removed.push(RunRemovalChange {
                        body_index,
                        inline_index,
                        before,
                    });
                    remove_indices.push(inline_index);
                }
            }
            boundaries.advance(child);
        }
        let mut inline_index = 0;
        block.children.retain(|content| {
            let Content::Element(node) = content else {
                return true;
            };
            if node.name == "w:pPr" {
                return true;
            }
            let remove = remove_indices.contains(&inline_index);
            inline_index += 1;
            !remove
        });
    }
    if !targets.is_empty() {
        return Err(unsupported("text target not found"));
    }
    Ok((changes, removed))
}
