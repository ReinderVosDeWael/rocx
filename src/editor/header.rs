//! Header/footer inspection and explicitly scoped header editing.

use super::{
    ALTERNATE, EditPreview, ImportedDocument, ImportedInline, ImportedInlineNode, ImportedSection,
    InlineLocation, insertion, missing_body, read_paragraph_inlines, unsupported,
};
use crate::ooxml::{WordParts, encode_header_image};
use crate::opc::Package;
use crate::xml::{Content, Node, W};
use crate::{HeaderFooterType, ResolvedImage, Result, SectionType};
use std::collections::BTreeMap;
use std::sync::Arc;

mod block_edits;
pub use block_edits::StoryReferenceEdit;

const HEADER_MIME: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml";
const FOOTER_MIME: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml";
const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const W14: &str = "http://schemas.microsoft.com/office/word/2010/wordml";
const XML: &str = "http://www.w3.org/XML/1998/namespace";

#[derive(Clone, Debug)]
pub struct HeaderAnchor {
    snapshot: Arc<()>,
    section_index: usize,
    variant: usize,
    kind: SectionStoryKind,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedHeader {
    /// Section-scoped target for the existing header image edit.
    pub anchor: HeaderAnchor,
    /// Zero-based index in the document's section order.
    pub section_index: usize,
    /// Physical package part name. Section aliases of the same part share inline anchors.
    pub part_name: String,
    pub inherited: bool,
    /// Direct children in document order; paragraphs have no edit anchors.
    pub blocks: Vec<ImportedHeaderBlock>,
    physical: Arc<StoryPart>,
}

impl ImportedHeader {
    /// Structured inlines for a paragraph at a zero-based index in the source
    /// `blocks` order. An empty paragraph returns an empty slice; an opaque block
    /// or out-of-range index returns `None`.
    pub fn inline_structure(&self, block_index: usize) -> Option<&[ImportedInlineNode]> {
        self.physical.inline_structure(block_index)
    }
}

/// Read-only direct header/footer content. Unsupported children remain in the package.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ImportedHeaderBlock {
    Paragraph(Vec<ImportedInline>),
    Opaque(String),
}

pub use ImportedHeaderBlock as ImportedStoryBlock;

/// Whether the story is at the top or bottom of a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SectionStoryKind {
    Header,
    Footer,
}

impl SectionStoryKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Footer => "footer",
        }
    }

    fn part_type(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Header => ("w:headerReference", "w:hdr", HEADER_MIME),
            Self::Footer => ("w:footerReference", "w:ftr", FOOTER_MIME),
        }
    }
}

const VARIANTS: [HeaderFooterType; 3] = [
    HeaderFooterType::Default,
    HeaderFooterType::First,
    HeaderFooterType::Even,
];

/// A section's effective reference to an existing header or footer part.
#[derive(Debug)]
#[non_exhaustive]
pub struct ImportedStory {
    /// Image-edit target for this effective section slot.
    pub anchor: Option<HeaderAnchor>,
    pub section_index: usize,
    pub kind: SectionStoryKind,
    /// The reference slot in a section's properties, not a pagination result.
    pub variant: HeaderFooterType,
    /// Physical package part name. Aliases of one part share inline anchors.
    pub part_name: String,
    /// Whether this slot takes its reference from an earlier section.
    pub inherited: bool,
    /// Direct children in source order. Unsupported children are opaque.
    pub blocks: Vec<ImportedStoryBlock>,
    physical: Arc<StoryPart>,
}

impl ImportedStory {
    /// Structured inlines by direct block index. Returns `None` for an opaque
    /// or missing block, and an empty slice for an empty paragraph.
    pub fn inline_structure(&self, block_index: usize) -> Option<&[ImportedInlineNode]> {
        self.physical.inline_structure(block_index)
    }
}

