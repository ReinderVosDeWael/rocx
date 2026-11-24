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

/// Edit one whole inline in a direct body paragraph at its original snapshot address.
#[derive(Clone, Debug)]
pub struct InlineEdit {
    pub target: InlineAnchor,
    pub action: InlineAction,
}

#[derive(Clone, Debug)]
pub enum InlineAction {
    InsertAfter(crate::ResolvedInline),
    Replace(crate::ResolvedInline),
    /// Set direct (non-complex-script) bold without rebuilding the run properties.
    SetBold(bool),
    /// Set (true/false) or clear (None) a direct property, without resolving styles.
    SetToggle(RunToggle, Option<bool>),
}

#[derive(Clone, Copy, Debug)]
pub enum RunToggle {
    Bold,
    Italic,
    Strike,
    Hidden,
}

impl RunToggle {
    fn tag(self) -> &'static str {
        match self {
            Self::Bold => "w:b",
            Self::Italic => "w:i",
            Self::Strike => "w:strike",
            Self::Hidden => "w:vanish",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "bold" => Some(Self::Bold),
            "italic" => Some(Self::Italic),
            "strike" => Some(Self::Strike),
            "hidden" => Some(Self::Hidden),
            _ => None,
        }
    }
}

pub(super) enum Action {
    Text(String),
    Remove,
    After(Vec<Node>),
    Replace(Vec<Node>),
    Toggle(RunToggle, Option<bool>),
}
pub(super) type Targets = BTreeMap<(usize, usize), Action>;

pub(super) fn prepare(
    package: crate::opc::Package,
    main: &str,
    edits: &[InlineEdit],
) -> Result<(crate::opc::Package, Targets)> {
    let mut ordered = BTreeMap::new();
    for edit in edits {
        let (InlineLocation::Body(body), [inline]) =
            (&edit.target.location, edit.target.path.as_slice())
        else {
            return Err(unsupported(
                "inline target must belong directly to a body paragraph",
            ));
        };
        if ordered.insert((*body, *inline), &edit.action).is_some() {
            return Err(unsupported("duplicate inline target"));
        }
    }
    render_actions(package, main, main, crate::StoryKind::Body, ordered)
}

fn render_actions<K: Ord>(
    package: crate::opc::Package,
    main: &str,
    owner: &str,
    kind: crate::StoryKind,
    ordered: BTreeMap<K, &InlineAction>,
) -> Result<(crate::opc::Package, BTreeMap<K, Action>)> {
    let payloads: Vec<_> = ordered
        .values()
        .filter_map(|action| match action {
            InlineAction::InsertAfter(i) | InlineAction::Replace(i) => Some(i),
            _ => None,
        })
        .collect();
    let (package, nodes) = if payloads.is_empty() {
        (package, vec![])
    } else {
        crate::ooxml::encode_editor_inlines(package, main, owner, kind, &payloads)?
    };
    let mut nodes = nodes.into_iter();
    Ok((
        package,
        ordered
            .into_iter()
            .map(|(key, action)| {
                (
                    key,
                    match action {
                        InlineAction::InsertAfter(_) => Action::After(nodes.next().unwrap()),
                        InlineAction::Replace(_) => Action::Replace(nodes.next().unwrap()),
                        InlineAction::SetBold(b) => Action::Toggle(RunToggle::Bold, Some(*b)),
                        InlineAction::SetToggle(prop, value) => Action::Toggle(*prop, *value),
                    },
                )
            })
            .collect(),
    ))
}

