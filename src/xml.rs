//! Small owned XML tree used for package parts and lossless retention of unrelated parts.
use crate::{Error, Result};

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
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Node {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Content>,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Content {
    Element(Node),
    Text(String),
    Raw(String),
}
impl Node {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            attrs: Vec::new(),
            children: Vec::new(),
        }
    }
    pub fn attr(mut self, key: impl Into<String>, value: impl ToString) -> Self {
        self.set(key, value);
        self
    }
    pub fn set(&mut self, key: impl Into<String>, value: impl ToString) {
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
        self.remove(&node.name);
        self.push(node);
    }
    pub fn namespaces(&mut self) {
        for (prefix, uri) in NS {
            self.set(format!("xmlns:{prefix}"), uri);
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
        let doc = roxmltree::Document::parse(text)
            .map_err(|e| Error::Validation(format!("invalid XML: {e}")))?;
        fn qname(n: roxmltree::Node<'_, '_>, ns: Option<&str>, local: &str) -> String {
            if ns == Some("http://www.w3.org/XML/1998/namespace") {
                return format!("xml:{local}");
            }
            let prefix = ns.and_then(|uri| {
                NS.iter()
                    .find(|(_, u)| *u == uri)
                    .map(|(p, _)| *p)
                    .or_else(|| n.lookup_prefix(uri))
            });
            match prefix {
                Some(p) if !p.is_empty() => format!("{p}:{local}"),
                _ => local.into(),
            }
        }
        fn convert(n: roxmltree::Node<'_, '_>, source: &str) -> Node {
            let mut node = Node::new(qname(n, n.tag_name().namespace(), n.tag_name().name()));
            for ns in n.namespaces() {
                if n.parent_element()
                    .is_none_or(|p| p.lookup_namespace_uri(ns.name()) != Some(ns.uri()))
                {
                    node.set(
                        ns.name()
                            .map(|p| format!("xmlns:{p}"))
                            .unwrap_or("xmlns".into()),
                        ns.uri(),
                    );
                }
            }
            for a in n.attributes() {
                node.set(qname(n, a.namespace(), a.name()), a.value());
            }
            for child in n.children() {
                if child.is_element() {
                    node.push(convert(child, source));
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
    pub fn sort_properties(&mut self) {
        for child in self.elements_mut() {
            child.sort_properties();
        }
        let order = match self.name.as_str() {
            "w:pPr" => {
                "pStyle keepNext keepLines pageBreakBefore framePr widowControl numPr suppressLineNumbers pBdr shd tabs suppressAutoHyphens kinsoku wordWrap overflowPunct topLinePunct autoSpaceDE autoSpaceDN bidi adjustRightInd snapToGrid spacing ind contextualSpacing mirrorIndents suppressOverlap jc textDirection textAlignment textboxTightWrap outlineLvl divId cnfStyle rPr sectPr pPrChange"
            }
            "w:rPr" => {
                "rStyle rFonts b bCs i iCs caps smallCaps strike dstrike outline shadow emboss imprint noProof snapToGrid vanish webHidden color spacing w kern position sz szCs highlight u effect bdr shd fitText vertAlign rtl cs em lang eastAsianLayout specVanish oMath rPrChange"
            }
            "w:sectPr" => {
                "headerReference footerReference footnotePr endnotePr type pgSz pgMar paperSrc pgBorders lnNumType pgNumType cols formProt vAlign noEndnote titlePg textDirection bidi rtlGutter docGrid printerSettings sectPrChange"
            }
            "w:tblPr" => {
                "tblStyle tblpPr tblOverlap bidiVisual tblStyleRowBandSize tblStyleColBandSize tblW jc tblCellSpacing tblInd tblBorders shd tblLayout tblCellMar tblLook tblCaption tblDescription tblPrChange"
            }
            "w:tcPr" => {
                "cnfStyle tcW gridSpan hMerge vMerge tcBorders shd noWrap tcMar textDirection tcFitText vAlign hideMark headers cellIns cellDel cellMerge tcPrChange"
            }
            "w:style" => {
                "name aliases basedOn next link autoRedefine hidden uiPriority semiHidden unhideWhenUsed qFormat locked personal personalCompose personalReply rsid pPr rPr tblPr trPr tcPr tblStylePr"
            }
            _ => return,
        };
        let names: Vec<_> = order.split_whitespace().collect();
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
        self.write(&mut out);
        out
    }
    pub fn bytes(&self) -> Vec<u8> {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>{}",
            self.xml()
        )
        .into_bytes()
    }
    fn write(&self, out: &mut String) {
        out.push('<');
        out.push_str(&self.name);
        for (k, v) in &self.attrs {
            out.push(' ');
            out.push_str(k);
            out.push_str("=\"");
            escape(out, v, true);
            out.push('"');
        }
        if self.children.is_empty() {
            out.push_str("/>");
            return;
        }
        out.push('>');
        for child in &self.children {
            match child {
                Content::Element(n) => n.write(out),
                Content::Text(t) => escape(out, t, false),
                Content::Raw(r) => out.push_str(r),
            }
        }
        out.push_str("</");
        out.push_str(&self.name);
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
