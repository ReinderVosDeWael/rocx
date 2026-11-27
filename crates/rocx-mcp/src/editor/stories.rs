use std::collections::BTreeMap;

use rocx::editor::{
    HeaderAnchor, ImportedHeader, ImportedInline, ImportedInlineNode, ImportedRun, ImportedStory,
    ImportedStoryBlock, InlineAnchor, InlineEdit, RunTextEdit, SectionStoryKind,
    StoryReferenceEdit,
    mcp_bridge::{all_linkable_story_parts, linkable_story_parts},
};
use rocx::{HeaderFooterType, ImageData, ImageRun, Paragraph, ResolvedBlock, ResolvedInline};
use serde_json::{Value, json};

use super::{
    ReadError, Snapshot,
    block_readback::inline_items,
    body_inlines::{InlineHandles, typed_inline_page},
    projected_run_text, resolve,
    run_format::format_candidate,
};
use crate::{
    diagnostics::ScopedEditError,
    schema::{
        payloads::{BlockInput, resolve_component},
        run_format::RunFormatChange,
        stories::{StoryBlockChange, StoryImageInput, StoryReferenceChange, StoryTextChange},
    },
};

pub(super) fn resolve_story_image(
    bytes: Option<Vec<u8>>,
    image: Option<StoryImageInput>,
) -> Result<rocx::ResolvedImage, String> {
    let run = match (bytes, image) {
        (Some(bytes), None) => ImageRun::new(bytes),
        (
            None,
            Some(StoryImageInput {
                bytes,
                source_name,
                transformation,
                alt_text,
            }),
        ) => {
            let data = match source_name {
                Some(source_name) => ImageData::NamedBytes { bytes, source_name },
                None => ImageData::Bytes(bytes),
            };
            ImageRun::new(data)
                .with_transformation(transformation.map(Into::into))
                .with_alt_text(alt_text.map(Into::into))
        }
        _ => return Err("supply exactly one of bytes and image".into()),
    };
    match resolve_component(run)? {
        ResolvedInline::Image { image, .. } => Ok(image),
        _ => Err("expected image".into()),
    }
}

const KINDS: [SectionStoryKind; 2] = [SectionStoryKind::Header, SectionStoryKind::Footer];
const VARIANTS: [HeaderFooterType; 3] = [
    HeaderFooterType::Default,
    HeaderFooterType::First,
    HeaderFooterType::Even,
];
const SLOTS_PER_SECTION: usize = KINDS.len() * VARIANTS.len();
const SLOT_ANCHOR: &str = "story-slot:";
const DEFAULT_HEADER_SLOT: &str = "default-header-slot:";
const DEFAULT_HEADER_OWNER: &str = "default-header-owner:";
const DEFAULT_HEADER_TEXT: &str = "default-header-text:";
const DEFAULT_HEADER_FORMAT: &str = "default-header-format:";
const LINKABLE_STORY_OWNER: &str = "linkable-story-owner:";
const ALL_LINKABLE_STORY_OWNER: &str = "linkable-story-owner-all:";

#[derive(Clone, Copy)]
struct Slot {
    section: usize,
    kind: SectionStoryKind,
    variant: HeaderFooterType,
}

impl Slot {
    fn at(index: usize) -> Self {
        Self {
            section: index / SLOTS_PER_SECTION,
            kind: KINDS[index % SLOTS_PER_SECTION / VARIANTS.len()],
            variant: VARIANTS[index % VARIANTS.len()],
        }
    }

    fn index(story: &ImportedStory) -> usize {
        // Use the same explicit order as Slot::at, independent of discriminants.
        let kind = KINDS.iter().position(|&kind| kind == story.kind).unwrap();
        let variant = VARIANTS
            .iter()
            .position(|&variant| variant == story.variant)
            .unwrap();
        story.section_index * SLOTS_PER_SECTION + kind * VARIANTS.len() + variant
    }

    fn variant_name(self) -> &'static str {
        match self.variant {
            HeaderFooterType::Default => "default",
            HeaderFooterType::First => "first",
            HeaderFooterType::Even => "even",
        }
    }

    fn anchor(index: usize) -> String {
        format!("{SLOT_ANCHOR}{index}")
    }
}

