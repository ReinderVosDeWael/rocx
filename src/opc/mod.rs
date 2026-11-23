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
    sync::Arc,
};
const RELATIONSHIPS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const RELATIONSHIPS_MIME: &str = "application/vnd.openxmlformats-package.relationships+xml";
const CONTENT_TYPES_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
#[derive(Clone, Debug)]
pub(crate) struct Package {
    // Snapshots share payloads; materializing edited XML replaces only its own entries.
    pub parts: BTreeMap<String, Arc<[u8]>>,
    xml_cache: std::cell::RefCell<BTreeMap<String, CachedXml>>,
}
#[derive(Clone, Debug)]
struct CachedXml {
    node: Node,
    dirty: bool,
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
            let name = file.name()?.into_owned();
            let part = PartName::new(&name)?;
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
                .at(name)
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
                .at(name)
                .into());
            }
            if parts.insert(name, bytes.into()).is_some() {
                return Err(Error::Validation("duplicate DOCX package part".into()));
            }
        }
        let p = Self {
            parts,
            xml_cache: Default::default(),
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
        if let Some(cached) = self.xml_cache.borrow().get(path) {
            return Ok(cached.node.clone());
        }
        let bytes = self
            .parts
            .get(path)
            .ok_or_else(|| Error::Validation(format!("missing package part {path}")))?;
        let node = Node::parse(bytes)?;
        self.xml_cache.borrow_mut().insert(
            path.into(),
            CachedXml {
                node: node.clone(),
                dirty: false,
            },
        );
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
        self.xml_cache
            .borrow_mut()
            .insert(path, CachedXml { node, dirty: true });
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
        let mut relationship = Node::new(crate::xml::Name::expanded(
            Some(RELATIONSHIPS_NS),
            "Relationship",
            None,
        ))
        .attr("Id", &id)
        .attr("Type", typ)
        .attr("Target", target);
        if external {
            relationship.set("TargetMode", "External");
        }
        rels.push(relationship);
        if self.content_type_for(&relpath)?.is_none() {
            self.content_type(&relpath, RELATIONSHIPS_MIME)?;
        }
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
                Node::new(crate::xml::Name::expanded(
                    Some(CONTENT_TYPES_NS),
                    "Override",
                    None,
                ))
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
    /// Keep relationship IDs and targets when copying a part within its own directory.
    pub(crate) fn copy_sibling_relationships(&mut self, from: &str, to: &str) -> Result<()> {
        let from = PartName::new(from)?;
        let to = PartName::new(to)?;
        let directory = |part: &str| part.rsplit_once('/').map_or("", |(dir, _)| dir).to_owned();
        let destination = relationships_path(to.zip_name());
        if directory(from.zip_name()) != directory(to.zip_name())
            || self
                .parts
                .keys()
                .any(|path| path.eq_ignore_ascii_case(&destination))
        {
            return Err(Error::Validation(
                "cannot copy relationships to this part".into(),
            ));
        }
        let source = relationships_path(from.zip_name());
        if let Some((_, bytes)) = self
            .parts
            .iter()
            .find(|(path, _)| path.eq_ignore_ascii_case(&source))
        {
            let bytes = bytes.clone();
            if self.content_type_for(&destination)?.is_none() {
                self.content_type(&destination, RELATIONSHIPS_MIME)?;
            }
            self.parts.insert(destination, bytes);
        }
        Ok(())
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
        for (path, cached) in self.xml_cache.take() {
            if cached.dirty {
                self.parts.insert(path, cached.node.bytes().into());
            }
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
            .map(|(name, bytes)| (name.as_str(), bytes.as_ref()))
    }
    pub fn part(&self, name: &PartName) -> Option<&[u8]> {
        self.package.parts.get(name.zip_name()).map(AsRef::as_ref)
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
    fn retained_plans_share_unchanged_media_with_the_source() {
        let source = Package::load_with_limits(
            include_bytes!("../../assets/default.docx"),
            ReadLimits::default(),
        )
        .unwrap();
        let media = PartName::new("docProps/thumbnail.jpeg").unwrap();
        let unchanged = &source.parts[media.zip_name()];
        let plans = [(); 3].map(|_| PackagePlan::new(source.clone()).unwrap());
        for plan in &plans {
            let bytes = plan.part(&media).unwrap();
            assert_eq!(bytes.len(), unchanged.len());
            assert_eq!(bytes.as_ptr(), unchanged.as_ptr(), "media was copied");
        }
    }

    #[test]
    fn reading_xml_preserves_payload_bytes_until_an_explicit_write() {
        let original = br#"<root attr = 'old'><!-- preserve lexical bytes --></root>"#.as_slice();
        let mut package = Package {
            parts: BTreeMap::from([("opaque.xml".into(), original.into())]),
            xml_cache: Default::default(),
        };
        let mut node = package.xml("opaque.xml").unwrap();
        node.set("attr", "uncommitted local mutation");
        package.materialize();
        assert_eq!(package.parts["opaque.xml"].as_ref(), original);
        package.put("opaque.xml", node).unwrap();
        assert_eq!(
            package.xml("opaque.xml").unwrap().get("attr"),
            Some("uncommitted local mutation")
        );
        package.materialize();
        assert_ne!(package.parts["opaque.xml"].as_ref(), original);
        assert_eq!(
            package.xml("opaque.xml").unwrap().get("attr"),
            Some("uncommitted local mutation")
        );
    }
    #[test]
    fn external_targets_are_explicit_and_internal_targets_are_validated() {
        let mut package = Package {
            parts: BTreeMap::from([("main.xml".into(), b"<document/>".as_slice().into()), ("[Content_Types].xml".into(), br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/></Types>"#.as_slice().into())]),
            xml_cache: Default::default(),
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
            parts: BTreeMap::from([("main.xml".into(), b"<document/>".as_slice().into()), ("[Content_Types].xml".into(), br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/></Types>"#.as_slice().into())]),
            xml_cache: Default::default(),
        };
        package.link("main.xml", "image", "missing.png").unwrap();
        assert!(
            PackagePlan::new(package)
                .unwrap_err()
                .to_string()
                .contains("missing relationship target")
        );
    }
    fn budget_fixture() -> (Vec<u8>, u64, u64) {
        let content_types = br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="bin" ContentType="application/octet-stream"/></Types>"#;
        let payload = vec![b'x'; content_types.len() + 1];
        let total = (content_types.len() + payload.len() + 1) as u64;
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .last_modified_time(zip::DateTime::default());
        for (name, bytes) in [
            ("[Content_Types].xml", content_types.as_slice()),
            ("a.bin", payload.as_slice()),
            ("b.bin", b"x".as_slice()),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }
        (
            zip.finish().unwrap().into_inner(),
            payload.len() as u64,
            total,
        )
    }

    #[test]
    fn read_budget_part_count_boundary() {
        let (archive, _, _) = budget_fixture();
        let mut limits = ReadLimits {
            max_parts: 3,
            ..Default::default()
        };
        assert_eq!(
            Package::load_with_limits(&archive, limits)
                .unwrap()
                .parts
                .len(),
            3
        );
        limits.max_parts = 2;
        let error = Package::load_with_limits(&archive, limits).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("package part count exceeds read limit"),
            "{error}"
        );
    }

    #[test]
    fn read_budget_part_bytes_boundary() {
        let (archive, largest_part, _) = budget_fixture();
        let mut limits = ReadLimits {
            max_part_bytes: largest_part,
            ..Default::default()
        };
        assert!(Package::load_with_limits(&archive, limits).is_ok());
        limits.max_part_bytes -= 1;
        let error = Package::load_with_limits(&archive, limits).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("package expanded bytes exceed read limit"),
            "{error}"
        );
        assert!(error.to_string().contains("a.bin"), "{error}");
    }

    #[test]
    fn read_budget_expanded_bytes_boundary() {
        let (archive, _, total) = budget_fixture();
        let mut limits = ReadLimits {
            max_expanded_bytes: total,
            ..Default::default()
        };
        assert!(Package::load_with_limits(&archive, limits).is_ok());
        limits.max_expanded_bytes -= 1;
        let error = Package::load_with_limits(&archive, limits).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("package expanded bytes exceed read limit"),
            "{error}"
        );
        assert!(error.to_string().contains("b.bin"), "{error}");
    }
}
