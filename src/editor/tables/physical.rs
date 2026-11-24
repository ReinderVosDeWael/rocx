//! Part-scoped edits in body, headers, footers and ordinary entry stories.
use super::{inspect, structure, validate};
use crate::editor::{
    BoundaryState, DirectTableProperties, EntryKind, ImportedDocument, PackagePlan, TableAnchor,
    TableSite, TableStructureEdit, unsupported,
};
use crate::{Result, ooxml::WordParts, xml::Node};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug)]
pub(in crate::editor) struct PhysicalTable {
    owner: Arc<str>,
    path: Vec<usize>,
    editable: bool,
}

/// A physical table in document order. `path` indexes XML element children
/// from the part root and is descriptive; only the snapshot anchor authorizes edits.
#[derive(Debug)]
pub struct LocatedTable {
    pub anchor: TableAnchor,
    pub part_name: String,
    pub path: Vec<usize>,
    pub depth: usize,
    pub entry_id: Option<String>,
    pub properties: DirectTableProperties,
    pub grid_widths: Vec<Option<u32>>,
    pub row_properties: Vec<DirectTableProperties>,
    pub cell_properties: Vec<Vec<DirectTableProperties>>,
}

struct Walker<'a> {
    snapshot: &'a Arc<()>,
    owner: Arc<str>,
    tables: Vec<LocatedTable>,
}
impl Walker<'_> {
    fn story(&mut self, root: &Node, path: &mut Vec<usize>, entry: Option<&str>) {
        let mut state = BoundaryState::default();
        for (i, n) in root.elements().enumerate() {
            let before = state.safe();
            state.advance(n);
            if n.name == "w:tbl" {
                path.push(i);
                self.table(
                    n,
                    path,
                    0,
                    entry,
                    before && state.safe() && !structure::contains_range(n),
                );
                path.pop();
            }
        }
    }
    fn table(
        &mut self,
        n: &Node,
        path: &mut Vec<usize>,
        depth: usize,
        entry: Option<&str>,
        safe: bool,
    ) {
        let safe = safe
            && validate(n, true).is_ok()
            && !n
                .child("w:tblPr")
                .is_some_and(|p| p.child("w:tblPrChange").is_some());
        self.tables.push(LocatedTable {
            anchor: TableAnchor {
                _snapshot: self.snapshot.clone(),
                _body_index: usize::MAX,
                physical: Some(PhysicalTable {
                    owner: self.owner.clone(),
                    path: path.clone(),
                    editable: safe,
                }),
            },
            part_name: self.owner.to_string(),
            path: path.clone(),
            depth,
            entry_id: entry.map(str::to_owned),
            properties: inspect(n, TableSite::Table),
            grid_widths: n
                .child("w:tblGrid")
                .into_iter()
                .flat_map(Node::elements)
                .filter(|c| c.name == "w:gridCol")
                .map(|c| c.get("w:w")?.parse().ok())
                .collect(),
            row_properties: n
                .elements()
                .filter(|r| r.name == "w:tr")
                .map(|r| inspect(r, TableSite::Row(0)))
                .collect(),
            cell_properties: n
                .elements()
                .filter(|r| r.name == "w:tr")
                .map(|r| {
                    r.elements()
                        .filter(|c| c.name == "w:tc")
                        .map(|c| inspect(c, TableSite::Cell(0, 0)))
                        .collect()
                })
                .collect(),
        });
        for (r, row) in n.elements().enumerate().filter(|(_, n)| n.name == "w:tr") {
            for (c, cell) in row.elements().enumerate().filter(|(_, n)| n.name == "w:tc") {
                let continuation = cell
                    .child("w:tcPr")
                    .and_then(|p| p.child("w:vMerge"))
                    .is_some_and(|m| m.get("w:val") != Some("restart"));
                for (b, block) in cell
                    .elements()
                    .enumerate()
                    .filter(|(_, n)| n.name == "w:tbl")
                {
                    path.extend([r, c, b]);
                    self.table(block, path, depth + 1, entry, safe && !continuation);
                    path.truncate(path.len() - 3);
                }
            }
        }
    }
}
impl ImportedDocument {
    /// Discover direct and nested physical tables in one owner. `None` selects
    /// the actual main part, including relocated packages. Header/footer aliases
    /// select the same owner. Reserved or ambiguously identified entries are skipped.
    pub fn part_tables(&self, owner_part: Option<&str>) -> Result<Vec<LocatedTable>> {
        let owner = owner_part.unwrap_or(&self.main);
        let mut w = Walker {
            snapshot: &self.snapshot,
            owner: Arc::from(owner),
            tables: Vec::new(),
        };
        if owner == self.main {
            let root = self.package.word(owner)?;
            let (i, body) = root
                .elements()
                .enumerate()
                .find(|(_, n)| n.name == "w:body")
                .ok_or_else(super::super::missing_body)?;
            if root.elements().filter(|n| n.name == "w:body").count() != 1 {
                return Err(unsupported("ambiguous main story"));
            }
            w.story(body, &mut vec![i], None);
        } else if self.stories()?.iter().any(|s| s.part_name == owner) {
            w.story(&self.package.word(owner)?, &mut Vec::new(), None);
        } else {
            let entries: Vec<_> = self
                .entry_stories()?
                .iter()
                .filter(|e| e.part_name == owner)
                .collect();
            if entries.is_empty() {
                return Err(unsupported("not a relationship-selected table story"));
            }
            let mut ids = BTreeMap::new();
            for id in entries
                .iter()
                .filter_map(|e| crate::ooxml::decimal_id(&e.id))
            {
                *ids.entry(id).or_insert(0) += 1;
            }
            let root = self.package.word(owner)?;
            let nodes: Vec<_> = root.elements().collect();
            for e in entries {
                let Some(id) = crate::ooxml::decimal_id(&e.id) else {
                    continue;
                };
                if ids.get(&id) != Some(&1)
                    || (e.kind != EntryKind::Comment
                        && (id.starts_with('-')
                            || !matches!(e.note_type.as_deref(), None | Some("normal"))))
                {
                    continue;
                }
                let entry = nodes
                    .get(e.element_index)
                    .ok_or_else(|| unsupported("missing story entry"))?;
                w.story(entry, &mut vec![e.element_index], Some(&e.id));
            }
        }
        Ok(w.tables)
    }

