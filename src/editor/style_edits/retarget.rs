//! Explicit migration, never best-effort removal: unresolved users abort the batch.
use super::*;

fn rewrite(
    n: &mut Node,
    old: &str,
    new: &str,
    kind: &str,
    owner_kind: Option<&str>,
    parent: &str,
) -> Result<bool> {
    let expected = match (parent, n.name.as_str()) {
        ("w:pPr" | "w:lvl", "w:pStyle") => Some("paragraph"),
        ("w:rPr", "w:rStyle") => Some("character"),
        ("w:tblPr", "w:tblStyle") => Some("table"),
        ("w:style", "w:basedOn") => owner_kind,
        ("w:style", "w:next") if owner_kind == Some("paragraph") => Some("paragraph"),
        ("w:style", "w:link") => match owner_kind {
            Some("paragraph") => Some("character"),
            Some("character") => Some("paragraph"),
            _ => None,
        },
        _ => None,
    };
    let mut changed = false;
    if n.get("w:val") == Some(old) && expected == Some(kind) {
        n.set("w:val", new);
        changed = true;
    }
    // Do not touch historic revisions, content controls, fields or unknown wrappers.
    // The final usage scan rejects residual uses instead of silently rewriting them.
    if !matches!(
        n.name.as_str(),
        "w:document"
            | "w:body"
            | "w:hdr"
            | "w:ftr"
            | "w:comments"
            | "w:comment"
            | "w:footnotes"
            | "w:footnote"
            | "w:endnotes"
            | "w:endnote"
            | "w:tbl"
            | "w:tr"
            | "w:tc"
            | "w:p"
            | "w:r"
            | "w:pPr"
            | "w:rPr"
            | "w:tblPr"
            | "w:hyperlink"
            | "w:numbering"
            | "w:abstractNum"
            | "w:num"
            | "w:lvlOverride"
            | "w:lvl"
            | "w:style"
            | "w:tblStylePr"
    ) {
        return Ok(changed);
    }
    let mut singletons = BTreeSet::new();
    for child in n.elements() {
        if matches!(
            child.name.as_str(),
            "w:pPr"
                | "w:rPr"
                | "w:tblPr"
                | "w:rStyle"
                | "w:pStyle"
                | "w:tblStyle"
                | "w:link"
                | "w:next"
                | "w:basedOn"
        ) && !singletons.insert(child.name.as_str())
        {
            return Err(unsupported("ambiguous style reference path"));
        }
    }
    let name = n.name.clone();
    let owner_kind = if name == "w:style" {
        n.get("w:type").map(str::to_owned)
    } else {
        None
    };
    for child in n.elements_mut() {
        changed |= rewrite(child, old, new, kind, owner_kind.as_deref(), name.as_str())?;
    }
    Ok(changed)
}

impl ImportedDocument {
    pub(in crate::editor) fn retarget_style(
        &self,
        pkg: &mut Package,
        target: &StyleAnchor,
        replacement: &StyleAnchor,
    ) -> Result<()> {
        self.validate_snapshot(&target.snapshot)?;
        self.validate_snapshot(&replacement.snapshot)?;
        if target.id == replacement.id {
            return Err(unsupported("identical retarget identities"));
        }
        let (path, _) = self
            .style_part()?
            .ok_or_else(|| unsupported("missing styles part"))?;
        let mut root = pkg.word(&path)?;
        let mut aliases = BTreeMap::new();
        let mut ids = BTreeSet::new();
        for style in root.elements() {
            if nested_style(style) {
                return Err(unsupported("opaque style identity"));
            }
            if style.name != "w:style" {
                continue;
            }
            let id = style
                .get("w:styleId")
                .ok_or_else(|| unsupported("missing style ID"))?;
            if !ids.insert(id.to_owned()) {
                return Err(unsupported("duplicate style ID"));
            }
            claim(&mut aliases, id, id)?;
            for tag in ["w:name", "w:aliases"] {
                if let Some(v) = unique_child(style, tag)? {
                    let value = v
                        .get("w:val")
                        .ok_or_else(|| unsupported("missing style name or aliases"))?;
                    if tag == "w:name" {
                        claim(&mut aliases, value, id)?;
                    } else {
                        for alias in value.split(',') {
                            claim(&mut aliases, alias.trim(), id)?;
                        }
                    }
                }
            }
        }
        let lookup = |id: &str| {
            root.elements()
                .find(|n| n.name == "w:style" && n.get("w:styleId") == Some(id))
                .ok_or_else(|| unsupported("missing style ID"))
        };
        let source = lookup(&target.id)?;
        let kind = source.get("w:type").unwrap_or("").to_owned();
        if !discardable(source) || lookup(&replacement.id)?.get("w:type") != Some(&kind) {
            return Err(unsupported(
                "retarget requires recognized custom source and same kind destination",
            ));
        }
        let mut affected = BTreeSet::from([replacement.id.clone()]);
        for style in root
            .elements_mut()
            .filter(|n| n.name == "w:style" && n.get("w:styleId") != Some(&target.id))
        {
            if rewrite(style, &target.id, &replacement.id, &kind, None, "w:styles")? {
                affected.insert(style.get("w:styleId").unwrap().to_owned());
            }
        }
        validate_chains(&root, &affected)?;
        for part in pkg.parts.keys().cloned().collect::<Vec<_>>() {
            if part == path {
                continue;
            }
            let mime = pkg.content_type_for(&part)?.unwrap_or_default();
            if !mime.starts_with("application/vnd.openxmlformats-officedocument.wordprocessingml.")
                || !mime.ends_with("+xml")
            {
                continue;
            }
            let mut node = pkg.word(&part)?;
            if matches!(
                node.name.as_str(),
                "w:document"
                    | "w:hdr"
                    | "w:ftr"
                    | "w:footnotes"
                    | "w:endnotes"
                    | "w:comments"
                    | "w:numbering"
            ) && rewrite(&mut node, &target.id, &replacement.id, &kind, None, "")?
            {
                pkg.put_word(part, node)?;
            }
        }
        let source = root
            .elements()
            .find(|n| n.name == "w:style" && n.get("w:styleId") == Some(&target.id))
            .unwrap();
        check_unused(pkg, &path, &root, source)?;
        root.children.retain(|c|!matches!(c,Content::Element(n) if n.name=="w:style" && n.get("w:styleId")==Some(&target.id)));
        pkg.put_word(path, root)
    }
}
