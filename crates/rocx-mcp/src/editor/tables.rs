use std::collections::BTreeMap;

use rocx::{
    HeaderFooterType,
    editor::{LocatedTable, TableStructureEdit},
};
use serde_json::{Value, json};

use super::Snapshot;
use crate::schema::tables::PhysicalTableChange;

const OWNER_PREFIX: &str = "physical-table-owner:";
const TABLE_PREFIX: &str = "physical-table:";

struct Owner {
    part: String,
    kind: &'static str,
    slots: Vec<Value>,
    tables: Vec<LocatedTable>,
}

impl Snapshot {
    fn physical_table_owners(&self) -> Result<Vec<Owner>, String> {
        let mut owners = Vec::new();
        // Passing None follows the package's actual main relationship even when relocated.
        let tables = self.document.part_tables(None).map_err(|e| e.to_string())?;
        let main = tables.first().map(|t| t.part_name.clone());
        if let Some(part) = &main {
            owners.push(Owner {
                part: part.clone(),
                kind: "body",
                slots: Vec::new(),
                tables,
            });
        }

        let mut by_part = BTreeMap::new();
        for story in self.document.stories().map_err(|e| e.to_string())? {
            let owner = by_part
                .entry(story.part_name.clone())
                .or_insert_with(|| Owner {
                    part: story.part_name,
                    kind: story.kind.as_str(),
                    slots: Vec::new(),
                    tables: Vec::new(),
                });
            owner.slots.push(json!({
                "section_index": story.section_index,
                "kind": story.kind.as_str(),
                "variant": match story.variant {
                    HeaderFooterType::Default => "default",
                    HeaderFooterType::First => "first",
                    HeaderFooterType::Even => "even",
                },
                "inherited": story.inherited,
            }));
        }
        for entry in self.document.entry_stories().map_err(|e| e.to_string())? {
            by_part
                .entry(entry.part_name.clone())
                .or_insert_with(|| Owner {
                    part: entry.part_name.clone(),
                    kind: entry.kind.as_str(),
                    slots: Vec::new(),
                    tables: Vec::new(),
                });
        }
        for (_, mut owner) in by_part {
            if main.as_ref() != Some(&owner.part) {
                // The native walker excludes reserved/ambiguous entries and guards
                // destructive edits against protected content and unsafe geometry.
                owner.tables = self
                    .document
                    .part_tables(Some(&owner.part))
                    .map_err(|e| e.to_string())?;
                if !owner.tables.is_empty() {
                    owners.push(owner);
                }
            }
        }
        Ok(owners)
    }

    pub fn physical_table_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        if limit == 0 {
            return Err("physical table page limit must be positive".into());
        }
        let owners = self.physical_table_owners()?;
        let count = owners.iter().map(|owner| owner.tables.len()).sum();
        let end = start.saturating_add(limit).min(count);
        let page = owners
            .iter()
            .enumerate()
            .flat_map(|(owner_index, owner)| {
                owner.tables.iter().enumerate().map(move |(index, table)| {
                    (owner_index, index, owner, table)
                })
            })
            .skip(start)
            .take(end.saturating_sub(start))
            .map(|(owner_index, index, owner, table)| {
                json!({
                    "anchor": format!("{TABLE_PREFIX}{owner_index}:{index}"),
                    "owner": {
                        "anchor": format!("{OWNER_PREFIX}{owner_index}"),
                        "part_name": owner.part,
                        "kind": owner.kind,
                        "slots": owner.slots,
                    },
                    "path": table.path, "depth": table.depth, "entry_id": table.entry_id,
                    "properties": table.properties, "grid_widths": table.grid_widths,
                    "row_properties": table.row_properties, "cell_properties": table.cell_properties,
                })
            })
            .collect();
        Ok((page, (end < count).then_some(end)))
    }

    // Session resolves only anchors issued to the caller for this snapshot.
    pub fn edit_part_tables(
        &self,
        local_owner: &str,
        edits: Vec<PhysicalTableChange>,
    ) -> Result<Vec<u8>, String> {
        let owner = local_owner
            .strip_prefix(OWNER_PREFIX)
            .and_then(index)
            .ok_or("wrong_anchor_type: expected a physical table owner")?;
        let owners = self.physical_table_owners()?;
        let owner = owners.get(owner).ok_or("unknown physical table owner")?;
        let edits = edits
            .into_iter()
            .map(|edit| {
                let (scope, table) = edit
                    .anchor
                    .strip_prefix(TABLE_PREFIX)
                    .and_then(|s| s.split_once(':'))
                    .and_then(|(scope, table)| Some((index(scope)?, index(table)?)))
                    .ok_or("wrong_anchor_type: expected a physical table anchor")?;
                let table = owners
                    .get(scope)
                    .and_then(|owner| owner.tables.get(table))
                    .ok_or("unknown physical table anchor")?;
                if table.part_name != owner.part {
                    return Err("physical table owner mismatch".into());
                }
                Ok(TableStructureEdit {
                    target: table.anchor.clone(),
                    operation: edit.edit.resolve()?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        self.document
            .edit_part_tables(&owner.part, &edits)
            .map_err(|e| e.to_string())
    }
}

fn index(digits: &str) -> Option<usize> {
    if !digits.bytes().all(|b| b.is_ascii_digit()) || digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    digits.parse().ok()
}