pub(super) fn section_story_slots(section: usize) -> Vec<Value> {
    let start = section * SLOTS_PER_SECTION;
    (start..start + SLOTS_PER_SECTION)
        .map(|index| {
            let slot = Slot::at(index);
            json!({
                "anchor": Slot::anchor(index),
                "kind": slot.kind.as_str(),
                "variant": slot.variant_name(),
            })
        })
        .collect()
}

struct Stories {
    slots: BTreeMap<usize, ImportedStory>,
    owners: BTreeMap<String, usize>,
}

impl Stories {
    fn owner(&self, local: &str) -> Result<(usize, &ImportedStory), ScopedEditError> {
        let index = Snapshot::story_owner_index(local).ok_or_else(|| {
            ScopedEditError::WrongAnchorType("expected a physical story owner".into())
        })?;
        Ok((index, self.owner_at(index)?))
    }

    fn owner_at(&self, index: usize) -> Result<&ImportedStory, String> {
        self.slots
            .get(&index)
            .filter(|story| self.owners.get(&story.part_name) == Some(&index))
            .ok_or_else(|| "unknown physical story owner".into())
    }
}

impl Snapshot {
    pub fn story_slot_index(local: &str) -> Option<usize> {
        anchor_index(local, SLOT_ANCHOR)
    }

    pub fn story_owner_index(local: &str) -> Option<usize> {
        anchor_index(local, "story-owner:")
    }

    // The session only passes canonical owner indices issued by `story_page`.
    pub fn story_inline_page(
        &self,
        owner_index: usize,
        block_index: usize,
        start: usize,
        limit: usize,
        handles: &mut InlineHandles<'_>,
    ) -> Result<(Vec<Value>, Option<usize>), ReadError> {
        let missing = "block_index does not identify a direct story paragraph";
        let slot = Slot::at(owner_index);
        if slot.kind == SectionStoryKind::Header && slot.variant == HeaderFooterType::Default {
            let headers = self.document.default_headers()?;
            let header = headers
                .iter()
                .find(|header| header.section_index == slot.section)
                .ok_or("missing default header for selected story")?;
            let nodes = header.inline_structure(block_index).ok_or(missing)?;
            let source = story_inline_source(&header.part_name, block_index);
            return Ok(typed_inline_page(nodes, start, limit, &source, handles));
        }
        let stories = self.stories()?;
        let story = stories.owner_at(owner_index)?;
        let nodes = story.inline_structure(block_index).ok_or(missing)?;
        let source = story_inline_source(&story.part_name, block_index);
        Ok(typed_inline_page(nodes, start, limit, &source, handles))
    }

    fn stories(&self) -> rocx::Result<Stories> {
        let mut slots = BTreeMap::new();
        let mut owners = BTreeMap::new();
        for story in self.document.stories()? {
            let index = Slot::index(&story);
            let owner = owners.entry(story.part_name.clone()).or_insert(index);
            *owner = (*owner).min(index);
            slots.insert(index, story);
        }
        Ok(Stories { slots, owners })
    }

