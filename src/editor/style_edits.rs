//! Identity-preserving edits to the relationship-selected style catalog.
use super::{ImportedDocument, unsupported};
use crate::{
    Result, RunFormat, StyleDefinition,
    ooxml::{
        WordParts,
        wordprocessing::{format, styles},
    },
    opc::Package,
    xml::{Content, Node},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
mod fields;
mod numbering;

#[derive(Clone, Debug)]
/// Exact style identity issued by one imported document snapshot.
pub struct StyleAnchor {
    snapshot: Arc<()>,
    id: String,
}

#[derive(Clone, Debug)]
/// Definition edits against the selected catalog. Omitted patch flags are retained.
pub enum StyleEdit {
    Add(StyleDefinition),
    /// Replace or clear one physical conditional table-style wrapper.
    Region {
        target: StyleAnchor,
        region: String,
        format: Option<Box<crate::TableSectionFormat>>,
    },
    /// Set or clear a selected direct scalar field, optionally in a table region.
    Field {
        target: StyleAnchor,
        region: Option<String>,
        field: String,
        value: Option<String>,
    },
    Patch {
        target: StyleAnchor,
        bold: Option<bool>,
        italic: Option<bool>,
    },
    Replace {
        target: StyleAnchor,
        definition: StyleDefinition,
    },
    Remove(StyleAnchor),
}

fn identity(def: &StyleDefinition) -> (&str, &str) {
    match def {
        StyleDefinition::Character(s) => (&s.name, "character"),
        StyleDefinition::Paragraph(s) => (&s.name, "paragraph"),
        StyleDefinition::Table(s) => (&s.name, "table"),
        StyleDefinition::Numbering(s) => (&s.name, "numbering"),
    }
}

fn unique_child<'a>(node: &'a Node, name: &str) -> Result<Option<&'a Node>> {
    let mut matches = node.elements().filter(|n| n.name == name);
    let first = matches.next();
    if matches.next().is_some() {
        return Err(unsupported("duplicate style property"));
    }
    Ok(first)
}

// The authoring codec accepts both IDs and display names; its lookup must be
// unambiguous before using it on an imported catalog.
fn claim(index: &mut BTreeMap<String, String>, alias: &str, id: &str) -> Result<()> {
    if alias.trim().is_empty()
        || index
            .insert(alias.to_ascii_lowercase(), id.to_owned())
            .is_some_and(|old| old != id)
    {
        return Err(unsupported("empty or ambiguous style identity"));
    }
    Ok(())
}

fn nested_style(node: &Node) -> bool {
    node.elements()
        .any(|n| n.name == "w:style" || nested_style(n))
}

fn discardable(n: &Node) -> bool {
    n.name == "w:style"
        && n.get("w:customStyle")
            .is_some_and(|v| matches!(v, "1" | "on" | "true"))
        && matches!(n.get("w:type"), Some("paragraph" | "character" | "table"))
        && n.attrs.iter().all(|(a, _)| {
            a.declaration().is_some()
                || matches!(a.as_str(), "w:type" | "w:styleId" | "w:customStyle")
        })
        && n.children.iter().all(|c| match c {
            Content::Text(t) => t.trim().is_empty(),
            Content::Element(e) => fields::known(e, "w:style"),
            Content::Raw(_) => false,
        })
}