pub(super) fn direct_toggle(run: &Node, prop: RunToggle) -> Result<Option<bool>> {
    let tag = prop.tag();
    if run.elements().filter(|n| n.name == "w:rPr").count() > 1
        || run.child("w:rPr").is_some_and(|p| {
            p.child(super::ALTERNATE).is_some()
                || p.child("w:rPrChange").is_some()
                || p.elements().filter(|n| n.name == tag).count() > 1
        })
    {
        return Err(unsupported("ambiguous run properties"));
    }
    run.child("w:rPr")
        .and_then(|p| p.child(tag))
        .map(|p| match p.get("w:val") {
            None | Some("true" | "1" | "on") => Ok(true),
            Some("false" | "0" | "off") => Ok(false),
            _ => Err(unsupported("unknown run toggle value")),
        })
        .transpose()
}

fn patch_toggle(run: &mut Node, prop: RunToggle, value: Option<bool>) -> Result<bool> {
    let tag = prop.tag();
    if direct_toggle(run, prop)? == value {
        return Ok(false);
    }
    if run.child("w:rPr").is_none() {
        run.children.insert(0, Content::Element(Node::new("w:rPr")));
    }
    let props = run.child_mut("w:rPr").unwrap();
    if let Some(value) = value {
        props
            .ensure(tag)
            .set("w:val", if value { "1" } else { "0" });
    } else {
        let node = props.child(tag).unwrap();
        if has_other_attrs(node, &["w:val"]) || !node.children.is_empty() {
            return Err(unsupported("cannot clear opaque run property"));
        }
        props.remove(tag);
    }
    Ok(true)
}

fn single_text(run: &Node) -> Result<String> {
    if run.name != "w:r"
        || run
            .elements()
            .filter(|n| n.name != "w:rPr")
            .map(|n| n.name.as_str())
            .ne(["w:t"])
    {
        return Err(unsupported(
            "target must be a single-text run outside fields and ranges",
        ));
    }
    let mut value = String::new();
    for c in &run.child("w:t").unwrap().children {
        if let Content::Text(t) = c {
            value.push_str(t);
        } else {
            return Err(unsupported("target text has opaque content"));
        }
    }
    Ok(value)
}

fn has_other_attrs(node: &Node, allowed: &[&str]) -> bool {
    node.attrs
        .iter()
        .any(|(name, _)| name.declaration().is_none() && !allowed.contains(&name.as_str()))
}

fn opaque_content(node: &Node) -> bool {
    node.children.iter().any(|c| match c {
        Content::Element(_) => false,
        Content::Text(t) => !t.trim().is_empty(),
        Content::Raw(_) => true,
    })
}

/// Signals retained markup not represented by the four direct toggle fields.
pub(super) fn has_opaque_formatting(run: &Node) -> bool {
    let mut properties = run.elements().filter(|p| p.name == "w:rPr");
    let Some(props) = properties.next() else {
        return false;
    };
    properties.next().is_some()
        || has_other_attrs(props, &[])
        || opaque_content(props)
        || props.elements().any(|node| {
            let prop = match node.name.as_str() {
                "w:b" => RunToggle::Bold,
                "w:i" => RunToggle::Italic,
                "w:strike" => RunToggle::Strike,
                "w:vanish" => RunToggle::Hidden,
                _ => return true,
            };
            has_other_attrs(node, &["w:val"])
                || !node.children.is_empty()
                || direct_toggle(run, prop).is_err()
        })
}

/// Only discard known local properties. Unknown markup may carry a relationship,
/// range ID, or revision, and cannot safely be reconstructed from inspection.
fn disposable_properties(props: &Node) -> bool {
    !has_other_attrs(props, &[])
        && !opaque_content(props)
        && props.elements().all(|p| {
            matches!(
                p.name.as_str(),
                "w:rStyle"
                    | "w:b"
                    | "w:bCs"
                    | "w:i"
                    | "w:iCs"
                    | "w:caps"
                    | "w:smallCaps"
                    | "w:strike"
                    | "w:dstrike"
                    | "w:outline"
                    | "w:shadow"
                    | "w:emboss"
                    | "w:imprint"
                    | "w:noProof"
                    | "w:snapToGrid"
                    | "w:vanish"
                    | "w:webHidden"
                    | "w:color"
                    | "w:spacing"
                    | "w:w"
                    | "w:kern"
                    | "w:position"
                    | "w:sz"
                    | "w:szCs"
                    | "w:highlight"
                    | "w:u"
                    | "w:effect"
                    | "w:vertAlign"
                    | "w:rtl"
                    | "w:cs"
                    | "w:em"
                    | "w:specVanish"
                    | "w:oMath"
            ) && !has_other_attrs(p, &["w:val"])
                && !opaque_content(p)
                && p.elements().next().is_none()
        })
}