#[derive(Debug)]
pub(super) struct StoryPart {
    pub(super) blocks: Vec<ImportedHeaderBlock>,
    structures: Vec<Option<Vec<ImportedInlineNode>>>,
}

impl StoryPart {
    pub(super) fn inline_structure(&self, index: usize) -> Option<&[ImportedInlineNode]> {
        self.structures.get(index)?.as_deref()
    }
}

#[derive(Debug)]
struct StoryAlias {
    section_index: usize,
    kind: SectionStoryKind,
    variant: HeaderFooterType,
    part_name: Arc<str>,
    inherited: bool,
}

#[derive(Debug, Default)]
pub(super) struct StoryInspection {
    aliases: Vec<StoryAlias>,
    parts: BTreeMap<Arc<str>, Arc<StoryPart>>,
}

impl StoryInspection {
    pub(super) fn inline_structure(
        &self,
        part: &str,
        block_index: usize,
    ) -> Option<&[ImportedInlineNode]> {
        self.parts.get(part)?.inline_structure(block_index)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderEditMode {
    /// Modify the header part itself, including all sections that reference it.
    Shared,
    /// Copy the header and redirect only the selected section's header slot.
    SectionLocal,
}

/// Append a paragraph containing one resolved inline image.
#[derive(Clone, Debug)]
pub struct HeaderImageEdit {
    pub target: HeaderAnchor,
    pub mode: HeaderEditMode,
    pub image: ResolvedImage,
}

impl HeaderImageEdit {
    pub(super) fn target_snapshot(&self) -> &Arc<()> {
        &self.target.snapshot
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct HeaderImageChange {
    pub source_part: String,
    pub output_part: String,
    /// Sections whose header content changes, including first/even aliases in shared mode.
    pub affected_sections: Vec<usize>,
    /// A following section given an explicit reference to retain its prior header.
    pub restored_section: Option<usize>,
    pub mode: HeaderEditMode,
}

#[derive(Debug)]
pub struct HeaderImagePreview {
    batch: EditPreview,
}

impl HeaderImagePreview {
    pub fn change(&self) -> &HeaderImageChange {
        &self.batch.header_changes()[0]
    }

    pub fn to_docx(&self) -> Result<Vec<u8>> {
        self.batch.to_docx()
    }
}

impl ImportedDocument {
    /// Inspect direct section boundaries, including an implicit final section.
    /// Ambiguous or hidden section boundaries are refused.
    pub fn sections(&self) -> Result<Vec<ImportedSection>> {
        let root = self.package.word(&self.main)?;
        let mut body_start = 0;
        section_slots(&root)?
            .into_iter()
            .enumerate()
            .map(|(index, (_, body_end, props))| {
                let section_type_xml = if let Some(props) = props {
                    if props.child(ALTERNATE).is_some() {
                        return Err(unsupported("ambiguous section properties"));
                    }
                    only_child(props, "w:type")?.and_then(|node| node.get("w:val"))
                } else {
                    None
                };
                let section_type = [
                    SectionType::NextPage,
                    SectionType::NextColumn,
                    SectionType::Continuous,
                    SectionType::EvenPage,
                    SectionType::OddPage,
                ]
                .into_iter()
                .find(|value| Some(value.xml()) == section_type_xml);
                Ok(ImportedSection {
                    anchor: super::SectionAnchor {
                        snapshot: self.snapshot.clone(),
                        index,
                    },
                    layout: super::layout::inspect(props, true),
                    flags: super::SectionFlag::read(props),
                    body_start: std::mem::replace(&mut body_start, body_end),
                    body_end,
                    section_type,
                    section_type_xml: section_type_xml.map(str::to_owned),
                })
            })
            .collect()
    }

    /// Inspect existing effective default headers. Sections without one are omitted.
    pub fn default_headers(&self) -> Result<Vec<ImportedHeader>> {
        if self.default_header_inspection.get().is_none() {
            // Only cache a successful inspection; concurrent readers may compute
            // equivalent views, but all anchors are issued from the stored view.
            let _ = self.default_header_inspection.set(
                self.inspect_stories(&[(SectionStoryKind::Header, &[HeaderFooterType::Default])])?,
            );
        }
        let inspection = self.default_header_inspection.get().unwrap();
        Ok(inspection
            .aliases
            .iter()
            .map(|alias| {
                let physical = inspection.parts[&alias.part_name].clone();
                ImportedHeader {
                    anchor: HeaderAnchor {
                        snapshot: self.snapshot.clone(),
                        section_index: alias.section_index,
                        variant: 0,
                        kind: alias.kind,
                    },
                    section_index: alias.section_index,
                    part_name: alias.part_name.to_string(),
                    inherited: alias.inherited,
                    blocks: physical.blocks.clone(),
                    physical,
                }
            })
            .collect())
    }

    /// Inspect effective header and footer slots in section order, with headers
    /// before footers and variants default, first, then even. Each slot inherits
    /// independently; slots without any reference are omitted. This does not
    /// determine which story is displayed on a page (`titlePg` and
    /// `evenAndOddHeaders` do not filter the result).
    pub fn stories(&self) -> Result<Vec<ImportedStory>> {
        if self.story_inspection.get().is_none() {
            let _ = self.story_inspection.set(self.inspect_stories(&[
                (SectionStoryKind::Header, &VARIANTS),
                (SectionStoryKind::Footer, &VARIANTS),
            ])?);
        }
        let inspection = self.story_inspection.get().unwrap();
        Ok(inspection
            .aliases
            .iter()
            .map(|alias| {
                let physical = inspection.parts[&alias.part_name].clone();
                ImportedStory {
                    anchor: Some(HeaderAnchor {
                        kind: alias.kind,
                        snapshot: self.snapshot.clone(),
                        section_index: alias.section_index,
                        variant: alias.variant as usize,
                    }),
                    section_index: alias.section_index,
                    kind: alias.kind,
                    variant: alias.variant,
                    part_name: alias.part_name.to_string(),
                    inherited: alias.inherited,
                    blocks: physical.blocks.clone(),
                    physical,
                }
            })
            .collect())
    }

    fn inspect_stories(
        &self,
        selection: &[(SectionStoryKind, &[HeaderFooterType])],
    ) -> Result<StoryInspection> {
        let root = self.package.word(&self.main)?;
        let slots = section_slots(&root)?;
        let mut inspection = StoryInspection::default();
        for &(kind, variants) in selection {
            let sections = story_sections(&self.package, &self.main, &slots, kind)?;
            for (section_index, section) in sections.iter().enumerate() {
                for &variant in variants {
                    let index = variant as usize;
                    let Some(reference) = &section.effective[index] else {
                        continue;
                    };
                    let part_name = if let Some((name, _)) =
                        inspection.parts.get_key_value(reference.part.as_str())
                    {
                        name.clone()
                    } else {
                        let part: Arc<str> = reference.part.as_str().into();
                        let physical = self.inspect_story_part(&part, kind)?;
                        inspection.parts.insert(part.clone(), Arc::new(physical));
                        part
                    };
                    inspection.aliases.push(StoryAlias {
                        section_index,
                        kind,
                        variant,
                        part_name,
                        inherited: !section.explicit[index],
                    });
                }
            }
        }
        inspection.aliases.sort_by_key(|alias| {
            (
                alias.section_index,
                alias.kind as usize,
                alias.variant as usize,
            )
        });
        Ok(inspection)
    }

    fn inspect_story_part(&self, part: &Arc<str>, kind: SectionStoryKind) -> Result<StoryPart> {
        let root = self.package.word(part)?;
        if root.name != kind.part_type().1 {
            return Err(unsupported(&format!("invalid {} root", kind.as_str())));
        }
        Ok(
            self.read_story_blocks(&root, |block_index| InlineLocation::Story {
                part: part.clone(),
                block_index,
            }),
        )
    }

    pub(super) fn read_story_blocks(
        &self,
        root: &Node,
        location: impl Fn(usize) -> InlineLocation,
    ) -> StoryPart {
        let mut blocks = Vec::new();
        let mut structures = Vec::new();
        for (block_index, node) in root.elements().enumerate() {
            if node.name == "w:p" {
                let (items, structure) =
                    read_paragraph_inlines(node, &self.snapshot, location(block_index));
                blocks.push(ImportedStoryBlock::Paragraph(items));
                structures.push(Some(structure));
            } else {
                blocks.push(ImportedStoryBlock::Opaque(node.name.to_string()));
                structures.push(None);
            }
        }
        StoryPart { blocks, structures }
    }

    /// Stage an image in one existing section story, with explicit sharing intent.
    pub fn preview_header_image(&self, edit: &HeaderImageEdit) -> Result<HeaderImagePreview> {
        Ok(HeaderImagePreview {
            batch: self.preview_transaction(&[], &[], std::slice::from_ref(edit))?,
        })
    }

    pub(super) fn plan_header_images<'a>(
        &self,
        root: &Node,
        edits: &'a [HeaderImageEdit],
    ) -> Result<HeaderBatch<'a>> {
        if edits.is_empty() {
            return Ok(HeaderBatch::default());
        }
        let mut targets = BTreeMap::new();
        for edit in edits {
            if targets
                .insert(
                    (
                        edit.target.section_index,
                        edit.target.kind,
                        edit.target.variant,
                    ),
                    edit,
                )
                .is_some()
            {
                return Err(unsupported(
                    "multiple header image edits target one section header slot",
                ));
            }
        }
        let slots = section_slots(root)?;
        let mut sections = BTreeMap::new();
        for &(_, kind, _) in targets.keys() {
            if let std::collections::btree_map::Entry::Vacant(entry) = sections.entry(kind) {
                entry.insert(story_sections(&self.package, &self.main, &slots, kind)?);
            }
        }
        let mut sources = BTreeMap::new();
        for (&(index, kind, variant), edit) in &targets {
            let source = header_source(&sections[&kind], index, variant)?;
            if let Some(previous) = sources.insert(&source.part, edit.mode)
                && (previous == HeaderEditMode::Shared || edit.mode == HeaderEditMode::Shared)
            {
                return Err(unsupported(
                    "shared header image edit conflicts with another edit of the same physical part",
                ));
            }
        }
        Ok(HeaderBatch { sections, targets })
    }
}

fn header_source(sections: &[SectionStoryRefs], index: usize, variant: usize) -> Result<&StoryRef> {
    sections
        .get(index)
        .and_then(|section| section.effective.get(variant))
        .and_then(Option::as_ref)
        .ok_or_else(|| unsupported("target has no effective header"))
}

#[derive(Default)]
pub(super) struct HeaderBatch<'a> {
    sections: BTreeMap<SectionStoryKind, Vec<SectionStoryRefs>>,
    targets: BTreeMap<(usize, SectionStoryKind, usize), &'a HeaderImageEdit>,
}

impl HeaderBatch<'_> {
    pub(super) fn stage(
        &self,
        mut package: Package,
        main: &str,
        body: &mut Node,
    ) -> Result<(Package, Vec<HeaderImageChange>)> {
        let mut changes = Vec::with_capacity(self.targets.len());
        let mut resources = None;
        for (&(index, kind, variant), edit) in &self.targets {
            let (mut header, change) =
                self.prepare_image(&mut package, main, body, (index, kind, variant), edit.mode)?;
            let (staged, paragraph) = encode_header_image(
                package,
                main,
                &change.output_part,
                &edit.image,
                &mut resources,
            )?;
            package = staged;
            header.push(paragraph);
            package.put_word(&change.output_part, header)?;
            changes.push(change);
        }
        Ok((package, changes))
    }

