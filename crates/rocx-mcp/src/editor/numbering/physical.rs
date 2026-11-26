use std::collections::BTreeSet;

use rocx::editor::{
    BodyBlock, ImportedCellBlock, ImportedEntry, ImportedInline, ImportedRowChild, ImportedStory,
    ImportedStoryBlock, ImportedTableChild, StoryParagraphAnchor,
};
use serde_json::{Value, json};

use super::assignment_value;
use crate::{
    editor::{Snapshot, projected_run_text},
    schema::numbering::PhysicalNumberingScope,
};

const CELL: &str = "numbering-cell";
const STORY: &str = "numbering-story";
const ENTRY: &str = "numbering-entry";

fn story_blocks(blocks: &[ImportedStoryBlock]) -> impl Iterator<Item = (usize, &[ImportedInline])> {
    blocks
        .iter()
        .enumerate()
        .filter_map(|(block, item)| match item {
            ImportedStoryBlock::Paragraph(items) => Some((block, items.as_slice())),
            _ => None,
        })
}

fn page<T>(
    nodes: impl Iterator<Item = T>,
    start: usize,
    limit: usize,
    row: impl Fn(T) -> Value,
) -> (Vec<Value>, Option<usize>) {
    let mut nodes = nodes.skip(start);
    let rows: Vec<_> = nodes.by_ref().take(limit).map(row).collect();
    let next = nodes.next().map(|_| start + rows.len());
    (rows, next)
}

impl Snapshot {
    pub fn physical_numbering_page(
        &self,
        scope: PhysicalNumberingScope,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        match scope {
            PhysicalNumberingScope::Cells => Ok(self.numbering_cells(start, limit)),
            PhysicalNumberingScope::Stories => {
                let stories = self.document.stories().map_err(|e| e.to_string())?;
                Ok(self.numbering_stories(&stories, start, limit))
            }
            PhysicalNumberingScope::Entries => {
                let entries = self.document.entry_stories().map_err(|e| e.to_string())?;
                Ok(self.numbering_entries(entries, start, limit))
            }
        }
    }

    fn numbering_stories(
        &self,
        stories: &[ImportedStory],
        start: usize,
        limit: usize,
    ) -> (Vec<Value>, Option<usize>) {
        let mut seen = BTreeSet::new();
        let nodes = stories
            .iter()
            .enumerate()
            .filter(|(_, story)| seen.insert(story.part_name.as_str()))
            .flat_map(|(index, story)| {
                story_blocks(&story.blocks).map(move |(block, items)| (index, story, block, items))
            });
        page(nodes, start, limit, |(index, story, block, items)| {
            let site = self
                .document
                .story_paragraph_anchor(&story.part_name, block, None);
            self.numbering_row(
                json!({
                    "kind": story.kind.as_str(), "part_name": story.part_name, "block_index": block,
                }),
                format!("{STORY}:{index}:{block}"),
                items,
                site,
            )
        })
    }

    fn numbering_entries(
        &self,
        entries: &[ImportedEntry],
        start: usize,
        limit: usize,
    ) -> (Vec<Value>, Option<usize>) {
        let nodes = entries.iter().enumerate().flat_map(|(index, entry)| {
            story_blocks(entry.blocks()).map(move |(block, items)| (index, entry, block, items))
        });
        page(nodes, start, limit, |(index, entry, block, items)| {
            let site =
                self.document
                    .story_paragraph_anchor(&entry.part_name, block, Some(&entry.id));
            self.numbering_row(
                json!({
                    "kind": entry.kind.as_str(), "part_name": entry.part_name, "entry_index": index,
                    "id": entry.id, "note_type": entry.note_type, "block_index": block,
                }),
                format!("{ENTRY}:{index}:{block}"),
                items,
                site,
            )
        })
    }

    fn numbering_cells(&self, start: usize, limit: usize) -> (Vec<Value>, Option<usize>) {
        let nodes = self
            .document
            .body()
            .iter()
            .enumerate()
            .filter_map(|(body, block)| match block {
                BodyBlock::Table(table) => Some((body, table)),
                _ => None,
            })
            .flat_map(|(body, table)| {
                table
                    .children
                    .iter()
                    .enumerate()
                    .filter_map(move |(row, child)| match child {
                        ImportedTableChild::Row(value) => Some((row, value)),
                        _ => None,
                    })
                    .flat_map(move |(row, value)| {
                        value
                            .children
                            .iter()
                            .enumerate()
                            .filter_map(move |(cell, child)| match child {
                                ImportedRowChild::Cell(value) => Some((cell, value)),
                                _ => None,
                            })
                            .flat_map(move |(cell, value)| {
                                value
                                    .blocks
                                    .iter()
                                    .enumerate()
                                    .filter_map(move |(block, item)| match item {
                                        ImportedCellBlock::Paragraph(items) => {
                                            Some((body, table, row, cell, block, items.as_slice()))
                                        }
                                        _ => None,
                                    })
                            })
                    })
            });
        page(
            nodes,
            start,
            limit,
            |(body, table, row, cell, block, items)| {
                let site = self
                    .document
                    .cell_paragraph_anchor(&table.anchor, row, cell, block);
                self.numbering_row(
                    json!({
                        "kind": "cell", "body_index": body, "row_index": row,
                        "cell_index": cell, "block_index": block,
                    }),
                    format!("{CELL}:{body}:{row}:{cell}:{block}"),
                    items,
                    site,
                )
            },
        )
    }

    fn numbering_row(
        &self,
        mut row: Value,
        local: String,
        items: &[ImportedInline],
        site: rocx::Result<StoryParagraphAnchor>,
    ) -> Value {
        let (text, opaque) = projected_run_text(items);
        row["text"] = json!(text);
        row["has_opaque_content"] = json!(opaque);
        match site.and_then(|site| self.document.story_paragraph_numbering(&site)) {
            Ok(value) => {
                row["anchor"] = json!(local);
                row["value"] = assignment_value(value);
            }
            Err(error) => row["unavailable"] = json!(error.to_string()),
        }
        row
    }

    pub fn physical_numbering_anchor(&self, local: &str) -> Result<StoryParagraphAnchor, String> {
        let bad = || "wrong_anchor_type: expected a physical numbering paragraph anchor".to_owned();
        let (kind, coords) = local.split_once(':').ok_or_else(bad)?;
        let indices: Vec<usize> = coords
            .split(':')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .map_err(|_| bad())?;
        let site = match (kind, indices.as_slice()) {
            (CELL, &[body, row, cell, block]) => {
                let Some(BodyBlock::Table(table)) = self.document.body().get(body) else {
                    return Err(bad());
                };
                self.document
                    .cell_paragraph_anchor(&table.anchor, row, cell, block)
            }
            (STORY, &[index, block]) => {
                let stories = self.document.stories().map_err(|e| e.to_string())?;
                let story = stories.get(index).ok_or_else(bad)?;
                self.document
                    .story_paragraph_anchor(&story.part_name, block, None)
            }
            (ENTRY, &[index, block]) => {
                let entries = self.document.entry_stories().map_err(|e| e.to_string())?;
                let entry = entries.get(index).ok_or_else(bad)?;
                self.document
                    .story_paragraph_anchor(&entry.part_name, block, Some(&entry.id))
            }
            _ => return Err(bad()),
        };
        site.map_err(|e| e.to_string())
    }
}