    pub fn story_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), ReadError> {
        if limit == 0 {
            return Err("story page limit must be positive".into());
        }
        let stories = self.stories()?;
        // Once a slot has a reference, it remains populated (by inheritance
        // or override) through the final section. Use that extent without
        // validating unrelated section layout. A wholly empty view uses
        // the same native boundary scan, without the section layout read.
        let len = match stories.slots.last_key_value() {
            Some((index, _)) => (Slot::at(*index).section + 1) * SLOTS_PER_SECTION,
            None => {
                rocx::editor::mcp_bridge::story_section_count(&self.document)? * SLOTS_PER_SECTION
            }
        };
        let end = start.saturating_add(limit).min(len);
        let rows = (start..end)
            .map(|index| {
                let slot = Slot::at(index);
                let story = stories.slots.get(&index);
                let (owner, blocks) = match story {
                    Some(story) => {
                        let owner = stories.owners[&story.part_name];
                        (
                            json!({
                                "anchor": format!("story-owner:{owner}"),
                                "part_name": story.part_name,
                            }),
                            story_blocks(story, owner),
                        )
                    }
                    None => (Value::Null, Vec::new()),
                };
                json!({
                    "anchor": Slot::anchor(index),
                    "section_index": slot.section,
                    "kind": slot.kind.as_str(),
                    "variant": slot.variant_name(),
                    "inherited": story.is_some_and(|story| story.inherited),
                    "owner": owner,
                    "blocks": blocks,
                })
            })
            .collect();
        Ok((rows, (end < len).then_some(end)))
    }

    pub fn default_header_page(
        &self,
        start: usize,
        limit: usize,
        handles: &mut InlineHandles<'_>,
    ) -> Result<(Vec<Value>, Option<usize>), ReadError> {
        let headers = self.document.default_headers()?;
        let end = start.saturating_add(limit).min(headers.len());
        let mut owners = BTreeMap::new();
        for (index, header) in headers[..end].iter().enumerate() {
            owners.entry(header.part_name.as_str()).or_insert(index);
        }
        let rows = (start..end)
            .map(|index| {
                let header = &headers[index];
                let owner = owners[header.part_name.as_str()];
                json!({
                    "section_index": header.section_index,
                    "part_name": header.part_name,
                    "inherited": header.inherited,
                    "image_slot": {
                        "anchor": format!("{DEFAULT_HEADER_SLOT}{}", header.section_index),
                    },
                    "owner": {
                        "anchor": format!("{DEFAULT_HEADER_OWNER}{owner}"),
                        "part_name": header.part_name,
                    },
                    "blocks": default_header_blocks(header, owner, handles),
                })
            })
            .collect();
        Ok((rows, (end < headers.len()).then_some(end)))
    }

    pub fn default_header_owner_index(local: &str) -> Option<usize> {
        anchor_index(local, DEFAULT_HEADER_OWNER)
    }

    pub fn default_header_slot_index(local: &str) -> Option<usize> {
        anchor_index(local, DEFAULT_HEADER_SLOT)
    }

    fn default_header_slot_anchor(&self, section: usize) -> rocx::Result<Option<HeaderAnchor>> {
        Ok(self
            .document
            .default_headers()?
            .into_iter()
            .find(|header| header.section_index == section)
            .map(|header| header.anchor))
    }

    fn default_header_owner(
        &self,
        local: &str,
    ) -> Result<(usize, ImportedHeader), ScopedEditError> {
        let index = Self::default_header_owner_index(local).ok_or_else(|| {
            ScopedEditError::WrongAnchorType("expected a physical default-header owner".into())
        })?;
        let header = self
            .document
            .default_headers()?
            .into_iter()
            .nth(index)
            .ok_or("unknown default-header owner")?;
        Ok((index, header))
    }

    pub fn edit_default_header_text(
        &self,
        local_owner: &str,
        edits: Vec<StoryTextChange>,
    ) -> Result<Vec<u8>, ScopedEditError> {
        let (owner, header) = self.default_header_owner(local_owner)?;
        let edits = edits
            .into_iter()
            .map(|edit| {
                let run = default_header_run(&header, owner, &edit.anchor, DEFAULT_HEADER_TEXT)?;
                Ok(RunTextEdit {
                    target: run.anchor.clone(),
                    text: edit.text,
                })
            })
            .collect::<Result<Vec<_>, ScopedEditError>>()?;
        Ok(self.document.edit_story_text(&header.part_name, &edits)?)
    }

    pub fn edit_default_header_format(
        &self,
        local_owner: &str,
        edits: Vec<RunFormatChange>,
    ) -> Result<Vec<u8>, ScopedEditError> {
        let (owner, header) = self.default_header_owner(local_owner)?;
        let edits = edits
            .into_iter()
            .map(|edit| {
                let run = default_header_run(&header, owner, &edit.anchor, DEFAULT_HEADER_FORMAT)?;
                Ok(InlineEdit {
                    target: run.anchor.clone(),
                    action: edit.edit.into(),
                })
            })
            .collect::<Result<Vec<_>, ScopedEditError>>()?;
        Ok(self
            .document
            .edit_run_format(Some(&header.part_name), &edits)?)
    }

    pub fn linkable_story_page(
        &self,
        kind: SectionStoryKind,
        include_referenced: bool,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), ReadError> {
        let parts = self.linkable_story_parts(kind, include_referenced)?;
        let prefix = if include_referenced {
            ALL_LINKABLE_STORY_OWNER
        } else {
            LINKABLE_STORY_OWNER
        };
        let end = start.saturating_add(limit).min(parts.len());
        let rows = (start..end)
            .map(|index| {
                let kind = kind.as_str();
                json!({
                    "anchor": format!("{prefix}{kind}:{index}"),
                    "kind": kind,
                    "part_name": parts[index],
                })
            })
            .collect();
        Ok((rows, (end < parts.len()).then_some(end)))
    }

    fn linkable_story_parts(
        &self,
        kind: SectionStoryKind,
        include_referenced: bool,
    ) -> rocx::Result<Vec<String>> {
        if include_referenced {
            all_linkable_story_parts(&self.document, kind)
        } else {
            linkable_story_parts(&self.document, kind)
        }
    }

    // The session resolves only anchors issued to the caller for this snapshot.
    pub fn story_image_anchor(&self, local: &str) -> Option<HeaderAnchor> {
        if let Some(section) = Self::default_header_slot_index(local) {
            return self.default_header_slot_anchor(section).ok().flatten();
        }
        let index = Self::story_slot_index(local)?;
        self.stories().ok()?.slots.remove(&index)?.anchor
    }

    pub fn edit_story_text(
        &self,
        local_owner: &str,
        edits: Vec<StoryTextChange>,
    ) -> Result<Vec<u8>, ScopedEditError> {
        let stories = self.stories()?;
        let (owner, story) = stories.owner(local_owner)?;
        let edits = edits
            .into_iter()
            .map(|edit| {
                Ok(RunTextEdit {
                    target: run_target(story, owner, &edit.anchor)?.clone(),
                    text: edit.text,
                })
            })
            .collect::<Result<Vec<_>, ScopedEditError>>()?;
        self.document
            .edit_story_text(&story.part_name, &edits)
            .map_err(Into::into)
    }

    pub fn edit_story_reference(
        &self,
        local_slot: &str,
        edit: StoryReferenceChange,
    ) -> Result<Vec<u8>, ScopedEditError> {
        let index = Self::story_slot_index(local_slot).ok_or_else(|| {
            ScopedEditError::WrongAnchorType("expected a logical story slot".into())
        })?;
        let slot = Slot::at(index);
        let edit = match edit {
            StoryReferenceChange::Create { paragraphs } => StoryReferenceEdit::Create(
                paragraphs
                    .into_iter()
                    .map(|text| resolve(Paragraph::new(text)).map(ResolvedBlock::Paragraph))
                    .collect::<Result<_, _>>()?,
            ),
            StoryReferenceChange::CreateRich { blocks } => StoryReferenceEdit::Create(
                blocks
                    .into_iter()
                    .map(BlockInput::resolve)
                    .collect::<Result<_, _>>()?,
            ),
            StoryReferenceChange::Link { anchor } => {
                StoryReferenceEdit::Link(self.story_link_part(slot.kind, &anchor)?)
            }
            StoryReferenceChange::Remove {} => StoryReferenceEdit::Remove,
        };
        self.document
            .edit_story_reference(slot.section, slot.kind, slot.variant, edit)
            .map_err(Into::into)
    }

    fn story_link_part(
        &self,
        kind: SectionStoryKind,
        anchor: &str,
    ) -> Result<String, ScopedEditError> {
        let wrong_kind = "expected a same-kind physical story owner";
        if Self::default_header_owner_index(anchor).is_some() {
            if kind != SectionStoryKind::Header {
                return Err(wrong_kind.into());
            }
            return Ok(self.default_header_owner(anchor)?.1.part_name);
        }
        let linkable = anchor
            .strip_prefix(LINKABLE_STORY_OWNER)
            .map(|candidate| (candidate, false))
            .or_else(|| {
                anchor
                    .strip_prefix(ALL_LINKABLE_STORY_OWNER)
                    .map(|candidate| (candidate, true))
            });
        if let Some((candidate, include_referenced)) = linkable {
            let (source_kind, index) = candidate
                .split_once(':')
                .and_then(|(kind, index)| Some((kind, index_number(index)?)))
                .ok_or("unknown linkable story owner")?;
            if source_kind != kind.as_str() {
                return Err(wrong_kind.into());
            }
            return self
                .linkable_story_parts(kind, include_referenced)?
                .into_iter()
                .nth(index)
                .ok_or_else(|| "unknown linkable story owner".into());
        }
        let stories = self.stories()?;
        let (_, source) = stories.owner(anchor)?;
        if source.kind != kind {
            return Err(wrong_kind.into());
        }
        Ok(source.part_name.clone())
    }

    pub fn edit_story_blocks(
        &self,
        local: &str,
        edit: StoryBlockChange,
    ) -> Result<Vec<u8>, ScopedEditError> {
        let wrong = || {
            ScopedEditError::WrongAnchorType(
                "expected a logical slot with an existing story".into(),
            )
        };
        let anchor = if let Some(section) = Self::default_header_slot_index(local) {
            self.default_header_slot_anchor(section)?
        } else {
            let index = Self::story_slot_index(local).ok_or_else(wrong)?;
            self.stories()?
                .slots
                .remove(&index)
                .and_then(|story| story.anchor)
        }
        .ok_or_else(wrong)?;
        let content = edit
            .blocks
            .into_iter()
            .map(BlockInput::resolve)
            .collect::<Result<Vec<_>, _>>()?;
        self.document
            .edit_story_blocks(&anchor, edit.mode.into(), edit.index, edit.remove, &content)
            .map_err(Into::into)
    }
}

