//! Opt-in direct body text changes (ECMA-376 Part 1 §17.13.5).
use super::{insertion::BoundaryState, *};
use crate::ooxml::revisions::RevisionIds;
use crate::{BookmarkName, RevisionKind, TextRevision, TextRevisionAction};
use std::ops::Range;
mod paragraphs;
pub use paragraphs::TrackedParagraph;

/// Explicit revision-producing edit for one supported imported body run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrackedText {
    InsertAfter(String),
    Replace(String),
    Delete,
}

type Site = (usize, usize); // raw XML child slots, including opaque content

fn node(content: &Content) -> &Node {
    let Content::Element(n) = content else {
        unreachable!("validated element slot")
    };
    n
}

fn paragraph(body: &Node, site: Site) -> &Node {
    node(&body.children[site.0])
}
fn at(body: &Node, site: Site) -> &Node {
    node(&paragraph(body, site).children[site.1])
}

fn splice(body: &mut Node, p: usize, slots: Range<usize>, nodes: Vec<Node>) {
    let Content::Element(p) = &mut body.children[p] else {
        unreachable!()
    };
    p.children
        .splice(slots, nodes.into_iter().map(Content::Element));
}

fn site(body: &Node, anchor: &InlineAnchor) -> Result<Site> {
    let (InlineLocation::Body(b), [i]) = (&anchor.location, anchor.path.as_slice()) else {
        return Err(unsupported("revision target must be a direct body inline"));
    };
    let elements = |c: &Content| matches!(c, Content::Element(_));
    let (p, block) = body
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| elements(c) && node(c).name != "w:sectPr")
        .nth(*b)
        .filter(|(_, c)| node(c).name == "w:p")
        .ok_or_else(|| unsupported("missing body paragraph"))?;
    let (slot, _) = node(block)
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| elements(c) && node(c).name != "w:pPr")
        .nth(*i)
        .ok_or_else(|| unsupported("missing inline"))?;
    Ok((p, slot))
}

fn guard_boundary(body: &Node, (p, slot): Site) -> Result<()> {
    let mut state = BoundaryState::default();
    for c in body.children[..p]
        .iter()
        .chain(&node(&body.children[p]).children[..slot])
    {
        if let Content::Element(n) = c {
            state.advance(n);
        }
    }
    if state.safe() {
        Ok(())
    } else {
        Err(unsupported(
            "revision intersects a protected range or field",
        ))
    }
}

fn simple_text(run: &Node, tag: &str) -> Result<String> {
    let invalid =
        || unsupported("revision requires a single local text run with supported formatting");
    if run.name != "w:r"
        || run.elements().last().is_none_or(|n| n.name != tag)
        || run
            .elements()
            .filter(|n| n.name != "w:rPr")
            .map(|n| n.name.as_str())
            .ne([tag])
    {
        return Err(invalid());
    }
    let mut text = String::new();
    for c in &run.child(tag).unwrap().children {
        let Content::Text(s) = c else {
            return Err(invalid());
        };
        text.push_str(s);
    }
    if !super::text::disposable_run(run, tag, &text) {
        return Err(invalid());
    }
    Ok(text)
}

fn metadata(kind: RevisionKind, author: &str, date: Option<&str>) -> Result<TextRevision> {
    crate::xml::validate_text(author)?;
    let rev = TextRevision {
        kind,
        author: author.into(),
        date: date.map(Into::into),
    };
    rev.validate()?;
    Ok(rev)
}

impl ImportedDocument {
    /// Propose a direct bold property on a plain or bold-only body text run.
    pub fn preview_tracked_bold(
        &self,
        target: &InlineAnchor,
        bold: bool,
        author: &str,
        date: Option<&str>,
    ) -> Result<EditPreview> {
        self.validate_snapshot(&target.snapshot)?;
        let rev = metadata(RevisionKind::Insertion, author, date)?;
        self.stage_revision(|body| {
            let s = site(body, target)?;
            guard_boundary(body, s)?;
            let mut run = at(body, s).clone();
            simple_text(&run, "w:t")?;
            let prior = run
                .child("w:rPr")
                .cloned()
                .unwrap_or_else(|| Node::new("w:rPr"));
            if bold_value(&prior)? == Some(bold) {
                return Ok(false);
            }
            let change = check_references(&self.package, None, &[])?.property_change(&rev)?;
            super::text::patch_toggle(&mut run, RunToggle::Bold, Some(bold))?;
            run.child_mut("w:rPr").unwrap().push(change.with(prior));
            splice(body, s.0, s.1..s.1 + 1, vec![run]);
            Ok(true)
        })
    }

