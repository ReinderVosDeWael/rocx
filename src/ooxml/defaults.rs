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
                p.parts.insert(path.into(), bytes);
            }
            p.link(main, kind, path)?;
            p.content_type(path, mime)?;
        }
    }
    let mut drawing = 0;
    for (name, data) in &p.parts {
        if name.ends_with(".xml")
            && (data.windows(5).any(|w| w == b"docPr" || w == b"cNvPr")
                || data.starts_with(&[0xff, 0xfe])
                || data.starts_with(&[0xfe, 0xff]))
            && let Ok(node) = Node::parse(data)
        {
            fn max_id(n: &Node) -> u32 {
                n.elements()
                    .map(max_id)
                    .chain(
                        matches!(n.name.as_str(), "wp:docPr" | "a:cNvPr" | "pic:cNvPr")
                            .then(|| n.get("id").and_then(|i| i.parse().ok()))
                            .flatten(),
                    )
                    .max()
                    .unwrap_or(0)
            }
            drawing = drawing.max(max_id(&node));
        }
    }
    Ok(drawing)
}