fn index_number(digits: &str) -> Option<usize> {
    if !digits.bytes().all(|b| b.is_ascii_digit()) || digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    digits.parse().ok()
}

fn anchor_index(local: &str, prefix: &str) -> Option<usize> {
    index_number(local.strip_prefix(prefix)?)
}

fn story_inline_source(part: &str, block: usize) -> String {
    // Frame the lexical package part name before appending numeric coordinates.
    format!("inline:story:{}:{part}:{block}", part.len())
}

fn default_header_run<'a>(
    header: &'a ImportedHeader,
    owner: usize,
    local: &str,
    prefix: &str,
) -> Result<&'a ImportedRun, ScopedEditError> {
    let mut rest = local
        .strip_prefix(prefix)
        .ok_or_else(|| {
            ScopedEditError::WrongAnchorType("expected a default-header run target".into())
        })?
        .split(':');
    let first = rest.next().and_then(index_number);
    if first != Some(owner) {
        return Err("default-header run owner mismatch".into());
    }
    let block = rest.next().and_then(index_number);
    let inline = rest.next().and_then(index_number);
    let child = rest.next();
    let invalid = "unknown default-header run target";
    if rest.next().is_some() {
        return Err(invalid.into());
    }
    let node = header
        .inline_structure(block.ok_or(invalid)?)
        .and_then(|nodes| nodes.get(inline?))
        .ok_or(invalid)?;
    match (prefix, node, child) {
        (DEFAULT_HEADER_TEXT, ImportedInlineNode::Run(run), None) if text_candidate(run) => Ok(run),
        (DEFAULT_HEADER_FORMAT, ImportedInlineNode::Run(run), None) if format_candidate(run) => {
            Ok(run)
        }
        (DEFAULT_HEADER_FORMAT, ImportedInlineNode::Hyperlink(link), Some(child)) => {
            match index_number(child).and_then(|index| link.children.get(index)) {
                Some(ImportedInlineNode::Run(run)) if format_candidate(run) => Ok(run),
                _ => Err(invalid.into()),
            }
        }
        _ => Err(invalid.into()),
    }
}

