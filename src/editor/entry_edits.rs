//! Relationship-selected definition edits. Never synthesize or move comment range markers.
use super::{blocks, revisions, text, *};
use crate::ooxml::decimal_id;

#[derive(Clone, Debug)]
pub enum NoteReferenceEdit {
    /// Insert a reference after the target and add its ordinary definition.
    Create { id: u32, text: String },
    /// Drop the target and its unique, discardable definition.
    Delete,
    /// Remove the sole reference and retain the note definition.
    Detach,
    /// Insert a reference after the target to an existing unused definition.
    Attach(u32),
    /// Point at an existing unused definition. Keep the original definition.
    Retarget(u32),
    /// Change the identity of a unique definition and its sole reference together.
    Renumber(u32),
}

#[derive(Clone, Debug)]
pub enum EntryEdit {
    DeleteUnusedNote,
    InsertParagraph { index: usize, text: String },
    ReplaceParagraph { index: usize, text: String },
    RemoveParagraph(usize),
    CommentAuthor(String),
    CommentInitials(Option<String>),
    CommentDate(Option<String>),
}

fn text_paragraph(s: &str) -> Result<Node> {
    text::validate_text(s)?;
    Ok(Node::new("w:p")
        .with(Node::new("w:r").with(Node::new("w:t").attr("xml:space", "preserve").text(s))))
}

fn element_mut(c: &mut Content) -> &mut Node {
    match c {
        Content::Element(n) => n,
        _ => unreachable!("validated element slot"),
    }
}

/// Check the entire declared XML graph, including settings special-note lists,
/// opaque wrapper descendants, relocated owners and glossary documents. Unknown
/// IDs cannot establish that allocating or deleting this identity is safe.
fn require_counts(
    doc: &ImportedDocument,
    tag: &str,
    reference: &str,
    id: &str,
    defs: usize,
    refs: usize,
) -> Result<()> {
    let mut counts = (0, 0);
    let mut unknown = false;
    doc.package.visit_source_xml(|n| {
        if n.name != tag && n.name != reference {
            return;
        }
        match n.get("w:id").and_then(decimal_id) {
            None => unknown = true,
            Some(value) if value == id => {
                if n.name == tag {
                    counts.0 += 1
                } else {
                    counts.1 += 1
                }
            }
            _ => (),
        }
    })?;
    if unknown || counts != (defs, refs) {
        return Err(unsupported(
            "note ID is ambiguous, reserved, missing, or already referenced",
        ));
    }
    Ok(())
}

fn entry_slot(root: &Node, tag: &str, id: &str) -> Result<usize> {
    let mut found = root
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match c {
            Content::Element(n)
                if n.name == tag && n.get("w:id").and_then(decimal_id).as_deref() == Some(id) =>
            {
                Some((i, n))
            }
            _ => None,
        });
    let (i, n) = found
        .next()
        .ok_or_else(|| unsupported("missing entry ID"))?;
    if found.next().is_some()
        || (tag != "w:comment"
            && (id.starts_with('-') || !matches!(n.get("w:type"), None | Some("normal"))))
    {
        return Err(unsupported("duplicate or reserved entry ID"));
    }
    Ok(i)
}

fn simple_reference(run: &Node, tag: &str, attrs: &[&str]) -> bool {
    let mut children = run.elements().peekable();
    run.name == "w:r"
        && !text::has_other_attrs(run, &[])
        && !text::opaque_content(run)
        && children
            .next_if(|n| n.name == "w:rPr")
            .is_none_or(text::disposable_properties)
        && children.map(|n| n.name.as_str()).eq([tag])
        && run
            .child(tag)
            .is_some_and(|leaf| leaf.children.is_empty() && !text::has_other_attrs(leaf, attrs))
}

fn first_mark(block: &Node, mark: &str) -> Option<usize> {
    let (i, c) = block
        .children
        .iter()
        .enumerate()
        .find(|(_, c)| matches!(c,Content::Element(n) if n.name != "w:pPr"))?;
    matches!(c,Content::Element(n) if simple_reference(n,mark,&[])).then_some(i)
}

