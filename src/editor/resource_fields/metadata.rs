//! Relationship-selected, direct metadata on an imported package.
use super::{ImportedDocument, ResourceEdit, scalar, unique, unsupported};
use crate::ooxml::WordParts;
use crate::opc::Package;
use crate::xml::{Content, Node, validate_text};
use crate::{Error, Result};
use std::collections::{BTreeMap, BTreeSet};

const EXT: &str = "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties";
const CP: &str = "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties";
const VT: &str = "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes";
const TERMS: &str = "http://purl.org/dc/terms/";
const FMTID: &str = "{D5CDD505-2E9C-101B-9397-08002B2CF9AE}";

macro_rules! fields {
    ($($variant:ident => ($key:literal, $xml:literal)),+ $(,)?) => {
        /// Direct scalar in the package's core or extended properties.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum MetadataField { $($variant),+ }
        impl std::str::FromStr for MetadataField {
            type Err = Error;
            fn from_str(s: &str) -> Result<Self> {
                match s { $($key => Ok(Self::$variant),)+ _ => Err(unsupported("unknown metadata field")) }
            }
        }
        impl MetadataField {
            fn xml(self) -> &'static str { match self { $(Self::$variant => $xml),+ } }
        }
    };
}
fields! {
    Title => ("title", "dc:title"), Subject => ("subject", "dc:subject"),
    Creator => ("creator", "dc:creator"), Description => ("description", "dc:description"),
    Keywords => ("keywords", "cp:keywords"), Category => ("category", "cp:category"),
    Version => ("version", "cp:version"), Created => ("created", "dcterms:created"),
    Modified => ("modified", "dcterms:modified"), LastPrinted => ("lastPrinted", "cp:lastPrinted"),
    LastModifiedBy => ("lastModifiedBy", "cp:lastModifiedBy"), Revision => ("revision", "cp:revision"),
    Identifier => ("identifier", "dc:identifier"), Language => ("language", "dc:language"),
    ContentStatus => ("contentStatus", "cp:contentStatus"),
    Company => ("company", "Company"), Manager => ("manager", "Manager"),
}

