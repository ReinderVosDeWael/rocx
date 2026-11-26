use std::collections::BTreeMap;

use rocx::editor::{
    HeaderAnchor, ImportedInline, ImportedInlineNode, ImportedRun, ImportedStory,
    ImportedStoryBlock, InlineAnchor, RunTextEdit, SectionStoryKind, StoryReferenceEdit,
};
use rocx::{HeaderFooterType, Paragraph, ResolvedBlock};
use serde_json::{Value, json};

use super::{Snapshot, projected_run_text, resolve};
use crate::schema::{
    payloads::BlockInput,
    stories::{StoryBlockChange, StoryReferenceChange, StoryTextChange},
};

const KINDS: [SectionStoryKind; 2] = [SectionStoryKind::Header, SectionStoryKind::Footer];
const VARIANTS: [HeaderFooterType; 3] = [
    HeaderFooterType::Default,
    HeaderFooterType::First,
    HeaderFooterType::Even,
];
const SLOTS_PER_SECTION: usize = KINDS.len() * VARIANTS.len();

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
}

struct Stories {
    slots: BTreeMap<usize, ImportedStory>,
    owners: BTreeMap<String, usize>,
}

impl Stories {
    fn owner(&self, local: &str) -> Result<(usize, &ImportedStory), String> {
        let index = anchor_index(local, "story-owner:")
            .ok_or("wrong_anchor_type: expected a physical story owner")?;
        self.slots
            .get(&index)
            .filter(|story| self.owners.get(&story.part_name) == Some(&index))
            .map(|story| (index, story))
            .ok_or_else(|| "unknown physical story owner".into())
    }
}

impl Snapshot {
    fn stories(&self) -> Result<Stories, String> {
        let mut slots = BTreeMap::new();
        let mut owners = BTreeMap::new();
        for story in self.document.stories().map_err(|e| e.to_string())? {
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
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        if limit == 0 {
            return Err("story page limit must be positive".into());
        }
        let len = self.story_slot_count()?;
        let stories = self.stories()?;
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
                    "anchor": format!("story-slot:{index}"),
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

    fn story_slot_count(&self) -> Result<usize, String> {
        self.document
            .sections()
            .map(|sections| sections.len() * SLOTS_PER_SECTION)
            .map_err(|e| e.to_string())
    }

    // The session resolves only anchors issued to the caller for this snapshot.
    pub fn story_image_anchor(&self, local: &str) -> Option<HeaderAnchor> {
        let index = anchor_index(local, "story-slot:")?;
        self.stories().ok()?.slots.remove(&index)?.anchor
    }

    pub fn edit_story_text(
        &self,
        local_owner: &str,
        edits: Vec<StoryTextChange>,
    ) -> Result<Vec<u8>, String> {
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
            .collect::<Result<Vec<_>, String>>()?;
        self.document
            .edit_story_text(&story.part_name, &edits)
            .map_err(|e| e.to_string())
    }

    pub fn edit_story_reference(
        &self,
        local_slot: &str,
        edit: StoryReferenceChange,
    ) -> Result<Vec<u8>, String> {
        let index = anchor_index(local_slot, "story-slot:")
            .ok_or("wrong_anchor_type: expected a logical story slot")?;
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
                let stories = self.stories()?;
                let (_, source) = stories.owner(&anchor)?;
                if source.kind != slot.kind {
                    return Err("expected a same-kind physical story owner".into());
                }
                StoryReferenceEdit::Link(source.part_name.clone())
            }
            StoryReferenceChange::Remove {} => StoryReferenceEdit::Remove,
        };
        self.document
            .edit_story_reference(slot.section, slot.kind, slot.variant, edit)
            .map_err(|e| e.to_string())
    }

    pub fn edit_story_blocks(
        &self,
        local: &str,
        edit: StoryBlockChange,
    ) -> Result<Vec<u8>, String> {
        let anchor = self
            .story_image_anchor(local)
            .ok_or("wrong_anchor_type: expected a logical slot with an existing story")?;
        let content = edit
            .blocks
            .into_iter()
            .map(BlockInput::resolve)
            .collect::<Result<Vec<_>, _>>()?;
        self.document
            .edit_story_blocks(&anchor, edit.mode.into(), edit.index, edit.remove, &content)
            .map_err(|e| e.to_string())
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
) -> Result<&'a InlineAnchor, String> {
    let invalid = "unknown story run candidate";
    let mut indexes = local
        .strip_prefix("story-run:")
        .ok_or("wrong_anchor_type: expected a story run candidate")?
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