pub(super) fn opaque_reference_context(n: &Node) -> bool {
    // Fields can assemble style names across runs, or use aliases. Also
    // decline deferred content and externally supplied style definitions.
    let baseline_extension = matches!(
        n.name.as_str(),
        "{http://schemas.microsoft.com/office/word/2010/wordml}docId"
            | "{http://schemas.microsoft.com/office/word/2010/wordml}defaultImageDpi"
            | "{urn:schemas-microsoft-com:office:office}shapedefaults"
            | "{urn:schemas-microsoft-com:office:office}shapelayout"
            | "{urn:schemas-microsoft-com:office:office}idmap"
    );
    (!baseline_extension
        && !matches!(
            n.name.namespace(),
            Some(crate::xml::W | "http://schemas.openxmlformats.org/officeDocument/2006/math")
        ))
        || matches!(
            n.name.as_str(),
            "w:fldSimple"
                | "w:fldChar"
                | "w:instrText"
                | "w:delInstrText"
                | "w:altChunk"
                | "w:subDoc"
                | "w:attachedTemplate"
        )
        || n.attrs.iter().any(|(a, _)| {
            a.declaration().is_none()
                && (!baseline_extension
                    && !matches!(
                        a.namespace(),
                        Some(
                            crate::xml::W
                                | crate::xml::R
                                | "http://www.w3.org/XML/1998/namespace"
                                | "http://schemas.openxmlformats.org/officeDocument/2006/math"
                        )
                    )
                    && a.as_str()
                        != "{http://schemas.openxmlformats.org/markup-compatibility/2006}Ignorable")
        })
}

fn node_may_reference(n: &Node, id: &str, name: &str) -> bool {
    opaque_reference_context(n)
        || n.attrs.iter().any(|(a, v)| {
            a.declaration().is_none()
                && (v.eq_ignore_ascii_case(id)
                    || v.eq_ignore_ascii_case(name)
                    || v.split(',').any(|s| {
                        s.trim().eq_ignore_ascii_case(id) || s.trim().eq_ignore_ascii_case(name)
                    }))
        })
}

fn possible_reference(n: &Node, id: &str, name: &str) -> bool {
    node_may_reference(n, id, name)
        || n.elements()
            .any(|child| possible_reference(child, id, name))
}

fn check_unused(package: &Package, path: &str, root: &Node, target: &Node) -> Result<()> {
    if !discardable(target) {
        return Err(unsupported(
            "style is built-in, default, or has unsupported markup",
        ));
    }
    let id = target.get("w:styleId").unwrap();
    let name = unique_child(target, "w:name")?
        .and_then(|n| n.get("w:val"))
        .ok_or_else(|| unsupported("missing style name"))?;
    if node_may_reference(root, id, name)
        || root
            .elements()
            .any(|n| !std::ptr::eq(n, target) && possible_reference(n, id, name))
    {
        return Err(unsupported("style has live or uninspectable references"));
    }
    check_other_parts(package, path, |n| possible_reference(n, id, name))
}

pub(super) fn check_other_parts(
    package: &Package,
    path: &str,
    may_reference: impl Fn(&Node) -> bool,
) -> Result<()> {
    for part in package
        .parts
        .keys()
        .filter(|p| !p.eq_ignore_ascii_case(path))
    {
        if part == "[Content_Types].xml" {
            continue;
        }
        let typ = package
            .content_type_for(part)?
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(
            typ.as_str(),
            "image/png"
                | "image/jpeg"
                | "image/gif"
                | "image/bmp"
                | "image/tiff"
                | "application/vnd.openxmlformats-package.relationships+xml"
                | "application/vnd.openxmlformats-package.core-properties+xml"
                | "application/vnd.openxmlformats-officedocument.extended-properties+xml"
                | "application/vnd.openxmlformats-officedocument.custom-properties+xml"
                | "application/vnd.openxmlformats-officedocument.theme+xml"
                | "application/vnd.openxmlformats-officedocument.customxmlproperties+xml"
        ) {
            continue;
        }
        let word = typ
            .starts_with("application/vnd.openxmlformats-officedocument.wordprocessingml.")
            && typ.ends_with("+xml")
            || typ == "application/vnd.ms-word.styleswitheffects+xml";
        // The stock Word template ships an empty bibliography data store.
        if typ == "application/xml" {
            let n = package.xml(part)?;
            if n.name.as_str()
                == "{http://schemas.openxmlformats.org/officeDocument/2006/bibliography}Sources"
                && n.elements().next().is_none()
            {
                continue;
            }
        }
        if !word || may_reference(&package.word(part)?) {
            return Err(unsupported("resource has live or uninspectable references"));
        }
    }
    Ok(())
}