fn plain_note(note: &Node, mark: &str) -> bool {
    if text::has_other_attrs(note, &["w:type", "w:id"]) || text::opaque_content(note) {
        return false;
    }
    let mut marked = false;
    for (index, block) in note.elements().enumerate() {
        let mut check = block.clone();
        if index == 0
            && let Some(i) = first_mark(&check, mark)
        {
            check.children.remove(i);
            marked = true;
        }
        if !blocks::local(&check, "body") {
            return false;
        }
    }
    marked
}

impl ImportedDocument {
    fn entry_owner_kind(&self, owner: &str) -> Result<EntryKind> {
        self.entry_stories()?; // validate roots, content types and relationship multiplicity
        for kind in [EntryKind::Footnote, EntryKind::Endnote, EntryKind::Comment] {
            if self
                .package
                .related(&self.main, &format!("{}s", kind.as_str()))?
                .is_some_and(|(_, p)| p == owner)
            {
                return Ok(kind);
            }
        }
        Err(unsupported("expected a relationship-selected entry part"))
    }

    pub fn edit_note_reference(
        &self,
        owner: &str,
        target: &InlineAnchor,
        edit: NoteReferenceEdit,
    ) -> Result<Vec<u8>> {
        self.validate_snapshot(&target.snapshot)?;
        let initial = match owner {
            "word/footnotes.xml" => Some(EntryKind::Footnote),
            "word/endnotes.xml" => Some(EntryKind::Endnote),
            _ => None,
        };
        let mut new_owner = false;
        let kind = if let Some(kind) = initial
            && matches!(edit, NoteReferenceEdit::Create { .. })
            && self
                .package
                .relationships(&self.main, &format!("{}s", kind.as_str()))?
                .is_empty()
        {
            let rels = format!("word/_rels/{}s.xml.rels", kind.as_str());
            if self
                .package
                .parts
                .keys()
                .any(|p| p.eq_ignore_ascii_case(owner) || p.eq_ignore_ascii_case(&rels))
            {
                return Err(unsupported("new note part path is already occupied"));
            }
            let tag = format!("w:{}", kind.as_str());
            let reference = format!("{tag}Reference");
            let mut existing = false;
            self.package.visit_source_xml(|n| {
                existing |= n.name == tag.as_str() || n.name == reference.as_str()
            })?;
            if existing {
                return Err(unsupported(
                    "cannot create a note part over existing references or settings",
                ));
            }
            new_owner = true;
            kind
        } else {
            self.entry_owner_kind(owner)?
        };
        if kind == EntryKind::Comment {
            return Err(unsupported("expected footnotes or endnotes"));
        }
        let tag = format!("w:{}", kind.as_str());
        let reference = format!("{tag}Reference");
        let mark = format!("{tag}Ref");
        let mut doc = self.package.word(&self.main)?;
        if doc.elements().filter(|n| n.name == "w:body").count() != 1 {
            return Err(unsupported("ambiguous document body"));
        }
        let body = doc.child_mut("w:body").unwrap();
        let (p, i) = revisions::site(body, target)
            .map_err(|_| unsupported("note reference edit requires a direct body inline"))?;
        revisions::guard_boundary(body, (p, i))
            .map_err(|_| unsupported("note reference intersects a protected range or field"))?;
        let paragraph = element_mut(&mut body.children[p]);
        let run = element_mut(&mut paragraph.children[i]);
        let mut notes = if new_owner {
            crate::ooxml::root(&format!("{tag}s"))
        } else {
            self.package.word(owner)?
        };
        let mutate_notes = !matches!(
            edit,
            NoteReferenceEdit::Retarget(_)
                | NoteReferenceEdit::Attach(_)
                | NoteReferenceEdit::Detach
        );
        if matches!(
            edit,
            NoteReferenceEdit::Create { .. } | NoteReferenceEdit::Attach(_)
        ) {
            text::check_run(run, false)?;
            let id = match edit {
                NoteReferenceEdit::Create { id, text } => {
                    require_counts(self, &tag, &reference, &id.to_string(), 0, 0)?;
                    let mut content = text_paragraph(&text)?;
                    content.children.insert(
                        0,
                        Content::Element(crate::ooxml::note_mark(
                            &format!("{}Ref", kind.as_str()),
                            None,
                        )),
                    );
                    notes.push(Node::new(tag).attr("w:id", id).with(content));
                    id
                }
                NoteReferenceEdit::Attach(id) => {
                    require_counts(self, &tag, &reference, &id.to_string(), 1, 0)?;
                    entry_slot(&notes, &tag, &id.to_string())?;
                    id
                }
                _ => unreachable!(),
            };
            paragraph.children.insert(
                i + 1,
                Content::Element(crate::ooxml::note_mark(
                    &format!("{}Reference", kind.as_str()),
                    Some(id),
                )),
            );
        } else {
            if !simple_reference(run, &reference, &["w:id"]) {
                return Err(unsupported(
                    "expected a simple direct note reference of the selected kind",
                ));
            }
            let leaf = run.child_mut(&reference).unwrap();
            let old = leaf
                .get("w:id")
                .and_then(decimal_id)
                .ok_or_else(|| unsupported("invalid note reference ID"))?;
            require_counts(self, &tag, &reference, &old, 1, 1)?;
            let slot = entry_slot(&notes, &tag, &old)?;
            match edit {
                NoteReferenceEdit::Delete => {
                    if !plain_note(element_mut(&mut notes.children[slot]), &mark) {
                        return Err(unsupported("cannot discard rich or opaque note content"));
                    }
                    notes.children.remove(slot);
                    paragraph.children.remove(i);
                }
                NoteReferenceEdit::Detach => {
                    paragraph.children.remove(i);
                }
                NoteReferenceEdit::Retarget(id) => {
                    let new = id.to_string();
                    require_counts(self, &tag, &reference, &new, 1, 0)?;
                    entry_slot(&notes, &tag, &new)?;
                    leaf.set("w:id", id);
                }
                NoteReferenceEdit::Renumber(id) => {
                    require_counts(self, &tag, &reference, &id.to_string(), 0, 0)?;
                    element_mut(&mut notes.children[slot]).set("w:id", id);
                    leaf.set("w:id", id);
                }
                NoteReferenceEdit::Create { .. } | NoteReferenceEdit::Attach(_) => unreachable!(),
            }
        }
        let mut package = self.package.clone();
        if new_owner {
            let plural = format!("{}s", kind.as_str());
            package.content_type(
                owner,
                &format!(
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.{plural}+xml"
                ),
            )?;
            package.link(&self.main, &plural, owner)?;
        }
        package.put_word(&self.main, doc)?;
        if mutate_notes {
            package.put_word(owner, notes)?;
        }
        PackagePlan::new(package)?.to_bytes()
    }

