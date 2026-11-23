//! Default-header inspection and explicitly scoped editing.

use super::{
    ALTERNATE, EditPreview, ImportedDocument, ImportedInline, ImportedInlineNode, ImportedSection,
    InlineLocation, insertion, missing_body, read_paragraph_inlines, unsupported,
};
use crate::ooxml::{WordParts, encode_header_image};
use crate::opc::Package;
use crate::xml::{Content, Node, W};
use crate::{DrawingId, ResolvedImage, Result, SectionType};
use std::collections::BTreeMap;
use std::sync::Arc;

const HEADER_MIME: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml";
const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const W14: &str = "http://schemas.microsoft.com/office/word/2010/wordml";
const XML: &str = "http://www.w3.org/XML/1998/namespace";

#[derive(Clone, Debug)]
pub struct HeaderAnchor {
    snapshot: Arc<()>,
    section_index: usize,
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
    physical: Arc<DefaultHeaderPart>,
}

impl ImportedHeader {
    /// Structured inlines for a paragraph at a zero-based index in the source
    /// `blocks` order. An empty paragraph returns an empty slice; an opaque block
    /// or out-of-range index returns `None`.
    pub fn inline_structure(&self, block_index: usize) -> Option<&[ImportedInlineNode]> {
        self.physical.inline_structure(block_index)
    }
}

/// Read-only direct header content. Unsupported children remain in the package.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ImportedHeaderBlock {
    Paragraph(Vec<ImportedInline>),
    Opaque(String),
}

#[derive(Debug)]
struct DefaultHeaderPart {
    blocks: Vec<ImportedHeaderBlock>,
    structures: Vec<Option<Vec<ImportedInlineNode>>>,
}

impl DefaultHeaderPart {
    fn inline_structure(&self, index: usize) -> Option<&[ImportedInlineNode]> {
        self.structures.get(index)?.as_deref()
    }
}

#[derive(Debug)]
struct DefaultHeaderAlias {
    section_index: usize,
    part_name: Arc<str>,
    inherited: bool,
}

#[derive(Debug, Default)]
pub(super) struct DefaultHeaderInspection {
    aliases: Vec<DefaultHeaderAlias>,
    parts: BTreeMap<Arc<str>, Arc<DefaultHeaderPart>>,
}

