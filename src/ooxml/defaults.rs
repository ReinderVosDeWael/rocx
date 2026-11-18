//! Word defaults and story discovery, above generic OPC (P1 §11.3).
use crate::Result;
use crate::opc::Package;
use crate::xml::Node;
use std::io::{Cursor, Read};
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
            && (data.windows(5).any(|w| w == b"docPr")
                || data.starts_with(&[0xff, 0xfe])
                || data.starts_with(&[0xfe, 0xff]))
            && let Ok(node) = Node::parse(data)
        {
            fn max_id(n: &Node) -> u32 {
                n.elements()
                    .map(max_id)
                    .chain(n.get("id").and_then(|i| i.parse().ok()))
                    .max()
                    .unwrap_or(0)
            }
            drawing = drawing.max(max_id(&node));
        }
    }
    Ok(drawing)
}
