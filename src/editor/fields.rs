//! Whole-field edits consume one balanced original span, never an isolated
//! begin/separate/end run. Unknown markup cannot be discarded implicitly.
use super::{
    BoundaryState, ImportedDocument, ParagraphAnchor,
    batch::{Site, Slots},
    text, unsupported,
};
use crate::{
    ResolvedInline, Result,
    ooxml::WordParts,
    opc::Package,
    xml::{Content, Node},
};

fn error() -> crate::Error {
    unsupported("expected a supported, balanced direct body field")
}

fn field_run_child(n: &Node) -> Option<&Node> {
    if n.name != "w:r" || text::has_other_attrs(n, &[]) || text::opaque_content(n) {
        return None;
    }
    let mut children = n.elements().peekable();
    if children
        .next_if(|p| p.name == "w:rPr")
        .is_some_and(|pr| !text::disposable_properties(pr))
    {
        return None;
    }
    let leaf = children.next()?;
    children.next().is_none().then_some(leaf)
}
fn marker(n: &Node) -> Option<&str> {
    let m = field_run_child(n).filter(|n| n.name == "w:fldChar")?;
    if !m.children.is_empty()
        || text::has_other_attrs(m, &["w:fldCharType", "w:dirty", "w:fldLock"])
    {
        return None;
    }
    m.get("w:fldCharType")
}

fn code_run(n: &Node) -> bool {
    field_run_child(n)
        .filter(|t| t.name == "w:instrText")
        .is_some_and(|t| {
            !text::has_other_attrs(t, &["xml:space"])
                && t.children.iter().all(|c| matches!(c, Content::Text(_)))
        })
}

// Return the exclusive end in physical child slots. Do not consume comments,
// processing instructions or extension nodes interleaved with a complex span.
fn span(nodes: &[Content], start: usize, depth: usize) -> Result<usize> {
    // Complex delimiters are siblings: XML's element-depth limit cannot
    // bound field nesting. Refuse excessive nesting before recursive descent.
    if depth > 64 {
        return Err(unsupported("field nesting exceeds editor limit"));
    }
    let Some(Content::Element(n)) = nodes.get(start) else {
        return Err(error());
    };
    if n.name == "w:fldSimple" {
        if text::has_other_attrs(n, &["w:instr", "w:fldLock", "w:dirty"])
            || text::opaque_content(n)
            || n.get("w:instr").is_none()
        {
            return Err(error());
        }
        let mut i = 0;
        while i < n.children.len() {
            i = result_item(&n.children, i, depth)?;
        }
        return Ok(start + 1);
    }
    if marker(n) != Some("begin") {
        return Err(error());
    }
    let (mut i, mut cached, mut code) = (start + 1, false, false);
    while let Some(c) = nodes.get(i) {
        match c {
            Content::Text(s) if s.trim().is_empty() => i += 1,
            Content::Element(n) if !cached && code_run(n) => {
                code = true;
                i += 1;
            }
            Content::Element(n) if code && !cached && marker(n) == Some("separate") => {
                cached = true;
                i += 1;
            }
            Content::Element(n) if code && marker(n) == Some("end") => return Ok(i + 1),
            _ if cached => i = result_item(nodes, i, depth)?,
            _ => return Err(error()),
        }
    }
    Err(error())
}
fn result_item(nodes: &[Content], i: usize, depth: usize) -> Result<usize> {
    match &nodes[i] {
        Content::Element(n) if n.name == "w:fldSimple" || marker(n) == Some("begin") => {
            span(nodes, i, depth + 1)
        }
        Content::Element(n) => {
            if n.name == "w:hyperlink" {
                if text::opaque_content(n)
                    || text::has_other_attrs(n, &["r:id", "w:anchor", "w:tooltip", "w:history"])
                {
                    return Err(error());
                }
                for r in n.elements() {
                    text::check_cache_run(r)?;
                }
            } else {
                text::check_cache_run(n)?;
            }
            Ok(i + 1)
        }
        Content::Text(s) if s.trim().is_empty() => Ok(i + 1),
        _ => Err(error()),
    }
}

