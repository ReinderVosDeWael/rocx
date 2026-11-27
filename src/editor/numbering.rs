//! Physical numbering identities in the relationship-selected part.
use super::{
    ImportedDocument, ParagraphAnchor, ResourceEdit,
    resource_fields::{Part, id, scalar, unique, unsigned_decimal},
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
mod links;
mod view;
mod wire;
pub(super) use links::validate_start_links;
pub use view::{
    ImportedConcreteNumbering, ImportedNumberingAbstract, ImportedNumberingCatalog,
    ImportedNumberingLevel,
};

#[derive(Clone, Debug)]
pub enum NumberingEdit {
    /// Replace a concrete link and its complete set of direct overrides.
    ReplaceInstance(Box<crate::NumberingDefinition>),
    /// Replace the physical levels while retaining abstract identity and metadata.
    ReplaceAbstract {
        abstract_id: u32,
        levels: Vec<crate::NumberingLevel>,
    },
    AbstractLevel {
        abstract_id: u32,
        level: u8,
        value: Option<Box<crate::NumberingLevel>>,
    },
    RelinkInstance {
        num_id: u32,
        abstract_id: u32,
    },
    StoryParagraph {
        target: StoryParagraphAnchor,
        value: Option<NumberingReference>,
    },
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
    unsigned_decimal(&scalar(node, true)?).ok_or_else(|| unsupported("invalid numbering value"))
}

fn leaf(name: &str, val: impl ToString) -> Node {
    Node::new(name).attr("w:val", val)
}

fn encode_level(level: &crate::NumberingLevel, styles: Option<&Node>) -> Result<Node> {
    if !super::resource_fields::is_numbering_format(&level.format) {
        return Err(unsupported("unsupported numbering format"));
    }
    crate::xml::validate_text(&level.text)?;
    if level.level > 8 {
        return Err(unsupported("numbering level must be in 0..=8"));
    }
    level.validate_formats()?;
    if let Some(name) = &level.paragraph_style {
        let styles = styles.ok_or_else(|| unsupported("missing paragraph style catalog"))?;
        let matches: Vec<_> = styles
            .elements()
            .filter(|n| {
                n.name == "w:style"
                    && (n.get("w:styleId") == Some(name)
                        || n.child("w:name")
                            .and_then(|n| n.get("w:val"))
                            .is_some_and(|s| s.eq_ignore_ascii_case(name)))
            })
            .collect();
        if matches.len() != 1
            || matches[0].get("w:type") != Some("paragraph")
            || matches[0].get("w:styleId").is_none_or(str::is_empty)
        {
            return Err(unsupported(
                "missing, ambiguous or non-paragraph numbering level style",
            ));
        }
    }
    crate::ooxml::wordprocessing::styles::numbering_level(level, styles)
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

pub(super) fn paragraph_reference(p: &Node) -> Result<Option<NumberingReference>> {
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
        if let Some(super::BodyBlock::Paragraph(p)) = self.body.get(target.body_index)
            && let Ok(reference) = &p.direct_numbering
        {
            return Ok(reference.clone());
        }
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
    let mut new_abstracts = BTreeSet::new();
    let mut new_instances = BTreeSet::new();
    for edit in edits {
        match edit {
            ResourceEdit::Numbering(NumberingEdit::AddAbstract { abstract_id, .. }) => {
                new_abstracts.insert(*abstract_id);
            }
            ResourceEdit::Numbering(NumberingEdit::AddInstance { num_id, .. }) => {
                new_instances.insert(*num_id);
            }
            _ => {}
        }
        if let ResourceEdit::Numbering(NumberingEdit::Paragraph { target, value }) = edit {
            doc.validate_snapshot(&target.snapshot)?;
            if targets.insert(target.body_index, value).is_some() {
                return Err(unsupported("duplicate paragraph numbering edit"));
            }
        }
    }
    // An unallocated identity may already have a dangling consumer. Do not
    // silently activate that consumer when creating a definition or part.
    if !new_abstracts.is_empty() || !new_instances.is_empty() {
        let mut claimed = false;
        doc.package.visit_source_xml(|n| {
            let ids = match n.name.as_str() {
                "w:abstractNumId" => &new_abstracts,
                "w:numId" => &new_instances,
                _ => return,
            };
            if !ids.is_empty()
                && n.get("w:val")
                    .and_then(unsigned_decimal)
                    .is_none_or(|v| ids.contains(&v))
            {
                claimed = true;
            }
        })?;
        if claimed {
            return Err(unsupported(
                "numbering identity already referenced in source",
            ));
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
        changed |= patch_paragraph(p, v)?;
    }
    Ok(changed)
}

fn patch_paragraph(p: &mut Node, v: &Option<NumberingReference>) -> Result<bool> {
    if p.name != "w:p" {
        return Err(unsupported("not a paragraph"));
    }
    if &paragraph_reference(p)? == v {
        return Ok(false);
    }
    super::revisions::guard_paragraph_mark(p)?;
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
    Ok(true)
}

type CellPath = (usize, usize, usize, usize);
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum StorySite {
    Part(String, Option<usize>, usize),
    Cell(CellPath),
}
#[derive(Clone, Debug)]
pub struct StoryParagraphAnchor {
    snapshot: std::sync::Arc<()>,
    site: StorySite,
}
impl ImportedDocument {
    /// Select a direct paragraph in one physical header/footer or ordinary entry.
    pub fn story_paragraph_anchor(
        &self,
        owner_part: &str,
        block_index: usize,
        entry_id: Option<&str>,
    ) -> Result<StoryParagraphAnchor> {
        let entry = if let Some(raw) = entry_id {
            let id = crate::ooxml::decimal_id(raw)
                .ok_or_else(|| unsupported("invalid story entry ID"))?;
            let mut matches = self.entry_stories()?.iter().filter(|e| {
                e.part_name == owner_part && crate::ooxml::decimal_id(&e.id).as_ref() == Some(&id)
            });
            let e = matches
                .next()
                .ok_or_else(|| unsupported("missing story entry"))?;
            if matches.next().is_some()
                || e.kind != super::EntryKind::Comment
                    && (id.starts_with('-')
                        || !matches!(e.note_type.as_deref(), None | Some("normal")))
            {
                return Err(unsupported("ambiguous or reserved story entry"));
            }
            Some(e.element_index)
        } else {
            if !self.stories()?.iter().any(|s| s.part_name == owner_part) {
                return Err(unsupported("missing effective header/footer owner"));
            }
            None
        };
        let mut root = self.package.word(owner_part)?;
        story_paragraph(&mut root, entry, block_index)?;
        Ok(StoryParagraphAnchor {
            snapshot: self.snapshot.clone(),
            site: StorySite::Part(owner_part.to_owned(), entry, block_index),
        })
    }
    pub fn cell_paragraph_anchor(
        &self,
        target: &super::TableAnchor,
        row_index: usize,
        cell_index: usize,
        block_index: usize,
    ) -> Result<StoryParagraphAnchor> {
        self.validate_snapshot(&target._snapshot)?;
        if target.physical.is_some() {
            return Err(unsupported(
                "cell numbering requires a direct body table anchor",
            ));
        }
        let site = (target._body_index, row_index, cell_index, block_index);
        let mut root = self.package.word(&self.main)?;
        cell_paragraph(
            root.child_mut("w:body").ok_or_else(super::missing_body)?,
            site,
        )?;
        Ok(StoryParagraphAnchor {
            snapshot: self.snapshot.clone(),
            site: StorySite::Cell(site),
        })
    }
    pub fn story_paragraph_numbering(
        &self,
        target: &StoryParagraphAnchor,
    ) -> Result<Option<NumberingReference>> {
        self.validate_snapshot(&target.snapshot)?;
        match &target.site {
            StorySite::Part(owner, entry, block) => paragraph_reference(story_paragraph(
                &mut self.package.word(owner)?,
                *entry,
                *block,
            )?),
            StorySite::Cell(site) => paragraph_reference(cell_paragraph(
                self.package
                    .word(&self.main)?
                    .child_mut("w:body")
                    .ok_or_else(super::missing_body)?,
                *site,
            )?),
        }
    }
    pub(super) fn stage_story_numbering(
        &self,
        pkg: &mut Package,
        edits: &[ResourceEdit],
    ) -> Result<()> {
        let mut seen = BTreeSet::new();
        let mut groups = BTreeMap::<String, Vec<_>>::new();
        for r in edits {
            if let ResourceEdit::Numbering(NumberingEdit::StoryParagraph { target, value }) = r {
                self.validate_snapshot(&target.snapshot)?;
                let StorySite::Part(owner, entry, block) = &target.site else {
                    continue;
                };
                if !seen.insert(target.site.clone()) {
                    return Err(unsupported(
                        "duplicate or mismatched physical story paragraph",
                    ));
                }
                groups
                    .entry(owner.clone())
                    .or_default()
                    .push((*entry, *block, value));
            }
        }
        for (owner, sites) in groups {
            let mut root = pkg.word(&owner)?;
            let mut changed = false;
            for (entry, block, value) in sites {
                changed |= patch_paragraph(story_paragraph(&mut root, entry, block)?, value)?;
            }
            if changed {
                pkg.put_word(owner, root)?;
            }
        }
        Ok(())
    }
}

pub(super) type CellTargets<'a> = BTreeMap<CellPath, &'a Option<NumberingReference>>;
pub(super) fn cell_targets<'a>(
    doc: &ImportedDocument,
    resources: &'a [ResourceEdit],
    structural: &[super::TableStructureEdit],
) -> Result<CellTargets<'a>> {
    let mut targets = BTreeMap::new();
    for r in resources {
        if let ResourceEdit::Numbering(NumberingEdit::StoryParagraph { target, value }) = r
            && let StorySite::Cell(site) = target.site
        {
            doc.validate_snapshot(&target.snapshot)?;
            if structural.iter().any(|e| e.target._body_index == site.0)
                || targets.insert(site, value).is_some()
            {
                return Err(unsupported(
                    "conflicting cell paragraph or table structure edit",
                ));
            }
        }
    }
    Ok(targets)
}
pub(super) fn patch_cells(body: &mut Node, targets: &CellTargets<'_>) -> Result<bool> {
    let mut changed = false;
    for (&site, value) in targets {
        changed |= patch_paragraph(cell_paragraph(body, site)?, value)?;
    }
    Ok(changed)
}
fn cell_paragraph(body: &mut Node, (table, row, cell, block): CellPath) -> Result<&mut Node> {
    fn select<'a>(
        parent: &'a mut Node,
        skip: &[&str],
        tag: &str,
        i: usize,
        state: &mut super::BoundaryState,
    ) -> Result<&'a mut Node> {
        let mut index = 0;
        for n in parent.elements_mut() {
            if !skip.contains(&n.name.as_str()) {
                if index == i {
                    if n.name != tag {
                        return Err(unsupported("cell paragraph target is opaque"));
                    }
                    return Ok(n);
                }
                index += 1;
            }
            state.advance(n);
        }
        Err(unsupported("missing cell paragraph"))
    }
    let mut state = super::BoundaryState::default();
    let t = select(body, &["w:sectPr"], "w:tbl", table, &mut state)?;
    super::tables::validate(t, false)?;
    let r = select(t, &["w:tblPr", "w:tblGrid"], "w:tr", row, &mut state)?;
    if r.child("w:trPr")
        .is_some_and(|n| n.child("w:ins").is_some() || n.child("w:del").is_some())
    {
        return Err(unsupported("tracked row paragraph"));
    }
    let c = select(r, &["w:trPr"], "w:tc", cell, &mut state)?;
    if c.child("w:tcPr")
        .and_then(|n| n.child("w:vMerge"))
        .is_some_and(|n| n.get("w:val") != Some("restart"))
    {
        return Err(unsupported("vertical merge continuation paragraph"));
    }
    let p = select(c, &["w:tcPr"], "w:p", block, &mut state)?;
    let before = state.safe();
    state.advance(p);
    if !before || !state.safe() {
        return Err(unsupported("protected cell paragraph"));
    }
    Ok(p)
}

