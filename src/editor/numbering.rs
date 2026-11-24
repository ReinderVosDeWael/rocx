//! Physical numbering identities in the relationship-selected part.
use super::{
    ImportedDocument, ParagraphAnchor, ResourceEdit,
    resource_fields::{Part, id, scalar, unique},
    style_edits::{check_other_parts, opaque_reference_context},
    unsupported,
};
use crate::{
    NumberingReference, NumberingStartOverride, Result,
    ooxml::WordParts,
    opc::Package,
    xml::{Content, Node},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub enum NumberingEdit {
    AddAbstract {
        abstract_id: u32,
        levels: Vec<crate::NumberingLevel>,
    },
    RemoveAbstract {
        abstract_id: u32,
    },
    AddInstance {
        num_id: u32,
        abstract_id: u32,
    },
    RemoveInstance {
        num_id: u32,
    },
    InstanceStart {
        num_id: u32,
        level: u8,
        start: Option<u32>,
    },
    InstanceLevel {
        num_id: u32,
        level: u8,
        value: Option<Box<crate::NumberingLevel>>,
    },
    Paragraph {
        target: ParagraphAnchor,
        value: Option<NumberingReference>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedNumberingInstance {
    pub num_id: u32,
    pub abstract_id: u32,
    pub start_overrides: Vec<NumberingStartOverride>,
    /// Physical levels containing a full replacement, independent of their start overrides.
    pub replacement_levels: Vec<u8>,
}

fn value(node: &Node) -> Result<u32> {
    scalar(node, true)?
        .parse()
        .map_err(|_| unsupported("invalid numbering value"))
}

fn leaf(name: &str, val: impl ToString) -> Node {
    Node::new(name).attr("w:val", val)
}

fn encode_level(level: &crate::NumberingLevel) -> Result<Node> {
    if level.paragraph_style.is_some()
        || !matches!(
            level.format.as_str(),
            "decimal"
                | "upperRoman"
                | "lowerRoman"
                | "upperLetter"
                | "lowerLetter"
                | "decimalZero"
                | "bullet"
                | "none"
        )
    {
        return Err(unsupported(
            "unsupported numbering format or linked paragraph style",
        ));
    }
    if level.level > 8 {
        return Err(unsupported("numbering level must be in 0..=8"));
    }
    level.validate_formats()?;
    crate::ooxml::wordprocessing::styles::numbering_level(level, None)
}

fn instances(root: &Node) -> Result<Vec<ImportedNumberingInstance>> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    if root.child(super::ALTERNATE).is_some() {
        return Err(unsupported("ambiguous numbering instances"));
    }
    for n in root.elements().filter(|n| n.name == "w:num") {
        let num_id = id(n, "w:numId")?;
        if num_id == 0 || !seen.insert(num_id) {
            return Err(unsupported("invalid or duplicate numbering instance"));
        }
        let abstract_id = value(
            unique(n, "w:abstractNumId")?
                .ok_or_else(|| unsupported("missing abstract reference"))?,
        )?;
        let mut levels = BTreeSet::new();
        let mut start_overrides = Vec::new();
        let mut replacement_levels = Vec::new();
        for o in n.elements().filter(|n| n.name == "w:lvlOverride") {
            let level = id(o, "w:ilvl")?;
            if level > 8 || !levels.insert(level) {
                return Err(unsupported("invalid or duplicate override level"));
            }
            if let Some(s) = unique(o, "w:startOverride")? {
                start_overrides.push(NumberingStartOverride {
                    level: level as u8,
                    start: value(s)?,
                });
            }
            if let Some(l) = unique(o, "w:lvl")? {
                if id(l, "w:ilvl")? != level {
                    return Err(unsupported("mismatched replacement level"));
                }
                replacement_levels.push(level as u8);
            }
        }
        out.push(ImportedNumberingInstance {
            num_id,
            abstract_id,
            start_overrides,
            replacement_levels,
        });
    }
    Ok(out)
}

fn simple(n: &Node, attrs: &[&str], children: &[&str]) -> bool {
    n.attrs
        .iter()
        .all(|(k, _)| k.declaration().is_some() || attrs.contains(&k.as_str()))
        && n.children.iter().all(|c| match c {
            Content::Element(n) => children.contains(&n.name.as_str()),
            Content::Text(s) => s.trim().is_empty(),
            Content::Raw(_) => false,
        })
}

fn paragraph_reference(p: &Node) -> Result<Option<NumberingReference>> {
    let Some(props) = unique(p, "w:pPr")? else {
        return Ok(None);
    };
    let Some(n) = unique(props, "w:numPr")? else {
        return Ok(None);
    };
    if !simple(n, &[], &["w:ilvl", "w:numId"]) {
        return Err(unsupported("annotated paragraph numbering reference"));
    }
    let direct = |key| {
        unique(n, key)?
            .ok_or_else(|| unsupported("incomplete direct numbering reference"))
            .and_then(value)
    };
    let level = direct("w:ilvl")?;
    if level > 8 {
        return Err(unsupported("invalid paragraph numbering level"));
    }
    Ok(Some(NumberingReference {
        id: direct("w:numId")?,
        level: level as u8,
    }))
}

impl ImportedDocument {
    pub fn numbering_instances(&self) -> Result<Vec<ImportedNumberingInstance>> {
        Part::Numbers
            .load(&self.package, &self.main)?
            .map_or_else(|| Ok(Vec::new()), |(_, root)| instances(&root))
    }
    pub fn paragraph_numbering(
        &self,
        target: &ParagraphAnchor,
    ) -> Result<Option<NumberingReference>> {
        self.validate_snapshot(&target.snapshot)?;
        let root = self.package.word(&self.main)?;
        let p = root
            .child("w:body")
            .and_then(|b| {
                b.elements()
                    .filter(|n| n.name != "w:sectPr")
                    .nth(target.body_index)
            })
            .filter(|n| n.name == "w:p")
            .ok_or_else(|| unsupported("unknown paragraph"))?;
        paragraph_reference(p)
    }
}

type Targets<'a> = BTreeMap<usize, &'a Option<NumberingReference>>;
pub(super) fn paragraph_targets<'a>(
    doc: &ImportedDocument,
    edits: &'a [ResourceEdit],
) -> Result<Targets<'a>> {
    let mut targets = BTreeMap::new();
    for edit in edits {
        if let ResourceEdit::Numbering(NumberingEdit::Paragraph { target, value }) = edit {
            doc.validate_snapshot(&target.snapshot)?;
            if targets.insert(target.body_index, value).is_some() {
                return Err(unsupported("duplicate paragraph numbering edit"));
            }
        }
    }
    Ok(targets)
}