    fn prepare_image(
        &self,
        package: &mut Package,
        main: &str,
        body: &mut Node,
        (index, kind, variant): (usize, SectionStoryKind, usize),
        mode: HeaderEditMode,
    ) -> Result<(Node, HeaderImageChange)> {
        let sections = &self.sections[&kind];
        let source = header_source(sections, index, variant)?;
        let mut header = package.word(&source.part)?;
        if header.name != kind.part_type().1 || !insertion::can_append(&header) {
            return Err(unsupported("header cannot be safely extended"));
        }
        let mut restored_section = None;
        let (output_part, affected_sections) = match mode {
            HeaderEditMode::Shared => {
                let affected = sections
                    .iter()
                    .enumerate()
                    .filter_map(|(i, section)| {
                        section
                            .effective
                            .iter()
                            .flatten()
                            .any(|reference| reference.part == source.part)
                            .then_some(i)
                    })
                    .collect();
                (source.part.clone(), affected)
            }
            HeaderEditMode::SectionLocal => {
                let links = package.relationships(&source.part, "hyperlink")?;
                check_copy(&mut header, &links)?;
                let prefix = source.part.rsplit_once('/').map_or_else(
                    || format!("rocx-{}", kind.as_str()),
                    |(dir, _)| format!("{dir}/rocx-{}", kind.as_str()),
                );
                let output = package.unique(&prefix, "xml");
                package.copy_sibling_relationships(&source.part, &output)?;
                package.content_type(&output, kind.part_type().2)?;
                let id = package.link(main, kind.as_str(), &output)?;
                set_story_reference(body, kind, sections[index].slot, variant, &id)?;
                // A following section with its own local edit gets that new
                // reference, not a restoration. It still forks the original.
                if let Some(next) = sections.get(index + 1).filter(|part| {
                    !part.explicit[variant]
                        && !self.targets.contains_key(&(index + 1, kind, variant))
                }) {
                    set_story_reference(body, kind, next.slot, variant, &source.id)?;
                    restored_section = Some(index + 1);
                }
                (output, vec![index])
            }
        };
        Ok((
            header,
            HeaderImageChange {
                source_part: source.part.clone(),
                output_part,
                affected_sections,
                restored_section,
                mode,
            },
        ))
    }
}