impl DefaultHeaderInspection {
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
    /// Copy the header and redirect only the selected section's default header.
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
            .map(|(_, body_end, props)| {
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
            let _ = self
                .default_header_inspection
                .set(self.inspect_default_headers()?);
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

    fn inspect_default_headers(&self) -> Result<DefaultHeaderInspection> {
        let root = self.package.word(&self.main)?;
        let sections = header_sections(&self.package, &self.main, &root)?;
        let mut inspection = DefaultHeaderInspection::default();
        for (section_index, section) in sections.iter().enumerate() {
            let Some(header) = &section.effective[0] else {
                continue;
            };
            let part_name =
                if let Some((name, _)) = inspection.parts.get_key_value(header.part.as_str()) {
                    name.clone()
                } else {
                    let part_name: Arc<str> = header.part.as_str().into();
                    let root = self.package.word(&header.part)?;
                    if root.name != "w:hdr" {
                        return Err(unsupported("invalid header root"));
                    }
                    let mut blocks = Vec::new();
                    let mut structures = Vec::new();
                    for (block_index, node) in root.elements().enumerate() {
                        if node.name == "w:p" {
                            let (items, structure) = read_paragraph_inlines(
                                node,
                                &self.snapshot,
                                InlineLocation::DefaultHeader {
                                    part: part_name.clone(),
                                    block_index,
                                },
                            );
                            blocks.push(ImportedHeaderBlock::Paragraph(items));
                            structures.push(Some(structure));
                        } else {
                            blocks.push(ImportedHeaderBlock::Opaque(node.name.to_string()));
                            structures.push(None);
                        }
                    }
                    inspection.parts.insert(
                        part_name.clone(),
                        Arc::new(DefaultHeaderPart { blocks, structures }),
                    );
                    part_name
                };
            inspection.aliases.push(DefaultHeaderAlias {
                section_index,
                part_name,
                inherited: !section.explicit_default,
            });
        }
        Ok(inspection)
    }

    /// Stage an image in one existing default header, with explicit sharing intent.
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
            if targets.insert(edit.target.section_index, edit).is_some() {
                return Err(unsupported(
                    "multiple header image edits target one section",
                ));
            }
        }
        let sections = header_sections(&self.package, &self.main, root)?;
        let mut sources = BTreeMap::new();
        for (&index, edit) in &targets {
            let source = default_source(&sections, index)?;
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

fn default_source(sections: &[SectionHeaders], index: usize) -> Result<&HeaderRef> {
    sections
        .get(index)
        .and_then(|section| section.effective[0].as_ref())
        .ok_or_else(|| unsupported("target has no effective default header"))
}

#[derive(Default)]
pub(super) struct HeaderBatch<'a> {
    sections: Vec<SectionHeaders>,
    targets: BTreeMap<usize, &'a HeaderImageEdit>,
}

impl HeaderBatch<'_> {
    pub(super) fn stage(
        &self,
        mut package: Package,
        main: &str,
        body: &mut Node,
    ) -> Result<(Package, Vec<HeaderImageChange>)> {
        let mut changes = Vec::with_capacity(self.targets.len());
        let mut last_drawing = None;
        for (&index, edit) in &self.targets {
            let (staged, change) =
                self.stage_image(package, main, body, index, edit, &mut last_drawing)?;
            package = staged;
            changes.push(change);
        }
        Ok((package, changes))
    }

    fn stage_image(
        &self,
        mut package: Package,
        main: &str,
        body: &mut Node,
        index: usize,
        edit: &HeaderImageEdit,
        last_drawing: &mut Option<DrawingId>,
    ) -> Result<(Package, HeaderImageChange)> {
        let sections = &self.sections;
        let source = default_source(sections, index)?;
        let mut header = package.word(&source.part)?;
        if header.name != "w:hdr" || !insertion::can_append(&header) {
            return Err(unsupported("header cannot be safely extended"));
        }
        let mut restored_section = None;
        let (output_part, affected_sections) = match edit.mode {
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
                    || "rocx-header".into(),
                    |(dir, _)| format!("{dir}/rocx-header"),
                );
                let output = package.unique(&prefix, "xml");
                package.copy_sibling_relationships(&source.part, &output)?;
                package.content_type(&output, HEADER_MIME)?;
                let id = package.link(main, "header", &output)?;
                set_default_reference(body, sections[index].slot, &id)?;
                // A following section with its own local edit gets that new
                // reference, not a restoration. It still forks the original.
                if let Some(next) = sections.get(index + 1).filter(|part| {
                    !part.explicit_default && !self.targets.contains_key(&(index + 1))
                }) {
                    set_default_reference(body, next.slot, &source.id)?;
                    restored_section = Some(index + 1);
                }
                (output, vec![index])
            }
        };
        let (mut package, mut paragraph) =
            encode_header_image(package, main, &output_part, &edit.image)?;
        reserve_staged_drawing(&mut paragraph, last_drawing)?;
        header.push(paragraph);
        package.put_word(&output_part, header)?;
        Ok((
            package,
            HeaderImageChange {
                source_part: source.part.clone(),
                output_part,
                affected_sections,
                restored_section,
                mode: edit.mode,
            },
        ))
    }
}

fn reserve_staged_drawing(paragraph: &mut Node, last: &mut Option<DrawingId>) -> Result<()> {
    // The encoder scans materialized source parts. Remember allocations from
    // earlier edits, whose header XML is still staged in the package cache.
    let doc_pr = paragraph
        .child_mut("w:r")
        .and_then(|run| run.child_mut("w:drawing"))
        .and_then(|drawing| drawing.child_mut("wp:inline"))
        .and_then(|inline| inline.child_mut("wp:docPr"))
        .ok_or_else(|| unsupported("encoded header image has no drawing properties"))?;
    let id = doc_pr
        .get("id")
        .and_then(|id| id.parse().ok())
        .map(DrawingId::new)
        .ok_or_else(|| unsupported("encoded header image has an invalid drawing ID"))?;
    let id = if let Some(previous) = *last
        && previous >= id
    {
        let next = previous.next()?;
        doc_pr.set("id", next);
        next
    } else {
        id
    };
    *last = Some(id);
    Ok(())
}

