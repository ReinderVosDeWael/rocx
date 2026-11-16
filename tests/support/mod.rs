use std::io::{Cursor, Read};

pub const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

pub fn part(bytes: &[u8], name: &str) -> String {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    xml
}

#[derive(Debug, PartialEq, Eq)]
pub struct Body {
    pub paragraphs: Vec<(String, bool)>,
    pub section_count: usize,
}

pub fn body(xml: &str) -> Body {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let body = doc
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    let paragraphs = body
        .children()
        .filter(|n| n.has_tag_name((W, "p")))
        .map(|p| {
            let mut text = String::new();
            for n in p.descendants() {
                if n.has_tag_name((W, "t")) {
                    text.push_str(n.text().unwrap_or(""));
                } else if n.has_tag_name((W, "tab")) {
                    text.push('\t');
                } else if n.has_tag_name((W, "br")) {
                    text.push('\n');
                }
            }
            (text, p.descendants().any(|n| n.has_tag_name((W, "sectPr"))))
        })
        .collect();
    let section_count = body
        .descendants()
        .filter(|n| n.has_tag_name((W, "sectPr")))
        .count();
    Body {
        paragraphs,
        section_count,
    }
}

// Each integration-test binary uses a different subset of these helpers.
#[allow(dead_code)]
pub fn texts(bytes: &[u8]) -> Vec<String> {
    body(&part(bytes, "word/document.xml"))
        .paragraphs
        .into_iter()
        .filter(|(_, is_break)| !is_break)
        .map(|(text, _)| text)
        .collect()
}
