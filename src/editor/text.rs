use super::{InlineAnchor, InlineLocation, insertion::BoundaryState, unsupported};
use crate::Result;
use crate::xml::{Content, Node};
use std::collections::BTreeMap;

/// Change one direct single-text run. Body preview and physical-story editing
/// accept targets from their respective scopes and preserve formatting.
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
        validate_text(&edit.text)?;
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
    apply_targets(body, targets, true)
}

type Targets<'a> = BTreeMap<(usize, usize), Option<&'a String>>;

fn validate_text(text: &str) -> Result<()> {
    if text.contains(['\t', '\r', '\n']) || crate::document::invalid_xml_character(text).is_some() {
        return Err(unsupported(
            "replacement must be single-line XML text without tabs",
        ));
    }
    Ok(())
}

fn apply_targets(
    body: &mut Node,
    mut targets: Targets<'_>,
    main_body: bool,
) -> Result<(Vec<RunTextChange>, Vec<RunRemovalChange>)> {
    let mut changes = Vec::new();
    let mut removed = Vec::new();
    let mut boundaries = BoundaryState::default();
    for (body_index, block) in body
        .elements_mut()
        .filter(|n| !main_body || n.name != "w:sectPr")
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
        if remove_indices.is_empty() {
            continue;
        }
        let mut indices = remove_indices.into_iter().peekable();
        let mut inline_index = 0;
        block.children.retain(|content| {
            let Content::Element(node) = content else {
                return true;
            };
            if node.name == "w:pPr" {
                return true;
            }
            let remove = indices.next_if_eq(&inline_index).is_some();
            inline_index += 1;
            !remove
        });
    }
    if !targets.is_empty() {
        return Err(unsupported("text target not found"));
    }
    Ok((changes, removed))
}

impl super::ImportedDocument {
    /// Replace direct single-text runs in one physical story owner. Section aliases
    /// share the change. Entry IDs, reference runs and main-document links are untouched.
    pub fn edit_story_text(&self, owner_part: &str, edits: &[RunTextEdit]) -> Result<Vec<u8>> {
        use crate::ooxml::WordParts;
        let mut groups = BTreeMap::<Option<usize>, Targets>::new();
        let mut entry_ids = None;
        if edits.is_empty() {
            return Err(unsupported("expected a story text edit"));
        }
        for edit in edits {
            self.validate_snapshot(&edit.target.snapshot)?;
            validate_text(&edit.text)?;
            let [inline] = edit.target.path.as_slice() else {
                return Err(unsupported("expected a direct story run"));
            };
            let (entry, block) = match &edit.target.location {
                InlineLocation::Story { part, block_index } if part.as_ref() == owner_part => {
                    (None, *block_index)
                }
                InlineLocation::Entry { index, block_index } => {
                    let entries = self.entry_stories()?;
                    let e = &entries[*index];
                    if e.part_name != owner_part {
                        return Err(unsupported("story text owner mismatch"));
                    }
                    let id = crate::ooxml::decimal_id(&e.id)
                        .ok_or_else(|| unsupported("invalid story entry ID"))?;
                    if e.kind != super::EntryKind::Comment
                        && (id.starts_with('-')
                            || !matches!(e.note_type.as_deref(), None | Some("normal")))
                    {
                        return Err(unsupported("reserved note entry"));
                    }
                    let counts = entry_ids.get_or_insert_with(|| {
                        let mut counts = BTreeMap::new();
                        for id in entries
                            .iter()
                            .filter(|other| other.part_name == owner_part)
                            .filter_map(|other| crate::ooxml::decimal_id(&other.id))
                        {
                            *counts.entry(id).or_insert(0usize) += 1;
                        }
                        counts
                    });
                    if counts.get(&id) != Some(&1) {
                        return Err(unsupported("ambiguous story entry ID"));
                    }
                    (Some(e.element_index), *block_index)
                }
                _ => return Err(unsupported("story text owner mismatch")),
            };
            if groups
                .entry(entry)
                .or_default()
                .insert((block, *inline), Some(&edit.text))
                .is_some()
            {
                return Err(unsupported("duplicate story text target"));
            }
        }
        let mut package = self.package.clone();
        let mut root = package.word(owner_part)?;
        let mut changed = false;
        if let Some(targets) = groups.remove(&None) {
            changed = !apply_targets(&mut root, targets, false)?.0.is_empty();
        } else {
            for (i, entry) in root.elements_mut().enumerate() {
                if let Some(targets) = groups.remove(&Some(i)) {
                    changed |= !apply_targets(entry, targets, false)?.0.is_empty();
                }
            }
        }
        if !groups.is_empty() {
            return Err(unsupported("story entry not found"));
        }
        if changed {
            package.put_word(owner_part, root)?;
        }
        super::PackagePlan::new(package)?.to_bytes()
    }
}