    fn stage_revision(&self, stage: impl FnOnce(&mut Node) -> Result<bool>) -> Result<EditPreview> {
        self.stage_body(stage)
    }

    /// Insert, replace or delete one direct body text run using explicit revision
    /// markup. Insertions inherit the selected run's local formatting.
    pub fn preview_tracked_text(
        &self,
        target: &InlineAnchor,
        edit: TrackedText,
        author: &str,
        date: Option<&str>,
    ) -> Result<EditPreview> {
        self.validate_snapshot(&target.snapshot)?;
        let mut rev = metadata(RevisionKind::Deletion, author, date)?;
        self.stage_revision(|body| {
            let s = site(body, target)?;
            guard_boundary(body, s)?;
            let run = at(body, s);
            let before = simple_text(run, "w:t")?;
            let mut ids = check_references(&self.package, None, &[])?;
            let mut nodes = vec![];
            let inserting = matches!(edit, TrackedText::InsertAfter(_));
            if !inserting {
                let mut deleted = run.clone();
                deleted.child_mut("w:t").unwrap().name = "w:delText".into();
                nodes.extend(ids.wrap(&self.package, deleted, &rev)?);
            }
            if let TrackedText::Replace(text) | TrackedText::InsertAfter(text) = edit {
                crate::xml::validate_text(&text)?;
                if text.is_empty() || text.contains(['\r', '\n', '\t']) {
                    return Err(unsupported(
                        "tracked text must be nonempty single-line text",
                    ));
                }
                if !inserting && text == before {
                    return Ok(false);
                }
                let mut added = run.clone();
                let t = added.child_mut("w:t").unwrap();
                if text.starts_with(' ') || text.ends_with(' ') {
                    t.set("xml:space", "preserve");
                }
                t.children = vec![Content::Text(text)];
                rev.kind = RevisionKind::Insertion;
                nodes.extend(ids.wrap(&self.package, added, &rev)?);
            }
            splice(body, s.0, s.1 + usize::from(inserting)..s.1 + 1, nodes);
            Ok(true)
        })
    }

    /// Track a move of a direct body run to immediately after another run.
    /// Both legs are generated together from the same source formatting.
    pub fn preview_tracked_move(
        &self,
        target: &InlineAnchor,
        after: &InlineAnchor,
        name: &str,
        author: &str,
        date: &str,
    ) -> Result<EditPreview> {
        self.validate_snapshot(&target.snapshot)?;
        self.validate_snapshot(&after.snapshot)?;
        let name = BookmarkName::new(name)?;
        let mut rev = metadata(RevisionKind::MoveSource(name.clone()), author, Some(date))?;
        self.stage_revision(|body| {
            let s = site(body, target)?;
            let d = site(body, after)?;
            if s == d {
                return Err(unsupported("move needs two distinct targets"));
            }
            guard_boundary(body, s)?;
            guard_boundary(body, d)?;
            if simple_text(at(body, s), "w:t")?.is_empty() {
                return Err(unsupported("empty move source"));
            }
            simple_text(at(body, d), "w:t")?;
            let mut ids = check_references(&self.package, Some((name.as_str(), 0)), &[])?;
            let from = ids.wrap(&self.package, at(body, s).clone(), &rev)?;
            rev.kind = RevisionKind::MoveDestination(name);
            let to = ids.wrap(&self.package, at(body, s).clone(), &rev)?;
            if s < d {
                splice(body, d.0, d.1 + 1..d.1 + 1, to);
                splice(body, s.0, s.1..s.1 + 1, from);
            } else {
                splice(body, s.0, s.1..s.1 + 1, from);
                splice(body, d.0, d.1 + 1..d.1 + 1, to);
            }
            Ok(true)
        })
    }