pub(super) fn patch_paragraphs(body: &mut Node, targets: &Targets<'_>) -> Result<bool> {
    if targets.is_empty() {
        return Ok(false);
    }
    let mut changed = false;
    for (i, p) in body
        .elements_mut()
        .filter(|n| n.name != "w:sectPr")
        .enumerate()
    {
        let Some(&v) = targets.get(&i) else { continue };
        if p.name != "w:p" {
            return Err(unsupported("not a paragraph"));
        }
        if &paragraph_reference(p)? == v {
            continue;
        }
        changed = true;
        if p.child("w:pPr").is_none() {
            p.children.insert(0, Content::Element(Node::new("w:pPr")));
        }
        let props = p.child_mut("w:pPr").unwrap();
        if let Some(r) = v {
            let num = props.ensure("w:numPr");
            num.ensure("w:ilvl").set("w:val", r.level);
            num.ensure("w:numId").set("w:val", r.id);
        } else {
            props.remove("w:numPr");
        }
    }
    Ok(changed)
}

fn named<'a>(n: &'a mut Node, tag: &str, attr: &str, key: u32) -> Option<&'a mut Node> {
    n.elements_mut()
        .find(|c| c.name == tag && id(c, attr).ok() == Some(key))
}

fn discardable(n: &Node) -> bool {
    let (attrs, children): (&[&str], &[&str]) = match n.name.as_str() {
        "w:abstractNum" => (&["w:abstractNumId"], &["w:lvl", "w:multiLevelType"]),
        "w:lvl" => (
            &["w:ilvl"],
            &[
                "w:start",
                "w:numFmt",
                "w:lvlText",
                "w:lvlRestart",
                "w:isLgl",
                "w:suff",
                "w:lvlJc",
            ],
        ),
        "w:num" => (&["w:numId"], &["w:abstractNumId", "w:lvlOverride"]),
        "w:lvlOverride" => (&["w:ilvl"], &["w:startOverride", "w:lvl"]),
        "w:abstractNumId" | "w:multiLevelType" | "w:start" | "w:numFmt" | "w:lvlText"
        | "w:startOverride" | "w:lvlRestart" | "w:isLgl" | "w:suff" | "w:lvlJc" => {
            (&["w:val"], &[])
        }
        _ => return false,
    };
    simple(n, attrs, children) && n.elements().all(discardable)
}

