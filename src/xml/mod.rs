//! Owned XML tree for edited package parts; imported XML is normalized.
//!
//! Untouched part payloads are retained by the package layer. Parsing and
//! rewriting XML does not guarantee lossless preservation of unknown markup.
//! XML character/name rules: [XML 1.0](https://www.w3.org/TR/REC-xml/).
//! Namespace identity/scoping: [Namespaces in XML](https://www.w3.org/TR/REC-xml-names/).
//! Compatibility markup: ECMA-376 Part 3 (2015), §§7–9.
//! [Official Part 3](https://ecma-international.org/wp-content/uploads/ECMA-376-3_5th_edition_december_2015.zip).
use crate::{Error, Result};
use std::collections::BTreeMap;
mod name;
pub(crate) use name::Name;
type OrderingPolicy = fn(&Name) -> Option<Vec<&'static str>>;

pub(crate) const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub(crate) const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub(crate) const NS: &[(&str, &str)] = &[
    ("w", W),
    ("r", R),
    ("a", "http://schemas.openxmlformats.org/drawingml/2006/main"),
    (
        "pic",
        "http://schemas.openxmlformats.org/drawingml/2006/picture",
    ),
    (
        "wp",
        "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
    ),
    (
        "cp",
        "http://schemas.openxmlformats.org/package/2006/metadata/core-properties",
    ),
    ("dc", "http://purl.org/dc/elements/1.1/"),
    ("dcterms", "http://purl.org/dc/terms/"),
    ("xsi", "http://www.w3.org/2001/XMLSchema-instance"),
];
#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub name: Name,
    pub attrs: Vec<(Name, String)>,
    imported: bool,
    ordering: Option<OrderingPolicy>,
    pub children: Vec<Content>,
}
#[derive(Debug, Clone)]
pub(crate) enum Content {
    Element(Node),
    Text(String),
    Raw(String),
}
impl Node {
    pub fn new(name: impl Into<Name>) -> Self {
        Self {
            name: name.into(),
            attrs: Vec::new(),
            imported: false,
            ordering: None,
            children: Vec::new(),
        }
    }
    pub fn attr(mut self, key: impl Into<Name>, value: impl ToString) -> Self {
        self.set(key, value);
        self
    }
    pub fn set(&mut self, key: impl Into<Name>, value: impl ToString) {
        let key = key.into();
        if let Some((_, v)) = self.attrs.iter_mut().find(|(k, _)| *k == key) {
            *v = value.to_string();
        } else {
            self.attrs.push((key, value.to_string()));
        }
    }
    pub fn get(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
    pub fn push(&mut self, node: Node) {
        if self.imported
            && let Some(order) = self.property_order()
        {
            let rank = |name: &Name| order.iter().position(|s| name == &format!("w:{s}"));
            if let Some(position) = rank(&node.name)
                && let Some(index) = self.children.iter().position(|c| matches!(c, Content::Element(n) if rank(&n.name).is_some_and(|r| r > position)))
            {
                self.children.insert(index, Content::Element(node));
                return;
            }
        }
        self.children.push(Content::Element(node));
    }
    pub fn with(mut self, node: Node) -> Self {
        self.push(node);
        self
    }
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.children.push(Content::Text(text.into()));
        self
    }
    pub fn elements(&self) -> impl Iterator<Item = &Node> {
        self.children.iter().filter_map(|c| {
            if let Content::Element(n) = c {
                Some(n)
            } else {
                None
            }
        })
    }
    pub fn elements_mut(&mut self) -> impl Iterator<Item = &mut Node> {
        self.children.iter_mut().filter_map(|c| {
            if let Content::Element(n) = c {
                Some(n)
            } else {
                None
            }
        })
    }
    pub fn child(&self, name: &str) -> Option<&Node> {
        self.elements().find(|n| n.name == name)
    }
    pub fn child_mut(&mut self, name: &str) -> Option<&mut Node> {
        self.elements_mut().find(|n| n.name == name)
    }
    pub fn remove(&mut self, name: &str) {
        self.children
            .retain(|c| !matches!(c,Content::Element(n) if n.name==name));
    }
    pub fn ensure(&mut self, name: &str) -> &mut Node {
        if self.child(name).is_none() {
            self.push(Self::new(name));
        }
        self.child_mut(name).unwrap()
    }
    pub fn replace(&mut self, node: Node) {
        if let Some(existing) = self.elements_mut().find(|n| n.name == node.name) {
            *existing = node;
        } else {
            self.push(node);
        }
    }
    pub fn namespaces(&mut self) {
        for (prefix, uri) in NS {
            let key = format!("xmlns:{prefix}");
            if self.get(&key).is_none() {
                self.set(key, uri);
            }
        }
    }
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let decoded;
        let utf16le = bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0x3c, 0]);
        let utf16be = bytes.starts_with(&[0xfe, 0xff]) || bytes.starts_with(&[0, 0x3c]);
        let text = if utf16le || utf16be {
            let offset = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
                2
            } else {
                0
            };
            if !(bytes.len() - offset).is_multiple_of(2) {
                return Err(Error::Validation(
                    "odd byte count in UTF-16 XML part".into(),
                ));
            }
            let units: Vec<_> = bytes[offset..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| {
                    if utf16le {
                        u16::from_le_bytes([b[0], b[1]])
                    } else {
                        u16::from_be_bytes([b[0], b[1]])
                    }
                })
                .collect();
            decoded = String::from_utf16(&units).map_err(|e| Error::Validation(e.to_string()))?;
            &decoded
        } else {
            std::str::from_utf8(bytes).map_err(|e| Error::Validation(e.to_string()))?
        };
        let doc = roxmltree::Document::parse_with_options(
            text,
            roxmltree::ParsingOptions {
                allow_dtd: false,
                nodes_limit: 1_000_000,
            },
        )
        .map_err(|e| Error::Validation(format!("invalid XML: {e}")))?;
        if doc
            .descendants()
            .any(|n| n.ancestors().take(258).count() > 256)
        {
            return Err(Error::Validation(
                "XML depth exceeds read limit (256)".into(),
            ));
        }
        fn convert(n: roxmltree::Node<'_, '_>, source: &str) -> Node {
            let mut node = Node::new(Name::expanded(
                n.tag_name().namespace(),
                n.tag_name().name(),
                n.tag_name()
                    .namespace()
                    .and_then(|uri| n.lookup_prefix(uri)),
            ));
            node.imported = true;
            // Preserve the full scope on opaque imported nodes. Prefix-valued
            // attributes (including MCE lists) must retain their bindings.
            for ns in n.namespaces() {
                node.set(
                    ns.name()
                        .map(|p| format!("xmlns:{p}"))
                        .unwrap_or("xmlns".into()),
                    ns.uri(),
                );
            }
            for a in n.attributes() {
                node.set(
                    Name::expanded(
                        a.namespace(),
                        a.name(),
                        a.namespace().and_then(|uri| n.lookup_prefix(uri)),
                    ),
                    a.value(),
                );
            }
            for child in n.children() {
                if child.is_element() {
                    node.children.push(Content::Element(convert(child, source)));
                } else if child.is_text() {
                    node.children
                        .push(Content::Text(child.text().unwrap_or("").into()));
                } else {
                    node.children
                        .push(Content::Raw(source[child.range()].into()));
                }
            }
            node
        }
        let mut root = convert(doc.root_element(), text);
        root.namespaces();
        Ok(root)
    }
    pub fn configure_ordering(&mut self, policy: OrderingPolicy) {
        self.ordering = Some(policy);
        for child in self.elements_mut() {
            child.configure_ordering(policy);
        }
    }
    fn property_order(&self) -> Option<Vec<&'static str>> {
        self.ordering.and_then(|policy| policy(&self.name))
    }
    pub fn sort_properties(&mut self) {
        for child in self.elements_mut() {
            child.sort_properties();
        }
        if self.imported {
            return;
        }
        let Some(names) = self.property_order() else {
            return;
        };
        self.children.sort_by_key(|c| match c {
            Content::Element(n) => names
                .iter()
                .position(|s| n.name == format!("w:{s}"))
                .unwrap_or(names.len()),
            _ => names.len() + 1,
        });
    }
    pub fn xml(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, &BTreeMap::new());
        out
    }
    pub fn bytes(&self) -> Vec<u8> {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>{}",
            self.xml()
        )
        .into_bytes()
    }
    fn write(&self, out: &mut String, inherited: &BTreeMap<String, String>) {
        let mut scope = inherited.clone();
        let mut declarations = Vec::new();
        for (name, value) in &self.attrs {
            if let Some(prefix) = name.declaration()
                && scope.get(prefix) != Some(value)
            {
                scope.insert(prefix.to_owned(), value.clone());
                declarations.push((name.as_str().to_owned(), value.clone()));
            }
        }
        if self.imported
            && self.name.namespace().is_none()
            && scope.get("").is_some_and(|uri| !uri.is_empty())
        {
            scope.insert(String::new(), String::new());
            declarations.push(("xmlns".into(), String::new()));
        }
        let element = self.name.lexical(&mut scope, &mut declarations, false);
        let attributes: Vec<_> = self
            .attrs
            .iter()
            .filter(|(name, _)| name.declaration().is_none())
            .map(|(name, value)| (name.lexical(&mut scope, &mut declarations, true), value))
            .collect();
        out.push('<');
        out.push_str(&element);
        for (name, value) in declarations
            .iter()
            .map(|(n, v)| (n.as_str(), v.as_str()))
            .chain(attributes.iter().map(|(n, v)| (n.as_str(), v.as_str())))
        {
            out.push(' ');
            out.push_str(name);
            out.push_str("=\"");
            escape(out, value, true);
            out.push('"');
        }
        if self.children.is_empty() {
            out.push_str("/>");
            return;
        }
        out.push('>');
        for child in &self.children {
            match child {
                Content::Element(n) => n.write(out, &scope),
                Content::Text(t) => escape(out, t, false),
                Content::Raw(r) => out.push_str(r),
            }
        }
        out.push_str("</");
        out.push_str(&element);
        out.push('>');
    }
}
fn escape(out: &mut String, text: &str, attribute: bool) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            '\n' if attribute => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' if attribute => out.push_str("&#9;"),
            _ => out.push(c),
        }
    }
}
pub(crate) fn val(name: &str, value: impl ToString) -> Node {
    Node::new(format!("w:{name}")).attr("w:val", value)
}