    /// Select one insertion/deletion, or both wrappers of a paired move.
    pub fn preview_revision(
        &self,
        target: &InlineAnchor,
        action: TextRevisionAction,
        paired: Option<&InlineAnchor>,
    ) -> Result<EditPreview> {
        self.validate_snapshot(&target.snapshot)?;
        if let Some(other) = paired {
            self.validate_snapshot(&other.snapshot)?;
        }
        self.stage_revision(|body| {
            let s = site(body, target)?;
            let accept = action == TextRevisionAction::Accept;
            if let Some(other) = paired {
                let d = site(body, other)?;
                let a = move_leg(body, s)?;
                let b = move_leg(body, d)?;
                if a.source == b.source
                    || a.name != b.name
                    || a.author != b.author
                    || a.date != b.date
                    || a.text != b.text
                {
                    return Err(unsupported(
                        "move legs must have matching name, text, author and date",
                    ));
                }
                check_references(
                    &self.package,
                    Some((&a.name, 2)),
                    &[(a.range, 2), (a.id, 1), (b.range, 2), (b.id, 1)],
                )?;
                let mut legs = [a, b];
                legs.sort_by_key(|leg| std::cmp::Reverse(leg.start));
                for leg in legs {
                    let keep = leg.source != accept;
                    splice(
                        body,
                        leg.start.0,
                        leg.start.1..leg.end + 1,
                        keep.then_some(leg.run).into_iter().collect(),
                    );
                }
            } else {
                guard_boundary(body, s)?;
                let wrapper = at(body, s);
                if wrapper.name == "w:r" {
                    let mut run = wrapper.clone();
                    let props = run
                        .child_mut("w:rPr")
                        .ok_or_else(|| unsupported("missing run property change"))?;
                    let change = props
                        .child("w:rPrChange")
                        .cloned()
                        .ok_or_else(|| unsupported("missing run property change"))?;
                    if props.elements().filter(|n| n.name == "w:rPrChange").count() != 1
                        || props.elements().last().unwrap().name != "w:rPrChange"
                        || !allowed(&change, &["w:id", "w:author", "w:date"])
                        || change.elements().count() != 1
                    {
                        return Err(unsupported("unsupported run property change"));
                    }
                    metadata(
                        RevisionKind::Insertion,
                        change.get("w:author").unwrap_or(""),
                        change.get("w:date"),
                    )?;
                    check_references(&self.package, None, &[(id(&change)?, 1)])?;
                    let prior = change
                        .child("w:rPr")
                        .ok_or_else(|| unsupported("missing prior run properties"))?;
                    bold_value(prior)?;
                    props.remove("w:rPrChange");
                    bold_value(props)?;
                    if !accept {
                        *props = prior.clone();
                    }
                    simple_text(&run, "w:t")?;
                    splice(body, s.0, s.1..s.1 + 1, vec![run]);
                    return Ok(true);
                }
                let tag = match wrapper.name.as_str() {
                    "w:ins" => "w:t",
                    "w:del" => "w:delText",
                    _ => {
                        return Err(unsupported(
                            "select one insertion/deletion or both move wrappers",
                        ));
                    }
                };
                let mut run = unwrap_run(wrapper, tag)?;
                check_references(&self.package, None, &[(id(wrapper)?, 1)])?;
                run.child_mut(tag).unwrap().name = "w:t".into();
                let keep = (tag == "w:t") == accept;
                splice(
                    body,
                    s.0,
                    s.1..s.1 + 1,
                    keep.then_some(run).into_iter().collect(),
                );
            }
            Ok(true)
        })
    }
}

fn id(n: &Node) -> Result<u32> {
    n.get("w:id")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| unsupported("missing or unsupported revision ID"))
}

fn bold_value(props: &Node) -> Result<Option<bool>> {
    let bad = || unsupported("expected empty or bold-only run properties");
    if !allowed(props, &[]) || props.elements().count() > 1 {
        return Err(bad());
    }
    let Some(b) = props.elements().next() else {
        return Ok(None);
    };
    if b.name != "w:b" || !allowed(b, &["w:val"]) || b.elements().next().is_some() {
        return Err(bad());
    }
    match b.get("w:val") {
        None | Some("true" | "1" | "on") => Ok(Some(true)),
        Some("false" | "0" | "off") => Ok(Some(false)),
        _ => Err(bad()),
    }
}

fn allowed(n: &Node, attrs: &[&str]) -> bool {
    n.attrs
        .iter()
        .all(|(k, _)| k.declaration().is_some() || attrs.contains(&k.as_str()))
        && n.children.iter().all(|c| {
            matches!(c, Content::Element(_)) || matches!(c,Content::Text(s) if s.trim().is_empty())
        })
}

fn unwrap_run(wrapper: &Node, tag: &str) -> Result<Node> {
    if !allowed(wrapper, &["w:id", "w:author", "w:date"]) || wrapper.elements().count() != 1 {
        return Err(unsupported(
            "revision wrapper has unsupported metadata or content",
        ));
    }
    metadata(
        RevisionKind::Insertion,
        wrapper.get("w:author").unwrap_or(""),
        wrapper.get("w:date"),
    )?;
    let run = wrapper.elements().next().unwrap();
    simple_text(run, tag)?;
    Ok(run.clone())
}

struct Leg {
    source: bool,
    start: Site,
    end: usize,
    name: String,
    author: String,
    date: String,
    range: u32,
    id: u32,
    text: String,
    run: Node,
}

