use std::collections::BTreeMap;

use rocx::{
    HeaderFooterType,
    editor::{LocatedTable, TableAnchor, TableStructureEdit},
};
use serde_json::{Value, json};

use super::{ReadError, Snapshot};
use crate::{diagnostics::ScopedEditError, schema::tables::PhysicalTableChange};

const OWNER_PREFIX: &str = "physical-table-owner:";
const TABLE_PREFIX: &str = "physical-table:";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PhysicalTableScope<S = usize> {
    All,
    Main,
    Story(S),
    Entry(usize),
}

pub(crate) enum PhysicalTableTarget {
    Owner(String),
    Table { part: String, anchor: TableAnchor },
}

pub(crate) struct PhysicalTablePage {
    pub rows: Vec<Value>,
    pub next: Option<usize>,
    pub targets: BTreeMap<String, PhysicalTableTarget>,
}

struct Owner {
    part: String,
    kind: &'static str,
    slots: Vec<Value>,
    tables: Vec<LocatedTable>,
}

impl Snapshot {
    fn physical_table_owners(
        &self,
        scope: PhysicalTableScope<(usize, &str)>,
    ) -> Result<Vec<Owner>, ReadError> {
        if let PhysicalTableScope::Entry(index) = scope {
            let entry = self
                .document
                .entry_stories()?
                .get(index)
                .ok_or("unknown physical entry owner")?;
            return Ok(vec![Owner {
                part: entry.part_name.clone(),
                kind: entry.kind.as_str(),
                slots: Vec::new(),
                tables: self.document.part_tables(Some(&entry.part_name))?,
            }]);
        }
        let mut owners = Vec::new();
        // Passing None follows the package's actual main relationship even when relocated.
        if !matches!(scope, PhysicalTableScope::Story(_)) {
            let tables = self.document.part_tables(None)?;
            if let Some(table) = tables.first() {
                owners.push(Owner {
                    part: table.part_name.clone(),
                    kind: "body",
                    slots: Vec::new(),
                    tables,
                });
            }
            if scope == PhysicalTableScope::Main {
                return Ok(owners);
            }
        }

        let mut by_part = BTreeMap::new();
        for story in self.document.stories()? {
            if matches!(scope, PhysicalTableScope::Story((_, part)) if story.part_name != part) {
                continue;
            }
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
        if scope == PhysicalTableScope::All {
            for entry in self.document.entry_stories()? {
                by_part
                    .entry(entry.part_name.clone())
                    .or_insert_with(|| Owner {
                        part: entry.part_name.clone(),
                        kind: entry.kind.as_str(),
                        slots: Vec::new(),
                        tables: Vec::new(),
                    });
            }
        }
        let main = owners.first().map(|owner| owner.part.clone());
        for (_, mut owner) in by_part {
            if main.as_ref() != Some(&owner.part) {
                // The native walker excludes reserved/ambiguous entries and guards
                // destructive edits against protected content and unsafe geometry.
                owner.tables = self.document.part_tables(Some(&owner.part))?;
                if !owner.tables.is_empty() {
                    owners.push(owner);
                }
            }
        }
        Ok(owners)
    }

    pub(crate) fn physical_table_page(
        &self,
        scope: PhysicalTableScope<(usize, &str)>,
        start: usize,
        limit: usize,
    ) -> Result<PhysicalTablePage, ReadError> {
        if limit == 0 {
            return Err("physical table page limit must be positive".into());
        }
        let owners = self.physical_table_owners(scope)?;
        let count = owners.iter().map(|owner| owner.tables.len()).sum();
        let end = start.saturating_add(limit).min(count);
        let mut targets = BTreeMap::new();
        let rows = owners
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
                let key = match scope {
                    PhysicalTableScope::All => owner_index.to_string(),
                    PhysicalTableScope::Main => "main".to_owned(),
                    PhysicalTableScope::Story((story, _)) => format!("story:{story}"),
                    PhysicalTableScope::Entry(entry) => format!("entry:{entry}"),
                };
                let owner_local = format!("{OWNER_PREFIX}{key}");
                let table_local = format!("{TABLE_PREFIX}{key}:{index}");
                targets.entry(owner_local.clone()).or_insert_with(|| {
                    PhysicalTableTarget::Owner(owner.part.clone())
                });
                targets.insert(table_local.clone(), PhysicalTableTarget::Table {
                    part: table.part_name.clone(),
                    anchor: table.anchor.clone(),
                });
                json!({
                    "anchor": table_local,
                    "owner": {
                        "anchor": owner_local,
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
        Ok(PhysicalTablePage {
            rows,
            next: (end < count).then_some(end),
            targets,
        })
    }

    // Resolve only retained native targets issued to the caller for this snapshot.
    pub fn edit_part_tables<'a>(
        &self,
        owner: Option<&PhysicalTableTarget>,
        lookup: impl Fn(&str) -> Option<&'a PhysicalTableTarget>,
        edits: Vec<PhysicalTableChange>,
    ) -> Result<Vec<u8>, ScopedEditError> {
        let Some(PhysicalTableTarget::Owner(owner)) = owner else {
            return Err(ScopedEditError::WrongAnchorType(
                "expected a physical table owner".into(),
            ));
        };
        let edits = edits
            .into_iter()
            .map(|edit| {
                let Some(PhysicalTableTarget::Table { part, anchor }) = lookup(&edit.anchor) else {
                    return Err(ScopedEditError::WrongAnchorType(
                        "expected a physical table anchor".into(),
                    ));
                };
                if part != owner {
                    return Err("physical table owner mismatch".into());
                }
                Ok(TableStructureEdit {
                    target: anchor.clone(),
                    operation: edit.edit.resolve()?,
                })
            })
            .collect::<Result<Vec<_>, ScopedEditError>>()?;
        self.document
            .edit_part_tables(owner, &edits)
            .map_err(Into::into)
    }
}
