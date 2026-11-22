//! ECMA-376 P1 §22.3: named custom properties with variant values.
use crate::opc::Package;
use crate::xml::{Content, Name, Node};
use crate::{Diagnostic, Metadata, Result, Rule};
use std::collections::{BTreeMap, BTreeSet};

const CP: &str = "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties";
const VT: &str = "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes";
const REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties";
const MIME: &str = "application/vnd.openxmlformats-officedocument.custom-properties+xml";
const FMTID: &str = "{D5CDD505-2E9C-101B-9397-08002B2CF9AE}";

fn fail(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(Rule::InvalidContent, message).at("metadata.custom_properties")
}

pub(super) fn apply(package: &mut Package, metadata: &Metadata) -> Result<()> {
    if metadata.custom_properties.is_empty() && metadata.custom_boolean_properties.is_empty() {
        return Ok(());
    }
    let values: BTreeMap<_, _> = metadata
        .custom_properties
        .iter()
        .map(|(name, text)| (name.as_str(), ("lpwstr", text.as_str(), "string")))
        .chain(
            metadata
                .custom_boolean_properties
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str(),
                        ("bool", if *value { "true" } else { "false" }, "boolean"),
                    )
                }),
        )
        .collect();
    let links = package.relationships("", REL)?;
    let (path, mut root, new) = match links.as_slice() {
        [] => {
            let preferred = "docProps/custom.xml";
            let path = if package
                .parts
                .keys()
                .any(|name| name.eq_ignore_ascii_case(preferred))
            {
                package.unique("docProps/custom", "xml")
            } else {
                preferred.into()
            };
            (
                path,
                Node::new(Name::expanded(Some(CP), "Properties", None)),
                true,
            )
        }
        [(id, _)] => {
            let (_, path) = package
                .related("", &format!("#{id}"))?
                .ok_or_else(|| fail("missing custom properties relationship"))?;
            if package
                .content_type_for(&path)?
                .is_none_or(|mime| !mime.eq_ignore_ascii_case(MIME))
            {
                return Err(fail("incorrect custom properties content type").into());
            }
            let root = package.xml(&path)?;
            if root.name.namespace() != Some(CP) || root.name.local() != "Properties" {
                return Err(fail("incorrect custom properties root").into());
            }
            (path, root, false)
        }
        _ => return Err(fail("multiple custom properties relationships").into()),
    };

    let mut ids = BTreeSet::new();
    let mut matches = BTreeMap::new();
    for (index, child) in root.children.iter().enumerate() {
        let Content::Element(property) = child else {
            continue;
        };
        if property.name.namespace() != Some(CP) || property.name.local() != "property" {
            continue;
        }
        let pid = property
            .get("pid")
            .and_then(|id| id.trim().parse::<i32>().ok())
            .filter(|pid| *pid >= 2)
            .ok_or_else(|| fail("invalid imported custom property ID"))?;
        if !ids.insert(pid) {
            return Err(fail("duplicate imported custom property ID").into());
        }
        if let Some(name) = property.get("name")
            && values.contains_key(name)
            && matches.insert(name.to_owned(), index).is_some()
        {
            return Err(fail(format!("ambiguous imported custom property {name:?}")).into());
        }
    }

    let mut free = 2;
    for (name, (tag, text, kind)) in values {
        if let Some(&index) = matches.get(name) {
            let Content::Element(property) = &mut root.children[index] else {
                unreachable!()
            };
            let link = property.get("linkTarget").is_some();
            let mut children = property.elements_mut();
            match (link, children.next(), children.next()) {
                (false, Some(value), None)
                    if value.name.namespace() == Some(VT)
                        && value.name.local() == tag
                        && value.elements().next().is_none() =>
                {
                    value.children = vec![Content::Text(text.to_owned())];
                }
                _ => {
                    return Err(fail(format!(
                        "custom property {name:?} is not an unlinked {kind}"
                    ))
                    .into());
                }
            }
        } else {
            while ids.contains(&free) {
                free = free
                    .checked_add(1)
                    .ok_or_else(|| fail("custom property IDs exhausted"))?;
            }
            root.push(
                Node::new(Name::expanded(Some(CP), "property", None))
                    .attr("fmtid", FMTID)
                    .attr("pid", free)
                    .attr("name", name)
                    .with(Node::new(Name::expanded(Some(VT), tag, Some("vt"))).text(text)),
            );
            ids.insert(free);
        }
    }
    package.put(&path, root)?;
    if new {
        package.content_type(&path, MIME)?;
        package.link("", REL, &path)?;
    }
    Ok(())
}