pub(crate) fn validate_text(text: &str) -> Result<()> {
    for (byte_offset, character) in text.char_indices() {
        if !matches!(character, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
        {
            return Err(Error::InvalidXmlCharacter {
                character,
                byte_offset,
            });
        }
    }
    Ok(())
}

pub(crate) fn is_ncname(value: &str) -> bool {
    fn start(c: char) -> bool {
        matches!(c,'A'..='Z'|'a'..='z'|'_'|'\u{c0}'..='\u{d6}'|'\u{d8}'..='\u{f6}'|'\u{f8}'..='\u{2ff}'|'\u{370}'..='\u{37d}'|'\u{37f}'..='\u{1fff}'|'\u{200c}'..='\u{200d}'|'\u{2070}'..='\u{218f}'|'\u{2c00}'..='\u{2fef}'|'\u{3001}'..='\u{d7ff}'|'\u{f900}'..='\u{fdcf}'|'\u{fdf0}'..='\u{fffd}'|'\u{10000}'..='\u{effff}')
    }
    let mut characters = value.chars();
    characters.next().is_some_and(start) && characters.all(|c| {
        start(c)
            || matches!(c,'-'|'.'|'0'..='9'|'\u{b7}'|'\u{300}'..='\u{36f}'|'\u{203f}'..='\u{2040}')
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn namespace_identity_survives_shadowing_and_prefix_valued_attributes() {
        let source = format!(
            r#"<w:document xmlns:w="{W}" xmlns:mc="urn:mc" xmlns:e="urn:ext" mc:Ignorable="e"><w:body><e:opaque xmlns:w="urn:shadow" xmlns:main="{W}" xmlns:q="urn:value" q:type="q:Thing"><main:p><main:r><main:t>keep</main:t></main:r></main:p><w:p/></e:opaque></w:body></w:document>"#
        );
        let tree = Node::parse(source.as_bytes()).unwrap();
        let output = tree.bytes();
        let output = std::str::from_utf8(&output).unwrap();
        let parsed = roxmltree::Document::parse(output).unwrap();
        assert!(parsed.descendants().any(|n| n.has_tag_name((W, "p"))));
        assert!(
            parsed
                .descendants()
                .any(|n| n.has_tag_name(("urn:shadow", "p")))
        );
        let opaque = parsed
            .descendants()
            .find(|n| n.has_tag_name(("urn:ext", "opaque")))
            .unwrap();
        assert_eq!(opaque.attribute(("urn:value", "type")), Some("q:Thing"));
        assert_eq!(opaque.lookup_namespace_uri(Some("q")), Some("urn:value"));
        assert_eq!(
            tree.child("w:body")
                .unwrap()
                .elements()
                .next()
                .unwrap()
                .elements()
                .filter(|n| n.name == "w:p")
                .count(),
            1
        );
    }
    #[test]
    fn imported_property_order_is_retained_but_generated_properties_are_ordered() {
        let source = format!(
            r#"<w:pPr xmlns:w="{W}" xmlns:e="urn:ext"><w:jc w:val="left"/><e:opaque/><w:pStyle w:val="X"/></w:pPr>"#
        );
        let mut imported = Node::parse(source.as_bytes()).unwrap();
        imported.configure_ordering(crate::ooxml::schema::order);
        imported.sort_properties();
        assert_eq!(
            imported
                .elements()
                .map(|n| n.name.as_str())
                .collect::<Vec<_>>(),
            ["w:jc", "{urn:ext}opaque", "w:pStyle"]
        );
        let mut generated = Node::new("w:pPr")
            .with(val("jc", "left"))
            .with(val("pStyle", "X"));
        generated.configure_ordering(crate::ooxml::schema::order);
        generated.sort_properties();
        assert_eq!(
            generated
                .elements()
                .map(|n| n.name.as_str())
                .collect::<Vec<_>>(),
            ["w:pStyle", "w:jc"]
        );
    }
    #[test]
    fn default_namespace_and_unqualified_attributes_remain_distinct() {
        let tree = Node::parse(
            br#"<root xmlns="urn:root"><child id="one"><local xmlns=""/></child></root>"#,
        )
        .unwrap();
        let output = tree.xml();
        let parsed = roxmltree::Document::parse(&output).unwrap();
        let child = parsed
            .descendants()
            .find(|n| n.has_tag_name(("urn:root", "child")))
            .unwrap();
        assert_eq!(child.attribute("id"), Some("one"));
        assert!(child.attributes().all(|a| a.namespace().is_none()));
        assert!(parsed.descendants().any(|n| n.tag_name().name() == "local"
            && n.tag_name().namespace().is_none_or(str::is_empty)));
    }
}