fn check_run(run: &Node, discard: bool) -> Result<()> {
    let error = || unsupported("run has unsupported properties or inline content");
    if run.name != "w:r" || opaque_content(run) || (discard && has_other_attrs(run, &[])) {
        return Err(error());
    }
    let mut elements = run.elements().peekable();
    if let Some(props) = elements.next_if(|n| n.name == "w:rPr")
        && (props.child(super::ALTERNATE).is_some()
            || props.child("w:rPrChange").is_some()
            || discard && !disposable_properties(props))
    {
        return Err(error());
    }
    let Some(leaf) = elements.next() else {
        return Err(error());
    };
    if elements.next().is_some() {
        return Err(error());
    }
    check_leaf(leaf, discard)
}

fn check_leaf(leaf: &Node, discard: bool) -> Result<()> {
    match leaf.name.as_str() {
        "w:t"
            if !(discard && has_other_attrs(leaf, &["xml:space"]))
                && leaf.children.iter().all(
                    |c| matches!(c,Content::Text(t) if !discard || !t.contains(['\t','\r','\n'])),
                ) =>
        {
            Ok(())
        }
        "w:br" | "w:cr" | "w:tab"
            if leaf.children.is_empty()
                && (!discard
                    || !has_other_attrs(
                        leaf,
                        if leaf.name == "w:br" {
                            &["w:type", "w:clear"]
                        } else {
                            &[]
                        },
                    )) =>
        {
            Ok(())
        }
        _ => Err(unsupported(
            "run has unsupported properties or inline content",
        )),
    }
}

/// A property-only patch can preserve multiple ordinary leaves without
/// interpreting their order or modifying their text or attributes.
fn check_toggle_target(run: &Node) -> Result<()> {
    if run.name != "w:r" || opaque_content(run) {
        return Err(unsupported("expected direct run with ordinary leaves"));
    }
    let mut children = run.elements().peekable();
    children.next_if(|n| n.name == "w:rPr");
    if children.peek().is_none() {
        return Err(unsupported("expected nonempty run"));
    }
    children.try_for_each(|leaf| check_leaf(leaf, false))
}

/// Revision operations may copy or discard either ordinary or deleted text.
/// The caller checks the leaf shape; use the same local-property allowlist as
/// whole-inline replacement.
pub(super) fn disposable_run(run: &Node, tag: &str, text: &str) -> bool {
    !has_other_attrs(run, &[])
        && !opaque_content(run)
        && !text.contains(['\t', '\r', '\n'])
        && run
            .child(tag)
            .is_some_and(|n| !has_other_attrs(n, &["xml:space"]))
        && run.elements().filter(|n| n.name == "w:rPr").count() <= 1
        && run.child("w:rPr").is_none_or(disposable_properties)
}

fn check_whole(node: &Node, discard: bool) -> Result<()> {
    if node.name != "w:hyperlink" {
        return check_run(node, discard);
    }
    if opaque_content(node)
        || (discard && has_other_attrs(node, &["r:id", "w:anchor", "w:history", "w:tooltip"]))
    {
        return Err(unsupported("hyperlink has opaque properties"));
    }
    for child in node.elements() {
        check_run(child, discard)?;
    }
    Ok(())
}

