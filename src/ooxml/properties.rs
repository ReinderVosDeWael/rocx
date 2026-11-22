//! OPC core properties: ECMA-376 Part 2 (2021), §8.
use crate::opc::Package;
use crate::xml::Node;
use crate::{Error, ResolvedDocument, Result};
mod custom;
mod extended;
pub(crate) fn metadata(
    package: &mut Package,
    doc: &ResolvedDocument,
    timestamp: &str,
) -> Result<()> {
    let core_path = package
        .related("", "core-properties")?
        .map(|(_, p)| p)
        .unwrap_or("docProps/core.xml".into());
    let mut core = if package.parts.contains_key(&core_path) {
        package.word(&core_path)?
    } else {
        crate::ooxml::root("cp:coreProperties")
    };
    let m = &doc.metadata;
    for (name, value) in [
        ("dc:creator", &m.creator),
        ("dc:title", &m.title),
        ("dc:subject", &m.subject),
        (
            "dc:description",
            if m.comments.as_ref().is_some_and(|s| !s.is_empty()) {
                &m.comments
            } else {
                &m.description
            },
        ),
        ("cp:keywords", &m.keywords),
        ("cp:category", &m.category),
        ("cp:version", &m.version),
    ] {
        if let Some(s) = value {
            if s.is_empty() {
                continue;
            }
            if s.chars().count() > 255 {
                return Err(Error::Validation(format!(
                    "metadata {name} exceeds 255 characters"
                )));
            }
            core.replace(Node::new(name).text(s));
        }
    }
    for name in ["dcterms:created", "dcterms:modified"] {
        core.replace(
            Node::new(name)
                // QName-valued attributes need their own explicit binding when
                // an imported root shadows the conventional prefix.
                .attr("xmlns:dcterms", "http://purl.org/dc/terms/")
                .attr("xsi:type", "dcterms:W3CDTF")
                .text(timestamp),
        );
    }
    package.put_word(&core_path, core)?;
    package.relation(
        "",
        "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties",
        &core_path,
    )?;
    package.content_type(
        &core_path,
        "application/vnd.openxmlformats-package.core-properties+xml",
    )?;
    custom::apply(package, m)?;
    extended::apply(package, m)
}

use crate::ooxml::WordParts;
