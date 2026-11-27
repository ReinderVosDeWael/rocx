//! Lifecycle of physical header/footer definitions and their section references.
use super::*;
use crate::{ResolvedBlock, StoryKind, editor::text, opc::PackagePlan};

#[derive(Clone, Debug)]
pub enum StoryReferenceEdit {
    /// Create a new part and install an explicit reference in a non-explicit slot.
    Create(Vec<ResolvedBlock>),
    /// Set an explicit reference to a same-kind, relationship-selected physical part.
    Link(String),
    /// Remove the explicit reference and resume inheritance.
    Remove,
}

fn render(
    package: Package,
    main: &str,
    owner: &str,
    kind: SectionStoryKind,
    blocks: &[ResolvedBlock],
) -> Result<(Package, Vec<Node>)> {
    let story = match kind {
        SectionStoryKind::Header => StoryKind::Header,
        SectionStoryKind::Footer => StoryKind::Footer,
    };
    crate::ooxml::encode_editor_owner_blocks(
        package,
        main,
        (story, owner),
        blocks,
        None,
        "editor.story",
    )
}

fn reference_guard(
    props: Option<&Node>,
    kind: SectionStoryKind,
    variant: HeaderFooterType,
) -> Result<()> {
    if let Some(p) = props
        && (p.child("w:sectPrChange").is_some()
            || p.elements().any(|n| {
                n.name == kind.part_type().0
                    && n.get("w:type") == Some(variant.xml())
                    && (text::has_other_attrs(n, &["r:id", "w:type"]) || !n.children.is_empty())
            }))
    {
        return Err(unsupported(
            "cannot change a revised section or opaque story reference",
        ));
    }
    Ok(())
}

// Only discard content known to carry no external identities or protected ranges.
// Relationship rows referenced by removed hyperlinks remain in their source owner.
fn disposable(n: &Node) -> bool {
    let attrs: &[&str] = match n.name.as_str() {
        "w:r" | "w:hyperlink" => return text::check_whole(n, true).is_ok(),
        "w:p" | "w:pPr" | "w:tbl" | "w:tblPr" | "w:tblGrid" | "w:tr" | "w:tc" | "w:tcPr" => &[],
        "w:pStyle" | "w:jc" | "w:keepNext" | "w:keepLines" | "w:pageBreakBefore"
        | "w:widowControl" | "w:tblStyle" | "w:gridSpan" | "w:vMerge" | "w:vAlign" => &["w:val"],
        "w:tblW" | "w:tcW" => &["w:w", "w:type"],
        "w:tblLayout" => &["w:type"],
        "w:tblLook" => &[
            "w:val",
            "w:firstRow",
            "w:lastRow",
            "w:firstColumn",
            "w:lastColumn",
            "w:noHBand",
            "w:noVBand",
        ],
        "w:gridCol" => &["w:w"],
        _ => return false,
    };
    !text::has_other_attrs(n, attrs) && !text::opaque_content(n) && n.elements().all(disposable)
}

impl ImportedDocument {
    /// Reference edits follow Word inheritance until the next explicit reference
    /// of this kind and variant. Removing a reference never deletes an OPC part.
    pub fn edit_story_reference(
        &self,
        section_index: usize,
        kind: SectionStoryKind,
        variant: HeaderFooterType,
        edit: StoryReferenceEdit,
    ) -> Result<Vec<u8>> {
        let mut root = self.package.word(&self.main)?;
        rewrite_section_ranges(&root, &self.package, &self.main)?;
        let slots = section_slots(&root)?;
        let &(slot, _, props) = slots
            .get(section_index)
            .ok_or_else(|| unsupported("section index out of bounds"))?;
        reference_guard(props, kind, variant)?;
        let sections = story_sections(&self.package, &self.main, &slots, kind)?;
        let v = variant as usize;
        let explicit = sections[section_index].explicit[v];
        let body = root.child_mut("w:body").ok_or_else(missing_body)?;
        let mut package = self.package.clone();
        match edit {
            StoryReferenceEdit::Remove => {
                if !explicit {
                    return Err(unsupported("section slot has no explicit reference"));
                }
                section_props(body, slot)?.children.retain(|c| {
                    !matches!(c, Content::Element(n) if n.name == kind.part_type().0
                        && n.get("w:type") == Some(variant.xml()))
                });
            }
            StoryReferenceEdit::Link(part) => {
                let mut found = None;
                for (id, _) in package.relationships(&self.main, kind.as_str())? {
                    if package
                        .related(&self.main, &format!("#{id}"))?
                        .is_some_and(|(_, p)| p == part)
                    {
                        found = Some(id);
                        break;
                    }
                }
                let id =
                    found.ok_or_else(|| unsupported("no existing same-kind story relationship"))?;
                if !super::valid_story_part(&package, kind, &part)? {
                    return Err(unsupported("invalid target story part"));
                }
                set_story_reference(body, kind, slot, v, &id)?;
            }
            StoryReferenceEdit::Create(blocks) => {
                if explicit {
                    return Err(unsupported(
                        "section slot already has an explicit reference",
                    ));
                }
                let dir = self
                    .main
                    .rsplit_once('/')
                    .map_or(String::new(), |(d, _)| format!("{d}/"));
                let part = package.unique(&format!("{dir}rocx-{}", kind.as_str()), "xml");
                let (mut staged, nodes) = render(package, &self.main, &part, kind, &blocks)?;
                let mut story = crate::ooxml::root(kind.part_type().1);
                for node in nodes {
                    story.push(node);
                }
                if story.elements().next().is_none() {
                    story.push(Node::new("w:p"));
                }
                staged.put_word(&part, story)?;
                staged.content_type(&part, kind.part_type().2)?;
                let id = staged.link(&self.main, kind.as_str(), &part)?;
                set_story_reference(body, kind, slot, v, &id)?;
                package = staged;
            }
        }
        package.put_word(&self.main, root)?;
        PackagePlan::new(package)?.to_bytes()
    }