fn story_paragraph(root: &mut Node, entry: Option<usize>, block: usize) -> Result<&mut Node> {
    let container = if let Some(i) = entry {
        root.elements_mut()
            .nth(i)
            .ok_or_else(|| unsupported("missing story entry"))?
    } else {
        root
    };
    let mut boundary = super::BoundaryState::default();
    for (i, n) in container.elements_mut().enumerate() {
        let safe = boundary.safe();
        boundary.advance(n);
        if i == block {
            if !safe || !boundary.safe() || n.name != "w:p" {
                return Err(unsupported("protected or missing story paragraph"));
            }
            return Ok(n);
        }
    }
    Err(unsupported("missing story paragraph"))
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
                "w:pStyle",
                "w:isLgl",
                "w:suff",
                "w:lvlJc",
                "w:pPr",
                "w:rPr",
            ],
        ),
        "w:num" => (&["w:numId"], &["w:abstractNumId", "w:lvlOverride"]),
        "w:lvlOverride" => (&["w:ilvl"], &["w:startOverride", "w:lvl"]),
        "w:pPr" => (
            &[],
            &[
                "w:bidi",
                "w:keepNext",
                "w:keepLines",
                "w:pageBreakBefore",
                "w:widowControl",
                "w:suppressLineNumbers",
                "w:contextualSpacing",
                "w:mirrorIndents",
                "w:snapToGrid",
                "w:spacing",
                "w:ind",
                "w:tabs",
                "w:jc",
            ],
        ),
        "w:rPr" => (
            &[],
            &[
                "w:rFonts",
                "w:b",
                "w:i",
                "w:bCs",
                "w:iCs",
                "w:rtl",
                "w:cs",
                "w:caps",
                "w:smallCaps",
                "w:snapToGrid",
                "w:strike",
                "w:color",
                "w:sz",
                "w:szCs",
                "w:lang",
                "w:u",
                "w:vertAlign",
            ],
        ),
        "w:rFonts" => (&["w:ascii", "w:hAnsi", "w:eastAsia", "w:cs"], &[]),
        "w:color" => (
            &["w:val", "w:themeColor", "w:themeTint", "w:themeShade"],
            &[],
        ),
        "w:lang" => (&["w:val", "w:eastAsia", "w:bidi"], &[]),
        "w:spacing" => (&["w:before", "w:after", "w:line", "w:lineRule"], &[]),
        "w:ind" => (&["w:left", "w:right", "w:firstLine", "w:hanging"], &[]),
        "w:tabs" => (&[], &["w:tab"]),
        "w:tab" => (&["w:val", "w:pos", "w:leader"], &[]),
        "w:bidi"
        | "w:keepNext"
        | "w:keepLines"
        | "w:pageBreakBefore"
        | "w:widowControl"
        | "w:suppressLineNumbers"
        | "w:contextualSpacing"
        | "w:mirrorIndents"
        | "w:snapToGrid"
        | "w:jc"
        | "w:b"
        | "w:i"
        | "w:bCs"
        | "w:iCs"
        | "w:rtl"
        | "w:cs"
        | "w:caps"
        | "w:smallCaps"
        | "w:strike"
        | "w:sz"
        | "w:szCs"
        | "w:u"
        | "w:vertAlign" => (&["w:val"], &[]),
        "w:abstractNumId" | "w:multiLevelType" | "w:start" | "w:numFmt" | "w:lvlText"
        | "w:pStyle" | "w:startOverride" | "w:lvlRestart" | "w:isLgl" | "w:suff" | "w:lvlJc" => {
            (&["w:val"], &[])
        }
        _ => return false,
    };
    let mut singleton = BTreeSet::new();
    simple(n, attrs, children)
        && n.elements().all(|c| {
            matches!(c.name.as_str(), "w:lvl" | "w:lvlOverride" | "w:tab")
                || singleton.insert(c.name.as_str())
        })
        && n.elements().all(discardable)
}