#[derive(Clone, Copy)]
pub(super) enum Slot {
    Paragraph(usize),
    Terminal,
}

#[derive(Clone)]
struct StoryRef {
    id: String,
    part: String,
}

struct SectionStoryRefs {
    slot: Slot,
    explicit: [bool; 3],
    effective: [Option<StoryRef>; 3],
}

/// A section rewrite must not reinterpret hidden boundaries or dangling
/// header/footer aliases. Check original references, including inherited ones.
pub(super) fn rewrite_section_ranges<'a>(
    root: &'a Node,
    package: &Package,
    main: &str,
) -> Result<Vec<(std::ops::Range<usize>, Option<&'a Node>)>> {
    let mut start = 0;
    Ok(validated_section_slots(root, package, main)?
        .into_iter()
        .map(|(_, end, owner)| (std::mem::replace(&mut start, end)..end, owner))
        .collect())
}

pub(super) fn validated_section_slots<'a>(
    root: &'a Node,
    package: &Package,
    main: &str,
) -> Result<Vec<(Slot, usize, Option<&'a Node>)>> {
    let slots = section_slots(root)?;
    for (_, _, owner) in &slots {
        if let Some(owner) = owner {
            only_child(owner, "w:type")?;
        }
    }
    for kind in [SectionStoryKind::Header, SectionStoryKind::Footer] {
        story_sections(package, main, &slots, kind)?;
    }
    Ok(slots)
}

