//! Package parts, relationships, content types, and ZIP I/O.
//!
//! References: ECMA-376 Part 2 (2021), §§6.2 (parts), 6.4 (relative
//! references), 6.5 (relationships), and 7.3 (ZIP mapping).
//! [Official Part 2](https://ecma-international.org/wp-content/uploads/ECMA-376-2_5th_edition_december_2021.zip).
//! Word-specific default seeding is a current implementation concern here;
//! it is not an OPC requirement.
use crate::xml::{NS, Node};
mod names;
use crate::{Error, Result};
use names::relationships_path;
pub use names::{ContentType, PartName, RelationshipId, RelationshipType, Target};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
};
#[derive(Debug)]
pub(crate) struct Package {
    pub parts: BTreeMap<String, Vec<u8>>,
}
impl Package {
    pub fn load(bytes: &[u8]) -> Result<Self> {
        Self::load_with_limits(bytes, ReadLimits::default())
    }
    pub fn load_with_limits(bytes: &[u8], limits: ReadLimits) -> Result<Self> {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
        if zip.len() > limits.max_parts {
            return Err(Error::Validation(
                "package part count exceeds read limit".into(),
            ));
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
                return Err(Error::Validation(
                    "package expanded bytes exceed read limit".into(),
                ));
            }
            let mut bytes = Vec::new();
            (&mut file)
                .take(limits.max_part_bytes.saturating_add(1))
                .read_to_end(&mut bytes)?;
            total += bytes.len() as u64;
            if bytes.len() as u64 > limits.max_part_bytes || total > limits.max_expanded_bytes {
                return Err(Error::Validation(
                    "package expanded bytes exceed read limit".into(),
                ));
            }
            if parts.insert(file.name().to_owned(), bytes).is_some() {
                return Err(Error::Validation("duplicate DOCX package part".into()));
            }
        }
        let p = Self { parts };
        Ok(p)
    }
    pub fn xml(&self, path: &str) -> Result<Node> {
        Node::parse(
            self.parts
                .get(path)
                .ok_or_else(|| Error::Validation(format!("missing package part {path}")))?,
        )
    }
    pub fn put(&mut self, path: impl Into<String>, mut node: Node) -> Result<()> {
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
        node.sort_properties();
        validate(&node)?;
        self.parts.insert(path.into(), node.bytes());
        Ok(())
    }
    pub fn relation(&mut self, owner: &str, kind: &str, target: &str) -> Result<String> {
        let typ = if kind.contains("://") {
            kind.to_owned()
        } else {
            format!("http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}")
        };
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
        let mut rels = if self.parts.contains_key(&relpath) {
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
        if !self.parts.contains_key(&relpath) {
            return Ok(None);
        }
        let rels = self.xml(&relpath)?;
        let found = rels.elements().find(|n| {
            selector
                .strip_prefix('#')
                .is_some_and(|id| n.get("Id") == Some(id))
                || (!selector.starts_with('#')
                    && n.get("Type")
                        .is_some_and(|t| t.ends_with(&format!("/{selector}"))))
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
        if !self.parts.contains_key(&path) {
            return Ok(Vec::new());
        }
        let relationships = self.xml(&path)?;
        relationships
            .elements()
            .filter(|n| {
                n.get("Type")
                    .is_some_and(|t| t.ends_with(&format!("/{kind}")))
            })
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
        if let Some(n) = types
            .elements_mut()
            .find(|n| n.get("PartName") == Some(part.as_str()))
        {
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
            if !self.parts.contains_key(&path) {
                return path;
            }
            i += 1;
        }
    }
    pub fn link(&mut self, owner: &str, kind: &str, target: &str) -> Result<String> {
        self.relation(owner, kind, &PartName::new(target)?.relative_to(owner)?)
    }
    pub fn finish(self) -> Result<Vec<u8>> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .last_modified_time(zip::DateTime::default());
        for (path, bytes) in self.parts {
            zip.start_file(path, opts)?;
            zip.write_all(&bytes)?;
        }
        Ok(zip.finish()?.into_inner())
    }
    pub fn root(name: &str) -> Node {
        let mut n = Node::new(name);
        for (p, u) in NS {
            n.set(format!("xmlns:{p}"), u);
        }
        n
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