#[derive(Clone, Copy)]
enum Slot {
    Paragraph(usize),
    Terminal,
}

#[derive(Clone)]
struct HeaderRef {
    id: String,
    part: String,
}

struct SectionHeaders {
    slot: Slot,
    explicit_default: bool,
    effective: [Option<HeaderRef>; 3],
}

fn section_slots(root: &Node) -> Result<Vec<(Slot, usize, Option<&Node>)>> {
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
                    if props.elements().any(hides_section) {
                        return Err(unsupported("ambiguous section properties"));
                    }
                }
                if node.elements().any(hides_section) {
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

fn header_sections(package: &Package, main: &str, root: &Node) -> Result<Vec<SectionHeaders>> {
    let slots = section_slots(root)?;
    let relationships: BTreeMap<_, _> =
        package.relationships(main, "header")?.into_iter().collect();
    let mut resolved = BTreeMap::<String, HeaderRef>::new();
    let mut effective = [None, None, None];
    let mut sections = Vec::with_capacity(slots.len());
    for (slot, _, props) in slots {
        let mut explicit = [false; 3];
        if let Some(props) = props {
            if props.child(ALTERNATE).is_some() {
                return Err(unsupported(
                    "ambiguous header reference in section properties",
                ));
            }
            for child in props
                .elements()
                .filter(|node| node.name == "w:headerReference")
            {
                let variant = match child.get("w:type") {
                    Some("default") => 0,
                    Some("first") => 1,
                    Some("even") => 2,
                    _ => return Err(unsupported("unknown header reference type")),
                };
                if std::mem::replace(&mut explicit[variant], true) {
                    return Err(unsupported("duplicate header reference type in a section"));
                }
                let id = child
                    .get("r:id")
                    .ok_or_else(|| unsupported("header reference has no ID"))?;
                if !resolved.contains_key(id) {
                    if !relationships.contains_key(id) {
                        return Err(unsupported(
                            "header reference does not identify a header relationship",
                        ));
                    }
                    let (_, part) = package
                        .related(main, &format!("#{id}"))?
                        .ok_or_else(|| unsupported("header target is missing"))?;
                    if package.content_type_for(&part)?.as_deref() != Some(HEADER_MIME) {
                        return Err(unsupported("incorrect header part content type"));
                    }
                    resolved.insert(
                        id.into(),
                        HeaderRef {
                            id: id.into(),
                            part,
                        },
                    );
                }
                effective[variant] = resolved.get(id).cloned();
            }
        }
        sections.push(SectionHeaders {
            slot,
            explicit_default: explicit[0],
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

fn hides_section(node: &Node) -> bool {
    node.name == ALTERNATE && contains_section(node)
}

fn set_default_reference(body: &mut Node, slot: Slot, id: &str) -> Result<()> {
    let props = match slot {
        Slot::Terminal => body.ensure("w:sectPr"),
        Slot::Paragraph(index) => match body.children.get_mut(index) {
            Some(Content::Element(paragraph)) => paragraph
                .child_mut("w:pPr")
                .and_then(|props| props.child_mut("w:sectPr"))
                .ok_or_else(|| unsupported("section properties not found"))?,
            _ => return Err(unsupported("section paragraph not found")),
        },
    };
    let existing = props
        .elements_mut()
        .find(|node| node.name == "w:headerReference" && node.get("w:type") == Some("default"));
    if let Some(existing) = existing {
        existing.set("r:id", id);
    } else {
        props.push(
            Node::new("w:headerReference")
                .attr("w:type", "default")
                .attr("r:id", id),
        );
    }
    Ok(())
}

fn check_copy(node: &mut Node, links: &[(String, String)]) -> Result<()> {
    // Limit forks to content without identities or dependencies we cannot safely duplicate.
    let name = node.name.as_str();
    if !matches!(
        name,
        "w:hdr"
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