fn default_header_blocks(
    header: &ImportedHeader,
    owner: usize,
    handles: &mut InlineHandles<'_>,
) -> Vec<Value> {
    header
        .blocks
        .iter()
        .enumerate()
        .map(|(index, block)| match block {
            ImportedStoryBlock::Paragraph(items) => {
                let (text, has_opaque_content) = projected_run_text(items);
                let nodes = header.inline_structure(index).unwrap_or_default();
                let source = story_inline_source(&header.part_name, index);
                let (inlines, _) = typed_inline_page(nodes, 0, nodes.len(), &source, handles);
                json!({
                    "type": "paragraph", "index": index, "text": text,
                    "has_opaque_content": has_opaque_content,
                    "inline_items": inline_items(items), "inlines": inlines,
                    "run_targets": default_header_run_targets(nodes, owner, index),
                })
            }
            ImportedStoryBlock::Opaque(name) => {
                json!({"type": "opaque", "index": index, "name": name})
            }
            _ => json!({"type": "opaque", "index": index, "name": "unknown"}),
        })
        .collect()
}

fn default_header_run_targets(
    nodes: &[ImportedInlineNode],
    owner: usize,
    block: usize,
) -> Vec<Value> {
    let mut targets = Vec::new();
    for (inline, node) in nodes.iter().enumerate() {
        let location = format!("{owner}:{block}:{inline}");
        match node {
            ImportedInlineNode::Run(run) => {
                let (text, format) = (text_candidate(run), format_candidate(run));
                if text || format {
                    let mut row = json!({"inline_index": inline});
                    if text {
                        row["anchor"] = json!(format!("{DEFAULT_HEADER_TEXT}{location}"));
                    }
                    if format {
                        row["format_target"] =
                            json!({"anchor": format!("{DEFAULT_HEADER_FORMAT}{location}")});
                    }
                    targets.push(row);
                }
            }
            ImportedInlineNode::Hyperlink(link) => {
                for (child, node) in link.children.iter().enumerate() {
                    if matches!(node, ImportedInlineNode::Run(run) if format_candidate(run)) {
                        targets.push(json!({
                            "inline_index": inline, "child_index": child,
                            "format_target": {
                                "anchor": format!("{DEFAULT_HEADER_FORMAT}{location}:{child}"),
                            },
                        }));
                    }
                }
            }
            _ => {}
        }
    }
    targets
}

