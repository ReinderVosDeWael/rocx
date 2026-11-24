//! Direct imported resource properties. No style cascade or numbering expansion.
use super::{ALTERNATE, ImportedDocument, read_alignment, unsupported};
use crate::ooxml::WordParts;
use crate::opc::Package;
use crate::xml::{Content, Node};
use crate::{Alignment, Result};
use std::collections::{BTreeMap, BTreeSet};
mod defaults;
pub(super) use defaults::is_numbering_format;
pub use defaults::{DefaultField, DefaultTab, DefaultValue};
mod metadata;
pub use metadata::{CustomValue, MetadataField};

/// A physical abstract numbering level (shared by all referencing instances).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbstractStart {
    pub abstract_id: u32,
    pub level: u8,
    /// Absent means there is no direct `w:start` element.
    pub start: Option<u32>,
}

/// Each supplied edit sets or clears one property. Omit the edit to retain it.
#[derive(Clone, Debug)]
pub enum ResourceEdit {
    Numbering(super::NumberingEdit),
    CoreTitle(Option<String>),
    CoreSubject(Option<String>),
    DefaultAlignment(Option<Alignment>),
    Default(DefaultField, Option<DefaultValue>),
    AbstractStart(AbstractStart),
    Metadata(MetadataField, Option<String>),
    CustomProperty(String, Option<CustomValue>),
}

pub(super) enum Part {
    Defaults,
    Settings,
    Numbers,
}
impl Part {
    pub(super) fn load(&self, pkg: &Package, main: &str) -> Result<Option<(String, Node)>> {
        let (owner, rel, mime, root) = match self {
            Self::Defaults => (
                main,
                "styles",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml",
                "w:styles",
            ),
            Self::Numbers => (
                main,
                "numbering",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml",
                "w:numbering",
            ),
            Self::Settings => (
                main,
                "settings",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml",
                "w:settings",
            ),
        };
        if pkg.relationships(owner, rel)?.len() > 1 {
            return Err(unsupported(&format!("ambiguous {rel} relationship")));
        }
        let Some((_, path)) = pkg.related(owner, rel)? else {
            return Ok(None);
        };
        if !pkg
            .content_type_for(&path)?
            .is_some_and(|ty| ty.eq_ignore_ascii_case(mime))
        {
            return Err(unsupported(&format!("invalid {rel} content type")));
        }
        let node = pkg.word(&path)?;
        if node.name != root {
            return Err(unsupported(&format!("invalid {rel} root")));
        }
        Ok(Some((path, node)))
    }
}

/// Allocate a relationship-selected Word part without claiming orphan parts.
fn free_resource_path(pkg: &Package, stem: &str, unnumbered: bool) -> Result<String> {
    let manifest = pkg.xml("[Content_Types].xml")?;
    let reserved: BTreeSet<_> = pkg
        .parts
        .keys()
        .map(|p| p.to_ascii_lowercase())
        .chain(
            manifest
                .elements()
                .filter_map(|n| n.get("PartName"))
                .map(|p| p.trim_start_matches('/').to_ascii_lowercase()),
        )
        .collect();
    for i in usize::from(!unnumbered).. {
        let path = if i == 0 {
            format!("{stem}.xml")
        } else {
            format!("{stem}{i}.xml")
        };
        let (dir, name) = path
            .rsplit_once('/')
            .map_or(("", path.as_str()), |(d, n)| (&path[..d.len() + 1], n));
        if !reserved.contains(&path.to_ascii_lowercase())
            && !reserved.contains(&format!("{dir}_rels/{name}.rels").to_ascii_lowercase())
        {
            return Ok(path);
        }
    }
    unreachable!()
}