    pub fn edit_entry(&self, owner: &str, id: &str, edit: EntryEdit) -> Result<Vec<u8>> {
        let kind = self.entry_owner_kind(owner)?;
        let id = decimal_id(id).ok_or_else(|| unsupported("invalid entry ID"))?;
        let mut root = self.package.word(owner)?;
        let i = entry_slot(&root, &format!("w:{}", kind.as_str()), &id)?;
        let entry = element_mut(&mut root.children[i]);
        if kind == EntryKind::Comment {
            // Modern comments can associate a definition's last paragraph with
            // extension parts, and associate authors with person identities.
            let mut extended = false;
            self.package.visit_source_xml(|n| {
                extended |= matches!(
                    n.name.local(),
                    "commentsEx" | "commentsIds" | "commentsExtensible" | "people"
                );
            })?;
            if extended {
                return Err(unsupported(
                    "extended comment metadata or threading is unsupported",
                ));
            }
        }
        match edit {
            EntryEdit::DeleteUnusedNote => {
                let tag = format!("w:{}", kind.as_str());
                if kind == EntryKind::Comment || !plain_note(entry, &format!("{tag}Ref")) {
                    return Err(unsupported("expected a discardable ordinary note"));
                }
                require_counts(self, &tag, &format!("{tag}Reference"), &id, 1, 0)?;
                root.children.remove(i);
            }
            EntryEdit::CommentAuthor(_)
            | EntryEdit::CommentInitials(_)
            | EntryEdit::CommentDate(_) => {
                if kind != EntryKind::Comment
                    || text::has_other_attrs(entry, &["w:id", "w:author", "w:date", "w:initials"])
                {
                    return Err(unsupported("metadata requires a base comment"));
                }
                let (name, value) = match edit {
                    EntryEdit::CommentAuthor(s) => ("w:author", Some(s)),
                    EntryEdit::CommentInitials(s) => ("w:initials", s),
                    EntryEdit::CommentDate(date) => {
                        crate::CommentMetadata {
                            date: date.clone(),
                            ..Default::default()
                        }
                        .validate()?;
                        ("w:date", date)
                    }
                    _ => unreachable!(),
                };
                if entry.get(name) == value.as_deref() {
                    return PackagePlan::new(self.package.clone())?.to_bytes();
                }
                if let Some(v) = value {
                    crate::xml::validate_text(&v)?;
                    entry.set(name, v);
                } else {
                    entry.attrs.retain(|(key, _)| key != name);
                }
            }
            _ => {
                let index = match &edit {
                    EntryEdit::InsertParagraph { index, .. }
                    | EntryEdit::ReplaceParagraph { index, .. }
                    | EntryEdit::RemoveParagraph(index) => *index,
                    _ => unreachable!(),
                };
                let insert = matches!(edit, EntryEdit::InsertParagraph { .. });
                let count = entry.elements().count();
                let first_note = kind != EntryKind::Comment && index == 0;
                let first_comment_mark = kind == EntryKind::Comment
                    && index == 0
                    && entry
                        .elements()
                        .next()
                        .is_some_and(|p| first_mark(p, "w:annotationRef").is_some());
                if index > count
                    || !insert && index == count
                    || (first_note || first_comment_mark)
                        && !matches!(edit, EntryEdit::ReplaceParagraph { .. })
                {
                    return Err(unsupported(
                        "invalid block index or protected first note paragraph",
                    ));
                }
                let slot = entry
                    .children
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| matches!(c, Content::Element(_)))
                    .nth(index)
                    .map_or(entry.children.len(), |(s, _)| s);
                let mut state = BoundaryState::default();
                for c in &entry.children[..slot] {
                    if let Content::Element(n) = c {
                        state.advance(n);
                    }
                }
                if !state.safe() {
                    return Err(unsupported(
                        "entry block intersects protected field or range",
                    ));
                }
                let mut retained_mark = None;
                let mut retained_style = None;
                if !insert {
                    let block = element_mut(&mut entry.children[slot]);
                    if first_note || first_comment_mark {
                        let tag = if first_note {
                            format!("w:{}Ref", kind.as_str())
                        } else {
                            "w:annotationRef".into()
                        };
                        let mark = first_mark(block, &tag).ok_or_else(|| {
                            unsupported("cannot replace a nonstandard first note paragraph")
                        })?;
                        retained_mark = Some(block.children.remove(mark));
                        if let Some(pos) = block
                            .children
                            .iter()
                            .position(|c| matches!(c,Content::Element(n) if n.name == "w:pPr"))
                            && let Content::Element(pr) = &block.children[pos]
                            && !text::has_other_attrs(pr, &[])
                            && !text::opaque_content(pr)
                            && pr.elements().map(|n| n.name.as_str()).eq(["w:pStyle"])
                            && pr.child("w:pStyle").is_some_and(|s| {
                                !text::has_other_attrs(s, &["w:val"]) && s.children.is_empty()
                            })
                        {
                            retained_style = Some(block.children.remove(pos));
                        }
                    }
                    if !blocks::local(block, "body") {
                        return Err(unsupported("cannot discard rich or opaque entry paragraph"));
                    }
                }
                match edit {
                    EntryEdit::InsertParagraph { text, .. } => entry
                        .children
                        .insert(slot, Content::Element(text_paragraph(&text)?)),
                    EntryEdit::ReplaceParagraph { text, .. } => {
                        let mut p = text_paragraph(&text)?;
                        if let Some(mark) = retained_mark {
                            p.children.insert(0, mark);
                        }
                        if let Some(style) = retained_style {
                            p.children.insert(0, style);
                        }
                        entry.children[slot] = Content::Element(p);
                    }
                    EntryEdit::RemoveParagraph(_) if count > 1 => {
                        entry.children.remove(slot);
                    }
                    _ => return Err(unsupported("cannot remove the last entry paragraph")),
                }
            }
        }
        let mut package = self.package.clone();
        package.put_word(owner, root)?;
        PackagePlan::new(package)?.to_bytes()
    }
}