pub(super) fn section_slots(root: &Node) -> Result<Vec<(Slot, usize, Option<&Node>)>> {
    let body = only_child(root, "w:body")?.ok_or_else(missing_body)?;
    let mut slots = Vec::new();
    let mut terminal = None;
    let mut body_end = 0;
    for (index, content) in body.children.iter().enumerate() {
        let Content::Element(node) = content else {
            continue;
        };
        if terminal.is_some() {
            return Err(unsupported("body section properties are not terminal"));
        }
        if node.name != "w:sectPr" {
            body_end += 1;
        }
        match node.name.as_str() {
            "w:sectPr" => terminal = Some(node),
            "w:p" => {
                if let Some(props) = only_child(node, "w:pPr")? {
                    if let Some(section) = only_child(props, "w:sectPr")? {
                        slots.push((Slot::Paragraph(index), body_end, Some(section)));
                    }
                    if props
                        .elements()
                        .any(|prop| prop.name != "w:sectPr" && contains_section(prop))
                    {
                        return Err(unsupported("ambiguous section properties"));
                    }
                }
                if node
                    .elements()
                    .any(|n| n.name != "w:pPr" && contains_section(n))
                {
                    return Err(unsupported("ambiguous section properties"));
                }
            }
            _ if contains_section(node) => return Err(unsupported("non-direct section boundary")),
            _ => (),
        }
    }
    slots.push((Slot::Terminal, body_end, terminal));
    Ok(slots)
}