pub(super) fn apply(
    body: &mut Node,
    edits: &[RunTextEdit],
    removals: &[InlineAnchor],
    mut targets: Targets,
    inline_changes: &mut Vec<(usize, usize)>,
) -> Result<(Vec<RunTextChange>, Vec<RunRemovalChange>)> {
    for edit in edits {
        let (InlineLocation::Body(body_index), [inline_index]) =
            (&edit.target.location, edit.target.path.as_slice())
        else {
            return Err(unsupported("text target must be a direct body run"));
        };
        validate_text(&edit.text)?;
        if targets
            .insert(
                (*body_index, *inline_index),
                Action::Text(edit.text.clone()),
            )
            .is_some()
        {
            return Err(unsupported("duplicate or conflicting run text target"));
        }
    }
    for target in removals {
        let (InlineLocation::Body(body_index), [inline_index]) =
            (&target.location, target.path.as_slice())
        else {
            return Err(unsupported("removal target must be a direct body run"));
        };
        if targets
            .insert((*body_index, *inline_index), Action::Remove)
            .is_some()
        {
            return Err(unsupported("duplicate or conflicting run target"));
        }
    }
    apply_targets(body, targets, true, inline_changes)
}

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
    mut targets: Targets,
    main_body: bool,
    inline_changes: &mut Vec<(usize, usize)>,
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
        let mut splices = Vec::new();
        for (inline_index, (slot, child)) in block
            .children
            .iter_mut()
            .enumerate()
            .filter_map(|(slot, c)| {
                if let Content::Element(n) = c
                    && n.name != "w:pPr"
                {
                    Some((slot, n))
                } else {
                    None
                }
            })
            .enumerate()
        {
            if let Some(after) = targets.remove(&(body_index, inline_index)) {
                if !boundaries.safe() {
                    return Err(unsupported("target must be outside fields and ranges"));
                }
                match after {
                    Action::After(nodes) => {
                        check_whole(child, false)?;
                        splices.push((slot + 1..slot + 1, nodes));
                        inline_changes.push((body_index, inline_index));
                    }
                    Action::Toggle(prop, value) => {
                        check_toggle_target(child)?;
                        if patch_toggle(child, prop, value)? {
                            inline_changes.push((body_index, inline_index));
                        }
                    }
                    Action::Text(after) => {
                        let before = single_text(child)?;
                        let t = child.child_mut("w:t").unwrap();
                        if before != after {
                            t.children = vec![Content::Text(after.clone())];
                            if after.starts_with(' ') || after.ends_with(' ') {
                                t.set("xml:space", "preserve");
                            }
                            changes.push(RunTextChange {
                                body_index,
                                inline_index,
                                before,
                                after,
                            });
                        }
                    }
                    Action::Replace(nodes) => {
                        check_whole(child, true)?;
                        splices.push((slot..slot + 1, nodes));
                        inline_changes.push((body_index, inline_index));
                    }
                    Action::Remove => {
                        let before = single_text(child)?;
                        check_run(child, true)?;
                        removed.push(RunRemovalChange {
                            body_index,
                            inline_index,
                            before,
                        });
                        splices.push((slot..slot + 1, Vec::new()));
                    }
                }
            }
            boundaries.advance(child);
        }
        if !splices.is_empty() {
            // Walk the original physical slots once: comments and whitespace
            // between addressed siblings keep their relative positions.
            let mut original = std::mem::take(&mut block.children).into_iter();
            let mut offset = 0;
            for (range, nodes) in splices {
                block
                    .children
                    .extend(original.by_ref().take(range.start - offset));
                if !range.is_empty() {
                    original.next();
                }
                block
                    .children
                    .extend(nodes.into_iter().map(Content::Element));
                offset = range.end;
            }
            block.children.extend(original);
        }
    }
    if !targets.is_empty() {
        return Err(unsupported("text target not found"));
    }
    Ok((changes, removed))
}

