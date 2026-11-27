//! Workspace-only checks for direct markup hidden by the typed layout views.

use std::collections::BTreeSet;

use super::{ImportedDocument, LayoutField, layout, missing_body};
use crate::{
    Diagnostic, Result, Rule,
    ooxml::WordParts,
    xml::{Content, Node},
};

type Check = std::result::Result<(), String>;

/// Check the main story's direct markup after the consumer's typed checks.
///
/// This does not resolve styles, document settings, or other package parts.
pub fn verify_plain_flow(doc: &ImportedDocument) -> Result<()> {
    verify_flow(doc, false)
}

/// As in [`verify_plain_flow`], additionally allowing direct before/after
/// spacing and direct twip indents. The consumer checks the resulting geometry.
pub fn verify_direct_flow(doc: &ImportedDocument) -> Result<()> {
    verify_flow(doc, true)
}

fn verify_flow(doc: &ImportedDocument, allow_direct: bool) -> Result<()> {
    let root = doc.package.word(&doc.main)?;
    verify_document(&root, allow_direct)
}

fn verify_document(root: &Node, allow_direct: bool) -> Result<()> {
    let error_at =
        |location: &str, message| Diagnostic::new(Rule::UnsupportedProfile, message).at(location);
    // Ignorable only names namespaces: their markup is still checked below.
    let ignorable = "{http://schemas.openxmlformats.org/markup-compatibility/2006}Ignorable";
    attrs(root, &[ignorable]).map_err(|e| error_at("main document", e))?;
    content(root, Children::Elements).map_err(|e| error_at("main document", e))?;
    let mut children = root.elements();
    let body = children.next().ok_or_else(missing_body)?;
    if body.name != "w:body" || children.next().is_some() {
        return Err(error_at("main document", "expected one direct body".into()).into());
    }
    attrs(body, &[]).map_err(|e| error_at("body", e))?;
    content(body, Children::Elements).map_err(|e| error_at("body", e))?;
    let mut section_seen = false;
    for (index, block) in body.elements().enumerate() {
        if section_seen {
            return Err(error_at(
                "section 0",
                "section properties must be unique and terminal".into(),
            )
            .into());
        }
        match block.name.as_str() {
            "w:sectPr" => {
                section_seen = true;
                section(block).map_err(|e| error_at("section 0", e))?;
            }
            "w:p" => paragraph(block, allow_direct)
                .map_err(|e| error_at(&format!("body block {index}"), e))?,
            _ => {
                return Err(error_at(
                    &format!("body block {index}"),
                    format!("unsupported {}", block.name),
                )
                .into());
            }
        }
    }
    Ok(())
}

fn attrs(node: &Node, allowed: &[&str]) -> Check {
    for (name, _) in &node.attrs {
        if name.declaration().is_none() && !allowed.contains(&name.as_str()) {
            return Err(format!("unsupported {} attribute {name}", node.name));
        }
    }
    Ok(())
}

enum Children {
    Elements,
    Text,
    Empty,
}