pub(super) fn create_word_part(
    pkg: &mut Package,
    main: &str,
    kind: &str,
) -> Result<(String, Node)> {
    let prefix = main.rsplit_once('/').map_or_else(
        || format!("rocx-{kind}"),
        |(dir, _)| format!("{dir}/rocx-{kind}"),
    );
    let path = free_resource_path(pkg, &prefix, false)?;
    pkg.content_type(
        &path,
        &format!("application/vnd.openxmlformats-officedocument.wordprocessingml.{kind}+xml"),
    )?;
    pkg.link(main, kind, &path)?;
    let mut root = Node::new(format!("w:{kind}"));
    root.namespaces();
    Ok((path, root))
}

pub(super) fn unique<'a>(parent: &'a Node, name: &str) -> Result<Option<&'a Node>> {
    let mut matches = parent.elements().filter(|n| n.name == name);
    let first = matches.next();
    if matches.next().is_some() || parent.child(ALTERNATE).is_some() {
        return Err(unsupported(&format!("ambiguous {name}")));
    }
    Ok(first)
}

pub(super) fn scalar(node: &Node, attr: bool) -> Result<String> {
    if node
        .attrs
        .iter()
        .any(|(k, _)| k.declaration().is_none() && !(attr && k == "w:val"))
    {
        return Err(unsupported("annotated resource property"));
    }
    if attr {
        if node.children.is_empty() {
            return node
                .get("w:val")
                .map(str::to_owned)
                .ok_or_else(|| unsupported("missing w:val"));
        }
    } else {
        return text_value(node);
    }
    Err(unsupported("non-simple resource property"))
}

fn text_value(node: &Node) -> Result<String> {
    let mut value = String::new();
    for child in &node.children {
        let Content::Text(t) = child else {
            return Err(unsupported("non-text resource property"));
        };
        value.push_str(t);
    }
    Ok(value)
}

const DEFAULT_PATH: &[&str] = &["w:docDefaults", "w:pPrDefault", "w:pPr", "w:jc"];

fn property(root: &Node, path: &[&str], attr: bool) -> Result<Option<String>> {
    let mut parent = root;
    for name in path {
        let Some(n) = unique(parent, name)? else {
            return Ok(None);
        };
        parent = n;
    }
    scalar(parent, attr).map(Some)
}

fn patch(root: &mut Node, path: &[&str], attr: bool, value: Option<String>) -> Result<bool> {
    let before = property(root, path, attr)?;
    let normalized = match path.last().copied() {
        Some("w:start") => before
            .map(|s| parse_start(&s).map(|n| n.to_string()))
            .transpose()?,
        Some("w:jc") => before
            .map(|s| parse_alignment(&s).map(|a| a.xml().to_owned()))
            .transpose()?,
        _ => before,
    };
    if normalized == value {
        return Ok(false);
    }
    let (name, parents) = path.split_last().unwrap();
    let parent = parents.iter().fold(root, |n, key| n.ensure(key));
    if let Some(s) = value {
        let leaf = parent.ensure(name);
        if attr {
            leaf.set("w:val", s);
        } else {
            leaf.children = vec![Content::Text(s)];
        }
    } else {
        parent.remove(name);
    }
    Ok(true)
}

fn parse_start(s: &str) -> Result<u32> {
    unsigned_decimal(s).ok_or_else(|| unsupported("invalid numbering start"))
}
pub(super) fn unsigned_decimal(s: &str) -> Option<u32> {
    crate::ooxml::decimal_id(s)?.parse().ok()
}
fn parse_alignment(s: &str) -> Result<Alignment> {
    read_alignment(s).ok_or_else(|| unsupported("unsupported default alignment"))
}

pub(super) fn id(n: &Node, key: &str) -> Result<u32> {
    n.get(key)
        .and_then(unsigned_decimal)
        .ok_or_else(|| unsupported(&format!("invalid {key}")))
}