fn story_blocks(story: &ImportedStory, owner: usize) -> Vec<Value> {
    story
        .blocks
        .iter()
        .enumerate()
        .map(|(index, block)| match block {
            ImportedStoryBlock::Paragraph(items) => {
                let (text, opaque) = projected_run_text(items);
                let inlines: Vec<_> = story
                    .inline_structure(index)
                    .unwrap_or_default()
                    .iter()
                    .enumerate()
                    .map(|(inline, node)| match node {
                        ImportedInlineNode::Hyperlink(link) => {
                            let children: Vec<_> = link
                                .children
                                .iter()
                                .enumerate()
                                .map(|(child, node)| inline_value(child, node, None))
                                .collect();
                            json!({
                                "type": "hyperlink", "index": inline, "children": children,
                                "relationship_id": link.relationship_id, "bookmark": link.bookmark,
                            })
                        }
                        _ => inline_value(inline, node, Some((owner, index))),
                    })
                    .collect();
                json!({
                    "type": "paragraph", "index": index, "text": text,
                    "has_opaque_content": opaque, "inlines": inlines,
                    "inline_items": inline_items(items),
                })
            }
            ImportedStoryBlock::Opaque(name) => {
                json!({"type": "opaque", "index": index, "name": name})
            }
            _ => json!({"type": "opaque", "index": index, "name": "unknown"}),
        })
        .collect()
}

fn text_candidate(run: &ImportedRun) -> bool {
    // A candidate is not edit permission: the native editor checks the exact
    // run content and preceding field/range boundaries in the physical part.
    matches!(run.items.as_slice(), [ImportedInline::Text(_)])
}

fn inline_value(index: usize, node: &ImportedInlineNode, direct: Option<(usize, usize)>) -> Value {
    if let ImportedInlineNode::Run(run) = node {
        let (text, opaque) = projected_run_text(&run.items);
        let mut value = json!({
            "type": "run", "index": index, "text": text, "has_opaque_content": opaque,
            "has_opaque_formatting": run.has_opaque_formatting,
        });
        if let Some((owner, block)) = direct
            && text_candidate(run)
        {
            value["anchor"] = json!(format!("story-run:{owner}:{block}:{index}"));
        }
        value
    } else {
        let name = match node {
            ImportedInlineNode::Opaque(name) => name,
            ImportedInlineNode::Fraction { .. } => "fraction",
            ImportedInlineNode::Revision(revision) => revision.kind,
            _ => "unknown",
        };
        json!({"type": "opaque", "index": index, "name": name})
    }
}

fn run_target<'a>(
    story: &'a ImportedStory,
    owner: usize,
    local: &str,
) -> Result<&'a InlineAnchor, ScopedEditError> {
    let invalid = "unknown story run candidate";
    let mut indexes = local
        .strip_prefix("story-run:")
        .ok_or_else(|| ScopedEditError::WrongAnchorType("expected a story run candidate".into()))?
        .split(':');
    let source = indexes.next().and_then(index_number).ok_or(invalid)?;
    if source != owner {
        return Err("story text owner mismatch".into());
    }
    let block = indexes.next().and_then(index_number).ok_or(invalid)?;
    let inline = indexes.next().and_then(index_number).ok_or(invalid)?;
    if indexes.next().is_some() {
        return Err(invalid.into());
    }
    let node = story
        .inline_structure(block)
        .and_then(|inlines| inlines.get(inline))
        .ok_or(invalid)?;
    match node {
        ImportedInlineNode::Run(run) if text_candidate(run) => Ok(&run.anchor),
        _ => Err(invalid.into()),
    }
}