fn content(node: &Node, children: Children) -> Check {
    let valid = node.children.iter().all(|child| match child {
        Content::Element(_) => matches!(children, Children::Elements),
        Content::Text(s) => {
            matches!(children, Children::Text)
                || s.bytes().all(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        }
        // An XML comment cannot alter the flow. Do not treat arbitrary raw
        // content or processing instructions as comments.
        Content::Raw(raw) => raw.starts_with("<!--"),
    });
    if valid {
        Ok(())
    } else {
        Err(format!("unsupported content in {}", node.name))
    }
}

fn unique_properties(props: &Node) -> Check {
    let mut names = BTreeSet::new();
    for item in props.elements() {
        if !names.insert(item.name.as_str()) {
            return Err(format!("duplicate {} in {}", item.name, props.name));
        }
    }
    Ok(())
}

// Revision-session IDs and Word's paragraph/text IDs identify source markup;
// they do not select formatting. Only these specific attributes are allowed.
const RUN_IDS: &[&str] = &["w:rsidR", "w:rsidRPr", "w:rsidDel"];
const SECTION_IDS: &[&str] = &["w:rsidR", "w:rsidRPr", "w:rsidDel", "w:rsidSect"];
const PARAGRAPH_IDS: &[&str] = &[
    "w:rsidR",
    "w:rsidRPr",
    "w:rsidDel",
    "w:rsidRDefault",
    "w:rsidP",
    "{http://schemas.microsoft.com/office/word/2010/wordml}paraId",
    "{http://schemas.microsoft.com/office/word/2010/wordml}textId",
];

fn paragraph(node: &Node, allow_direct: bool) -> Check {
    attrs(node, PARAGRAPH_IDS)?;
    content(node, Children::Elements)?;
    let mut children = node.elements().peekable();
    if children.peek().is_some_and(|p| p.name == "w:pPr") {
        paragraph_properties(children.next().unwrap(), allow_direct)?;
    }
    for (index, child) in children.enumerate() {
        match child.name.as_str() {
            "w:pPr" => return Err("duplicate or misplaced w:pPr".into()),
            "w:r" => run(child).map_err(|e| format!("run {index}: {e}"))?,
            _ => return Err(format!("unsupported {}", child.name)),
        }
    }
    Ok(())
}

fn paragraph_properties(props: &Node, allow_direct: bool) -> Check {
    use LayoutField::*;
    attrs(props, &[])?;
    content(props, Children::Elements)?;
    unique_properties(props)?;
    let values = layout::inspect(Some(props), false);
    for prop in props.elements() {
        let field = match prop.name.as_str() {
            "w:jc" => Alignment,
            "w:keepNext" => KeepWithNext,
            "w:keepLines" => KeepTogether,
            "w:pageBreakBefore" => PageBreakBefore,
            "w:widowControl" => WidowControl,
            "w:suppressLineNumbers" => SuppressLineNumbers,
            "w:bidi" => ParagraphBidi,
            "w:snapToGrid" => SnapToGrid,
            "w:contextualSpacing" => ContextualSpacing,
            "w:mirrorIndents" => MirrorIndents,
            "w:spacing" if allow_direct => {
                attrs(prop, &["w:before", "w:after"])?;
                content(prop, Children::Empty)?;
                for (attr, field) in [("w:before", SpaceBefore), ("w:after", SpaceAfter)] {
                    if prop.get(attr).is_some() && !values.contains_key(&field) {
                        return Err(format!("unreadable w:spacing attribute {attr}"));
                    }
                }
                continue;
            }
            "w:ind" if allow_direct => {
                indents(prop)?;
                continue;
            }
            // An empty paragraph-mark formatting container is a no-op.
            "w:rPr" => {
                attrs(prop, &[])?;
                content(prop, Children::Empty)?;
                continue;
            }
            _ => return Err(format!("unsupported paragraph property {}", prop.name)),
        };
        attrs(prop, &["w:val"])?;
        content(prop, Children::Empty)?;
        if !values.contains_key(&field) {
            return Err(format!("unreadable paragraph property {}", prop.name));
        }
    }
    Ok(())
}

fn indents(node: &Node) -> Check {
    attrs(node, &["w:left", "w:right", "w:firstLine", "w:hanging"])?;
    content(node, Children::Empty)?;
    if node.get("w:firstLine").is_some() && node.get("w:hanging").is_some() {
        return Err("w:ind cannot contain both w:firstLine and w:hanging".into());
    }
    // The typed view aliases both first-line attributes into a signed field
    // (and omits invalid hanging values). Check the original magnitudes here.
    let twips = |attr| {
        let Some(raw) = node.get(attr) else {
            return Ok(0);
        };
        let value = raw
            .parse::<i32>()
            .map_err(|_| format!("unreadable w:ind attribute {attr}"))?;
        if value < 0 {
            return Err(format!("negative w:ind attribute {attr}"));
        }
        Ok(value)
    };
    let left = twips("w:left")?;
    twips("w:right")?;
    twips("w:firstLine")?;
    if twips("w:hanging")? > left {
        return Err("paragraph hanging indent crosses the content's left edge".into());
    }
    Ok(())
}

fn run(node: &Node) -> Check {
    attrs(node, RUN_IDS)?;
    content(node, Children::Elements)?;
    for (index, child) in node.elements().enumerate() {
        match child.name.as_str() {
            "w:rPr" if index == 0 => {
                attrs(child, &[])?;
                content(child, Children::Empty)?;
            }
            "w:rPr" => return Err("duplicate or misplaced w:rPr".into()),
            "w:t" => {
                attrs(child, &["xml:space"])?;
                content(child, Children::Text)?;
                match child.get("xml:space") {
                    Some("preserve") => (),
                    None | Some("default") => {
                        let mut pieces = child.children.iter().filter_map(|item| match item {
                            Content::Text(s) if !s.is_empty() => Some(s.as_str()),
                            _ => None,
                        });
                        let leading = pieces.clone().next().is_some_and(|s| s.starts_with(' '));
                        let trailing = pieces.next_back().is_some_and(|s| s.ends_with(' '));
                        if leading || trailing {
                            return Err("w:t boundary spaces require xml:space=\"preserve\"".into());
                        }
                    }
                    Some(_) => return Err("unsupported w:t xml:space value".into()),
                }
            }
            "w:br" => {
                attrs(child, &["w:type", "w:clear"])?;
                content(child, Children::Empty)?;
                if !matches!(child.get("w:clear"), None | Some("none")) {
                    return Err("unsupported w:br w:clear value".into());
                }
            }
            "w:cr" => {
                attrs(child, &[])?;
                content(child, Children::Empty)?;
            }
            _ => return Err(format!("unsupported {}", child.name)),
        }
    }
    Ok(())
}

fn section(props: &Node) -> Check {
    use LayoutField::*;
    attrs(props, SECTION_IDS)?;
    content(props, Children::Elements)?;
    unique_properties(props)?;
    // Use the same lexical decoder as the typed view, but verify that it
    // produced a value for each explicit attribute rather than omitting it.
    let values = layout::inspect(Some(props), true);
    for prop in props.elements() {
        let fields: &[(&str, LayoutField)] = match prop.name.as_str() {
            "w:pgSz" => &[
                ("w:w", PageWidth),
                ("w:h", PageHeight),
                ("w:orient", Orientation),
            ],
            "w:pgMar" => &[
                ("w:left", MarginLeft),
                ("w:right", MarginRight),
                ("w:top", MarginTop),
                ("w:bottom", MarginBottom),
                ("w:header", MarginHeader),
                ("w:footer", MarginFooter),
                ("w:gutter", MarginGutter),
            ],
            "w:cols" => &[
                ("w:num", ColumnCount),
                ("w:space", ColumnSpace),
                ("w:sep", ColumnSeparator),
            ],
            "w:docGrid" => &[
                ("w:type", GridMode),
                ("w:linePitch", GridPitch),
                ("w:charSpace", GridCharSpace),
            ],
            "w:type" => &[("w:val", SectionType)],
            "w:vAlign" => &[("w:val", VerticalAlign)],
            "w:titlePg" => &[("w:val", TitlePage)],
            "w:bidi" => &[("w:val", Bidi)],
            "w:rtlGutter" => &[("w:val", RtlGutter)],
            _ => return Err(format!("unsupported section property {}", prop.name)),
        };
        content(prop, Children::Empty)?;
        for (name, value) in &prop.attrs {
            if name.declaration().is_some() {
                continue;
            }
            if prop.name == "w:cols"
                && name == "w:equalWidth"
                && matches!(value.as_str(), "true" | "on" | "1")
            {
                continue;
            }
            let Some((_, field)) = fields.iter().find(|(key, _)| name == *key) else {
                return Err(format!("unsupported {} attribute {name}", prop.name));
            };
            if !values.contains_key(field) {
                return Err(format!("unreadable {} attribute {name}", prop.name));
            }
        }
        if let [("w:val", field)] = fields
            && !values.contains_key(field)
        {
            return Err(format!("unreadable section property {}", prop.name));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_body(body: &str) -> Result<()> {
        check_body_for(body, false)
    }

    fn check_body_for(body: &str, allow_direct: bool) -> Result<()> {
        let source = format!(
            "<w:document xmlns:w='{}' \
                xmlns:w14='http://schemas.microsoft.com/office/word/2010/wordml' \
                xmlns:mc='http://schemas.openxmlformats.org/markup-compatibility/2006' \
                mc:Ignorable='w14'><w:body>{body}</w:body></w:document>",
            crate::xml::W,
        );
        verify_document(&Node::parse(source.as_bytes())?, allow_direct)
    }

    #[test]
    fn refuses_paragraph_and_run_markup_omitted_from_typed_views() {
        let cases = [
            (
                "<w:pPr><w:ind w:start='100'/></w:pPr>",
                "unsupported paragraph property w:ind",
            ),
            (
                "<w:pPr><w:spacing w:afterLines='100'/></w:pPr>",
                "unsupported paragraph property w:spacing",
            ),
            (
                "<w:pPr><w:textDirection w:val='tbRl'/></w:pPr>",
                "unsupported paragraph property w:textDirection",
            ),
            (
                "<w:pPr><w:rPr><w:sz w:val='36'/></w:rPr></w:pPr>",
                "unsupported content in w:rPr",
            ),
            (
                "<w:pPr><w:bidi w:val='maybe'/></w:pPr>",
                "unreadable paragraph property w:bidi",
            ),
            (
                "<w:pPr><w:jc w:val='left'/><w:jc w:val='center'/></w:pPr>",
                "duplicate w:jc in w:pPr",
            ),
            ("<w:pPr/><w:pPr/>", "duplicate or misplaced w:pPr"),
            ("<w:r w14:unknown='1'/>", "unsupported w:r attribute"),
            (
                "<w:r><w:t w:val='extra'>word</w:t></w:r>",
                "unsupported w:t attribute w:val",
            ),
            (
                "<w:r><w:t> word </w:t></w:r>",
                "w:t boundary spaces require xml:space",
            ),
            (
                "<w:r><w:br><w:t>hidden</w:t></w:br></w:r>",
                "unsupported content in w:br",
            ),
        ];
        for (markup, expected) in cases {
            let err = check_body(&format!("<w:p>{markup}</w:p><w:sectPr/>"))
                .unwrap_err()
                .to_string();
            assert!(err.contains(expected), "{markup}: {err}");
        }
    }

    #[test]
    fn refuses_columns_and_section_markup_omitted_from_typed_views() {
        let cases = [
            (
                "<w:cols><w:col w:w='300'/></w:cols>",
                "unsupported content in w:cols",
            ),
            (
                "<w:cols w:equalWidth='false'/>",
                "unsupported w:cols attribute w:equalWidth",
            ),
            ("<w:cols w:num='0'/>", "unreadable w:cols attribute w:num"),
            (
                "<w:cols/><w:cols w:num='2'/>",
                "duplicate w:cols in w:sectPr",
            ),
            (
                "<w:docGrid w:type='unknown'/>",
                "unreadable w:docGrid attribute w:type",
            ),
            (
                "<w:pgMar w:gutter='bogus'/>",
                "unreadable w:pgMar attribute w:gutter",
            ),
            (
                "<w:sectPrChange/>",
                "unsupported section property w:sectPrChange",
            ),
        ];
        for (markup, expected) in cases {
            let err = check_body(&format!("<w:sectPr>{markup}</w:sectPr>"))
                .unwrap_err()
                .to_string();
            assert!(err.contains(expected), "{markup}: {err}");
        }
        let err = check_body("<w:sectPr/><w:sectPr/>")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("section properties must be unique and terminal"),
            "{err}"
        );
    }

    #[test]
    fn accepts_known_non_layout_metadata_and_neutral_attributes() {
        check_body(
            "<!-- XML comments have no layout -->\
             <w:p w14:paraId='3ABC34D0' w14:textId='77777777' w:rsidRDefault='00FC693F'>\
             <w:pPr><w:jc w:val='left'/><w:bidi w:val='false'/><w:rPr/></w:pPr>\
             <w:r w:rsidRPr='00FC693F'><w:t xml:space='preserve'> words </w:t>\
             <w:br w:type='textWrapping' w:clear='none'/><w:cr/></w:r></w:p>\
             <w:sectPr w:rsidSect='00034616'><w:cols w:equalWidth='1'/>\
             <w:docGrid w:linePitch='360'/></w:sectPr>",
        )
        .unwrap();
    }

    #[test]
    fn direct_spacing_allows_only_unambiguous_nonnegative_twips() {
        let body = |spacing| format!("<w:p><w:pPr>{spacing}</w:pPr></w:p><w:sectPr/>");
        let direct = body("<w:spacing w:before='0' w:after='13'/>");
        check_body_for(&direct, true).unwrap();
        assert!(check_body(&direct).is_err());
        for (spacing, expected) in [
            (
                "<w:spacing w:before='-1'/>",
                "unreadable w:spacing attribute w:before",
            ),
            (
                "<w:spacing w:after='abc'/>",
                "unreadable w:spacing attribute w:after",
            ),
            (
                "<w:spacing w:beforeAutospacing='1'/>",
                "unsupported w:spacing attribute w:beforeAutospacing",
            ),
            (
                "<w:spacing w:before='1'/><w:spacing w:after='2'/>",
                "duplicate w:spacing",
            ),
        ] {
            let err = check_body_for(&body(spacing), true)
                .unwrap_err()
                .to_string();
            assert!(err.contains(expected), "{spacing}: {err}");
        }
    }

    #[test]
    fn direct_indent_accepts_only_unambiguous_nonnegative_twip_magnitudes() {
        let body = |indent: &str| format!("<w:p><w:pPr>{indent}</w:pPr></w:p><w:sectPr/>");
        for attrs in [
            "w:left='2' w:hanging='2'",
            "w:right='5' w:firstLine='4'",
            "w:hanging='0'",
        ] {
            let direct = body(&format!("<w:ind {attrs}/>"));
            check_body_for(&direct, true).unwrap();
            assert!(check_body(&direct).is_err());
        }
        for (indent, expected) in [
            (
                "<w:ind w:left='1' w:hanging='2'/>",
                "hanging indent crosses",
            ),
            (
                "<w:ind w:left='2' w:hanging='-1'/>",
                "negative w:ind attribute w:hanging",
            ),
            (
                "<w:ind w:firstLine='-1'/>",
                "negative w:ind attribute w:firstLine",
            ),
            (
                "<w:ind w:firstLine='0' w:hanging='0'/>",
                "cannot contain both",
            ),
            (
                "<w:ind w:left='10' w:leftChars='100'/>",
                "unsupported w:ind attribute w:leftChars",
            ),
            (
                "<w:ind w:start='10'/>",
                "unsupported w:ind attribute w:start",
            ),
            ("<w:ind w:left='1in'/>", "unreadable w:ind attribute w:left"),
            ("<w:ind w:left='1'/><w:ind w:right='2'/>", "duplicate w:ind"),
            (
                "<w:ind w:left='1'><w:pPrChange/></w:ind>",
                "unsupported content in w:ind",
            ),
        ] {
            let err = check_body_for(&body(indent), true).unwrap_err().to_string();
            assert!(err.contains(expected), "{indent}: {err}");
        }
    }
}