// Validate the affected inheritance chains in the final catalog, including
// imported ancestors. A next-style self-loop and reciprocal style links are
// legitimate: only basedOn is an inheritance edge.
fn validate_chains(root: &Node, affected: &BTreeSet<String>) -> Result<()> {
    let by_id: BTreeMap<_, _> = root
        .elements()
        .filter(|n| n.name == "w:style")
        .filter_map(|n| Some((n.get("w:styleId")?, n)))
        .collect();
    let mut done = BTreeSet::new();
    for id in affected {
        let mut chain = BTreeSet::new();
        let mut current = Some(id.as_str());
        while let Some(id) = current {
            if done.contains(id) {
                break;
            }
            if !chain.insert(id) {
                return Err(unsupported("style inheritance cycle"));
            }
            let n = by_id
                .get(id)
                .ok_or_else(|| unsupported("missing style ID"))?;
            let kind = n.get("w:type");
            current = None;
            for (tag, required) in [
                ("w:basedOn", kind),
                ("w:next", kind.filter(|k| *k == "paragraph")),
                (
                    "w:link",
                    match kind {
                        Some("paragraph") => Some("character"),
                        Some("character") => Some("paragraph"),
                        _ => None,
                    },
                ),
            ] {
                if let Some(property) = unique_child(n, tag)? {
                    let value = property
                        .get("w:val")
                        .ok_or_else(|| unsupported("missing style reference"))?;
                    if required.is_none()
                        || by_id.get(value).and_then(|n| n.get("w:type")) != required
                    {
                        return Err(unsupported("style reference missing or has wrong type"));
                    }
                    if tag == "w:basedOn" {
                        current = Some(value);
                    }
                }
            }
        }
        done.extend(chain);
    }
    Ok(())
}

fn patch_flags(style: &mut Node, bold: Option<bool>, italic: Option<bool>) -> Result<bool> {
    if !matches!(style.get("w:type"), Some("paragraph" | "character")) {
        return Err(unsupported(
            "run properties require a paragraph or character style",
        ));
    }
    let props = format::run(&RunFormat {
        bold,
        italic,
        ..Default::default()
    })?;
    let mut changed = false;
    for p in props.elements() {
        let old = unique_child(style, "w:rPr")?
            .map(|n| unique_child(n, p.name.as_str()))
            .transpose()?
            .flatten()
            .and_then(|n| match n.get("w:val") {
                None | Some("1" | "on" | "true") => Some("1"),
                Some("0" | "off" | "false") => Some("0"),
                _ => None,
            });
        if old != p.get("w:val") {
            changed = true;
            style
                .ensure("w:rPr")
                .ensure(p.name.as_str())
                .set("w:val", p.get("w:val").unwrap());
        }
    }
    Ok(changed)
}

impl ImportedDocument {
    /// Recheck references after body payloads and other resources have been
    /// encoded. Do not replay mutations or overwrite a newly staged catalog.
    pub(super) fn check_final_style_uses(
        &self,
        package: &Package,
        edits: &[StyleEdit],
    ) -> Result<()> {
        let (path, source) = self
            .style_part()?
            .ok_or_else(|| unsupported("missing styles part"))?;
        let final_root = package.word(&path)?;
        for edit in edits {
            let anchor = match edit {
                StyleEdit::Remove(a) | StyleEdit::Replace { target: a, .. } => a,
                _ => continue,
            };
            let id = anchor.id.as_str();
            let name = source
                .elements()
                .find(|n| n.get("w:styleId") == Some(id))
                .and_then(|n| n.child("w:name"))
                .and_then(|n| n.get("w:val"))
                .ok_or_else(|| unsupported("missing style identity"))?;
            if node_may_reference(&final_root, id, name)
                || final_root.elements().any(|n| {
                    !(n.name == "w:style" && n.get("w:styleId") == Some(id))
                        && possible_reference(n, id, name)
                })
            {
                return Err(unsupported(
                    "style acquired live or uninspectable references",
                ));
            }
            check_other_parts(package, &path, |n| possible_reference(n, id, name))?;
        }
        Ok(())
    }
    /// Resolve an exact ID in the selected catalog; names are not edit addresses.
    pub fn style_anchor(&self, id: &str) -> Result<StyleAnchor> {
        let (_, root) = self
            .style_part()?
            .ok_or_else(|| unsupported("missing styles part"))?;
        if root
            .elements()
            .filter(|s| s.name == "w:style" && s.get("w:styleId") == Some(id))
            .count()
            != 1
        {
            return Err(unsupported("missing or duplicate style ID"));
        }
        Ok(StyleAnchor {
            snapshot: self.snapshot.clone(),
            id: id.to_owned(),
        })
    }