fn locate(
    body: &Node,
    target: &ParagraphAnchor,
    inline_index: usize,
) -> Result<(Vec<Site>, Vec<Content>)> {
    let mut state = BoundaryState::default();
    let (mut sites, mut nodes) = (Vec::new(), Vec::new());
    let mut depth = 0usize;
    // Only direct sibling paragraphs can continue a field. Never flatten
    // tables, alternate branches, revision containers, or section breaks.
    for (bi, c) in body.elements().enumerate() {
        if c.name == "w:sectPr" {
            break;
        }
        if bi < target.body_index {
            state.advance(c);
            continue;
        }
        if c.name != "w:p" {
            return Err(error());
        }
        let mut index = 0;
        for child in &c.children {
            let Content::Element(n) = child else {
                if !nodes.is_empty() && !matches!(child,Content::Text(s) if s.trim().is_empty()) {
                    return Err(error());
                }
                continue;
            };
            if n.name == "w:pPr" {
                continue;
            }
            let i = index;
            index += 1;
            if bi == target.body_index && i < inline_index {
                state.advance(n);
                continue;
            }
            if sites.is_empty() {
                if bi != target.body_index || i != inline_index {
                    return Err(error());
                }
                if !state.safe() {
                    return Err(unsupported("field target is inside another field or range"));
                }
                if n.name != "w:fldSimple" && marker(n) != Some("begin") {
                    return Err(error());
                }
            }
            sites.push((bi, i));
            nodes.push(child.clone());
            match marker(n) {
                Some("begin") => depth += 1,
                Some("end") => depth = depth.checked_sub(1).ok_or_else(error)?,
                _ => (),
            }
            if depth == 0 {
                if span(&nodes, 0, 0)? != nodes.len() {
                    return Err(error());
                }
                return Ok((sites, nodes));
            }
        }
        fn protected_break(n: &Node) -> bool {
            matches!(
                n.name.as_str(),
                "w:sectPr"
                    | "w:pPrChange"
                    | "w:rPrChange"
                    | "w:ins"
                    | "w:del"
                    | "w:moveTo"
                    | "w:moveFrom"
            ) || n.elements().any(protected_break)
        }
        if c.elements()
            .filter(|n| n.name == "w:pPr")
            .any(protected_break)
        {
            return Err(error());
        }
    }
    Err(error())
}

/// One direct field property. Clearing a flag restores the absent state.
#[derive(Clone, Debug)]
pub enum FieldPatch {
    Instruction(String),
    Dirty(Option<bool>),
    Locked(Option<bool>),
}
#[derive(Clone, Debug)]
pub enum FieldAction {
    Patch(FieldPatch),
    Replace(Box<ResolvedInline>),
    Cache(Vec<ResolvedInline>),
    Remove,
}
#[derive(Clone, Debug)]
pub struct FieldEdit {
    pub target: ParagraphAnchor,
    pub inline_index: usize,
    pub action: FieldAction,
}

/// Original XML values of one supported field; flags retain their lexical spelling.
#[derive(Debug)]
pub struct ImportedField {
    pub complex: bool,
    pub instruction: String,
    pub dirty_xml: Option<String>,
    pub locked_xml: Option<String>,
    pub inline_count: usize,
}

impl ImportedDocument {
    pub fn field_at(&self, target: &ParagraphAnchor, inline_index: usize) -> Result<ImportedField> {
        self.validate_snapshot(&target.snapshot)?;
        let root = self.package.word(&self.main)?;
        let (_, nodes) = locate(
            root.child("w:body").ok_or_else(error)?,
            target,
            inline_index,
        )?;
        let Content::Element(n) = &nodes[0] else {
            unreachable!()
        };
        let complex = n.name == "w:r";
        let props = if complex {
            n.child("w:fldChar").unwrap()
        } else {
            n
        };
        let mut instruction = props.get("w:instr").unwrap_or("").to_owned();
        if complex {
            for c in &nodes[1..] {
                if let Content::Element(r) = c {
                    let Some(t) = r.child("w:instrText") else {
                        break;
                    };
                    for c in &t.children {
                        if let Content::Text(s) = c {
                            instruction.push_str(s);
                        }
                    }
                }
            }
        }
        Ok(ImportedField {
            complex,
            instruction,
            dirty_xml: props.get("w:dirty").map(str::to_owned),
            locked_xml: props.get("w:fldLock").map(str::to_owned),
            inline_count: nodes.len(),
        })
    }
    /// Patch one original property without re-encoding the cached display.
    pub fn patch_field(
        &self,
        target: &ParagraphAnchor,
        inline_index: usize,
        patch: FieldPatch,
    ) -> Result<Vec<u8>> {
        self.preview_edits(super::EditBatch {
            fields: &[FieldEdit {
                target: target.clone(),
                inline_index,
                action: FieldAction::Patch(patch),
            }],
            ..Default::default()
        })?
        .to_docx()
    }
    pub fn replace_field(
        &self,
        target: &ParagraphAnchor,
        inline_index: usize,
        field: Option<&ResolvedInline>,
    ) -> Result<Vec<u8>> {
        self.preview_edits(super::EditBatch {
            fields: &[FieldEdit {
                target: target.clone(),
                inline_index,
                action: field.map_or(FieldAction::Remove, |f| {
                    FieldAction::Replace(Box::new(f.clone()))
                }),
            }],
            ..Default::default()
        })?
        .to_docx()
    }
}

