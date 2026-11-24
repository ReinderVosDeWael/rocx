//! Word defaults and story discovery, above generic OPC (P1 §11.3).
use super::WordParts;
use crate::opc::Package;
use crate::xml::Node;
use crate::{DocumentDefaults, Error, Result};
use std::io::{Cursor, Read};
// P1 (2016) §§17.15.1.50 and 17.15.1.57, document page-layout requests.
pub(super) fn apply_settings(
    package: &mut Package,
    main: &str,
    defaults: &DocumentDefaults,
) -> Result<()> {
    let flags = [
        // P1 (2016) §17.15.1, tracking future edits.
        ("trackRevisions", defaults.track_revisions),
        ("mirrorMargins", defaults.mirror_margins),
        ("gutterAtTop", defaults.gutter_at_top),
    ];
    if flags.iter().all(|(_, value)| value.is_none())
        && defaults.footnote_numbering.is_none()
        && defaults.endnote_numbering.is_none()
        && defaults.footnote_position.is_none()
        && defaults.endnote_position.is_none()
    {
        return Ok(());
    }
    let (_, path) = package
        .related(main, "settings")?
        .ok_or_else(|| Error::Validation("missing settings part".into()))?;
    let mut root = package.word(&path)?;
    for (tag, value) in flags {
        if let Some(value) = value {
            root.replace(crate::xml::val(tag, u8::from(value)));
        }
    }
    super::notes::apply_defaults(&mut root, defaults);
    package.put_word(path, root)
}

pub(super) fn imported_word_values<T: Ord>(
    package: &Package,
    select: fn(&Node) -> Option<T>,
) -> std::collections::BTreeSet<T> {
    fn reserve<T: Ord>(
        node: &Node,
        select: fn(&Node) -> Option<T>,
        ids: &mut std::collections::BTreeSet<T>,
    ) {
        if let Some(id) = select(node) {
            ids.insert(id);
        }
        for child in node.elements() {
            reserve(child, select, ids);
        }
    }
    let mut ids = std::collections::BTreeSet::new();
    for (name, bytes) in &package.parts {
        if (name.ends_with(".xml")
            || bytes.starts_with(b"<")
            || bytes.starts_with(&[0, b'<'])
            || bytes.starts_with(&[0xff, 0xfe])
            || bytes.starts_with(&[0xfe, 0xff])
            || bytes.starts_with(&[0xef, 0xbb, 0xbf]))
            && let Ok(root) = Node::parse(bytes)
        {
            reserve(&root, select, &mut ids);
        }
    }
    ids
}

pub(crate) fn prepare(p: &mut Package, main: &str) -> Result<u32> {
    // Minimal valid OPC templates may omit optional Word parts. Create the
    // same default style/numbering/settings parts that python-docx supplies lazily.
    let missing_styles = p.related(main, "styles")?.is_none();
    let mut defaults = zip::ZipArchive::new(Cursor::new(
        include_bytes!("../../assets/default.docx").as_slice(),
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
        if p.related(main, kind)?.is_none() {
            if !p.parts.contains_key(path) {
                let mut bytes = Vec::new();
                defaults.by_name(path)?.read_to_end(&mut bytes)?;
                p.parts.insert(path.into(), bytes.into());
            }
            p.link(main, kind, path)?;
            p.content_type(path, mime)?;
        }
    }
    Ok(drawing_ceiling(p))
}

pub(crate) fn drawing_node_ceiling(n: &Node) -> u32 {
    n.elements()
        .map(drawing_node_ceiling)
        .chain(
            matches!(n.name.as_str(), "wp:docPr" | "a:cNvPr" | "pic:cNvPr")
                .then(|| n.get("id").and_then(|i| i.trim().parse().ok()))
                .flatten(),
        )
        .max()
        .unwrap_or(0)
}

pub(super) fn drawing_ceiling(p: &Package) -> u32 {
    let mut drawing = 0;
    // OPC part names need not end in .xml, even for Word story parts.
    // Filter by content, skipping binary media before scanning for names.
    for data in p.parts.values() {
        let utf16 = data.starts_with(&[0xff, 0xfe])
            || data.starts_with(&[0xfe, 0xff])
            || data.starts_with(&[0x3c, 0])
            || data.starts_with(&[0, 0x3c]);
        let utf8 = data.strip_prefix(b"\xef\xbb\xbf").unwrap_or(data);
        let looks_like_xml = utf8.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'<');
        if (utf16 || (looks_like_xml && utf8.windows(5).any(|w| w == b"docPr" || w == b"cNvPr")))
            && let Ok(node) = Node::parse(data)
        {
            drawing = drawing.max(drawing_node_ceiling(&node));
        }
    }
    drawing
}

#[cfg(test)]
mod drawing_id_tests {
    use super::*;

    #[test]
    fn reserves_drawing_ids_in_relocated_header_parts() {
        let mut package = Package::load_with_limits(
            include_bytes!("../../assets/default.docx"),
            crate::opc::ReadLimits::default(),
        )
        .unwrap();
        // Same drawing element emitted by python-docx, in a relocated header.
        package.parts.insert(
            "word/header1.part".into(),
            br#"<?xml version="1.0" encoding="UTF-8"?><w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"><w:p><w:r><w:drawing><wp:docPr id="42" name="Picture 1"/></w:drawing></w:r></w:p></w:hdr>"#.as_slice().into(),
        );
        assert_eq!(drawing_ceiling(&package), 42);
    }
}