// Apply the same interpretation as readback: reapplying modeled values must
// not normalize numeric lexemes, namespace declarations or property order.
fn same_level(a: &Node, b: &Node) -> bool {
    matches!((view::read_level(a), view::read_level(b)), (Ok(a), Ok(b))
        if !a.has_opaque_content && !b.has_opaque_content && a == b)
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
    fn modeled(n: &Node) -> Result<bool> {
        if !discardable(n) {
            return Ok(false);
        }
        if n.name == "w:lvl" {
            return Ok(!view::read_level(n)?.has_opaque_content);
        }
        for child in n.elements() {
            if !modeled(child)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    let Content::Element(node) = &root.children[i] else {
        unreachable!()
    };
    if !modeled(node)? {
        return Err(unsupported("cannot discard rich numbering definition"));
    }
    root.children.remove(i);
    Ok(())
}

pub(super) struct RemovedIdentities {
    path: String,
    nums: BTreeSet<u32>,
    defs: BTreeSet<u32>,
}

impl RemovedIdentities {
    /// Check the completed package, including references from body payloads
    /// and reciprocal links written by numbering-style additions.
    pub(super) fn check(&self, pkg: &Package) -> Result<()> {
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
        let check = |n: &Node| referenced(n, &self.nums, &self.defs);
        if check(&pkg.word(&self.path)?) {
            return Err(unsupported(
                "numbering has live or uninspectable references",
            ));
        }
        check_other_parts(pkg, &self.path, check)
    }
}

type Levels = BTreeMap<u32, BTreeSet<u32>>;
fn editable_abstract<'a>(
    root: &'a mut Node,
    aid: u32,
    source: &Levels,
    removed: &BTreeSet<u32>,
) -> Result<&'a mut Node> {
    if removed.contains(&aid) || !source.contains_key(&aid) {
        return Err(unsupported("unknown or conflicting abstract identity"));
    }
    let n = named(root, "w:abstractNum", "w:abstractNumId", aid).unwrap();
    if n.child("w:numStyleLink").is_some() {
        return Err(unsupported(
            "inherited abstract has no direct editable level",
        ));
    }
    Ok(n)
}