    /// Edit one explicitly named physical owner. Distinct siblings may share
    /// a batch; overlapping ancestor/descendant targets cannot.
    pub fn edit_part_tables(
        &self,
        owner_part: &str,
        edits: &[TableStructureEdit],
    ) -> Result<Vec<u8>> {
        if edits.is_empty() {
            return Err(unsupported("expected a physical table edit"));
        }
        let mut groups = BTreeMap::<&[usize], Vec<_>>::new();
        for e in edits {
            self.validate_snapshot(&e.target._snapshot)?;
            let a = e
                .target
                .physical
                .as_ref()
                .ok_or_else(|| unsupported("expected a physical table anchor"))?;
            if a.owner.as_ref() != owner_part {
                return Err(unsupported("physical table owner mismatch"));
            }
            if !a.editable {
                return Err(unsupported(
                    "physical table intersects protected content or unsafe geometry",
                ));
            }
            groups.entry(&a.path).or_default().push(&e.operation);
        }
        // Sorted paths place descendants directly after their ancestor.
        if groups
            .keys()
            .zip(groups.keys().skip(1))
            .any(|(a, b)| b.starts_with(a))
        {
            return Err(unsupported("overlapping physical table targets"));
        }
        let mut package = self.package.clone();
        let mut root = package.word(owner_part)?;
        use crate::StoryKind;
        let kind = match root.name.as_str() {
            "w:document" => StoryKind::Body,
            "w:hdr" => StoryKind::Header,
            "w:ftr" => StoryKind::Footer,
            "w:comments" => StoryKind::Comment,
            "w:footnotes" => StoryKind::Footnote,
            "w:endnotes" => StoryKind::Endnote,
            _ => return Err(unsupported("invalid physical table story")),
        };
        let mut changed = false;
        for (path, ops) in groups {
            let mut n = &mut root;
            for &i in path {
                n = n
                    .elements_mut()
                    .nth(i)
                    .ok_or_else(|| unsupported("physical table not found"))?;
            }
            let (staged, table_changed) =
                structure::physical(n, &ops, package, &self.main, (kind, owner_part))?;
            package = staged;
            changed |= table_changed;
        }
        if changed {
            package.put_word(owner_part, root)?;
        }
        PackagePlan::new(package)?.to_bytes()
    }
}
