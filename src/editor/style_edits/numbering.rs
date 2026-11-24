//! Bind a named style to an existing concrete instance; create no lists or levels.
use super::{ImportedDocument, Node, Package, Result, WordParts, unique_child, unsupported};
use crate::{NumberingStyleDefinition, editor::ALTERNATE, xml::val};

fn numeric(n: &Node, key: &str) -> Result<u32> {
    n.get(key)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| unsupported("invalid numbering identity"))
}

pub(super) fn add(
    doc: &ImportedDocument,
    package: &mut Package,
    root: &mut Node,
    id: &str,
    s: &NumberingStyleDefinition,
) -> Result<()> {
    if s.numbering_id == 0 || package.relationships(&doc.main, "numbering")?.len() != 1 {
        return Err(unsupported(
            "numbering style requires an existing numbering instance",
        ));
    }
    let (_, path) = package
        .related(&doc.main, "numbering")?
        .ok_or_else(|| unsupported("missing numbering part"))?;
    if package.content_type_for(&path)?.as_deref()
        != Some("application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml")
    {
        return Err(unsupported("invalid numbering content type"));
    }
    let mut nums = package.word(&path)?;
    if nums.name != "w:numbering" || nums.child(ALTERNATE).is_some() {
        return Err(unsupported("invalid or ambiguous numbering part"));
    }
    let mut concrete = std::collections::BTreeMap::new();
    let mut abstracts = std::collections::BTreeMap::new();
    for (i, child) in nums.children.iter().enumerate() {
        let crate::xml::Content::Element(n) = child else {
            continue;
        };
        let duplicate = match n.name.as_str() {
            "w:num" => {
                let abs = unique_child(n, "w:abstractNumId")?
                    .ok_or_else(|| unsupported("missing abstract numbering reference"))?;
                if n.child(ALTERNATE).is_some() {
                    return Err(unsupported("ambiguous numbering instance"));
                }
                concrete
                    .insert(numeric(n, "w:numId")?, numeric(abs, "w:val")?)
                    .is_some()
            }
            "w:abstractNum" => abstracts
                .insert(numeric(n, "w:abstractNumId")?, i)
                .is_some(),
            _ => false,
        };
        if duplicate {
            return Err(unsupported("duplicate numbering identity"));
        }
    }
    let abs = concrete
        .get(&s.numbering_id)
        .ok_or_else(|| unsupported("missing numbering instance"))?;
    let index = abstracts
        .get(abs)
        .ok_or_else(|| unsupported("missing abstract numbering definition"))?;
    // Do not claim an abstract that another numbering style already uses,
    // even through a different concrete instance or without a reverse link.
    'styles: for style in root
        .elements()
        .filter(|n| n.name == "w:style" && n.get("w:type") == Some("numbering"))
    {
        let mut leaf = style;
        for tag in ["w:pPr", "w:numPr", "w:numId"] {
            if leaf.child(ALTERNATE).is_some() {
                return Err(unsupported("uninspectable numbering style"));
            }
            let Some(next) = unique_child(leaf, tag)? else {
                continue 'styles;
            };
            leaf = next;
        }
        if concrete.get(&numeric(leaf, "w:val")?) == Some(abs) {
            return Err(unsupported("abstract already owns a numbering style"));
        }
    }
    let crate::xml::Content::Element(node) = &mut nums.children[*index] else {
        unreachable!()
    };
    if node.child(ALTERNATE).is_some()
        || node.child("w:lvl").is_none()
        || node.child("w:numStyleLink").is_some()
    {
        return Err(unsupported(
            "numbering style requires a local abstract with levels",
        ));
    }
    if let Some(link) = unique_child(node, "w:styleLink")? {
        // A second style validation pass sees the link from our first pass.
        let original = doc.package.word(&path)?;
        if link.get("w:val") != Some(id)
            || original
                .elements()
                .filter(|n| {
                    n.name == "w:abstractNum"
                        && numeric(n, "w:abstractNumId").ok().as_ref() == Some(abs)
                })
                .any(|n| n.child("w:styleLink").is_some())
        {
            return Err(unsupported("abstract already owns a numbering style"));
        }
    } else {
        node.push(val("styleLink", id));
        package.put_word(&path, nums)?;
    }
    root.push(
        Node::new("w:style")
            .attr("w:type", "numbering")
            .attr("w:customStyle", "1")
            .attr("w:styleId", id)
            .with(val("name", &s.name))
            .with(Node::new("w:pPr").with(Node::new("w:numPr").with(val("numId", s.numbering_id)))),
    );
    Ok(())
}