// Rebuild the affected address sets before checking consumers, so a batch can
// remove an unused level and move its old instances together.
#[allow(clippy::too_many_arguments)]
fn edit_identities(
    edits: &[&NumberingEdit],
    resources: &[ResourceEdit],
    root: &mut Node,
    defs: &mut Levels,
    nums: &mut BTreeMap<u32, u32>,
    source_defs: &Levels,
    source_nums: &BTreeMap<u32, u32>,
    removed_defs: &BTreeSet<u32>,
    removed_nums: &BTreeSet<u32>,
    style_nodes: Option<&Node>,
    links: &links::StyleLinks,
) -> Result<bool> {
    use NumberingEdit::*;
    let mut whole = BTreeSet::new();
    let mut touched = BTreeSet::new();
    let mut relinked = BTreeSet::new();
    let mut changed = false;
    for e in edits {
        match *e {
            ReplaceAbstract {
                abstract_id: a,
                levels,
            } => {
                if levels.is_empty()
                    || !whole.insert(*a)
                    || touched.iter().any(|(id, _)| id == a)
                    || resources
                        .iter()
                        .any(|r| matches!(r,ResourceEdit::AbstractStart(s) if s.abstract_id==*a))
                {
                    return Err(unsupported("empty or conflicting abstract replacement"));
                }
                links.provider(root, source_nums, *a)?;
                let def = editable_abstract(root, *a, source_defs, removed_defs)?;
                let mut new = BTreeMap::new();
                for l in levels {
                    if new
                        .insert(u32::from(l.level), encode_level(l, style_nodes)?)
                        .is_some()
                    {
                        return Err(unsupported("duplicate abstract replacement level"));
                    }
                }
                defs.insert(*a, new.keys().copied().collect());
                for l in def.elements().filter(|n| n.name == "w:lvl") {
                    if view::read_level(l)?.has_opaque_content {
                        return Err(unsupported(
                            "cannot discard unmodeled abstract level markup",
                        ));
                    }
                }
                // Keep comments, extension nodes, names/IDs and other siblings
                // in their original slots for every surviving level.
                def.children.retain_mut(|c| {
                    if let Content::Element(n) = c
                        && n.name == "w:lvl"
                    {
                        if let Some(v) = new.remove(&id(n, "w:ilvl").unwrap()) {
                            if !same_level(n, &v) {
                                *n = v;
                                changed = true;
                            }
                            true
                        } else {
                            changed = true;
                            false
                        }
                    } else {
                        true
                    }
                });
                for n in new.into_values() {
                    def.push(n);
                    changed = true;
                }
            }
            AbstractLevel {
                abstract_id: a,
                level,
                value,
            } => {
                if *level>8 || whole.contains(a)||!touched.insert((*a,*level))||resources.iter().any(|r|matches!(r,ResourceEdit::AbstractStart(s) if (s.abstract_id,s.level)==(*a,*level))){return Err(unsupported("invalid or conflicting abstract level edit"))}
                links.provider(root, source_nums, *a)?;
                let def = editable_abstract(root, *a, source_defs, removed_defs)?;
                let indices = defs.get_mut(a).unwrap();
                let old = named(def, "w:lvl", "w:ilvl", (*level).into());
                if let Some(old) = old.as_deref()
                    && view::read_level(old)?.has_opaque_content
                {
                    return Err(unsupported(
                        "cannot discard unmodeled abstract level markup",
                    ));
                }
                if let Some(l) = value {
                    if l.level != *level {
                        return Err(unsupported("mismatched abstract level"));
                    }
                    let new = encode_level(l, style_nodes)?;
                    if let Some(old) = old {
                        if !same_level(old, &new) {
                            *old = new;
                            changed = true;
                        }
                    } else {
                        def.push(new);
                        changed = true;
                    }
                    indices.insert((*level).into());
                } else if old.is_some() {
                    remove(def, "w:lvl", "w:ilvl", (*level).into())?;
                    indices.remove(&u32::from(*level));
                    changed = true;
                }
            }
            RelinkInstance {
                num_id: i,
                abstract_id: a,
            } => {
                if !source_nums.contains_key(i)
                    || !relinked.insert(*i)
                    || removed_nums.contains(i)
                    || removed_defs.contains(a)
                    || !defs.contains_key(a)
                {
                    return Err(unsupported("missing or conflicting instance relink"));
                }
                if nums[i] != *a {
                    if links.owns(*i) {
                        return Err(unsupported(
                            "cannot move a numbering style's owning instance",
                        ));
                    }
                    nums.insert(*i, *a);
                    named(root, "w:num", "w:numId", *i)
                        .unwrap()
                        .child_mut("w:abstractNumId")
                        .unwrap()
                        .set("w:val", a);
                    changed = true;
                }
            }
            _ => {}
        }
    }
    for (&num, &old_a) in source_nums {
        if removed_nums.contains(&num) {
            continue;
        }
        let new_a = nums[&num];
        if old_a != new_a || whole.contains(&new_a) || touched.iter().any(|(id, _)| *id == new_a) {
            let old_provider = links.provider(root, source_nums, old_a)?;
            let new_provider = links.provider(root, nums, new_a)?;
            let old = source_defs
                .get(&old_provider)
                .ok_or_else(|| unsupported("unresolved source abstract"))?;
            let new = &defs[&new_provider];
            if old.is_empty() || !old.is_subset(new) {
                return Err(unsupported(
                    "would lose numbering levels of a surviving instance",
                ));
            }
            if named(root, "w:num", "w:numId", num)
                .unwrap()
                .elements()
                .filter(|o| o.name == "w:lvlOverride")
                .any(|o| !new.contains(&id(o, "w:ilvl").unwrap()))
            {
                return Err(unsupported("instance override would lose its level"));
            }
        }
    }
    Ok(changed)
}

