//! Whole-field edits consume one balanced original span, never an isolated
//! begin/separate/end run. Unknown markup cannot be discarded implicitly.
use super::{BoundaryState, ImportedDocument, ParagraphAnchor, text, unsupported};
use crate::{
    ResolvedInline, Result,
    ooxml::WordParts,
    opc::PackagePlan,
    xml::{Content, Node},
};

fn error() -> crate::Error {
    unsupported("expected a supported, balanced paragraph-local field")
}

fn marker(n: &Node) -> Option<&str> {
    if n.name != "w:r"
        || text::has_other_attrs(n, &[])
        || text::opaque_content(n)
        || n.elements().count() != 1
    {
        return None;
    }
    let m = n.child("w:fldChar")?;
    if !m.children.is_empty()
        || text::has_other_attrs(m, &["w:fldCharType", "w:dirty", "w:fldLock"])
    {
        return None;
    }
    m.get("w:fldCharType")
}

fn code_run(n: &Node) -> bool {
    n.name == "w:r"
        && !text::has_other_attrs(n, &[])
        && !text::opaque_content(n)
        && n.elements().count() == 1
        && n.child("w:instrText").is_some_and(|t| {
            !text::has_other_attrs(t, &["xml:space"])
                && t.children.iter().all(|c| matches!(c, Content::Text(_)))
        })
}

// Return the exclusive end in physical child slots. Do not consume comments,
// processing instructions or extension nodes interleaved with a complex span.
fn span(nodes: &[Content], start: usize) -> Result<usize> {
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
            i = result_item(&n.children, i)?;
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
            _ if cached => i = result_item(nodes, i)?,
            _ => return Err(error()),
        }
    }
    Err(error())
}
fn result_item(nodes: &[Content], i: usize) -> Result<usize> {
    match &nodes[i] {
        Content::Element(n) if n.name == "w:fldSimple" || marker(n) == Some("begin") => {
            span(nodes, i)
        }
        Content::Element(n) => {
            text::check_whole(n, true)?;
            Ok(i + 1)
        }
        Content::Text(s) if s.trim().is_empty() => Ok(i + 1),
        _ => Err(error()),
    }
}

fn locate<'a>(
    root: &'a mut Node,
    target: &ParagraphAnchor,
    inline_index: usize,
) -> Result<(usize, usize, &'a mut Node)> {
    let body = root.child_mut("w:body").ok_or_else(error)?;
    let mut state = BoundaryState::default();
    let mut blocks = body.elements_mut().filter(|n| n.name != "w:sectPr");
    for p in blocks.by_ref().take(target.body_index) {
        state.advance(p);
    }
    let p = blocks
        .next()
        .filter(|p| p.name == "w:p")
        .ok_or_else(error)?;
    let mut slots = p.children.iter().enumerate().filter_map(|(i, c)| match c {
        Content::Element(n) if n.name != "w:pPr" => Some((i, n)),
        _ => None,
    });
    for (_, n) in slots.by_ref().take(inline_index) {
        state.advance(n);
    }
    let (start, _) = slots.next().ok_or_else(error)?;
    if !state.safe() {
        return Err(unsupported("field target is inside another field or range"));
    }
    let end = span(&p.children, start)?;
    Ok((start, end, p))
}

/// One direct field property. Clearing a flag restores the absent state.
pub enum FieldPatch {
    Instruction(String),
    Dirty(Option<bool>),
    Locked(Option<bool>),
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
        let mut root = self.package.word(&self.main)?;
        let (start, end, p) = locate(&mut root, target, inline_index)?;
        let Content::Element(n) = &p.children[start] else {
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
            for c in &p.children[start + 1..end] {
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
            inline_count: p.children[start..end]
                .iter()
                .filter(|c| matches!(c, Content::Element(_)))
                .count(),
        })
    }
    /// Patch one original property without re-encoding the cached display.
    pub fn patch_field(
        &self,
        target: &ParagraphAnchor,
        inline_index: usize,
        patch: FieldPatch,
    ) -> Result<Vec<u8>> {
        self.validate_snapshot(&target.snapshot)?;
        let mut root = self.package.word(&self.main)?;
        let (start, end, p) = locate(&mut root, target, inline_index)?;
        let Content::Element(n) = &mut p.children[start] else {
            unreachable!()
        };
        match patch {
            FieldPatch::Instruction(s) => {
                crate::FieldInstruction::raw(&s)?;
                if n.name == "w:fldSimple" {
                    n.set("w:instr", s);
                } else {
                    let mut replacement = Some(s);
                    for c in &mut p.children[start + 1..end] {
                        if let Content::Element(r) = c {
                            let Some(t) = r.child_mut("w:instrText") else {
                                break;
                            };
                            t.children.clear();
                            if let Some(s) = replacement.take() {
                                t.children.push(Content::Text(s));
                                t.set("xml:space", "preserve");
                            }
                        }
                    }
                }
            }
            FieldPatch::Dirty(v) | FieldPatch::Locked(v) => {
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
                    n.set(key, if b { "1" } else { "0" });
                } else {
                    n.attrs.retain(|(k, _)| k != key);
                }
            }
        }
        let mut package = self.package.clone();
        package.put_word(&self.main, root)?;
        PackagePlan::new(package)?.to_bytes()
    }
    /// Replace a direct simple field or an entire direct complex field, selected
    /// by its first physical inline index. `None` removes its cached display too.
    pub fn replace_field(
        &self,
        target: &ParagraphAnchor,
        inline_index: usize,
        field: Option<&ResolvedInline>,
    ) -> Result<Vec<u8>> {
        self.validate_snapshot(&target.snapshot)?;
        if field.is_some_and(|f| !matches!(f, ResolvedInline::Field(_))) {
            return Err(error());
        }
        let mut root = self.package.word(&self.main)?;
        let (start, end, p) = locate(&mut root, target, inline_index)?;
        let (mut package, nodes) = if let Some(field) = field {
            let (pkg, mut groups) =
                crate::ooxml::encode_editor_inlines(self.package.clone(), &self.main, &[field])?;
            (pkg, groups.pop().unwrap())
        } else {
            (self.package.clone(), Vec::new())
        };
        p.children
            .splice(start..end, nodes.into_iter().map(Content::Element));
        package.put_word(&self.main, root)?;
        PackagePlan::new(package)?.to_bytes()
    }
}
