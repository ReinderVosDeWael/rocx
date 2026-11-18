//! Package parts, relationships, content types, and ZIP I/O.
//!
//! References: ECMA-376 Part 2 (2021), §§6.2 (parts), 6.4 (relative
//! references), 6.5 (relationships), and 7.3 (ZIP mapping).
//! [Official Part 2](https://ecma-international.org/wp-content/uploads/ECMA-376-2_5th_edition_december_2021.zip).
//! Word-specific default seeding is a current implementation concern here;
//! it is not an OPC requirement.
use crate::xml::Node;
mod names;
use crate::{Error, Result};
pub use names::{ContentType, PartName, RelationshipId, RelationshipType, Target};
use names::{relationship_type, relationships_path};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
};
#[derive(Debug)]
pub(crate) struct Package {
    pub parts: BTreeMap<String, Vec<u8>>,
    edited: std::cell::RefCell<BTreeMap<String, Node>>,
}
impl Package {
    pub fn load_with_limits(bytes: &[u8], limits: ReadLimits) -> Result<Self> {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
        if zip.len() > limits.max_parts {
            return Err(crate::Diagnostic::new(
                crate::Rule::InvalidPackage,
                "package part count exceeds read limit",
            )
            .at("package")
            .into());
        }
        let mut total = 0_u64;
        let mut identities = std::collections::BTreeSet::new();
        let mut parts = BTreeMap::new();
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            if file.is_dir() {
                continue;
            }
            let part = PartName::new(file.name())?;
            if !identities.insert(part.as_str().to_ascii_lowercase()) {
                return Err(Error::Validation(
                    "duplicate equivalent OPC part name".into(),
                ));
            }
            if file.size() > limits.max_part_bytes
                || total.saturating_add(file.size()) > limits.max_expanded_bytes
            {
                return Err(crate::Diagnostic::new(
                    crate::Rule::InvalidPackage,
                    "package expanded bytes exceed read limit",
                )
                .at(file.name())
                .into());
            }
            let mut bytes = Vec::new();
            (&mut file)
                .take(limits.max_part_bytes.saturating_add(1))
                .read_to_end(&mut bytes)?;
            total += bytes.len() as u64;
            if bytes.len() as u64 > limits.max_part_bytes || total > limits.max_expanded_bytes {
                return Err(crate::Diagnostic::new(
                    crate::Rule::InvalidPackage,
                    "package expanded bytes exceed read limit",
                )
                .at(file.name())
                .into());
            }
            if parts.insert(file.name().to_owned(), bytes).is_some() {
                return Err(Error::Validation("duplicate DOCX package part".into()));
            }
        }
        let p = Self {
            parts,
            edited: Default::default(),
        };
        p.validate_graph()?;
        Ok(p)
    }
    pub fn xml(&self, path: &str) -> Result<Node> {
        let path = self
            .parts
            .keys()
            .find(|p| p.eq_ignore_ascii_case(path))
            .map(String::as_str)
            .unwrap_or(path);
        if let Some(node) = self.edited.borrow().get(path) {
            return Ok(node.clone());
        }
        let bytes = self
            .parts
            .get(path)
            .ok_or_else(|| Error::Validation(format!("missing package part {path}")))?;
        let node = Node::parse(bytes)?;
        self.edited.borrow_mut().insert(path.into(), node.clone());
        Ok(node)
    }
    pub fn put(&mut self, path: impl Into<String>, node: Node) -> Result<()> {
        fn validate(n: &Node) -> Result<()> {
            for (_, v) in &n.attrs {
                crate::xml::validate_text(v)?;
            }
            for c in &n.children {
                match c {
                    crate::xml::Content::Element(n) => validate(n)?,
                    crate::xml::Content::Text(t) => crate::xml::validate_text(t)?,
                    _ => (),
                }
            }
            Ok(())
        }
        validate(&node)?;
        let path = path.into();
        PartName::new(&path)?;
        let path = self
            .parts
            .keys()
            .find(|p| p.eq_ignore_ascii_case(&path))
            .cloned()
            .unwrap_or(path);
        self.parts.entry(path.clone()).or_default();
        self.edited.borrow_mut().insert(path, node);
        Ok(())
    }
    pub fn relation(&mut self, owner: &str, kind: &str, target: &str) -> Result<String> {
        let typ = relationship_type(kind);
        self.add_relationship(
            owner,
            &RelationshipType::new(&typ)?,
            Target::Internal(PartName::resolve(owner, target)?),
        )
    }
    pub fn add_relationship(
        &mut self,
        owner: &str,
        kind: &RelationshipType,
        target: Target,
    ) -> Result<String> {
        let (target, external) = match target {
            Target::Internal(part) => (part.relative_to(owner)?, false),
            Target::External(uri) => {
                if uri.is_empty() || uri.chars().any(|c| c.is_control() || c.is_whitespace()) {
                    return Err(Error::Validation(
                        "invalid external relationship target".into(),
                    ));
                }
                (uri, true)
            }
        };
        let relpath = relationships_path(owner);
        let mut rels = if self.parts.keys().any(|p| p.eq_ignore_ascii_case(&relpath)) {
            self.xml(&relpath)?
        } else {
            Node::new("Relationships").attr(
                "xmlns",
                "http://schemas.openxmlformats.org/package/2006/relationships",
            )
        };
        let typ = kind.as_str();
        if let Some(id) = rels
            .elements()
            .find(|n| {
                n.get("Type") == Some(typ)
                    && n.get("Target") == Some(target.as_str())
                    && (n.get("TargetMode") == Some("External")) == external
            })
            .and_then(|n| n.get("Id"))
        {
            return Ok(id.into());
        }
        let mut index = 1;
        while rels
            .elements()
            .any(|n| n.get("Id") == Some(format!("rId{index}").as_str()))
        {
            index += 1;
        }
        let id = format!("rId{index}");
        let mut relationship = Node::new("Relationship")
            .attr("Id", &id)
            .attr("Type", typ)
            .attr("Target", target);
        if external {
            relationship.set("TargetMode", "External");
        }
        rels.push(relationship);
        self.put(relpath, rels)?;
        Ok(id)
    }
    pub fn related(&self, owner: &str, selector: &str) -> Result<Option<(String, String)>> {
        let relpath = relationships_path(owner);
        if !self.parts.keys().any(|p| p.eq_ignore_ascii_case(&relpath)) {
            return Ok(None);
        }
        let rels = self.xml(&relpath)?;
        let found = rels.elements().find(|n| {
            selector
                .strip_prefix('#')
                .is_some_and(|id| n.get("Id") == Some(id))
                || (!selector.starts_with('#')
                    && n.get("Type")
                        .is_some_and(|t| t == relationship_type(selector)))
        });
        let Some(rel) = found else { return Ok(None) };
        if rel.get("TargetMode") == Some("External") {
            return Err(Error::Validation(
                "internal document part cannot have an external relationship".into(),
            ));
        }
        let target = rel
            .get("Target")
            .ok_or_else(|| Error::Validation("relationship has no target".into()))?;
        let target = PartName::resolve(owner, target)?;
        Ok(Some((
            rel.get("Id").unwrap_or("").into(),
            self.parts
                .keys()
                .find(|p| p.eq_ignore_ascii_case(target.zip_name()))
                .cloned()
                .unwrap_or_else(|| target.zip_name().to_owned()),
        )))
    }
    pub fn relationships(&self, owner: &str, kind: &str) -> Result<Vec<(String, String)>> {
        let path = relationships_path(owner);
        if !self.parts.keys().any(|p| p.eq_ignore_ascii_case(&path)) {
            return Ok(Vec::new());
        }
        let relationships = self.xml(&path)?;
        relationships
            .elements()
            .filter(|n| n.get("Type").is_some_and(|t| t == relationship_type(kind)))
            .map(|n| {
                Ok((
                    n.get("Id")
                        .ok_or_else(|| Error::Validation("relationship has no ID".into()))?
                        .into(),
                    n.get("Target").unwrap_or("").into(),
                ))
            })
            .collect()
    }
    pub fn content_type(&mut self, path: &str, mime: &str) -> Result<()> {
        ContentType::new(mime)?;
        PartName::new(path)?;
        let mut types = self.xml("[Content_Types].xml")?;
        let part = format!("/{path}");
        if let Some(n) = types.elements_mut().find(|n| {
            n.get("PartName")
                .is_some_and(|name| name.eq_ignore_ascii_case(&part))
        }) {
            n.set("ContentType", mime);
        } else {
            types.push(
                Node::new("Override")
                    .attr("PartName", part)
                    .attr("ContentType", mime),
            );
        }
        self.put("[Content_Types].xml", types)
    }
    pub fn unique(&self, prefix: &str, extension: &str) -> String {
        let mut i = 1;
        loop {
            let path = format!("{prefix}{i}.{extension}");
            if !self.parts.keys().any(|p| p.eq_ignore_ascii_case(&path)) {
                return path;
            }
            i += 1;
        }
    }
    pub fn link(&mut self, owner: &str, kind: &str, target: &str) -> Result<String> {
        self.relation(owner, kind, &PartName::new(target)?.relative_to(owner)?)
    }
    fn content_types(&self) -> Result<(BTreeMap<String, String>, BTreeMap<String, String>)> {
        let root = self.xml("[Content_Types].xml")?;
        let namespace = "http://schemas.openxmlformats.org/package/2006/content-types";
        if root.name.namespace() != Some(namespace) || root.name.local() != "Types" {
            return Err(Error::from(crate::Diagnostic::new(
                crate::Rule::InvalidPackage,
                "wrong content-types namespace",
            )));
        }
        let mut defaults = BTreeMap::new();
        let mut overrides = BTreeMap::new();
        for child in root.elements() {
            if child.name.namespace() != Some(namespace)
                || !matches!(child.name.local(), "Default" | "Override")
            {
                return Err(Error::Validation(
                    "invalid content-type declaration namespace".into(),
                ));
            }
            let mime = child
                .get("ContentType")
                .ok_or_else(|| Error::Validation("missing content type".into()))?;
            ContentType::new(mime)?;
            if let Some(extension) = child.get("Extension") {
                if extension.is_empty()
                    || extension.contains(['.', '/', '\\'])
                    || defaults
                        .insert(extension.to_ascii_lowercase(), mime.into())
                        .is_some()
                {
                    return Err(Error::Validation(
                        "duplicate or invalid default content type".into(),
                    ));
                }
            } else if let Some(name) = child.get("PartName") {
                let name = PartName::new(name)?;
                if overrides
                    .insert(name.zip_name().to_ascii_lowercase(), mime.into())
                    .is_some()
                {
                    return Err(Error::Validation("duplicate content-type override".into()));
                }
            } else {
                return Err(Error::Validation(
                    "content type lacks part/extension".into(),
                ));
            }
        }
        Ok((defaults, overrides))
    }
    pub fn content_type_for(&self, path: &str) -> Result<Option<String>> {
        let (defaults, overrides) = self.content_types()?;
        Ok(overrides
            .get(&path.trim_start_matches('/').to_ascii_lowercase())
            .cloned()
            .or_else(|| {
                path.rsplit_once('.').and_then(|(_, extension)| {
                    defaults.get(&extension.to_ascii_lowercase()).cloned()
                })
            }))
    }
    fn validate_content_types(&self) -> Result<()> {
        let (defaults, overrides) = self.content_types()?;
        for path in self
            .parts
            .keys()
            .filter(|p| p.as_str() != "[Content_Types].xml")
        {
            let explicit = overrides.contains_key(&path.to_ascii_lowercase());
            let default = path.rsplit_once('.').is_some_and(|(_, extension)| {
                defaults.contains_key(&extension.to_ascii_lowercase())
            });
            if !explicit && !default {
                return Err(Error::from(crate::Diagnostic::new(
                    crate::Rule::InvalidPackage,
                    format!("part has no content type: {path}"),
                )));
            }
        }
        Ok(())
    }
    fn materialize(&mut self) {
        for (path, node) in self.edited.take() {
            self.parts.insert(path, node.bytes());
        }
    }
    fn validate_graph(&self) -> Result<()> {
        self.validate_content_types()?;
        for path in self
            .parts
            .keys()
            .filter(|p| p.to_ascii_lowercase().ends_with(".rels"))
        {
            let owner = if path.eq_ignore_ascii_case("_rels/.rels") {
                String::new()
            } else {
                let lower = path.to_ascii_lowercase();
                let split = lower.rfind("_rels/").ok_or_else(|| {
                    Error::Validation("invalid relationship part location".into())
                })?;
                format!("{}{}", &path[..split], &path[split + 6..path.len() - 5])
            };
            if !owner.is_empty()
                && !self
                    .parts
                    .keys()
                    .any(|part| part.eq_ignore_ascii_case(&owner))
            {
                return Err(Error::Validation(format!(
                    "relationship owner is missing: {owner}"
                )));
            }
            let root = self.xml(path)?;
            if root.name.namespace()
                != Some("http://schemas.openxmlformats.org/package/2006/relationships")
                || root.name.local() != "Relationships"
            {
                return Err(Error::from(crate::Diagnostic::new(
                    crate::Rule::InvalidRelationship,
                    "wrong relationship-part namespace",
                )));
            }
            let mut ids = std::collections::BTreeSet::new();
            for relationship in root.elements() {
                if relationship.name.namespace() != root.name.namespace()
                    || relationship.name.local() != "Relationship"
                {
                    return Err(Error::from(crate::Diagnostic::new(
                        crate::Rule::InvalidRelationship,
                        "wrong relationship namespace",
                    )));
                }
                let id = relationship
                    .get("Id")
                    .ok_or_else(|| Error::Validation("relationship has no ID".into()))?;
                RelationshipId::new(id)?;
                if !ids.insert(id) {
                    return Err(Error::Validation(format!(
                        "duplicate relationship ID {id} in {path}"
                    )));
                }
                RelationshipType::new(relationship.get("Type").unwrap_or(""))?;
                let target = relationship
                    .get("Target")
                    .ok_or_else(|| Error::Validation("relationship has no target".into()))?;
                match relationship.get("TargetMode") {
                    Some("External") => (),
                    None | Some("Internal") => {
                        let target = PartName::resolve(&owner, target)?;
                        if !self
                            .parts
                            .keys()
                            .any(|p| p.eq_ignore_ascii_case(target.zip_name()))
                        {
                            return Err(Error::Validation(format!(
                                "missing relationship target {} from {owner}",
                                target.as_str()
                            )));
                        }
                    }
                    Some(mode) => {
                        return Err(Error::Validation(format!(
                            "invalid relationship target mode {mode}"
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Application-selected read budgets, not normative OOXML numeric limits.
#[derive(Clone, Copy, Debug)]
pub struct ReadLimits {
    pub max_parts: usize,
    pub max_part_bytes: u64,
    pub max_expanded_bytes: u64,
}
impl Default for ReadLimits {
    fn default() -> Self {
        Self {
            max_parts: 10_000,
            max_part_bytes: 64 * 1024 * 1024,
            max_expanded_bytes: 256 * 1024 * 1024,
        }
    }
}

/// An immutable, inspectable set of package-part writes. Encoding performs no I/O.
#[derive(Debug)]
pub struct PackagePlan {
    package: Package,
}
impl PackagePlan {
    pub(crate) fn new(mut package: Package) -> Result<Self> {
        package.materialize();
        package.validate_graph()?;
        Ok(Self { package })
    }
    pub fn parts(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.package
            .parts
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
    }
    pub fn part(&self, name: &PartName) -> Option<&[u8]> {
        self.package.parts.get(name.zip_name()).map(Vec::as_slice)
    }
    pub fn validate(&self) -> Result<()> {
        self.package.validate_graph()
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .last_modified_time(zip::DateTime::default());
        for (path, bytes) in &self.package.parts {
            zip.start_file(path, options)?;
            zip.write_all(bytes)?;
        }
        Ok(zip.finish()?.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_targets_are_explicit_and_internal_targets_are_validated() {
        let mut package = Package {
            parts: BTreeMap::from([("main.xml".into(), b"<document/>".to_vec()), ("[Content_Types].xml".into(), br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/></Types>"#.to_vec())]),
            edited: Default::default(),
        };
        let external = package
            .add_relationship(
                "main.xml",
                &RelationshipType::new("https://example.test/hyperlink").unwrap(),
                Target::External("https://example.test/page".into()),
            )
            .unwrap();
        assert_eq!(external, "rId1");
        PackagePlan::new(package).unwrap().validate().unwrap();
        let mut package = Package {
            parts: BTreeMap::from([("main.xml".into(), b"<document/>".to_vec()), ("[Content_Types].xml".into(), br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/></Types>"#.to_vec())]),
            edited: Default::default(),
        };
        package.link("main.xml", "image", "missing.png").unwrap();
        assert!(
            PackagePlan::new(package)
                .unwrap_err()
                .to_string()
                .contains("missing relationship target")
        );
    }
    #[test]
    fn explicit_read_budgets_fail_before_expansion() {
        let options = ReadLimits {
            max_parts: 1,
            ..Default::default()
        };
        assert!(
            Package::load_with_limits(include_bytes!("../../assets/default.docx"), options)
                .unwrap_err()
                .to_string()
                .contains("part count")
        );
        let options = ReadLimits {
            max_part_bytes: 1,
            ..Default::default()
        };
        assert!(
            Package::load_with_limits(include_bytes!("../../assets/default.docx"), options)
                .is_err()
        );
    }
}