fn concrete_replacements(edits: &[&NumberingEdit], root: &Node) -> Result<Vec<NumberingEdit>> {
    use NumberingEdit::*;
    let mut replacements = Vec::new();
    let mut targets = BTreeSet::new();
    let instances = instances(root)?;
    for e in edits {
        let ReplaceInstance(d) = e else { continue };
        let id = d.id;
        let Some(a) = d.abstract_id else {
            return Err(unsupported("concrete replacement needs an abstract ID"));
        };
        if !d.levels.is_empty() || d.inherit_style.is_some() {
            return Err(unsupported(
                "concrete replacement cannot redefine abstract levels or styles",
            ));
        }
        if !targets.insert(id)||edits.iter().any(|e|matches!(e, RelinkInstance{num_id,..}|RemoveInstance{num_id}|AddInstance{num_id,..}|InstanceStart{num_id,..}|InstanceLevel{num_id,..} if *num_id==id)){
            return Err(unsupported("conflicting concrete replacement"));
        }
        let instance = instances
            .iter()
            .find(|n| n.num_id == id)
            .ok_or_else(|| unsupported("missing concrete instance"))?;
        replacements.push(RelinkInstance {
            num_id: id,
            abstract_id: a,
        });
        let mut starts: BTreeMap<_, _> = d
            .start_overrides
            .iter()
            .map(|s| (s.level, Some(s.start)))
            .collect();
        let mut levels: BTreeMap<_, _> = d
            .level_overrides
            .iter()
            .map(|l| (l.level, Some(Box::new(l.clone()))))
            .collect();
        if starts.len() != d.start_overrides.len() || levels.len() != d.level_overrides.len() {
            return Err(unsupported("duplicate concrete replacement override"));
        }
        for o in &instance.start_overrides {
            starts.entry(o.level).or_insert(None);
        }
        for o in &instance.replacement_levels {
            levels.entry(*o).or_insert(None);
        }
        replacements.extend(starts.into_iter().map(|(level, start)| InstanceStart {
            num_id: id,
            level,
            start,
        }));
        replacements.extend(levels.into_iter().map(|(level, value)| InstanceLevel {
            num_id: id,
            level,
            value,
        }));
    }
    Ok(replacements)
}