fn visit_levels(
    root: &mut Node,
    mut visit: impl FnMut(u32, u8, &mut Node) -> Result<()>,
) -> Result<()> {
    let mut ids = BTreeSet::new();
    if root.child(ALTERNATE).is_some() {
        return Err(unsupported("ambiguous numbering definitions"));
    }
    for def in root.elements_mut().filter(|n| n.name == "w:abstractNum") {
        let abstract_id = id(def, "w:abstractNumId")?;
        if !ids.insert(abstract_id) || def.child(ALTERNATE).is_some() {
            return Err(unsupported("ambiguous abstract numbering definition"));
        }
        let mut levels = BTreeSet::new();
        for lvl in def.elements_mut().filter(|n| n.name == "w:lvl") {
            let level = id(lvl, "w:ilvl")?;
            if level > 8 || !levels.insert(level) {
                return Err(unsupported("invalid or duplicate abstract numbering level"));
            }
            visit(abstract_id, level as u8, lvl)?;
        }
    }
    Ok(())
}

impl ImportedDocument {
    pub fn core_title(&self) -> Result<Option<String>> {
        self.metadata_field(MetadataField::Title)
    }
    pub fn core_subject(&self) -> Result<Option<String>> {
        self.metadata_field(MetadataField::Subject)
    }
    pub fn default_alignment(&self) -> Result<Option<Alignment>> {
        let Some((_, root)) = Part::Defaults.load(&self.package, &self.main)? else {
            return Ok(None);
        };
        property(&root, DEFAULT_PATH, true)?
            .map(|s| parse_alignment(&s))
            .transpose()
    }
    pub fn abstract_starts(&self) -> Result<Vec<AbstractStart>> {
        let mut result = Vec::new();
        if let Some((_, mut root)) = Part::Numbers.load(&self.package, &self.main)? {
            visit_levels(&mut root, |abstract_id, level, n| {
                let start = property(n, &["w:start"], true)?
                    .map(|v| parse_start(&v))
                    .transpose()?;
                result.push(AbstractStart {
                    abstract_id,
                    level,
                    start,
                });
                Ok(())
            })?;
        }
        Ok(result)
    }
}

pub(super) fn apply(pkg: &mut Package, main: &str, edits: &[ResourceEdit]) -> Result<()> {
    defaults::apply(pkg, main, edits)?;
    metadata::apply(pkg, edits)?;
    let (mut alignment, mut starts) = (None, BTreeMap::new());
    for edit in edits {
        let duplicate = match edit {
            ResourceEdit::Default(_, _) => continue,
            ResourceEdit::Numbering(_) => continue,
            ResourceEdit::CoreTitle(_) | ResourceEdit::CoreSubject(_) => continue,
            ResourceEdit::DefaultAlignment(v) => alignment.replace(v).is_some(),
            ResourceEdit::Metadata(..) | ResourceEdit::CustomProperty(..) => continue,
            ResourceEdit::AbstractStart(v) => {
                starts.insert((v.abstract_id, v.level), v.start).is_some()
            }
        };
        if duplicate {
            return Err(unsupported("duplicate resource property edit"));
        }
    }
    if let Some(value) = alignment
        && let Some((name, mut root)) = Part::Defaults.load(pkg, main)?
        && patch(
            &mut root,
            DEFAULT_PATH,
            true,
            value.map(|a| a.xml().to_owned()),
        )?
    {
        pkg.put_word(name, root)?;
    }
    if !starts.is_empty() {
        let (path, mut root) = Part::Numbers
            .load(pkg, main)?
            .ok_or_else(|| unsupported("missing numbering part"))?;
        super::numbering::validate_start_links(pkg, main, &root, starts.keys().map(|(id, _)| *id))?;
        let mut changed = false;
        visit_levels(&mut root, |a, l, n| {
            if let Some(start) = starts.remove(&(a, l)) {
                changed |= patch(n, &["w:start"], true, start.map(|s| s.to_string()))?;
            }
            Ok(())
        })?;
        if !starts.is_empty() {
            return Err(unsupported("unknown abstract numbering level"));
        }
        if changed {
            pkg.put_word(path, root)?;
        }
    }
    Ok(())
}