fn patch_nodes(
    nodes: &mut [Content],
    patch: &FieldPatch,
    changed: &mut std::collections::BTreeSet<usize>,
) -> Result<()> {
    let Content::Element(n) = &mut nodes[0] else {
        unreachable!()
    };
    match patch {
        FieldPatch::Instruction(s) => {
            crate::FieldInstruction::raw(s)?;
            if n.name == "w:fldSimple" {
                changed.insert(0);
                n.set("w:instr", s);
            } else {
                let mut replacement = Some(s.clone());
                for (i, c) in nodes[1..].iter_mut().enumerate() {
                    if let Content::Element(r) = c {
                        let Some(t) = r.child_mut("w:instrText") else {
                            break;
                        };
                        t.children.clear();
                        changed.insert(i + 1);
                        if let Some(s) = replacement.take() {
                            t.children.push(Content::Text(s));
                            t.set("xml:space", "preserve");
                        }
                    }
                }
            }
        }
        FieldPatch::Dirty(v) | FieldPatch::Locked(v) => {
            changed.insert(0);
            let key = if matches!(patch, FieldPatch::Dirty(_)) {
                "w:dirty"
            } else {
                "w:fldLock"
            };
            let n = if n.name == "w:r" {
                n.child_mut("w:fldChar").unwrap()
            } else {
                n
            };
            if let Some(b) = v {
                n.set(key, if *b { "1" } else { "0" });
            } else {
                n.attrs.retain(|(k, _)| k != key);
            }
        }
    }
    Ok(())
}
pub(super) fn prepare(
    body: &Node,
    mut package: Package,
    main: &str,
    edits: &[FieldEdit],
    slots: &mut Slots,
) -> Result<Package> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut groups = BTreeMap::<Site, Vec<&FieldEdit>>::new();
    for e in edits {
        groups
            .entry((e.target.body_index, e.inline_index))
            .or_default()
            .push(e);
    }
    for (_, mut group) in groups {
        group.sort_by_key(|e| match e.action {
            FieldAction::Patch(FieldPatch::Instruction(_)) => 0,
            FieldAction::Patch(FieldPatch::Dirty(_)) => 1,
            FieldAction::Patch(FieldPatch::Locked(_)) => 2,
            _ => 3,
        });
        let (sites, mut nodes) = locate(body, &group[0].target, group[0].inline_index)?;
        let (mut seen, mut changed) = (BTreeSet::new(), BTreeSet::new());
        for s in &sites {
            if slots.contains_key(s) {
                return Err(unsupported("overlapping field/range edit"));
            }
            slots.entry(*s).or_default().field = true;
        }
        for e in &group {
            match &e.action {
                FieldAction::Patch(p) => {
                    let prop = match p {
                        FieldPatch::Instruction(_) => 0,
                        FieldPatch::Dirty(_) => 1,
                        FieldPatch::Locked(_) => 2,
                    };
                    if !seen.insert(prop) {
                        return Err(unsupported("duplicate field property"));
                    }
                    patch_nodes(&mut nodes, p, &mut changed)?;
                }
                FieldAction::Cache(children) => {
                    if !seen.insert(3) {
                        return Err(unsupported("duplicate cached field edit"));
                    }
                    let cache = ResolvedInline::Field(crate::ResolvedField::new(
                        crate::FieldInstruction::Page,
                        children.clone(),
                        Default::default(),
                    )?);
                    let (pkg, mut encoded) = crate::ooxml::encode_editor_inlines(
                        package,
                        main,
                        main,
                        crate::StoryKind::Body,
                        &[&cache],
                    )?;
                    package = pkg;
                    let mut result = encoded.pop().unwrap().pop().unwrap();
                    let Content::Element(start) = &mut nodes[0] else {
                        unreachable!()
                    };
                    if start.name == "w:fldSimple" {
                        start.children = std::mem::take(&mut result.children);
                        changed.insert(0);
                    } else {
                        let end = sites.len() - 1;
                        let sep = nodes.iter().position(
                            |c| matches!(c,Content::Element(r) if marker(r)==Some("separate")),
                        );
                        let cache = result.elements().cloned();
                        if let Some(sep) = sep {
                            slots.get_mut(&sites[sep]).unwrap().after.extend(cache);
                            for s in &sites[sep + 1..end] {
                                slots.get_mut(s).unwrap().replacement = Some(Vec::new());
                            }
                        } else {
                            let insert = &mut slots.get_mut(&sites[end]).unwrap().before;
                            insert
                                .push(Node::new("w:r").with(
                                    Node::new("w:fldChar").attr("w:fldCharType", "separate"),
                                ));
                            insert.extend(cache);
                        }
                    }
                }
                action => {
                    if group.len() != 1 {
                        return Err(unsupported("conflicting whole-field edit"));
                    }
                    let replacement = if let FieldAction::Replace(f) = action {
                        if !matches!(f.as_ref(), ResolvedInline::Field(_)) {
                            return Err(error());
                        }
                        let (pkg, mut encoded) = crate::ooxml::encode_editor_inlines(
                            package,
                            main,
                            main,
                            crate::StoryKind::Body,
                            &[f.as_ref()],
                        )?;
                        package = pkg;
                        encoded.pop().unwrap()
                    } else {
                        Vec::new()
                    };
                    for s in &sites {
                        slots.get_mut(s).unwrap().replacement = Some(Vec::new());
                    }
                    slots.get_mut(&sites[0]).unwrap().replacement = Some(replacement);
                }
            }
        }
        for i in changed {
            let Content::Element(n) = &nodes[i] else {
                unreachable!()
            };
            slots.get_mut(&sites[i]).unwrap().replacement = Some(vec![n.clone()]);
        }
    }
    Ok(package)
}
