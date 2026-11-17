use crate::xml::{NS, Node};
use crate::*;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
};
#[derive(Debug)]
pub(crate) struct Package {
    pub parts: BTreeMap<String, Vec<u8>>,
    pub drawing: u32,
}
impl Package {
    pub fn load(bytes: &[u8]) -> Result<Self> {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
        let mut parts = BTreeMap::new();
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            if file.is_dir() {
                continue;
            }
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            if parts.insert(file.name().to_owned(), bytes).is_some() {
                return Err(Error::Validation("duplicate DOCX package part".into()));
            }
        }
        let mut p = Self { parts, drawing: 0 };
        if !p.parts.contains_key("word/document.xml") {
            return Err(Error::Validation(
                "template must contain word/document.xml".into(),
            ));
        }
        // Minimal valid OPC templates may omit optional Word parts. Create the
        // same default style/numbering/settings parts that python-docx supplies lazily.
        let missing_styles = p.related("word/document.xml", "styles")?.is_none();
        let mut defaults = zip::ZipArchive::new(Cursor::new(
            include_bytes!("../assets/default.docx").as_slice(),
        ))?;
        for (kind, path, mime) in [
            (
                "styles",
                "word/styles.xml",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml",
            ),
            (
                "settings",
                "word/settings.xml",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml",
            ),
            (
                "numbering",
                "word/numbering.xml",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml",
            ),
            (
                "theme",
                "word/theme/theme1.xml",
                "application/vnd.openxmlformats-officedocument.theme+xml",
            ),
            (
                "fontTable",
                "word/fontTable.xml",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.fontTable+xml",
            ),
        ] {
            if !missing_styles && ["theme", "fontTable"].contains(&kind) {
                continue;
            }
            if p.related("word/document.xml", kind)?.is_none() {
                if !p.parts.contains_key(path) {
                    let mut bytes = Vec::new();
                    defaults.by_name(path)?.read_to_end(&mut bytes)?;
                    p.parts.insert(path.into(), bytes);
                }
                p.relation(
                    "word/document.xml",
                    kind,
                    path.strip_prefix("word/").unwrap(),
                )?;
                p.content_type(path, mime)?;
            }
        }
        for (name, data) in &p.parts {
            if (name == "word/document.xml"
                || name.starts_with("word/header")
                || name.starts_with("word/footer"))
                && name.ends_with(".xml")
                && let Ok(node) = Node::parse(data)
            {
                fn max_id(n: &Node) -> u32 {
                    n.elements()
                        .map(max_id)
                        .chain(n.get("id").and_then(|i| i.parse().ok()))
                        .max()
                        .unwrap_or(0)
                }
                p.drawing = p.drawing.max(max_id(&node));
            }
        }
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
                crate::render::validate_text(v)?;
            }
            for c in &n.children {
                match c {
                    crate::xml::Content::Element(n) => validate(n)?,
                    crate::xml::Content::Text(t) => crate::render::validate_text(t)?,
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
        let (path, file) = owner.rsplit_once('/').unwrap_or(("", owner));
        let relpath = if owner.is_empty() {
            "_rels/.rels".into()
        } else {
            format!("{path}/_rels/{file}.rels")
        };
        let mut rels = if self.parts.contains_key(&relpath) {
            self.xml(&relpath)?
        } else {
            Node::new("Relationships").attr(
                "xmlns",
                "http://schemas.openxmlformats.org/package/2006/relationships",
            )
        };
        let typ = if kind.contains("://") {
            kind.to_owned()
        } else {
            format!("http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}")
        };
        if let Some(id) = rels
            .elements()
            .find(|n| n.get("Type") == Some(typ.as_str()) && n.get("Target") == Some(target))
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
        rels.push(
            Node::new("Relationship")
                .attr("Id", &id)
                .attr("Type", typ)
                .attr("Target", target),
        );
        self.put(relpath, rels)?;
        Ok(id)
    }
    pub fn related(&self, owner: &str, selector: &str) -> Result<Option<(String, String)>> {
        let (dir, file) = owner.rsplit_once('/').unwrap_or(("", owner));
        let relpath = if owner.is_empty() {
            "_rels/.rels".to_owned()
        } else {
            format!("{dir}/_rels/{file}.rels")
        };
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
        let path = if target.starts_with('/') {
            target.to_owned()
        } else {
            format!("{dir}/{target}")
        };
        let mut normalized = Vec::new();
        for p in path.split('/') {
            match p {
                "" | "." => (),
                ".." => {
                    normalized.pop();
                }
                _ => normalized.push(p),
            }
        }
        Ok(Some((
            rel.get("Id").unwrap_or("").into(),
            normalized.join("/"),
        )))
    }
    pub fn content_type(&mut self, path: &str, mime: &str) -> Result<()> {
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