/// Supported unlinked custom property values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CustomValue {
    Text(String),
    Boolean(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Part {
    Core,
    Extended,
    Custom,
}
impl Part {
    fn descriptor(self) -> (&'static str, &'static str, String, &'static str) {
        match self {
            Self::Core => (
                "core-properties",
                "application/vnd.openxmlformats-package.core-properties+xml",
                "cp:coreProperties".into(),
                "docProps/core",
            ),
            Self::Extended => (
                "extended-properties",
                "application/vnd.openxmlformats-officedocument.extended-properties+xml",
                format!("{{{EXT}}}Properties"),
                "docProps/app",
            ),
            Self::Custom => (
                "custom-properties",
                "application/vnd.openxmlformats-officedocument.custom-properties+xml",
                format!("{{{CP}}}Properties"),
                "docProps/custom",
            ),
        }
    }
    fn load(self, pkg: &Package) -> Result<Option<(String, Node)>> {
        let (rel, mime, root, _) = self.descriptor();
        if pkg.relationships("", rel)?.len() > 1 {
            return Err(unsupported("ambiguous metadata relationship"));
        }
        let Some((_, path)) = pkg.related("", rel)? else {
            return Ok(None);
        };
        let node = pkg.word(&path)?;
        if node.name.as_str() != root
            || !pkg
                .content_type_for(&path)?
                .is_some_and(|t| t.eq_ignore_ascii_case(mime))
        {
            return Err(unsupported("invalid metadata part"));
        }
        Ok(Some((path, node)))
    }
    fn change(self, pkg: &mut Package, edit: impl FnOnce(&mut Node) -> Result<bool>) -> Result<()> {
        let (rel, mime, root_name, stem) = self.descriptor();
        let loaded = self.load(pkg)?;
        let new = loaded.is_none();
        let (path, mut root) = loaded.unwrap_or_else(|| {
            let path = format!("{stem}.xml");
            let collision = pkg.parts.keys().any(|p| p.eq_ignore_ascii_case(&path));
            (
                if collision {
                    pkg.unique(stem, "xml")
                } else {
                    path
                },
                Node::new(root_name),
            )
        });
        if edit(&mut root)? {
            if new {
                pkg.content_type(&path, mime)?;
                pkg.link("", rel, &path)?;
            }
            pkg.put(path, root)?;
        }
        Ok(())
    }
}

impl MetadataField {
    fn part(self) -> Part {
        if matches!(self, Self::Manager | Self::Company) {
            Part::Extended
        } else {
            Part::Core
        }
    }
    fn name(self) -> String {
        if self.part() == Part::Extended {
            format!("{{{EXT}}}{}", self.xml())
        } else {
            self.xml().into()
        }
    }
    fn typed_date(self) -> bool {
        matches!(self, Self::Created | Self::Modified)
    }
    fn read(self, root: &Node) -> Result<Option<String>> {
        let Some(node) = unique(root, &self.name())? else {
            return Ok(None);
        };
        if !self.typed_date() {
            return scalar(node, false).map(Some);
        }
        // xsi:type is a QName resolved in the leaf's namespace scope.
        let typ = node
            .get("xsi:type")
            .ok_or_else(|| unsupported("missing core date type"))?;
        let (prefix, local) = typ.split_once(':').unwrap_or(("", typ));
        let decl = if prefix.is_empty() {
            "xmlns".into()
        } else {
            format!("xmlns:{prefix}")
        };
        // Imported leaves retain their full source namespace scope.
        if local != "W3CDTF" || node.get(&decl) != Some(TERMS) {
            return Err(unsupported("unknown core date type"));
        }
        if node
            .attrs
            .iter()
            .any(|(k, _)| k.declaration().is_none() && k != "xsi:type")
        {
            return Err(unsupported("annotated date property"));
        }
        super::text_value(node).map(Some)
    }
    fn patch(self, root: &mut Node, value: Option<&String>) -> Result<bool> {
        let before = self.read(root)?;
        if let Some(s) = value {
            validate_text(s)?;
            if self.typed_date() || self == Self::LastPrinted {
                crate::document::validate_utc_date(s, "metadata")?;
            } else if self.part() == Part::Core && s.chars().count() > 255 {
                return Err(unsupported("core property exceeds 255 characters"));
            }
        }
        if before.as_ref() == value {
            return Ok(false);
        }
        let name = self.name();
        if let Some(s) = value {
            let leaf = root.ensure(&name);
            if self.typed_date() && before.is_none() {
                leaf.set("xmlns:dcterms", TERMS);
                leaf.set("xsi:type", "dcterms:W3CDTF");
            }
            leaf.children = vec![Content::Text(s.clone())];
        } else {
            root.remove(&name);
        }
        Ok(true)
    }
}

struct CustomIndex {
    names: BTreeMap<String, usize>,
    ids: BTreeSet<i32>,
}
impl CustomIndex {
    fn new(root: &Node) -> Result<Self> {
        let mut index = Self {
            names: BTreeMap::new(),
            ids: BTreeSet::new(),
        };
        if root.child(crate::editor::ALTERNATE).is_some() {
            return Err(unsupported("ambiguous custom properties"));
        }
        for (i, child) in root.children.iter().enumerate() {
            let Content::Element(n) = child else { continue };
            if n.name.namespace() != Some(CP) || n.name.local() != "property" {
                continue;
            }
            let pid = n
                .get("pid")
                .and_then(|s| s.trim().parse().ok())
                .filter(|&p| p >= 2)
                .ok_or_else(|| unsupported("invalid custom property ID"))?;
            if !index.ids.insert(pid) {
                return Err(unsupported("duplicate custom property ID"));
            }
            if let Some(name) = n.get("name")
                && index.names.insert(name.into(), i).is_some()
            {
                return Err(unsupported("duplicate custom property name"));
            }
        }
        Ok(index)
    }
}
fn read_custom(prop: &Node) -> Result<CustomValue> {
    if !prop
        .get("fmtid")
        .is_some_and(|v| v.eq_ignore_ascii_case(FMTID))
        || prop.attrs.iter().any(|(k, _)| {
            k.declaration().is_none() && !matches!(k.as_str(), "name" | "pid" | "fmtid")
        })
    {
        return Err(unsupported("linked or annotated custom property"));
    }
    let mut elements = prop.elements();
    let val = elements
        .next()
        .ok_or_else(|| unsupported("empty custom property"))?;
    if elements.next().is_some()
        || val.name.namespace() != Some(VT)
        || prop.children.iter().any(|c| match c {
            Content::Text(t) => !t.trim().is_empty(),
            Content::Raw(_) => true,
            Content::Element(_) => false,
        })
    {
        return Err(unsupported("complex custom property"));
    }
    let s = scalar(val, false)?;
    match val.name.local() {
        "lpwstr" => Ok(CustomValue::Text(s)),
        "bool" => match s.trim() {
            "true" | "1" => Ok(CustomValue::Boolean(true)),
            "false" | "0" => Ok(CustomValue::Boolean(false)),
            _ => Err(unsupported("invalid custom boolean")),
        },
        _ => Err(unsupported("unsupported custom property type")),
    }
}
fn validate_name(name: &str) -> Result<()> {
    validate_text(name)?;
    if name.trim().is_empty() {
        return Err(unsupported("blank custom property name"));
    }
    Ok(())
}
impl ImportedDocument {
    /// Read a physical core or extended property without resolving or changing timestamps.
    pub fn metadata_field(&self, field: MetadataField) -> Result<Option<String>> {
        match field.part().load(&self.package)? {
            Some((_, root)) => field.read(&root),
            None => Ok(None),
        }
    }
    /// Read an unlinked string or boolean by exact name; unsupported variants return an error.
    pub fn custom_property(&self, name: &str) -> Result<Option<CustomValue>> {
        validate_name(name)?;
        let Some((_, root)) = Part::Custom.load(&self.package)? else {
            return Ok(None);
        };
        let index = CustomIndex::new(&root)?;
        index
            .names
            .get(name)
            .map(|i| {
                let Content::Element(n) = &root.children[*i] else {
                    unreachable!()
                };
                read_custom(n)
            })
            .transpose()
    }
}

pub(super) fn apply(pkg: &mut Package, edits: &[ResourceEdit]) -> Result<()> {
    let mut fields = BTreeMap::new();
    let mut custom = BTreeMap::new();
    for edit in edits {
        let duplicate = match edit {
            ResourceEdit::CoreTitle(v) => fields
                .entry(Part::Core)
                .or_insert_with(BTreeMap::new)
                .insert(MetadataField::Title, v.as_ref())
                .is_some(),
            ResourceEdit::CoreSubject(v) => fields
                .entry(Part::Core)
                .or_insert_with(BTreeMap::new)
                .insert(MetadataField::Subject, v.as_ref())
                .is_some(),
            ResourceEdit::Metadata(f, v) => fields
                .entry(f.part())
                .or_insert_with(BTreeMap::new)
                .insert(*f, v.as_ref())
                .is_some(),
            ResourceEdit::CustomProperty(k, v) => {
                validate_name(k)?;
                custom.insert(k, v.as_ref()).is_some()
            }
            _ => continue,
        };
        if duplicate {
            return Err(unsupported("duplicate metadata target"));
        }
    }
    for (part, fields) in fields {
        part.change(pkg, |root| {
            let mut changed = false;
            for (f, v) in fields {
                changed |= f.patch(root, v)?;
            }
            Ok(changed)
        })?;
    }
    if custom.is_empty() {
        return Ok(());
    }
    Part::Custom.change(pkg, |root| {
        let mut index = CustomIndex::new(root)?;
        let mut removed = BTreeSet::new();
        let mut free = 2i32;
        let mut changed = false;
        for (name, v) in custom {
            let prop = if let Some(&i) = index.names.get(name) {
                let Content::Element(prop) = &mut root.children[i] else {
                    unreachable!()
                };
                if Some(&read_custom(prop)?) == v {
                    continue;
                }
                changed = true;
                if v.is_none() {
                    removed.insert(i);
                    continue;
                }
                prop
            } else {
                if v.is_none() {
                    continue;
                }
                changed = true;
                while index.ids.contains(&free) {
                    free = free
                        .checked_add(1)
                        .ok_or_else(|| unsupported("custom property IDs exhausted"))?;
                }
                index.ids.insert(free);
                root.children.push(Content::Element(
                    Node::new(format!("{{{CP}}}property"))
                        .attr("name", name)
                        .attr("pid", free)
                        .attr("fmtid", FMTID),
                ));
                let Some(Content::Element(prop)) = root.children.last_mut() else {
                    unreachable!()
                };
                prop
            };
            let (tag, s) = match v.unwrap() {
                CustomValue::Text(s) => ("lpwstr", s.clone()),
                CustomValue::Boolean(v) => ("bool", v.to_string()),
            };
            let leaf = Node::new(format!("{{{VT}}}{tag}")).text(s);
            if let Some(old) = prop.elements_mut().next() {
                *old = leaf;
            } else {
                prop.push(leaf);
            }
        }
        if !removed.is_empty() {
            let mut i = 0;
            root.children.retain(|_| {
                i += 1;
                !removed.contains(&(i - 1))
            });
        }
        Ok(changed)
    })
}
