//! ECMA-376 P1 §22.2: selected Extended File Properties.
use crate::opc::Package;
use crate::xml::{Content, Name, Node};
use crate::{Diagnostic, Metadata, Result, Rule};

const NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties";
const REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties";
const MIME: &str = "application/vnd.openxmlformats-officedocument.extended-properties+xml";

fn fail(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(Rule::InvalidContent, message).at("metadata")
}

pub(super) fn apply(package: &mut Package, metadata: &Metadata) -> Result<()> {
    if metadata.company.is_none() && metadata.manager.is_none() {
        return Ok(());
    }
    let links = package.relationships("", REL)?;
    let (path, mut root, new) = match links.as_slice() {
        [] => {
            let preferred = "docProps/app.xml";
            let path = if package
                .parts
                .keys()
                .any(|name| name.eq_ignore_ascii_case(preferred))
            {
                package.unique("docProps/app", "xml")
            } else {
                preferred.into()
            };
            (
                path,
                Node::new(Name::expanded(Some(NS), "Properties", None)),
                true,
            )
        }
        [(id, _)] => {
            let (_, path) = package
                .related("", &format!("#{id}"))?
                .ok_or_else(|| fail("missing extended properties relationship"))?;
            if package
                .content_type_for(&path)?
                .is_none_or(|mime| !mime.eq_ignore_ascii_case(MIME))
            {
                return Err(fail("incorrect extended properties content type").into());
            }
            let root = package.xml(&path)?;
            if root.name.namespace() != Some(NS) || root.name.local() != "Properties" {
                return Err(fail("incorrect extended properties root").into());
            }
            (path, root, false)
        }
        _ => return Err(fail("multiple extended properties relationships").into()),
    };
    for (name, value) in [
        ("Company", &metadata.company),
        ("Manager", &metadata.manager),
    ] {
        let Some(value) = value else { continue };
        let missing = {
            let mut fields = root
                .elements_mut()
                .filter(|node| node.name.namespace() == Some(NS) && node.name.local() == name);
            match (fields.next(), fields.next()) {
                (None, _) => true,
                (Some(field), None) if field.elements().next().is_none() => {
                    field.children = vec![Content::Text(value.clone())];
                    false
                }
                _ => {
                    return Err(
                        fail(format!("invalid or duplicate extended property {name}")).into(),
                    );
                }
            }
        };
        if missing {
            root.push(Node::new(Name::expanded(Some(NS), name, None)).text(value));
        }
    }
    package.put(&path, root)?;
    if new {
        package.content_type(&path, MIME)?;
        package.link("", REL, &path)?;
    }
    Ok(())
}