fn story_sections(
    package: &Package,
    main: &str,
    slots: &[(Slot, usize, Option<&Node>)],
    kind: SectionStoryKind,
) -> Result<Vec<SectionStoryRefs>> {
    let (reference_name, _, mime) = kind.part_type();
    let name = kind.as_str();
    let relationships: BTreeMap<_, _> = package.relationships(main, name)?.into_iter().collect();
    let mut resolved = BTreeMap::<String, StoryRef>::new();
    let mut effective = [None, None, None];
    let mut sections = Vec::with_capacity(slots.len());
    for &(slot, _, props) in slots {
        let mut explicit = [false; 3];
        if let Some(props) = props {
            if props.child(ALTERNATE).is_some() {
                return Err(unsupported(&format!(
                    "ambiguous {name} reference in section properties"
                )));
            }
            for child in props.elements().filter(|node| node.name == reference_name) {
                let variant = VARIANTS
                    .iter()
                    .position(|variant| Some(variant.xml()) == child.get("w:type"))
                    .ok_or_else(|| unsupported(&format!("unknown {name} reference type")))?;
                if std::mem::replace(&mut explicit[variant], true) {
                    return Err(unsupported(&format!(
                        "duplicate {name} reference type in a section"
                    )));
                }
                let id = child
                    .get("r:id")
                    .ok_or_else(|| unsupported(&format!("{name} reference has no ID")))?;
                if !resolved.contains_key(id) {
                    if !relationships.contains_key(id) {
                        return Err(unsupported(&format!(
                            "{name} reference does not identify a {name} relationship"
                        )));
                    }
                    let (_, part) = package
                        .related(main, &format!("#{id}"))?
                        .ok_or_else(|| unsupported(&format!("{name} target is missing")))?;
                    if package.content_type_for(&part)?.as_deref() != Some(mime) {
                        return Err(unsupported(&format!("incorrect {name} part content type")));
                    }
                    resolved.insert(
                        id.into(),
                        StoryRef {
                            id: id.into(),
                            part,
                        },
                    );
                }
                effective[variant] = resolved.get(id).cloned();
            }
        }
        sections.push(SectionStoryRefs {
            slot,
            explicit,
            effective: effective.clone(),
        });
    }
    Ok(sections)
}

fn only_child<'a>(node: &'a Node, name: &str) -> Result<Option<&'a Node>> {
    let mut matches = node.elements().filter(|child| child.name == name);
    let first = matches.next();
    if matches.next().is_some() {
        return Err(unsupported("ambiguous section structure"));
    }
    Ok(first)
}