fn move_leg(body: &Node, s: Site) -> Result<Leg> {
    let wrapper = at(body, s);
    let bad = || unsupported("unsupported or inconsistent move range");
    let (source, start_tag, end_tag) = match wrapper.name.as_str() {
        "w:moveFrom" => (true, "w:moveFromRangeStart", "w:moveFromRangeEnd"),
        "w:moveTo" => (false, "w:moveToRangeStart", "w:moveToRangeEnd"),
        _ => return Err(bad()),
    };
    let children = &paragraph(body, s).children;
    let significant = |c: &Content| !matches!(c, Content::Text(ws) if ws.trim().is_empty());
    let start = children[..s.1]
        .iter()
        .rposition(significant)
        .ok_or_else(bad)?;
    let end = s.1
        + 1
        + children[s.1 + 1..]
            .iter()
            .position(significant)
            .ok_or_else(bad)?;
    let (Content::Element(a), Content::Element(b)) = (&children[start], &children[end]) else {
        return Err(bad());
    };
    if a.name != start_tag
        || b.name != end_tag
        || id(a)? != id(b)?
        || a.elements().next().is_some()
        || b.elements().next().is_some()
        || !allowed(a, &["w:id", "w:name", "w:author", "w:date"])
        || !allowed(b, &["w:id"])
    {
        return Err(bad());
    }
    guard_boundary(body, (s.0, start))?;
    let author = a.get("w:author").ok_or_else(bad)?;
    let date = a.get("w:date").ok_or_else(bad)?;
    if wrapper.get("w:author") != Some(author) || wrapper.get("w:date") != Some(date) {
        return Err(bad());
    }
    let tag = if source
        && wrapper
            .child("w:r")
            .and_then(|r| r.child("w:delText"))
            .is_some()
    {
        "w:delText"
    } else {
        "w:t"
    };
    let mut run = unwrap_run(wrapper, tag)?;
    let text = simple_text(&run, tag)?;
    if text.is_empty() || author.trim().is_empty() || date.trim().is_empty() {
        return Err(bad());
    }
    run.child_mut(tag).unwrap().name = "w:t".into();
    let name = a
        .get("w:name")
        .filter(|s| !s.is_empty())
        .ok_or_else(bad)?
        .into();
    Ok(Leg {
        source,
        start: (s.0, start),
        end,
        name,
        author: author.into(),
        date: date.into(),
        range: id(a)?,
        id: id(wrapper)?,
        text,
        run,
    })
}

/// Reject ambiguous IDs, reused names, and name references. Complex field
/// instructions can be split across runs, so do not guess at their targets.
fn check_references(
    package: &Package,
    name: Option<(&str, usize)>,
    ids: &[(u32, usize)],
) -> Result<RevisionIds> {
    let mut counts = BTreeMap::new();
    let mut names = 0;
    let mut reference = false;
    let mut move_names = BTreeSet::new();
    package.visit_source_xml(|n| {
        if let Some(id) = n.get("w:id").and_then(|s| s.trim().parse::<u32>().ok()) {
            *counts.entry(id).or_insert(0) += 1;
        }
        if matches!(
            n.name.as_str(),
            "w:moveFromRangeStart" | "w:moveToRangeStart"
        ) && let Some(name) = n.get("w:name")
        {
            move_names.insert(name.to_owned());
        }
        if let Some((name, _)) = name {
            if n.get("w:name")
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
            {
                names += 1;
            }
            reference |= n
                .get("w:anchor")
                .is_some_and(|a| a.eq_ignore_ascii_case(name))
                || n.get("w:docLocation")
                    .is_some_and(|a| a.eq_ignore_ascii_case(name))
                || n.get("Target")
                    .and_then(|a| a.split_once('#'))
                    .is_some_and(|(_, fragment)| fragment.eq_ignore_ascii_case(name))
                || (n.name == "w:fldSimple"
                    && !n
                        .get("w:instr")
                        .is_some_and(|s| s.trim().eq_ignore_ascii_case("PAGE")))
                || matches!(n.name.as_str(), "w:instrText" | "w:delInstrText");
        }
    })?;
    if reference
        || name.is_some_and(|(_, expected)| names != expected)
        || ids.iter().map(|(id, _)| id).collect::<BTreeSet<_>>().len() != ids.len()
        || ids
            .iter()
            .any(|(id, expected)| counts.get(id) != Some(expected))
    {
        return Err(unsupported(
            "ambiguous revision ID, move name or field reference",
        ));
    }
    Ok(RevisionIds::reserved(
        counts.into_keys().collect(),
        move_names,
    ))
}