    /// Splice direct physical story blocks at original zero-based coordinates.
    pub fn edit_story_blocks(
        &self,
        target: &HeaderAnchor,
        mode: HeaderEditMode,
        index: usize,
        remove: usize,
        blocks: &[ResolvedBlock],
    ) -> Result<Vec<u8>> {
        self.validate_snapshot(&target.snapshot)?;
        if remove == 0 && blocks.is_empty() {
            return Err(unsupported("expected a story block edit"));
        }
        let mut root = self.package.word(&self.main)?;
        let kind = target.kind;
        let slots = section_slots(&root)?;
        let sections = story_sections(&self.package, &self.main, &slots, kind)?;
        let i = target.section_index;
        let v = target.variant;
        let source = header_source(&sections, i, v)?;
        let mut package = self.package.clone();
        let (mut story, owner) = if mode == HeaderEditMode::Shared {
            (package.word(&source.part)?, source.part.clone())
        } else {
            rewrite_section_ranges(&root, &self.package, &self.main)?;
            reference_guard(slots[i].2, kind, VARIANTS[v])?;
            if let Some(next) = sections.get(i + 1)
                && !next.explicit[v]
            {
                reference_guard(slots[i + 1].2, kind, VARIANTS[v])?;
            }
            let plan = HeaderBatch {
                sections: BTreeMap::from([(kind, sections)]),
                targets: BTreeMap::new(),
            };
            let (part, change) = plan.prepare_image(
                &mut package,
                &self.main,
                root.child_mut("w:body").ok_or_else(missing_body)?,
                (i, kind, v),
                mode,
            )?;
            (part, change.output_part)
        };
        if story.name != kind.part_type().1 {
            return Err(unsupported("invalid story root"));
        }
        let sites: Vec<_> = story
            .children
            .iter()
            .enumerate()
            .filter_map(|(i, c)| matches!(c, Content::Element(_)).then_some(i))
            .collect();
        let end = index
            .checked_add(remove)
            .filter(|&end| end <= sites.len())
            .ok_or_else(|| unsupported("story block index out of bounds"))?;
        let slot = sites.get(index).copied().unwrap_or(story.children.len());
        let mut boundaries = insertion::BoundaryState::default();
        for c in &story.children[..slot] {
            if let Content::Element(n) = c {
                boundaries.advance(n);
            }
        }
        if !boundaries.safe() {
            return Err(unsupported(
                "story block intersects a protected range or field",
            ));
        }
        for &pos in &sites[index..end] {
            if !matches!(&story.children[pos],Content::Element(n) if (n.name=="w:p"||n.name=="w:tbl") && disposable(n))
            {
                return Err(unsupported("cannot discard rich or opaque story block"));
            }
        }
        let (mut package, nodes) = render(package, &self.main, &owner, kind, blocks)?;
        // Keep raw XML comments and whitespace between removed block siblings.
        for &pos in sites[index..end].iter().rev() {
            story.children.remove(pos);
        }
        story
            .children
            .splice(slot..slot, nodes.into_iter().map(Content::Element));
        if story.elements().next().is_none() {
            story.push(Node::new("w:p"));
        }
        package.put_word(owner, story)?;
        if mode == HeaderEditMode::SectionLocal {
            package.put_word(&self.main, root)?;
        }
        PackagePlan::new(package)?.to_bytes()
    }
}