fn contains_section(node: &Node) -> bool {
    node.name == "w:sectPr"
        || (node.name != "w:txbxContent" && node.elements().any(contains_section))
}

fn set_story_reference(
    body: &mut Node,
    kind: SectionStoryKind,
    slot: Slot,
    variant: usize,
    id: &str,
) -> Result<()> {
    let variant = VARIANTS[variant].xml();
    let props = section_props(body, slot)?;
    let existing = props
        .elements_mut()
        .find(|node| node.name == kind.part_type().0 && node.get("w:type") == Some(variant));
    if let Some(existing) = existing {
        existing.set("r:id", id);
    } else {
        props.push(
            Node::new(kind.part_type().0)
                .attr("w:type", variant)
                .attr("r:id", id),
        );
    }
    Ok(())
}

pub(super) fn section_props(body: &mut Node, slot: Slot) -> Result<&mut Node> {
    Ok(match slot {
        Slot::Terminal => body.ensure("w:sectPr"),
        Slot::Paragraph(index) => match body.children.get_mut(index) {
            Some(Content::Element(paragraph)) => paragraph
                .child_mut("w:pPr")
                .and_then(|props| props.child_mut("w:sectPr"))
                .ok_or_else(|| unsupported("section properties not found"))?,
            _ => return Err(unsupported("section paragraph not found")),
        },
    })
}

fn check_copy(node: &mut Node, links: &[(String, String)]) -> Result<()> {
    // Limit forks to content without identities or dependencies we cannot safely duplicate.
    let name = node.name.as_str();
    if !matches!(
        name,
        "w:hdr"
            | "w:ftr"
            | "w:p"
            | "w:r"
            | "w:t"
            | "w:tab"
            | "w:br"
            | "w:cr"
            | "w:hyperlink"
            | "w:pPr"
            | "w:rPr"
            | "w:pStyle"
            | "w:rStyle"
            | "w:jc"
            | "w:spacing"
            | "w:ind"
            | "w:b"
            | "w:bCs"
            | "w:i"
            | "w:iCs"
            | "w:u"
            | "w:color"
            | "w:sz"
            | "w:szCs"
            | "w:rFonts"
            | "w:lang"
            | "w:rtl"
            | "w:cs"
            | "w:tabs"
            | "w:keepNext"
            | "w:keepLines"
            | "w:widowControl"
            | "w:tbl"
            | "w:tblPr"
            | "w:tblGrid"
            | "w:gridCol"
            | "w:tblW"
            | "w:tblStyle"
            | "w:tblLayout"
            | "w:tblLook"
            | "w:tr"
            | "w:tc"
            | "w:tcPr"
            | "w:tcW"
            | "w:gridSpan"
            | "w:vMerge"
            | "w:vAlign"
    ) {
        return Err(unsupported(
            "header contains content that cannot be safely copied",
        ));
    }
    if let Some(id) = node.get("r:id")
        && (name != "w:hyperlink" || !links.iter().any(|(known, _)| known == id))
    {
        return Err(unsupported(
            "header contains an unsupported relationship reference",
        ));
    }
    // Word paragraph/text IDs identify occurrences, not reusable markup.
    node.attrs.retain(|(attr, _)| {
        !(name == "w:p"
            && attr.namespace() == Some(W14)
            && matches!(attr.local(), "paraId" | "textId"))
    });
    for (attr, _) in &node.attrs {
        if attr.declaration().is_none()
            && !matches!(
                (attr.namespace(), attr.local()),
                (Some(W), local) if local != "id"
            )
            && !matches!(
                (attr.namespace(), attr.local()),
                (Some(MC), "Ignorable") | (Some(XML), "space" | "lang")
            )
            && attr != "r:id"
        {
            return Err(unsupported(
                "header contains attributes that cannot be safely copied",
            ));
        }
    }
    for child in node.elements_mut() {
        check_copy(child, links)?;
    }
    Ok(())
}
