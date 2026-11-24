//! Narrow ECMA-376 Part 3 guards for editable content.
use crate::opc::Package;
use crate::xml::{Node, R, W};
use crate::{Diagnostic, Result, Rule};
use std::{borrow::Cow, collections::BTreeSet};

const MUST_UNDERSTAND: &str =
    "{http://schemas.openxmlformats.org/markup-compatibility/2006}MustUnderstand";
const XML: &str = "http://www.w3.org/XML/1998/namespace";
const PROCESS: &str = "{http://schemas.openxmlformats.org/markup-compatibility/2006}ProcessContent";
const MATH: &str = "http://schemas.openxmlformats.org/officeDocument/2006/math";

// Occurrence edits inspect direct properties in these DrawingML containers.
// Recurse through known vocabulary, but not through opaque extension payloads.
fn inspect_process_children(namespace: Option<&str>) -> bool {
    matches!(
        namespace,
        Some(
            W | MATH
                | "http://schemas.openxmlformats.org/drawingml/2006/main"
                | "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                | "http://schemas.openxmlformats.org/drawingml/2006/picture"
                | "http://schemas.openxmlformats.org/drawingml/2006/lockedCanvas"
                | "http://schemas.openxmlformats.org/drawingml/2006/chart"
        )
    )
}

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
    check_descendants(node, path)?;
    check_process_content(node, node.name.namespace(), &BTreeSet::new(), path)
}

/// Physical child indices and direct-property uniqueness would be wrong if an
/// MCE consumer removed a matching wrapper but retained its children (§9.2).
/// Expand names where they are declared: descendant prefix shadowing does not
/// change inherited rules. Do not parse inside unrelated opaque extensions.
fn check_process_content<'a>(
    node: &'a Node,
    root_ns: Option<&str>,
    inherited: &BTreeSet<(&'a str, &'a str)>,
    path: &str,
) -> Result<()> {
    let mut rules = Cow::Borrowed(inherited);
    if let Some(list) = node.get(PROCESS) {
        for token in list.split_ascii_whitespace() {
            let (uri, local) = token
                .split_once(':')
                .and_then(|(prefix, local)| Some((node.get(&format!("xmlns:{prefix}"))?, local)))
                .ok_or_else(|| {
                    Diagnostic::new(Rule::UnsupportedProfile, "invalid mc:ProcessContent name")
                        .at(path)
                })?;
            rules.to_mut().insert((uri, local));
        }
    }
    if let Some(ns) = node.name.namespace()
        && (rules.contains(&(ns, "*")) || rules.contains(&(ns, node.name.local())))
    {
        return Err(Diagnostic::new(
            Rule::UnsupportedProfile,
            "unsupported mc:ProcessContent wrapper",
        )
        .at(path)
        .into());
    }
    if node.name.namespace() == root_ns || inspect_process_children(node.name.namespace()) {
        for child in node.elements() {
            check_process_content(child, root_ns, &rules, path)?;
        }
    }
    Ok(())
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
