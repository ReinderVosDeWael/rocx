use super::{InlineAnchor, InlineLocation, insertion::BoundaryState, unsupported};
use super::{RunFormatField, run_format};
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
    /// Remove a supported direct body hyperlink including its displayed runs.
    RemoveHyperlink,
    /// Set or clear the tooltip without rewriting the displayed runs.
    HyperlinkTooltip(Option<String>),
    /// Set or clear the hyperlink's history flag.
    HyperlinkHistory(Option<bool>),
    SetDrawingDescription(Option<String>),
    SetImageAsset(crate::ResolvedImage),
    PatchDrawing(super::DrawingProperty, crate::ResolvedInline),
    RemoveObject,
    /// Set (true/false) or clear (None) a direct property, without resolving styles.
    SetToggle(RunToggle, Option<bool>),
    /// Omitted properties are retained; clear restores inheritance for selected slots.
    PatchFormat {
        set: Box<crate::RunFormat>,
        clear: Vec<RunFormatField>,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum RunToggle {
    Bold,
    Italic,
    Strike,
    Hidden,
}

impl RunToggle {
    fn field(self) -> RunFormatField {
        match self {
            Self::Bold => RunFormatField::Bold,
            Self::Italic => RunFormatField::Italic,
            Self::Strike => RunFormatField::Strike,
            Self::Hidden => RunFormatField::Hidden,
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
    Format(run_format::Slots),
    Description(Option<String>),
    Asset(String),
    Drawing(super::DrawingProperty, Node),
    Chart(String),
    RemoveObject,
    RemoveHyperlink,
    LinkProperties(BTreeMap<&'static str, Option<String>>),
}
pub(super) type Targets = BTreeMap<(usize, usize), Action>;

pub(super) fn prepare(
    package: crate::opc::Package,
    doc: &super::ImportedDocument,
    edits: &[InlineEdit],
    images: &mut Option<crate::ooxml::HeaderImageResources>,
) -> Result<(crate::opc::Package, Targets)> {
    let main = doc.main.as_str();
    let mut ordered = BTreeMap::new();
    let mut charts = BTreeMap::new();
    for edit in edits {
        let (InlineLocation::Body(body), [inline]) =
            (&edit.target.location, edit.target.path.as_slice())
        else {
            return Err(unsupported(
                "inline target must belong directly to a body paragraph",
            ));
        };
        ordered
            .entry((*body, *inline))
            .or_insert_with(Vec::new)
            .push(&edit.action);
        if let InlineAction::PatchDrawing(super::DrawingProperty::ChartSeries, i) = &edit.action {
            let crate::ResolvedInline::ColumnChart(chart) = i else {
                return Err(unsupported("expected a resolved column chart"));
            };
            charts.insert((*body, *inline), (doc.chart_id(&edit.target)?, chart));
        }
    }
    render_actions(
        package,
        main,
        main,
        crate::StoryKind::Body,
        ordered,
        charts.into_values().collect(),
        Some(images),
    )
}

fn render_actions<K: Ord>(
    package: crate::opc::Package,
    main: &str,
    owner: &str,
    kind: crate::StoryKind,
    ordered: BTreeMap<K, Vec<&InlineAction>>,
    charts: Vec<(&str, &crate::ColumnChartProperties)>,
    resources: Option<&mut Option<crate::ooxml::HeaderImageResources>>,
) -> Result<(crate::opc::Package, BTreeMap<K, Action>)> {
    let payloads: Vec<_> = ordered
        .values()
        .flatten()
        .filter_map(|action| match action {
            InlineAction::InsertAfter(i) | InlineAction::Replace(i) => Some(i),
            _ => None,
        })
        .collect();
    let (mut package, nodes) = if payloads.is_empty() {
        (package, vec![])
    } else {
        crate::ooxml::encode_owner_inlines(package, main, owner, kind, &payloads, resources)?
    };
    let mut nodes = nodes.into_iter();
    let images: Vec<_> = ordered
        .values()
        .flatten()
        .filter_map(|a| match a {
            InlineAction::SetImageAsset(i) => Some(i),
            _ => None,
        })
        .collect();
    let mut links = crate::ooxml::image_asset_links(&mut package, owner, &images)?.into_iter();
    let mut chart_links = charts
        .into_iter()
        .map(|(id, c)| super::objects::chart::fork(&mut package, owner, id, c))
        .collect::<Result<Vec<_>>>()?
        .into_iter();
    let patches: Vec<_> = ordered
        .values()
        .flatten()
        .filter_map(|a| match a {
            InlineAction::PatchDrawing(super::DrawingProperty::ChartSeries, _) => None,
            InlineAction::PatchDrawing(_, i) => Some(i),
            _ => None,
        })
        .collect();
    let mut rendered = Vec::new();
    if !patches.is_empty() {
        if patches.iter().any(|i| {
            !matches!(
                i,
                crate::ResolvedInline::Shape(_)
                    | crate::ResolvedInline::ColumnChart(_)
                    | crate::ResolvedInline::Image { comment: None, .. }
            )
        }) {
            return Err(unsupported("expected a resolved drawing"));
        }
        // The declaration supplies property XML only. Its media, drawing IDs
        // and chart parts must not leak into the output package.
        rendered =
            crate::ooxml::encode_editor_inlines(package.clone(), main, owner, kind, &patches)?.1;
    }
    let mut rendered = rendered.into_iter();
    let mut targets = BTreeMap::new();
    for (key, group) in ordered {
        let action = match group.as_slice() {
            [InlineAction::InsertAfter(_)] => Action::After(nodes.next().unwrap()),
            [InlineAction::Replace(_)] => Action::Replace(nodes.next().unwrap()),
            [InlineAction::SetDrawingDescription(s)] => Action::Description(s.clone()),
            [InlineAction::SetImageAsset(_)] => Action::Asset(links.next().unwrap()),
            [InlineAction::PatchDrawing(super::DrawingProperty::ChartSeries, _)] => {
                Action::Chart(chart_links.next().unwrap())
            }
            [InlineAction::PatchDrawing(property, _)] => {
                Action::Drawing(*property, rendered.next().unwrap().pop().unwrap())
            }
            [InlineAction::RemoveObject] => Action::RemoveObject,
            [InlineAction::RemoveHyperlink] => Action::RemoveHyperlink,
            _ if group.iter().any(|a| {
                matches!(
                    a,
                    InlineAction::HyperlinkTooltip(_) | InlineAction::HyperlinkHistory(_)
                )
            }) =>
            {
                let mut properties = BTreeMap::new();
                for action in group {
                    let (name, value) = match action {
                        InlineAction::HyperlinkTooltip(v) => ("w:tooltip", v.clone()),
                        InlineAction::HyperlinkHistory(v) => {
                            ("w:history", v.map(|b| if b { "1" } else { "0" }.into()))
                        }
                        _ => return Err(unsupported("conflicting hyperlink action")),
                    };
                    if properties.insert(name, value).is_some() {
                        return Err(unsupported("duplicate hyperlink property"));
                    }
                }
                Action::LinkProperties(properties)
            }
            _ => {
                let mut slots = run_format::Slots::new();
                for action in group {
                    compile_format(&mut slots, action, &package, main)?;
                }
                Action::Format(slots)
            }
        };
        targets.insert(key, action);
    }
    Ok((package, targets))
}

pub(super) fn compile_format(
    slots: &mut run_format::Slots,
    action: &InlineAction,
    package: &crate::opc::Package,
    main: &str,
) -> Result<()> {
    let (prop, value) = match action {
        InlineAction::PatchFormat { set, clear } => {
            return run_format::compile(slots, set, clear, package, main);
        }
        InlineAction::SetBold(b) => (RunToggle::Bold, Some(*b)),
        InlineAction::SetToggle(p, v) => (*p, *v),
        _ => {
            return Err(unsupported(
                "duplicate/conflicting target or non-format inline edit",
            ));
        }
    };
    let field = prop.field();
    run_format::add(
        slots,
        field,
        value.map(|b| Node::new(field.tag()).attr("w:val", if b { "1" } else { "0" })),
    )
}

pub(super) fn direct_toggle(run: &Node, prop: RunToggle) -> Result<Option<bool>> {
    let tag = prop.field().tag();
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

pub(super) fn patch_toggle(run: &mut Node, prop: RunToggle, value: Option<bool>) -> Result<bool> {
    let tag = prop.field().tag();
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

pub(super) fn has_other_attrs(node: &Node, allowed: &[&str]) -> bool {
    node.attrs
        .iter()
        .any(|(name, _)| name.declaration().is_none() && !allowed.contains(&name.as_str()))
}

pub(super) fn opaque_content(node: &Node) -> bool {
    node.children.iter().any(|c| match c {
        Content::Element(_) => false,
        Content::Text(t) => !t.trim().is_empty(),
        Content::Raw(_) => true,
    })
}

/// Only discard known local properties. Unknown markup may carry a relationship,
/// range ID, or revision, and cannot safely be reconstructed from inspection.
pub(super) fn disposable_properties(props: &Node) -> bool {
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
    check_run_children(run, discard, false)
}

pub(super) fn check_cache_run(run: &Node) -> Result<()> {
    check_run_children(run, true, true)
}
fn check_run_children(run: &Node, discard: bool, multiple: bool) -> Result<()> {
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
    if !multiple && elements.peek().is_some() {
        return Err(error());
    }
    for leaf in std::iter::once(leaf).chain(elements) {
        check_leaf(leaf, discard)?;
    }
    Ok(())
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
pub(super) fn check_format_target(run: &Node) -> Result<()> {
    if run.name != "w:r" || opaque_content(run) {
        return Err(unsupported("expected direct run with ordinary leaves"));
    }
    let mut children = run.elements().peekable();
    children.next_if(|n| n.name == "w:rPr");
    if children.peek().is_none() {
        return Err(unsupported("expected nonempty run"));
    }
    children.try_for_each(|leaf| match leaf.name.as_str() {
        "w:noBreakHyphen" | "w:softHyphen" | "w:sym" | "w:lastRenderedPageBreak" | "w:ptab"
            if leaf.children.is_empty() =>
        {
            Ok(())
        }
        "w:drawing"
            if leaf.elements().count() == 1
                && leaf
                    .elements()
                    .all(|n| matches!(n.name.as_str(), "wp:inline" | "wp:anchor")) =>
        {
            Ok(())
        }
        _ => check_leaf(leaf, false),
    })
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

pub(super) fn check_whole(node: &Node, discard: bool) -> Result<()> {
    if super::objects::fraction(node).is_some() {
        return Ok(());
    }
    if super::objects::drawing(node).is_some() {
        if opaque_content(node)
            || node.elements().filter(|n| n.name == "w:rPr").count() > 1
            || node.child("w:rPr").is_some_and(|p| {
                p.child("w:rPrChange").is_some()
                    || p.child(super::ALTERNATE).is_some()
                    || discard && !disposable_properties(p)
            })
            || discard
                && (has_other_attrs(node, &[])
                    || !super::objects::disposable(node.child("w:drawing").unwrap()))
        {
            return Err(unsupported("drawing has opaque content"));
        }
        return Ok(());
    }
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
    slots: super::batch::Slots,
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
    apply_targets(body, targets, true, inline_changes, slots)
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
    mut slots: super::batch::Slots,
) -> Result<(Vec<RunTextChange>, Vec<RunRemovalChange>)> {
    let mut changes = Vec::new();
    let mut removed = Vec::new();
    let mut boundaries = BoundaryState::default();
    for (body_index, block) in body
        .elements_mut()
        .filter(|n| !main_body || n.name != "w:sectPr")
        .enumerate()
    {
        if targets.is_empty() && slots.is_empty() {
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
            let site = (body_index, inline_index);
            let structured = slots.remove(&site);
            let action = targets.remove(&site);
            if let Some(s) = &structured {
                if action.is_some()
                    && (s.field
                        || s.replacement.is_some()
                        || !matches!(
                            action,
                            Some(Action::Text(_) | Action::Format(_) | Action::LinkProperties(_))
                        ))
                {
                    return Err(unsupported("conflicting structured and inline targets"));
                }
                if !s.before.is_empty() || !s.after.is_empty() || s.replacement.is_some() {
                    inline_changes.push(site);
                }
            }
            let mut tail = Vec::new();
            if let Some(s) = structured {
                if !s.before.is_empty() {
                    splices.push((slot..slot, s.before));
                }
                if let Some(nodes) = s.replacement {
                    splices.push((slot..slot + 1, nodes));
                }
                tail = s.after;
            }
            if let Some(after) = action {
                if !boundaries.safe() {
                    return Err(unsupported("target must be outside fields and ranges"));
                }
                match after {
                    Action::RemoveHyperlink => {
                        if child.name != "w:hyperlink" {
                            return Err(unsupported("expected a hyperlink"));
                        }
                        check_whole(child, true)?;
                        splices.push((slot..slot + 1, Vec::new()));
                        inline_changes.push((body_index, inline_index));
                    }
                    Action::Chart(id) => {
                        check_whole(child, false)?;
                        if super::objects::chart_link(child, &id)? {
                            inline_changes.push((body_index, inline_index));
                        }
                    }
                    Action::Drawing(property, src) => {
                        check_whole(child, false)?;
                        if super::objects::patch(child, &src, property)? {
                            inline_changes.push((body_index, inline_index));
                        }
                    }
                    Action::Asset(id) => {
                        check_whole(child, false)?;
                        if super::objects::image_asset(child, &id)? {
                            inline_changes.push((body_index, inline_index));
                        }
                    }
                    Action::RemoveObject => {
                        if super::objects::drawing(child).is_none()
                            && super::objects::fraction(child).is_none()
                        {
                            return Err(unsupported("expected a supported object occurrence"));
                        }
                        check_whole(child, true)?;
                        splices.push((slot..slot + 1, Vec::new()));
                        inline_changes.push((body_index, inline_index));
                    }
                    Action::LinkProperties(props) => {
                        if child.name != "w:hyperlink" {
                            return Err(unsupported("expected a hyperlink"));
                        }
                        for (key, value) in props {
                            if value.as_ref().is_some_and(|v| {
                                crate::document::invalid_xml_character(v).is_some()
                            }) {
                                return Err(unsupported("invalid XML hyperlink property"));
                            }
                            if child.get(key) != value.as_deref() {
                                child.attrs.retain(|(k, _)| k != key);
                                if let Some(v) = value {
                                    child.set(key, v);
                                }
                                inline_changes.push((body_index, inline_index));
                            }
                        }
                    }
                    Action::Description(value) => {
                        check_whole(child, false)?;
                        if super::objects::description(child, value.as_deref())? {
                            inline_changes.push((body_index, inline_index));
                        }
                    }
                    Action::After(nodes) => {
                        check_whole(child, false)?;
                        splices.push((slot + 1..slot + 1, nodes));
                        inline_changes.push((body_index, inline_index));
                    }
                    Action::Format(slots) => {
                        check_format_target(child)?;
                        if run_format::apply(child, slots)? {
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
            if !tail.is_empty() {
                splices.push((slot + 1..slot + 1, tail));
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
    if !targets.is_empty() || !slots.is_empty() {
        return Err(unsupported("text target not found"));
    }
    Ok((changes, removed))
}

impl super::ImportedDocument {
    /// Apply whole-inline operations to ordinary entries in one physical
    /// footnote, endnote or comment part, using original snapshot anchors.
    pub fn edit_entry_inlines(&self, owner_part: &str, edits: &[InlineEdit]) -> Result<Vec<u8>> {
        use crate::ooxml::WordParts;
        if !edits.is_empty()
            && edits.iter().all(|e| {
                matches!(
                    e.action,
                    InlineAction::SetBold(_)
                        | InlineAction::SetToggle(_, _)
                        | InlineAction::PatchFormat { .. }
                )
            })
        {
            if !edits
                .iter()
                .all(|e| matches!(e.target.location, InlineLocation::Entry { .. }))
            {
                return Err(unsupported("expected an entry inline"));
            }
            return self.edit_run_format(Some(owner_part), edits);
        }
        let mut ordered = BTreeMap::new();
        let mut counts = None;
        let mut kind = None;
        for edit in edits {
            self.validate_snapshot(&edit.target.snapshot)?;
            if matches!(
                edit.action,
                InlineAction::RemoveHyperlink
                    | InlineAction::HyperlinkTooltip(_)
                    | InlineAction::HyperlinkHistory(_)
            ) {
                return Err(unsupported(
                    "hyperlink property edits and removal require a direct body hyperlink",
                ));
            }
            let (InlineLocation::Entry { index, block_index }, [inline]) =
                (&edit.target.location, edit.target.path.as_slice())
            else {
                return Err(unsupported("expected a direct entry inline"));
            };
            let e = self.editable_entry(owner_part, *index, &mut counts)?;
            if matches!(
                edit.action,
                InlineAction::PatchDrawing(super::DrawingProperty::ChartSeries, _)
            ) {
                return Err(unsupported(
                    "chart series patches require a direct body occurrence",
                ));
            }
            let node = self.inline_at(&edit.target)?;
            if matches!(node, super::ImportedInlineNode::Fraction { .. })
                || matches!(node, super::ImportedInlineNode::Run(run)
                    if run.drawing.as_ref().is_some_and(|drawing| matches!(drawing.kind, "chart" | "shape")))
            {
                return Err(unsupported(
                    "only picture occurrences are editable in entry stories",
                ));
            }
            kind = Some(match e.kind {
                super::EntryKind::Comment => crate::StoryKind::Comment,
                super::EntryKind::Footnote => crate::StoryKind::Footnote,
                super::EntryKind::Endnote => crate::StoryKind::Endnote,
            });
            ordered
                .entry((e.element_index, *block_index, *inline))
                .or_insert_with(Vec::new)
                .push(&edit.action);
        }
        let kind = kind.ok_or_else(|| unsupported("expected an entry inline edit"))?;
        let (mut package, actions) = render_actions(
            self.package.clone(),
            &self.main,
            owner_part,
            kind,
            ordered,
            vec![],
            None,
        )?;
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
                apply_targets(entry, targets, false, &mut changes, Default::default())?;
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

    pub(super) fn editable_entry(
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
            changed = !apply_targets(
                &mut root,
                targets,
                false,
                &mut Vec::new(),
                Default::default(),
            )?
            .0
            .is_empty();
        } else {
            for (i, entry) in root.elements_mut().enumerate() {
                if let Some(targets) = groups.remove(&Some(i)) {
                    changed |=
                        !apply_targets(entry, targets, false, &mut Vec::new(), Default::default())?
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
