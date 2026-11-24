//! Ordinary note parts and references; P1 (2016) §§11.3.4, 11.3.7, 17.11.
use super::{Compiler, PartContext, StoryContent, WordParts};
use crate::document::story_inlines;
use crate::opc::Package;
use crate::xml::{Content, Node};
use crate::{
    Diagnostic, NoteKind, ResolvedDocument, ResolvedInline, ResolvedNote, Result, Rule, StoryKind,
};
use std::collections::BTreeMap;

// P1 (2016) §§17.11.4, 17.11.12, 17.11.19–20 and Annex A.1.
pub(super) fn apply_defaults(root: &mut Node, defaults: &crate::DocumentDefaults) {
    let settings = [
        (
            "w:footnotePr",
            defaults.footnote_numbering.as_ref(),
            defaults.footnote_position.map(crate::FootnotePosition::xml),
        ),
        (
            "w:endnotePr",
            defaults.endnote_numbering.as_ref(),
            defaults.endnote_position.map(crate::EndnotePosition::xml),
        ),
    ];
    for (tag, numbering, position) in settings {
        if let Some(position) = position {
            root.ensure(tag).replace(crate::xml::val("pos", position));
        }
        let Some(numbering) = numbering else { continue };
        if let Some(format) = numbering.format {
            root.ensure(tag)
                .replace(crate::xml::val("numFmt", format.xml()));
        }
        if let Some(start) = numbering.start {
            root.ensure(tag).replace(crate::xml::val("numStart", start));
        }
        if let Some(restart) = numbering.restart {
            root.ensure(tag)
                .replace(crate::xml::val("numRestart", restart.xml()));
        }
    }
}

struct NotePart {
    path: String,
    root: Node,
    ids: BTreeMap<String, bool>,
}
#[derive(Default)]
pub(super) struct NoteCatalog {
    parts: BTreeMap<NoteKind, NotePart>,
}
fn invalid(message: impl Into<String>, path: &str) -> crate::Error {
    Diagnostic::new(Rule::InvalidContent, message)
        .at(path)
        .into()
}
fn id(node: &Node, path: &str) -> Result<String> {
    let raw = node
        .get("w:id")
        .ok_or_else(|| invalid("note ID is missing", path))?;
    decimal_id(raw).ok_or_else(|| invalid("invalid decimal note ID", path))
}