impl super::ImportedDocument {
    /// Apply whole-inline operations to ordinary entries in one physical
    /// footnote, endnote or comment part, using original snapshot anchors.
    pub fn edit_entry_inlines(&self, owner_part: &str, edits: &[InlineEdit]) -> Result<Vec<u8>> {
        use crate::ooxml::WordParts;
        let mut ordered = BTreeMap::new();
        let mut counts = None;
        let mut kind = None;
        for edit in edits {
            self.validate_snapshot(&edit.target.snapshot)?;
            let (InlineLocation::Entry { index, block_index }, [inline]) =
                (&edit.target.location, edit.target.path.as_slice())
            else {
                return Err(unsupported("expected a direct entry inline"));
            };
            let e = self.editable_entry(owner_part, *index, &mut counts)?;
            kind = Some(match e.kind {
                super::EntryKind::Comment => crate::StoryKind::Comment,
                super::EntryKind::Footnote => crate::StoryKind::Footnote,
                super::EntryKind::Endnote => crate::StoryKind::Endnote,
            });
            if ordered
                .insert((e.element_index, *block_index, *inline), &edit.action)
                .is_some()
            {
                return Err(unsupported("duplicate entry inline target"));
            }
        }
        let kind = kind.ok_or_else(|| unsupported("expected an entry inline edit"))?;
        let (mut package, actions) =
            render_actions(self.package.clone(), &self.main, owner_part, kind, ordered)?;
        let mut groups = BTreeMap::<usize, Targets>::new();
        for ((entry, block, inline), action) in actions {
            groups
                .entry(entry)
                .or_default()
                .insert((block, inline), action);
        }
        let mut root = package.word(owner_part)?;
        let mut changes = Vec::new();
        for (index, entry) in root.elements_mut().enumerate() {
            if let Some(targets) = groups.remove(&index) {
                apply_targets(entry, targets, false, &mut changes)?;
            }
        }
        if !groups.is_empty() {
            return Err(unsupported("story entry not found"));
        }
        if !changes.is_empty() {
            package.put_word(owner_part, root)?;
        }
        super::PackagePlan::new(package)?.to_bytes()
    }

    fn editable_entry(
        &self,
        owner_part: &str,
        index: usize,
        entry_ids: &mut Option<BTreeMap<String, usize>>,
    ) -> Result<&super::ImportedEntry> {
        let entries = self.entry_stories()?;
        let e = entries
            .get(index)
            .ok_or_else(|| unsupported("story entry not found"))?;
        if e.part_name != owner_part {
            return Err(unsupported("story text owner mismatch"));
        }
        let id =
            crate::ooxml::decimal_id(&e.id).ok_or_else(|| unsupported("invalid story entry ID"))?;
        if e.kind != super::EntryKind::Comment
            && (id.starts_with('-') || !matches!(e.note_type.as_deref(), None | Some("normal")))
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
                *counts.entry(id).or_default() += 1;
            }
            counts
        });
        if counts.get(&id) != Some(&1) {
            return Err(unsupported("ambiguous story entry ID"));
        }
        Ok(e)
    }
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
                    let e = self.editable_entry(owner_part, *index, &mut entry_ids)?;
                    (Some(e.element_index), *block_index)
                }
                _ => return Err(unsupported("story text owner mismatch")),
            };
            if groups
                .entry(entry)
                .or_default()
                .insert((block, *inline), Action::Text(edit.text.clone()))
                .is_some()
            {
                return Err(unsupported("duplicate story text target"));
            }
        }
        let mut package = self.package.clone();
        let mut root = package.word(owner_part)?;
        let mut changed = false;
        if let Some(targets) = groups.remove(&None) {
            changed = !apply_targets(&mut root, targets, false, &mut Vec::new())?
                .0
                .is_empty();
        } else {
            for (i, entry) in root.elements_mut().enumerate() {
                if let Some(targets) = groups.remove(&Some(i)) {
                    changed |= !apply_targets(entry, targets, false, &mut Vec::new())?
                        .0
                        .is_empty();
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
