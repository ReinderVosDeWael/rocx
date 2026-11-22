//! Narrow ECMA-376 Part 3 guard for `MustUnderstand` in editable content.
use crate::opc::Package;
use crate::xml::{Node, R, W};
use crate::{Diagnostic, Result, Rule};

const MUST_UNDERSTAND: &str =
    "{http://schemas.openxmlformats.org/markup-compatibility/2006}MustUnderstand";
const XML: &str = "http://www.w3.org/XML/1998/namespace";

pub(super) fn check(package: &Package, path: &str, node: &Node) -> Result<()> {
    if let Some(prefixes) = node.get(MUST_UNDERSTAND) {
        // Only the root may contain inferred baseline bindings. A prefix-valued
        // attribute refers to source bindings, not those defaults.
        let source = package
            .parts
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(path))
            .map(|(_, bytes)| Node::parse_original(bytes))
            .transpose()?;
        check_bindings(prefixes, source.as_ref(), path)?;
    }
    check_descendants(node, path)
}

fn check_descendants(node: &Node, path: &str) -> Result<()> {
    // Imported descendants retain all in-scope source bindings. Foreign
    // subtrees remain opaque, including all AlternateContent alternatives.
    for child in node
        .elements()
        .filter(|child| child.name.namespace() == Some(W))
    {
        if let Some(prefixes) = child.get(MUST_UNDERSTAND) {
            check_bindings(prefixes, Some(child), path)?;
        }
        check_descendants(child, path)?;
    }
    Ok(())
}

fn check_bindings(prefixes: &str, source: Option<&Node>, path: &str) -> Result<()> {
    for prefix in prefixes.split_ascii_whitespace() {
        let uri = if prefix == "xml" {
            Some(XML)
        } else {
            source.and_then(|node| node.get(&format!("xmlns:{prefix}")))
        };
        if !matches!(uri, Some(W | R | XML)) {
            let message = match uri {
                None => format!("unbound prefix {prefix:?} in mc:MustUnderstand"),
                Some(uri) => {
                    format!("unsupported mandatory namespace {uri:?} in mc:MustUnderstand")
                }
            };
            return Err(Diagnostic::new(Rule::UnsupportedProfile, message)
                .at(path)
                .into());
        }
    }
    Ok(())
}