/// Value-space identity for arbitrary-precision OOXML decimal IDs.
pub(crate) fn decimal_id(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let (negative, digits) = if let Some(digits) = raw.strip_prefix('-') {
        (true, digits)
    } else {
        (false, raw.strip_prefix('+').unwrap_or(raw))
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let digits = digits.trim_start_matches('0');
    Some(if digits.is_empty() {
        "0".into()
    } else if negative {
        format!("-{digits}")
    } else {
        digits.into()
    })
}
fn references(
    node: &Node,
    definitions: &BTreeMap<(NoteKind, String), bool>,
    owner: &str,
    allowed: bool,
) -> Result<()> {
    let kind = match node.name.as_str() {
        "w:footnoteReference" => Some(NoteKind::Footnote),
        "w:endnoteReference" => Some(NoteKind::Endnote),
        _ => None,
    };
    if let Some(kind) = kind {
        if !allowed {
            return Err(invalid(
                "note references are supported only in the body story",
                owner,
            ));
        }
        if definitions.get(&(kind, id(node, owner)?)) != Some(&true) {
            return Err(invalid(
                "note reference is missing a normal note definition (special entries cannot be referenced)",
                owner,
            ));
        }
    }
    for child in node.elements() {
        references(child, definitions, owner, allowed)?;
    }
    Ok(())
}
impl NoteCatalog {
    pub fn new(package: &Package, main: &str, document: &ResolvedDocument) -> Result<Self> {
        let mut parts = BTreeMap::new();
        let mut definitions = BTreeMap::new();
        for kind in [NoteKind::Footnote, NoteKind::Endnote] {
            let name = kind.as_str();
            let relations = package.relationships(main, &format!("{name}s"))?;
            if relations.len() > 1 {
                return Err(invalid(
                    "a main document can own only one part per note kind",
                    main,
                ));
            }
            let existing = relations
                .first()
                .map(|(id, _)| package.related(main, &format!("#{id}")))
                .transpose()?
                .flatten();
            if existing.is_none() && !document.notes().iter().any(|n| n.target().kind == kind) {
                continue;
            }
            let path = if let Some((_, path)) = existing {
                path
            } else {
                let candidate = format!("word/{name}s.xml");
                if package
                    .parts
                    .keys()
                    .any(|p| p.eq_ignore_ascii_case(&candidate))
                {
                    package.unique(&format!("word/{name}s"), "xml")
                } else {
                    candidate
                }
            };
            let root = if package.parts.contains_key(&path) {
                let expected = format!(
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.{name}s+xml"
                );
                if package.content_type_for(&path)?.as_deref() != Some(expected.as_str()) {
                    return Err(Diagnostic::new(
                        Rule::InvalidPackage,
                        "incorrect note part content type",
                    )
                    .at(&path)
                    .into());
                }
                package.word(&path)?
            } else {
                super::root(&format!("w:{name}s"))
            };
            if root.name != format!("w:{name}s").as_str() {
                return Err(invalid("incorrect note part root/profile", &path));
            }
            let mut ids = BTreeMap::new();
            for note in root
                .elements()
                .filter(|n| n.name == format!("w:{name}").as_str())
            {
                let value = id(note, &path)?;
                let typ = note.get("w:type").unwrap_or("normal");
                if ![
                    "normal",
                    "separator",
                    "continuationSeparator",
                    "continuationNotice",
                ]
                .contains(&typ)
                {
                    return Err(invalid("unknown note entry type", &path));
                }
                let normal = typ == "normal";
                if ids.insert(value.clone(), normal).is_some() {
                    return Err(invalid("duplicate numeric note definition ID", &path));
                }
                definitions.insert((kind, value), normal);
            }
            parts.insert(kind, NotePart { path, root, ids });
        }
        for (i, note) in document.notes().iter().enumerate() {
            let target = note.target();
            let key = (target.kind, target.id.get().to_string());
            if definitions.insert(key, true).is_some() {
                return Err(invalid(
                    "note definition collides with an imported ID",
                    &format!("notes[{i}].target"),
                ));
            }
        }
        for (_, runs) in story_inlines(document) {
            for (run, path) in runs {
                if let ResolvedInline::NoteReference(target) = run
                    && definitions.get(&(target.kind, target.id.get().to_string())) != Some(&true)
                {
                    return Err(invalid(
                        "note reference is missing a normal definition (special entries cannot be referenced)",
                        &format!("{path}.target"),
                    ));
                }
            }
        }
        references(&package.word(main)?, &definitions, main, true)?;
        for part in parts.values() {
            references(&part.root, &definitions, &part.path, false)?;
        }
        for kind in ["header", "footer", "comments"] {
            for (id, _) in package.relationships(main, kind)? {
                if let Some((_, path)) = package.related(main, &format!("#{id}"))? {
                    references(&package.word(&path)?, &definitions, &path, false)?;
                }
            }
        }
        Ok(Self { parts })
    }
}
impl Compiler<'_> {
    pub(super) fn compile_notes(
        &mut self,
        mut catalog: NoteCatalog,
        definitions: &[ResolvedNote],
        width: i64,
    ) -> Result<()> {
        for (kind, part) in &mut catalog.parts {
            let notes: Vec<_> = definitions
                .iter()
                .filter(|n| n.target().kind == *kind)
                .collect();
            if notes.is_empty() {
                continue;
            }
            let name = kind.as_str();
            let mut specials = Vec::new();
            for typ in ["separator", "continuationSeparator"] {
                if let Some(note) = part.root.elements().find(|n| {
                    n.name == format!("w:{name}").as_str() && n.get("w:type") == Some(typ)
                }) {
                    specials.push(id(note, &part.path)?);
                } else {
                    let mut special = -1i128;
                    while part.ids.contains_key(&special.to_string()) {
                        special -= 1;
                    }
                    part.ids.insert(special.to_string(), false);
                    part.root.push(
                        Node::new(format!("w:{name}"))
                            .attr("w:id", special)
                            .attr("w:type", typ)
                            .with(
                                Node::new("w:p")
                                    .with(Node::new("w:r").with(Node::new(format!("w:{typ}")))),
                            ),
                    );
                    specials.push(special.to_string());
                }
            }
            for note in notes {
                let story = self.stories.register(
                    match kind {
                        NoteKind::Footnote => StoryKind::Footnote,
                        NoteKind::Endnote => StoryKind::Endnote,
                    },
                    crate::opc::PartName::new(&part.path)?,
                    StoryContent::Blocks,
                )?;
                let context = self.context(story, width);
                let mut blocks = self.blocks(note.blocks(), &context)?;
                if blocks.first().is_none_or(|n| n.name != "w:p") {
                    blocks.insert(0, Node::new("w:p"));
                }
                let paragraph = &mut blocks[0];
                let index = usize::from(
                    paragraph
                        .children
                        .first()
                        .is_some_and(|c| matches!(c,Content::Element(n) if n.name=="w:pPr")),
                );
                paragraph.children.insert(
                    index,
                    Content::Element(note_mark(&format!("{name}Ref"), None)),
                );
                let mut node = Node::new(format!("w:{name}")).attr("w:id", note.target().id.get());
                for block in blocks {
                    node.push(block);
                }
                part.root.push(node);
            }
            self.package.put_word(&part.path, part.root.clone())?;
            self.package.content_type(
                &part.path,
                &format!(
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.{name}s+xml"
                ),
            )?;
            self.package
                .link(&self.main.clone(), &format!("{name}s"), &part.path)?;
            let settings_path = self
                .package
                .related(&self.main, "settings")?
                .map(|(_, p)| p)
                .ok_or_else(|| invalid("missing settings part", &self.main))?;
            let mut settings = self.package.word(&settings_path)?;
            let properties = settings.ensure(&format!("w:{name}Pr"));
            for reference in properties
                .elements()
                .filter(|n| n.name == format!("w:{name}").as_str())
            {
                if part.ids.get(&id(reference, &settings_path)?) != Some(&false) {
                    return Err(invalid(
                        "note settings must reference special entries, not ordinary/missing notes",
                        &settings_path,
                    ));
                }
            }
            for special in specials {
                if !properties.elements().any(|n| {
                    n.name == format!("w:{name}").as_str()
                        && id(n, &settings_path).ok().as_ref() == Some(&special)
                }) {
                    if properties
                        .elements()
                        .filter(|n| n.name == format!("w:{name}").as_str())
                        .count()
                        >= 3
                    {
                        return Err(invalid(
                            "note special-entry settings exceed supported cardinality",
                            &settings_path,
                        ));
                    }
                    properties.push(Node::new(format!("w:{name}")).attr("w:id", special));
                }
            }
            self.package.put_word(&settings_path, settings)?;
        }
        Ok(())
    }
    pub(super) fn note_reference(
        &self,
        target: crate::NoteTarget,
        context: &PartContext,
    ) -> Result<Node> {
        if context.kind != StoryKind::Body {
            return Err(invalid(
                "note references are supported only in the body story",
                context.source.as_str(),
            ));
        }
        Ok(note_mark(
            &format!("{}Reference", target.kind.as_str()),
            Some(target.id.get()),
        ))
    }
}
pub(crate) fn note_mark(name: &str, id: Option<u32>) -> Node {
    let mut mark = Node::new(format!("w:{name}"));
    if let Some(id) = id {
        mark.set("w:id", id);
    }
    Node::new("w:r")
        .with(Node::new("w:rPr").with(Node::new("w:vertAlign").attr("w:val", "superscript")))
        .with(mark)
}