    pub(super) fn stage_styles(&self, package: &mut Package, edits: &[StyleEdit]) -> Result<()> {
        if edits.is_empty() {
            return Ok(());
        }
        let (path, mut root) = match self.style_part()? {
            Some(part) => part,
            None if edits.iter().all(|e| matches!(e, StyleEdit::Add(_))) => {
                let (path, root) =
                    super::resource_fields::create_word_part(package, &self.main, "styles")?;
                package.put_word(&path, root.clone())?;
                (path, root)
            }
            None => return Err(unsupported("missing styles part")),
        };
        let mut aliases = BTreeMap::new();
        let mut slots = BTreeMap::new();
        for (slot, content) in root.children.iter().enumerate() {
            let Content::Element(node) = content else {
                continue;
            };
            if nested_style(node) || (node.name != "w:style" && node.get("w:styleId").is_some()) {
                return Err(unsupported("opaque style identity"));
            }
            if node.name != "w:style" {
                continue;
            }
            let id = node
                .get("w:styleId")
                .ok_or_else(|| unsupported("missing style ID"))?;
            if slots.insert(id.to_owned(), slot).is_some() {
                return Err(unsupported("duplicate style ID"));
            }
            claim(&mut aliases, id, id)?;
            if let Some(name) = unique_child(node, "w:name")?.and_then(|n| n.get("w:val")) {
                claim(&mut aliases, name, id)?;
            }
            // Fields such as STYLEREF can identify a style by any of its
            // comma-separated alternate names, even without using its ID.
            if let Some(list) = unique_child(node, "w:aliases")? {
                let value = list
                    .get("w:val")
                    .ok_or_else(|| unsupported("missing style aliases value"))?;
                for alias in value.split(',') {
                    claim(&mut aliases, alias.trim(), id)?;
                }
            }
        }
        let mut targets = BTreeSet::new();
        let original = edits
            .iter()
            .any(|edit| matches!(edit, StyleEdit::Replace { .. } | StyleEdit::Remove(_)))
            .then(|| root.clone());
        let mut definitions = Vec::new();
        let mut added = Vec::new();
        let mut added_numbering = false;
        let mut affected = BTreeSet::new();
        let mut replace_slots = BTreeMap::new();
        let mut remove = BTreeSet::new();
        let mut changed = false;
        for edit in edits {
            let target = match edit {
                StyleEdit::Add(_) => None,
                StyleEdit::Patch { target, .. }
                | StyleEdit::Field { target, .. }
                | StyleEdit::Region { target, .. }
                | StyleEdit::Replace { target, .. }
                | StyleEdit::Remove(target) => {
                    self.validate_snapshot(&target.snapshot)?;
                    if !targets.insert(target.id.as_str()) {
                        return Err(unsupported("duplicate style edit target"));
                    }
                    Some(target.id.as_str())
                }
            };
            let slot = target.map(|id| slots[id]);
            if let StyleEdit::Add(def)
            | StyleEdit::Replace {
                definition: def, ..
            } = edit
            {
                let (name, kind) = identity(def);
                let id = name.replace(' ', "");
                if let Some(i) = slot {
                    let Content::Element(n) = &root.children[i] else {
                        unreachable!()
                    };
                    if n.get("w:type") != Some(kind)
                        || target != Some(&id)
                        || n.child("w:name").and_then(|v| v.get("w:val")) != Some(name)
                    {
                        return Err(unsupported(
                            "replacement must preserve style ID, name and type",
                        ));
                    }
                    replace_slots.insert(i, id.clone());
                } else if aliases.contains_key(&id.to_ascii_lowercase())
                    || aliases.contains_key(&name.to_ascii_lowercase())
                {
                    return Err(unsupported("duplicate style ID, name or alias"));
                }
                claim(&mut aliases, &id, &id)?;
                claim(&mut aliases, name, &id)?;
                if slot.is_none() {
                    added.push(id.clone());
                }
                if let StyleDefinition::Numbering(s) = def {
                    if slot.is_some() {
                        return Err(unsupported("cannot replace a linked numbering style"));
                    }
                    numbering::add(self, package, &mut root, &id, s)?;
                    added_numbering = true;
                    changed = true;
                } else {
                    definitions.push(def.clone());
                }
                affected.insert(id);
            }
            if let Some(slot) = slot {
                let Content::Element(style) = &root.children[slot] else {
                    unreachable!()
                };
                if matches!(
                    edit,
                    StyleEdit::Patch { .. } | StyleEdit::Field { .. } | StyleEdit::Region { .. }
                ) {
                    affected.insert(style.get("w:styleId").unwrap().to_owned());
                    let Content::Element(style) = &mut root.children[slot] else {
                        unreachable!()
                    };
                    changed |= match edit {
                        StyleEdit::Region { region, format, .. } => {
                            fields::region(style, region, format.as_deref())?
                        }
                        StyleEdit::Field {
                            region,
                            field,
                            value,
                            ..
                        } => fields::patch(style, region.as_deref(), field, value.as_deref())?,
                        StyleEdit::Patch { bold, italic, .. } => {
                            patch_flags(style, *bold, *italic)?
                        }
                        _ => unreachable!(),
                    };
                } else {
                    let original = original.as_ref().unwrap();
                    let Content::Element(style) = &original.children[slot] else {
                        unreachable!()
                    };
                    check_unused(package, &path, original, style)?;
                    remove.insert(slot);
                }
            }
        }
        if !remove.is_empty() {
            root.children = root
                .children
                .into_iter()
                .enumerate()
                .filter_map(|(i, n)| (!remove.contains(&i)).then_some(n))
                .collect();
            changed = true;
        }
        if changed {
            package.put_word(&path, root)?;
        }
        if !affected.is_empty() {
            let mut final_root = if definitions.is_empty() {
                package.word(&path)?
            } else {
                styles::apply(package, &self.main, &definitions, &[], None)?
            };
            validate_chains(&final_root, &affected)?;
            for id in replace_slots.values() {
                let name = final_root
                    .elements()
                    .find(|n| n.get("w:styleId") == Some(id))
                    .and_then(|n| n.child("w:name"))
                    .and_then(|n| n.get("w:val"))
                    .ok_or_else(|| unsupported("missing replacement name"))?;
                if final_root
                    .elements()
                    .any(|n| n.get("w:styleId") != Some(id) && possible_reference(n, id, name))
                {
                    return Err(unsupported("replacement acquired a live style reference"));
                }
            }
            // The authoring codec appends new definitions. Move replacements
            // back into their original slots, including around opaque siblings.
            if !replace_slots.is_empty() || added_numbering {
                let moving: BTreeSet<_> = replace_slots
                    .values()
                    .chain(&added)
                    .map(String::as_str)
                    .collect();
                let mut nodes = BTreeMap::new();
                final_root.children = final_root
                    .children
                    .into_iter()
                    .filter_map(|c| {
                        if let Content::Element(n) = &c
                            && let Some(id) = n.get("w:styleId").filter(|id| moving.contains(id))
                        {
                            nodes.insert(id.to_owned(), c);
                            return None;
                        }
                        Some(c)
                    })
                    .collect();
                for (restored, (i, id)) in replace_slots.iter().enumerate() {
                    final_root.children.insert(
                        i - remove.range(..*i).count() + restored,
                        nodes.remove(id).unwrap(),
                    );
                }
                for id in added {
                    final_root.children.push(nodes.remove(&id).unwrap());
                }
                package.put_word(&path, final_root)?;
            }
        }
        Ok(())
    }
}