pub(super) fn stage(
    pkg: &mut Package,
    main: &str,
    resources: &[ResourceEdit],
) -> Result<Option<RemovedIdentities>> {
    use NumberingEdit::*;
    let mut edits: Vec<_> = resources
        .iter()
        .filter_map(|r| {
            if let ResourceEdit::Numbering(e) = r {
                Some(e)
            } else {
                None
            }
        })
        .collect();
    if edits.iter().all(|e| {
        matches!(
            e,
            Paragraph { value: None, .. } | StoryParagraph { value: None, .. }
        )
    }) {
        return Ok(None);
    }
    let (path, mut root) = match Part::Numbers.load(pkg, main)? {
        Some(part) => part,
        None if edits.iter().any(|e| matches!(e, AddAbstract { .. })) => {
            super::resource_fields::create_word_part(pkg, main, "numbering")?
        }
        None => return Err(unsupported("missing numbering part")),
    };
    let expanded = if edits.iter().any(|e| matches!(e, ReplaceInstance(_))) {
        concrete_replacements(&edits, &root)?
    } else {
        Vec::new()
    };
    edits.extend(expanded.iter());
    let style_nodes = super::resources::style_part(pkg, main)?.map(|(_, root)| root);
    let links = links::StyleLinks::from_styles(style_nodes.as_ref())?;
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
    let source_defs = defs.clone();
    let source_nums = nums.clone();
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
                    def.push(encode_level(l, style_nodes.as_ref())?);
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
    changed |= edit_identities(
        &edits,
        resources,
        &mut root,
        &mut defs,
        &mut nums,
        &source_defs,
        &source_nums,
        &removed_defs,
        &removed_nums,
        style_nodes.as_ref(),
        &links,
    )?;
    let providers = edits
        .iter()
        .filter_map(|e| match e {
            Paragraph { value: Some(r), .. } | StoryParagraph { value: Some(r), .. } => Some(r.id),
            InstanceStart { num_id, .. }
            | InstanceLevel { num_id, .. }
            | AddInstance { num_id, .. } => Some(*num_id),
            _ => None,
        })
        .filter_map(|n| nums.get(&n).copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|aid| Ok((aid, links.provider(&root, &nums, aid)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let valid_level = |num_id: u32, level: u8| {
        if !removed_nums.contains(&num_id)
            && nums.get(&num_id).is_some_and(|a| {
                providers.get(a).is_some_and(|provider| {
                    !removed_defs.contains(a)
                        && !removed_defs.contains(provider)
                        && defs
                            .get(provider)
                            .is_some_and(|s| s.contains(&u32::from(level)))
                })
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
            }
            | StoryParagraph {
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
                let new = value
                    .as_deref()
                    .map(|v| encode_level(v, style_nodes.as_ref()))
                    .transpose()?;
                if let Some(o) = named(num, "w:lvlOverride", "w:ilvl", level.into()) {
                    if let Some(existing) = unique(o, "w:lvl")? {
                        if id(existing, "w:ilvl")? != u32::from(level)
                            || view::read_level(existing)?.has_opaque_content
                        {
                            return Err(unsupported(
                                "cannot discard rich or mismatched replacement level",
                            ));
                        }
                        if new.as_ref().is_some_and(|n| same_level(existing, n)) {
                            continue;
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
        pkg.put_word(&path, root)?;
        return Ok(Some(RemovedIdentities {
            path,
            nums: removed_nums,
            defs: removed_defs,
        }));
    }
    if changed {
        pkg.put_word(path, root)?;
    }
    Ok(None)
}