fn prune_empty_override(num: &mut Node, level: u8) {
    num.children.retain(|c| !matches!(c,Content::Element(n) if n.name=="w:lvlOverride" && id(n,"w:ilvl").ok()==Some(level.into())
        && simple(n,&["w:ilvl"],&[]) && n.attrs.iter().all(|(k,_)| k=="w:ilvl")));
}

fn remove(root: &mut Node, tag: &str, attr: &str, key: u32) -> Result<()> {
    let i = root
        .children
        .iter()
        .position(
            |c| matches!(c,Content::Element(n) if n.name == tag && id(n,attr).ok() == Some(key)),
        )
        .ok_or_else(|| unsupported("unknown numbering identity"))?;
    if !matches!(&root.children[i], Content::Element(n) if discardable(n)) {
        return Err(unsupported("cannot discard rich numbering definition"));
    }
    root.children.remove(i);
    Ok(())
}

pub(super) fn apply(pkg: &mut Package, main: &str, resources: &[ResourceEdit]) -> Result<()> {
    use NumberingEdit::*;
    let edits: Vec<_> = resources
        .iter()
        .filter_map(|r| {
            if let ResourceEdit::Numbering(e) = r {
                Some(e)
            } else {
                None
            }
        })
        .collect();
    if edits
        .iter()
        .all(|e| matches!(e, Paragraph { value: None, .. }))
    {
        return Ok(());
    }
    let (path, mut root) = Part::Numbers
        .load(pkg, main)?
        .ok_or_else(|| unsupported("missing numbering part"))?;
    let mut defs = BTreeMap::<u32, BTreeSet<u32>>::new();
    for n in root.elements().filter(|n| n.name == "w:abstractNum") {
        let aid = id(n, "w:abstractNumId")?;
        let mut levels = BTreeSet::new();
        for l in n.elements().filter(|l| l.name == "w:lvl") {
            let level = id(l, "w:ilvl")?;
            if level > 8 || !levels.insert(level) {
                return Err(unsupported("ambiguous abstract level"));
            }
        }
        if n.child(super::ALTERNATE).is_some() || defs.insert(aid, levels).is_some() {
            return Err(unsupported("ambiguous abstract definition"));
        }
    }
    let mut nums: BTreeMap<_, _> = instances(&root)?
        .into_iter()
        .map(|n| (n.num_id, n.abstract_id))
        .collect();
    let mut changed = false;
    let mut removed_defs = BTreeSet::new();
    let mut removed_nums = BTreeSet::new();
    let mut override_keys = BTreeSet::new();
    for e in &edits {
        match **e {
            AddAbstract {
                abstract_id,
                ref levels,
            } => {
                let ids: BTreeSet<_> = levels.iter().map(|l| u32::from(l.level)).collect();
                if levels.is_empty() || ids.len() != levels.len() {
                    return Err(unsupported("empty or duplicate numbering levels"));
                }
                if defs.insert(abstract_id, ids).is_some() {
                    return Err(unsupported("duplicate abstract ID"));
                }
                if resources.iter().any(
                    |r| matches!(r, ResourceEdit::AbstractStart(s) if s.abstract_id==abstract_id),
                ) {
                    return Err(unsupported("abstract creation and start patch conflict"));
                }
                let mut def = Node::new("w:abstractNum").attr("w:abstractNumId", abstract_id);
                for l in levels {
                    def.push(encode_level(l)?);
                }
                root.push(def);
                changed = true;
            }
            AddInstance {
                num_id,
                abstract_id,
            } => {
                if num_id == 0 || nums.insert(num_id, abstract_id).is_some() {
                    return Err(unsupported("invalid or duplicate instance ID"));
                }
                root.push(
                    Node::new("w:num")
                        .attr("w:numId", num_id)
                        .with(leaf("w:abstractNumId", abstract_id)),
                );
                changed = true;
            }
            RemoveAbstract { abstract_id } => {
                if !removed_defs.insert(abstract_id) {
                    return Err(unsupported("duplicate abstract removal"));
                }
            }
            RemoveInstance { num_id } if !removed_nums.insert(num_id) => {
                return Err(unsupported("duplicate instance removal"));
            }
            _ => {}
        }
    }
    let valid_level = |num_id: u32, level: u8| {
        if !removed_nums.contains(&num_id)
            && nums.get(&num_id).is_some_and(|a| {
                !removed_defs.contains(a)
                    && defs.get(a).is_some_and(|s| s.contains(&u32::from(level)))
            })
        {
            Ok(())
        } else {
            Err(unsupported("missing numbering instance or abstract level"))
        }
    };
    for e in edits {
        match *e {
            AddAbstract { abstract_id, .. } if removed_defs.contains(&abstract_id) => {
                return Err(unsupported("conflicting abstract edits"));
            }
            AddInstance {
                num_id,
                abstract_id,
            } if !defs.contains_key(&abstract_id)
                || removed_defs.contains(&abstract_id)
                || removed_nums.contains(&num_id) =>
            {
                return Err(unsupported("conflicting or missing numbering identity"));
            }
            Paragraph {
                value: Some(ref r), ..
            } => valid_level(r.id, r.level)?,
            InstanceStart {
                num_id,
                level,
                start,
            } => {
                valid_level(num_id, level)?;
                if !override_keys.insert((num_id, level, "start")) {
                    return Err(unsupported("duplicate instance start edit"));
                }
                let num = named(&mut root, "w:num", "w:numId", num_id).unwrap();
                let Some(o) = named(num, "w:lvlOverride", "w:ilvl", level.into()) else {
                    if let Some(s) = start {
                        num.push(
                            Node::new("w:lvlOverride")
                                .attr("w:ilvl", level)
                                .with(leaf("w:startOverride", s)),
                        );
                        changed = true;
                    }
                    continue;
                };
                if o.child("w:startOverride").map(value).transpose()? == start {
                    continue;
                }
                changed = true;
                if let Some(s) = start {
                    o.ensure("w:startOverride").set("w:val", s);
                } else {
                    o.remove("w:startOverride");
                    prune_empty_override(num, level);
                }
            }
            InstanceLevel {
                num_id,
                level,
                ref value,
            } => {
                valid_level(num_id, level)?;
                if !override_keys.insert((num_id, level, "level"))
                    || value.as_ref().is_some_and(|v| v.level != level)
                {
                    return Err(unsupported("duplicate or mismatched replacement level"));
                }
                let num = named(&mut root, "w:num", "w:numId", num_id).unwrap();
                let new = value.as_deref().map(encode_level).transpose()?;
                if let Some(o) = named(num, "w:lvlOverride", "w:ilvl", level.into()) {
                    if let Some(existing) = unique(o, "w:lvl")? {
                        if id(existing, "w:ilvl")? != u32::from(level) || !discardable(existing) {
                            return Err(unsupported(
                                "cannot discard rich or mismatched replacement level",
                            ));
                        }
                    } else if new.is_none() {
                        continue;
                    }
                    if let Some(n) = new {
                        o.replace(n);
                    } else {
                        o.remove("w:lvl");
                    }
                    prune_empty_override(num, level);
                } else if let Some(n) = new {
                    num.push(Node::new("w:lvlOverride").attr("w:ilvl", level).with(n));
                } else {
                    continue;
                }
                changed = true;
            }
            _ => {}
        }
    }
    if !removed_nums.is_empty() || !removed_defs.is_empty() {
        for &i in &removed_nums {
            remove(&mut root, "w:num", "w:numId", i)?
        }
        for &a in &removed_defs {
            remove(&mut root, "w:abstractNum", "w:abstractNumId", a)?
        }
        fn referenced(n: &Node, nums: &BTreeSet<u32>, defs: &BTreeSet<u32>) -> bool {
            opaque_reference_context(n)
                || n.name == "w:numberingChange"
                || [(nums, "w:numId"), (defs, "w:abstractNumId")]
                    .into_iter()
                    .any(|(ids, tag)| {
                        !ids.is_empty()
                            && n.name == tag
                            && value(n).map_or(true, |id| ids.contains(&id))
                    })
                || n.elements().any(|c| referenced(c, nums, defs))
        }
        let check = |n: &Node| referenced(n, &removed_nums, &removed_defs);
        if check(&root) {
            return Err(unsupported(
                "numbering has live or uninspectable references",
            ));
        }
        check_other_parts(pkg, &path, check)?;
        changed = true;
    }
    if changed {
        pkg.put_word(path, root)?;
    }
    Ok(())
}
