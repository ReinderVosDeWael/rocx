use crate::{
    diagnostics::{ScopedEditError, ToolError},
    editor::{InlineHandles, PhysicalTableScope, PhysicalTableTarget, Snapshot},
    schema::{
        Edit, PhysicalTableSelector,
        entry_inlines::EntryInlineChange,
        layout::{LayoutChange, LayoutScope},
        notes::{NoteChange, NoteKind},
        numbering::{NumberingChange, PhysicalNumberingScope},
        resources::ResourceQuery,
        revisions::RevisionChange,
        run_format::RunFormatChange,
        stories::{StoryBlockChange, StoryReferenceChange, StoryTextChange},
        tables::PhysicalTableChange,
    },
};
use rocx::editor::{InlineAnchor, SectionStoryKind};
use rocx::opc::ReadLimits;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

const MAX_DOCUMENTS: usize = 16;
const MAX_PREVIEWS: usize = 64;
const MAX_PREVIEW_BYTES: usize = 256 * 1024 * 1024;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct Preview {
    bytes: Vec<u8>,
    digest: String,
}

struct Document {
    snapshot: Snapshot,
    view_anchors: HashMap<String, PageView>,
    inline_handles: HashMap<String, InlineAnchor>,
    story_parts: HashMap<usize, String>,
    physical_table_targets: BTreeMap<String, PhysicalTableTarget>,
    cursors: HashMap<String, (PageView, usize)>,
    sources: Vec<PathBuf>,
    previews: HashMap<String, Preview>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageView {
    BodyInlines,
    RangeMarkers,
    Controls,
    ControlSites,
    StoryControls,
    BodyTables,
    Entries,
    EntryRuns,
    EntryInlines,
    Fields,
    FormatRuns,
    Layout(LayoutScope),
    NoteReferences,
    NumberingAbstracts,
    NumberingInstances,
    NumberingParagraphs(PhysicalNumberingScope),
    PhysicalTables(PhysicalTableScope),
    Revisions,
    Sections,
    Stories,
    DefaultHeaders,
    LinkableStories(SectionStoryKind),
    StoryInlines(usize, usize),
    Styles,
    TableCellInlines(usize, usize, usize, usize),
}

pub(crate) enum InlineSelector<'a> {
    Story {
        anchor: &'a str,
        block: usize,
    },
    TableCell {
        anchor: &'a str,
        row: usize,
        cell: usize,
        block: usize,
    },
}

impl PageView {
    fn key(self) -> &'static str {
        match self {
            Self::BodyInlines => "body-inlines",
            Self::RangeMarkers => "range-markers",
            Self::Controls => "controls",
            Self::ControlSites => "control-sites",
            Self::StoryControls => "story-controls",
            Self::BodyTables => "body-tables",
            Self::Entries => "entries",
            Self::EntryRuns => "entry-runs",
            Self::EntryInlines => "entry-inlines",
            Self::Fields => "fields",
            Self::FormatRuns => "format-runs",
            Self::Layout(_) => "layout",
            Self::NoteReferences => "note-references",
            Self::NumberingAbstracts => "numbering-abstracts",
            Self::NumberingInstances => "numbering-instances",
            Self::NumberingParagraphs(scope) => match scope {
                PhysicalNumberingScope::Cells => "numbering-paragraphs-cells",
                PhysicalNumberingScope::Stories => "numbering-paragraphs-stories",
                PhysicalNumberingScope::Entries => "numbering-paragraphs-entries",
            },
            Self::PhysicalTables(_) => "physical-tables",
            Self::Revisions => "revisions",
            Self::Sections => "sections",
            Self::Stories => "stories",
            Self::DefaultHeaders => "default-headers",
            Self::LinkableStories(_) => "linkable-stories",
            Self::StoryInlines(..) => "story-inlines",
            Self::Styles => "styles",
            Self::TableCellInlines(..) => "table-cell-inlines",
        }
    }

    fn cursor_key(self, next: usize) -> String {
        let key = self.key();
        match self {
            Self::Layout(LayoutScope::Paragraphs) => format!("{key}:paragraphs:{next}"),
            Self::Layout(LayoutScope::Sections) => format!("{key}:sections:{next}"),
            Self::PhysicalTables(PhysicalTableScope::Main) => format!("{key}:main:{next}"),
            Self::PhysicalTables(PhysicalTableScope::Story(owner)) => {
                format!("{key}:story:{owner}:{next}")
            }
            Self::PhysicalTables(PhysicalTableScope::Entry(owner)) => {
                format!("{key}:entry:{owner}:{next}")
            }
            Self::LinkableStories(kind) => format!("{key}:{}:{next}", kind.as_str()),
            Self::StoryInlines(owner, block) => format!("{key}:{owner}:{block}:{next}"),
            Self::TableCellInlines(table, row, cell, block) => {
                format!("{key}:{table}:{row}:{cell}:{block}:{next}")
            }
            _ => format!("{key}:{next}"),
        }
    }
}

#[derive(Default)]
pub struct Session {
    documents: HashMap<String, Document>,
}

const UNKNOWN: &str = "invalid_document: open or create a document in this session";
const CAPACITY: &str = "capacity: close a document to release retained snapshots and previews";

/// Resolve directory aliases but keep the leaf, even when it no longer exists.
fn path_slot(path: &Path) -> Result<PathBuf, String> {
    let leaf = path
        .file_name()
        .ok_or("invalid_path: supply a path with a file name")?;
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    Ok(fs::canonicalize(parent.unwrap_or(Path::new(".")))
        .map_err(|e| format!("io: cannot resolve parent directory: {e}"))?
        .join(leaf))
}

fn scope_anchors(value: &mut Value, owner: &str, issued: &mut impl FnMut(&str)) {
    match value {
        Value::Object(fields) => {
            for (name, field) in fields {
                if name == "anchor" {
                    if let Value::String(local) = field {
                        issued(local);
                        *local = format!("{owner}/{local}");
                    }
                } else {
                    scope_anchors(field, owner, issued);
                }
            }
        }
        Value::Array(items) => items
            .iter_mut()
            .for_each(|v| scope_anchors(v, owner, issued)),
        _ => {}
    }
}

impl Session {
    pub fn is_default_header_owner(anchor: &str) -> bool {
        anchor
            .split_once('/')
            .and_then(|(_, local)| Snapshot::default_header_owner_index(local))
            .is_some()
    }

    fn is_default_header_slot(anchor: &str) -> bool {
        anchor
            .split_once('/')
            .and_then(|(_, local)| Snapshot::default_header_slot_index(local))
            .is_some()
    }

    pub fn preview_default_header_text(
        &mut self,
        document: String,
        anchor: &str,
        mut edits: Vec<StoryTextChange>,
    ) -> Result<Value, ToolError> {
        let doc = self.document(&document)?;
        let view = PageView::DefaultHeaders;
        for edit in &mut edits {
            edit.anchor = Self::scoped_anchor(doc, &document, view, &edit.anchor)?.to_owned();
        }
        self.stage_scoped_edit(document, view, anchor, None, |snapshot, local, _| {
            snapshot
                .edit_default_header_text(local, edits)
                .map(|bytes| (bytes, Value::Null))
        })
    }

    pub fn preview_default_header_format(
        &mut self,
        document: String,
        anchor: &str,
        mut edits: Vec<RunFormatChange>,
    ) -> Result<Value, ToolError> {
        let doc = self.document(&document)?;
        let view = PageView::DefaultHeaders;
        Self::scoped_anchor(doc, &document, view, anchor)?;
        for edit in &mut edits {
            edit.anchor = Self::scoped_anchor(doc, &document, view, &edit.anchor)?.to_owned();
        }
        self.stage_scoped_edit(document, view, anchor, None, |snapshot, local, _| {
            snapshot
                .edit_default_header_format(local, edits)
                .map(|bytes| (bytes, Value::Null))
        })
    }

    fn document(&self, id: &str) -> Result<&Document, String> {
        self.documents.get(id).ok_or_else(|| UNKNOWN.into())
    }

    fn check_document_limit(&self) -> Result<(), String> {
        if self.documents.len() >= MAX_DOCUMENTS {
            return Err(CAPACITY.into());
        }
        Ok(())
    }

    fn check_preview_limit(&self, additional: usize) -> Result<(), String> {
        let retained = self.documents.values().flat_map(|d| d.previews.values());
        if retained.clone().count() >= MAX_PREVIEWS
            || retained
                .map(|p| p.bytes.len())
                .sum::<usize>()
                .saturating_add(additional)
                > MAX_PREVIEW_BYTES
        {
            return Err(CAPACITY.into());
        }
        Ok(())
    }

    fn register(
        &mut self,
        bytes: Vec<u8>,
        sources: Vec<PathBuf>,
        initial: bool,
    ) -> Result<Value, String> {
        let snapshot = Snapshot::open(&bytes).map_err(|e| format!("invalid_document_data: {e}"))?;
        self.register_snapshot(snapshot, bytes, sources, initial)
    }

    fn register_snapshot(
        &mut self,
        snapshot: Snapshot,
        bytes: Vec<u8>,
        sources: Vec<PathBuf>,
        initial: bool,
    ) -> Result<Value, String> {
        let id = Uuid::new_v4().to_string();
        let digest = hash(&bytes);
        let mut response = json!({"document": id, "snapshot": digest});
        let mut previews = HashMap::new();
        if initial {
            self.check_preview_limit(bytes.len())?;
            let handle = format!("{id}/{}", Uuid::new_v4());
            response["preview"] = json!(handle);
            response["output_snapshot"] = json!(digest);
            previews.insert(handle, Preview { bytes, digest });
        }
        self.documents.insert(
            id,
            Document {
                snapshot,
                view_anchors: HashMap::new(),
                inline_handles: HashMap::new(),
                story_parts: HashMap::new(),
                physical_table_targets: BTreeMap::new(),
                cursors: HashMap::new(),
                sources,
                previews,
            },
        );
        Ok(response)
    }

    pub fn open_document(
        &mut self,
        path: String,
        limits: Option<ReadLimits>,
    ) -> Result<Value, ToolError> {
        self.check_document_limit()?;
        let slot = path_slot(Path::new(&path))?;
        let absolute = std::path::absolute(&path).map_err(|e| format!("io: invalid path: {e}"))?;
        let resolved =
            fs::canonicalize(&path).map_err(|e| format!("io: cannot open source: {e}"))?;
        let bytes = fs::read(&resolved).map_err(|e| format!("io: cannot read source: {e}"))?;
        let snapshot =
            Snapshot::open_with_limits(&bytes, limits.unwrap_or_default()).map_err(|error| {
                ToolError::native("invalid_document_data", error, ToString::to_string)
            })?;
        self.register_snapshot(snapshot, bytes, vec![absolute, slot, resolved], false)
            .map_err(ToolError::from)
    }

    pub fn create_document(&mut self, paragraphs: Vec<String>) -> Result<Value, String> {
        self.check_document_limit()?;
        let bytes = Snapshot::create(paragraphs).map_err(|e| format!("invalid_paragraphs: {e}"))?;
        self.register(bytes, Vec::new(), true)
    }

    pub fn inspect_document(&mut self, document: String) -> Result<Value, String> {
        let doc = self.document(&document)?;
        let mut result = doc
            .snapshot
            .inspect()
            .map_err(|e| format!("inspection_failed: {e}"))?;
        scope_anchors(&mut result, &document, &mut |_| {});
        result["document"] = json!(document);
        Ok(result)
    }

    pub fn inspect_body(&self, document: String) -> Result<Value, String> {
        let mut result = self.document(&document)?.snapshot.inspect_body();
        scope_anchors(&mut result, &document, &mut |_| {});
        result["document"] = json!(document);
        Ok(result)
    }

    pub fn inspect_resources(
        &self,
        document: String,
        custom_property: Option<String>,
    ) -> Result<Value, String> {
        let doc = self.document(&document)?;
        let mut result = doc
            .snapshot
            .inspect_resources(custom_property)
            .map_err(|e| format!("inspection_failed: {e}"))?;
        result["document"] = json!(document);
        Ok(result)
    }

    pub fn inspect_resource_value(
        &self,
        document: String,
        query: ResourceQuery,
    ) -> Result<Value, String> {
        let mut result = self
            .document(&document)?
            .snapshot
            .inspect_resource_value(query)
            .map_err(|e| format!("inspection_failed: {e}"))?;
        result["document"] = json!(document);
        Ok(result)
    }

    pub fn inspect_chart_series(&self, document: String, anchor: String) -> Result<Value, String> {
        let doc = self.document(&document)?;
        let local = Self::scoped_anchor(doc, &document, PageView::BodyInlines, &anchor)?;
        let mut result = doc
            .snapshot
            .chart_series(local)
            .map_err(|e| format!("inspection_failed: {e}"))?;
        result["document"] = json!(document);
        Ok(result)
    }

    fn scoped_numbering_paragraph<'a>(
        doc: &Document,
        document: &str,
        anchor: &'a str,
        physical: bool,
    ) -> Result<&'a str, String> {
        let source = if physical {
            "numbering-paragraphs"
        } else {
            "compact body"
        };
        let (owner, local) = anchor
            .split_once('/')
            .ok_or_else(|| format!("invalid_anchor: inspect {source} for a paragraph anchor"))?;
        if owner != document {
            return Err("wrong_owner: use a paragraph anchor from the selected document".into());
        }
        let compact = doc.snapshot.has_issued_anchor(local);
        let view = doc.view_anchors.get(local);
        let eligible = if physical {
            matches!(view, Some(PageView::NumberingParagraphs(_)))
        } else {
            compact
        };
        if eligible {
            Ok(local)
        } else if compact || view.is_some() {
            Err(format!(
                "wrong_anchor_type: expected a paragraph anchor from {source}"
            ))
        } else {
            Err(format!(
                "invalid_anchor: inspect {source} for a paragraph anchor"
            ))
        }
    }

    pub fn inspect_paragraph_numbering(
        &self,
        document: String,
        anchor: String,
    ) -> Result<Value, String> {
        let doc = self.document(&document)?;
        let local = Self::scoped_numbering_paragraph(doc, &document, &anchor, false)?;
        let value = doc.snapshot.paragraph_numbering(local).map_err(|error| {
            if error.starts_with("wrong_anchor_type:") {
                error
            } else {
                format!("inspection_failed: {error}")
            }
        })?;
        Ok(json!({"document":document, "value":value}))
    }

    pub fn inspect_inline_at(&mut self, document: String, handle: String) -> Result<Value, String> {
        let doc = self.documents.get_mut(&document).ok_or(UNKNOWN)?;
        let invalid = "invalid_inline_handle: use an inline_handle issued by an inline view";
        let (owner, local) = handle.split_once('/').ok_or(invalid)?;
        if owner != document {
            return Err("wrong_owner: use an inline handle from the selected document".into());
        }
        let anchor = doc.inline_handles.get(local).ok_or(invalid)?.clone();
        let mut handles = InlineHandles {
            document: &document,
            issued: &mut doc.inline_handles,
        };
        let inline = doc
            .snapshot
            .inspect_inline(&anchor, local, &mut handles)
            .map_err(|e| format!("inspection_failed: {e}"))?;
        Ok(json!({"document":document,"inline":inline}))
    }

    pub fn inspect_inline_page(
        &mut self,
        document: String,
        selector: InlineSelector<'_>,
        limit: usize,
        cursor: Option<String>,
    ) -> Result<Value, String> {
        let doc = self.document(&document)?;
        let view = match selector {
            InlineSelector::Story { anchor, block } => {
                let local = Self::scoped_anchor(doc, &document, PageView::Stories, anchor)?;
                let owner = Snapshot::story_owner_index(local)
                    .ok_or("wrong_anchor_type: expected a physical story owner anchor")?;
                PageView::StoryInlines(owner, block)
            }
            InlineSelector::TableCell {
                anchor,
                row,
                cell,
                block,
            } => {
                let local = Self::scoped_anchor(doc, &document, PageView::BodyTables, anchor)?;
                let table = Snapshot::body_table_index(local)
                    .ok_or("wrong_anchor_type: expected a body-tables anchor")?;
                PageView::TableCellInlines(table, row, cell, block)
            }
        };
        self.inspect_page(document, view, limit, cursor)
    }

    pub fn inspect_physical_table_page(
        &mut self,
        document: String,
        selector: Option<PhysicalTableSelector>,
        limit: usize,
        cursor: Option<String>,
    ) -> Result<Value, String> {
        let doc = self.document(&document)?;
        let scope = match selector {
            None => PhysicalTableScope::All,
            Some(PhysicalTableSelector::Main) => PhysicalTableScope::Main,
            Some(PhysicalTableSelector::Story { anchor }) => {
                let local = Self::scoped_anchor(doc, &document, PageView::Stories, &anchor)?;
                let owner = Snapshot::story_owner_index(local)
                    .ok_or("wrong_anchor_type: expected a physical story owner anchor")?;
                PhysicalTableScope::Story(owner)
            }
            Some(PhysicalTableSelector::Entry { anchor }) => {
                let (view, prefix) = if anchor
                    .split_once('/')
                    .is_some_and(|(_, local)| local.starts_with("entry-inline-owner:"))
                {
                    (PageView::EntryInlines, "entry-inline-owner:")
                } else {
                    (PageView::EntryRuns, "entry-run-owner:")
                };
                let local = Self::scoped_anchor(doc, &document, view, &anchor)?;
                let owner = local
                    .strip_prefix(prefix)
                    .and_then(|index| index.parse().ok())
                    .ok_or("wrong_anchor_type: expected a physical entry owner anchor")?;
                PhysicalTableScope::Entry(owner)
            }
        };
        self.inspect_page(document, PageView::PhysicalTables(scope), limit, cursor)
    }

    pub fn inspect_page(
        &mut self,
        document: String,
        view: PageView,
        limit: usize,
        cursor: Option<String>,
    ) -> Result<Value, String> {
        let doc = self.documents.get_mut(&document).ok_or(UNKNOWN)?;
        if !(1..=100).contains(&limit) {
            return Err("invalid_input: use a page limit between 1 and 100".into());
        }
        let start = match cursor {
            None => 0,
            Some(cursor) => {
                let (owner, local) = cursor
                    .split_once('/')
                    .ok_or("invalid_cursor: restart the selected view")?;
                if owner != document {
                    return Err("wrong_owner: use a cursor from the selected document".into());
                }
                doc.cursors
                    .get(local)
                    .and_then(|&(issued_view, start)| (issued_view == view).then_some(start))
                    .ok_or("invalid_cursor: restart the selected view")?
            }
        };
        let mut inline_handles = InlineHandles {
            document: &document,
            issued: &mut doc.inline_handles,
        };
        let (mut rows, next) = match view {
            PageView::BodyInlines => {
                doc.snapshot
                    .body_inline_page(start, limit, &mut inline_handles)
            }
            PageView::RangeMarkers => doc.snapshot.range_marker_page(start, limit),
            PageView::Controls => Ok(doc.snapshot.control_page(start, limit)),
            PageView::ControlSites => Ok(doc.snapshot.control_site_page(start, limit)),
            PageView::StoryControls => doc.snapshot.story_control_page(start, limit),
            PageView::BodyTables => doc.snapshot.body_table_page(start, limit),
            PageView::Entries => doc.snapshot.entry_page(start, limit),
            PageView::EntryRuns => doc.snapshot.entry_run_page(start, limit),
            PageView::EntryInlines => {
                doc.snapshot
                    .entry_inline_page(start, limit, &mut inline_handles)
            }
            PageView::Fields => doc.snapshot.field_page(start, limit),
            PageView::FormatRuns => doc.snapshot.format_run_page(start, limit),
            PageView::Layout(scope) => doc.snapshot.layout_page(scope, start, limit),
            PageView::NoteReferences => doc.snapshot.note_reference_page(start, limit),
            PageView::NumberingAbstracts => doc.snapshot.numbering_page(true, start, limit),
            PageView::NumberingInstances => doc.snapshot.numbering_page(false, start, limit),
            PageView::NumberingParagraphs(scope) => {
                doc.snapshot.physical_numbering_page(scope, start, limit)
            }
            PageView::PhysicalTables(scope) => {
                let scope = match scope {
                    PhysicalTableScope::All => PhysicalTableScope::All,
                    PhysicalTableScope::Main => PhysicalTableScope::Main,
                    PhysicalTableScope::Story(owner) => {
                        let part = doc
                            .story_parts
                            .get(&owner)
                            .ok_or("invalid_anchor: inspect the stories view")?;
                        PhysicalTableScope::Story((owner, part.as_str()))
                    }
                    PhysicalTableScope::Entry(owner) => PhysicalTableScope::Entry(owner),
                };
                doc.snapshot
                    .physical_table_page(scope, start, limit)
                    .map(|page| {
                        doc.physical_table_targets.extend(page.targets);
                        (page.rows, page.next)
                    })
            }
            PageView::Revisions => doc.snapshot.revision_page(start, limit),
            PageView::Sections => doc.snapshot.section_page(start, limit),
            PageView::Stories => doc.snapshot.story_page(start, limit),
            PageView::DefaultHeaders => {
                doc.snapshot
                    .default_header_page(start, limit, &mut inline_handles)
            }
            PageView::LinkableStories(kind) => doc.snapshot.linkable_story_page(kind, start, limit),
            PageView::StoryInlines(owner, block) => {
                doc.snapshot
                    .story_inline_page(owner, block, start, limit, &mut inline_handles)
            }
            PageView::Styles => doc.snapshot.style_page(start, limit),
            PageView::TableCellInlines(table, row, cell, block) => {
                doc.snapshot.table_cell_inline_page(
                    table,
                    (row, cell),
                    block,
                    start,
                    limit,
                    &mut inline_handles,
                )
            }
        }
        .map_err(|e| format!("inspection_failed: {e}"))?;
        // All selectors publish the same target types, independent of page scope.
        let anchor_view = match view {
            PageView::PhysicalTables(_) => PageView::PhysicalTables(PhysicalTableScope::All),
            PageView::Layout(_) => PageView::Layout(LayoutScope::All),
            _ => view,
        };
        for row in &mut rows {
            if view == PageView::Stories {
                let owner = &row["owner"];
                if let Some((index, part)) = owner["anchor"]
                    .as_str()
                    .and_then(Snapshot::story_owner_index)
                    .zip(owner["part_name"].as_str())
                {
                    doc.story_parts
                        .entry(index)
                        .or_insert_with(|| part.to_owned());
                }
            }
            scope_anchors(row, &document, &mut |local| {
                if view == PageView::Sections && Snapshot::story_slot_index(local).is_some() {
                    // A section-issued slot can edit references, but must not
                    // grant or revoke the capabilities of a stories-issued slot.
                    doc.view_anchors
                        .entry(local.to_owned())
                        .or_insert(anchor_view);
                } else {
                    doc.view_anchors.insert(local.to_owned(), anchor_view);
                }
            });
        }
        let next_cursor = next.map(|n| {
            let local = view.cursor_key(n);
            doc.cursors.insert(local.clone(), (view, n));
            format!("{document}/{local}")
        });
        let field = match view {
            PageView::BodyInlines => "body_inlines",
            PageView::RangeMarkers => "range_markers",
            PageView::ControlSites => "control_sites",
            PageView::StoryControls => "story_controls",
            PageView::BodyTables => "body_tables",
            PageView::FormatRuns => "format_runs",
            PageView::NoteReferences => "note_references",
            PageView::NumberingAbstracts => "abstracts",
            PageView::NumberingInstances => "instances",
            PageView::NumberingParagraphs(_) => "paragraphs",
            PageView::EntryRuns => "entry_runs",
            PageView::EntryInlines => "entry_inlines",
            PageView::PhysicalTables(_) => "physical_tables",
            PageView::DefaultHeaders => "default_headers",
            PageView::LinkableStories(_) => "linkable_stories",
            PageView::StoryInlines(..) | PageView::TableCellInlines(..) => "inlines",
            _ => view.key(),
        };
        let mut result = json!({"document":document, (field):rows, "next_cursor":next_cursor});
        if view == PageView::Styles {
            result["part_name"] = json!(
                doc.snapshot
                    .styles_part_name()
                    .map_err(|e| format!("inspection_failed: {e}"))?
            );
        }
        Ok(result)
    }

    pub fn preview_entry(
        &mut self,
        document: String,
        anchor: String,
        edit: rocx::editor::EntryEdit,
    ) -> Result<Value, ToolError> {
        self.stage_scoped_edit(
            document,
            PageView::Entries,
            &anchor,
            None,
            |snapshot, local, _| {
                snapshot
                    .edit_entry(local, edit)
                    .map(|bytes| (bytes, Value::Null))
            },
        )
    }

    pub fn preview_note_reference(
        &mut self,
        document: String,
        anchor: String,
        kind: NoteKind,
        edit: NoteChange,
    ) -> Result<Value, ToolError> {
        self.stage_scoped_edit(
            document,
            PageView::NoteReferences,
            &anchor,
            None,
            |snapshot, local, _| {
                snapshot
                    .edit_note_reference(local, kind, edit)
                    .map(|bytes| (bytes, Value::Null))
            },
        )
    }

    pub fn preview_revision(
        &mut self,
        document: String,
        anchor: String,
        paired: Option<String>,
        edit: RevisionChange,
    ) -> Result<Value, ToolError> {
        self.stage_scoped_edit(
            document,
            PageView::Revisions,
            &anchor,
            paired.as_deref(),
            |snapshot, local, paired| snapshot.preview_revision(local, paired, edit),
        )
    }

    pub fn preview_layout(
        &mut self,
        document: String,
        mut edits: Vec<LayoutChange>,
    ) -> Result<Value, ToolError> {
        let doc = self.document(&document)?;
        for edit in &mut edits {
            edit.anchor = Self::scoped_anchor(
                doc,
                &document,
                PageView::Layout(LayoutScope::All),
                &edit.anchor,
            )?
            .to_owned();
        }
        self.check_preview_limit(0)?;
        let (bytes, changes) = doc
            .snapshot
            .preview_layout(edits)
            .map_err(|error| ToolError::scoped(error, "revise the layout edits and retry"))?;
        self.retain_preview(document, bytes, changes)
            .map_err(ToolError::from)
    }

    pub fn preview_story_text(
        &mut self,
        document: String,
        anchor: &str,
        mut edits: Vec<StoryTextChange>,
    ) -> Result<Value, ToolError> {
        let doc = self.document(&document)?;
        let view = if anchor
            .split_once('/')
            .is_some_and(|(_, local)| doc.view_anchors.get(local) == Some(&PageView::EntryRuns))
        {
            PageView::EntryRuns
        } else {
            PageView::Stories
        };
        for edit in &mut edits {
            edit.anchor = Self::scoped_anchor(doc, &document, view, &edit.anchor)?.to_owned();
        }
        self.stage_scoped_edit(document, view, anchor, None, |snapshot, local, _| {
            let bytes = match view {
                PageView::EntryRuns => snapshot.edit_entry_story_text(local, edits),
                _ => snapshot.edit_story_text(local, edits),
            }?;
            Ok((bytes, Value::Null))
        })
    }

    pub fn preview_story_reference(
        &mut self,
        document: String,
        anchor: &str,
        mut edit: StoryReferenceChange,
    ) -> Result<Value, ToolError> {
        let doc = self.document(&document)?;
        if let StoryReferenceChange::Link { anchor } = &mut edit {
            let view = match anchor
                .split_once('/')
                .and_then(|(_, local)| doc.view_anchors.get(local))
            {
                Some(view @ PageView::LinkableStories(_)) => *view,
                Some(PageView::DefaultHeaders) => PageView::DefaultHeaders,
                _ => PageView::Stories,
            };
            *anchor = Self::scoped_anchor(doc, &document, view, anchor)?.to_owned();
        }
        let view = match anchor.split_once('/') {
            Some((_, local))
                if Snapshot::story_slot_index(local).is_some()
                    && doc.view_anchors.get(local) == Some(&PageView::Sections) =>
            {
                PageView::Sections
            }
            _ => PageView::Stories,
        };
        self.stage_scoped_edit(document, view, anchor, None, |snapshot, local, _| {
            snapshot
                .edit_story_reference(local, edit)
                .map(|bytes| (bytes, Value::Null))
        })
    }

    pub fn preview_entry_inlines(
        &mut self,
        document: String,
        anchor: &str,
        mut edits: Vec<EntryInlineChange>,
    ) -> Result<Value, ToolError> {
        let doc = self.document(&document)?;
        let view = PageView::EntryInlines;
        Self::scoped_anchor(doc, &document, view, anchor)?;
        if edits.is_empty() {
            return Err("invalid_batch: supply at least one entry inline edit"
                .to_owned()
                .into());
        }
        for edit in &mut edits {
            edit.anchor = Self::scoped_anchor(doc, &document, view, &edit.anchor)?.to_owned();
        }
        self.stage_scoped_edit(document, view, anchor, None, |snapshot, local, _| {
            snapshot
                .edit_entry_inlines(local, edits)
                .map(|bytes| (bytes, Value::Null))
        })
    }

    pub fn preview_noop(&mut self, document: String) -> Result<Value, ToolError> {
        let doc = self.document(&document)?;
        self.check_preview_limit(0)?;
        let (bytes, changes) = doc
            .snapshot
            .preview_noop()
            .map_err(|error| ToolError::native("rejected_batch", error, ToString::to_string))?;
        self.retain_preview(document, bytes, changes)
            .map_err(ToolError::from)
    }

    pub fn preview_story_blocks(
        &mut self,
        document: String,
        anchor: &str,
        edit: StoryBlockChange,
    ) -> Result<Value, ToolError> {
        let view = if Self::is_default_header_slot(anchor) {
            PageView::DefaultHeaders
        } else {
            PageView::Stories
        };
        self.stage_scoped_edit(document, view, anchor, None, |snapshot, local, _| {
            snapshot
                .edit_story_blocks(local, edit)
                .map(|bytes| (bytes, Value::Null))
        })
    }

    pub fn preview_run_format(
        &mut self,
        document: String,
        anchor: &str,
        mut edits: Vec<RunFormatChange>,
    ) -> Result<Value, ToolError> {
        let doc = self.document(&document)?;
        let view = PageView::FormatRuns;
        Self::scoped_anchor(doc, &document, view, anchor)?;
        for edit in &mut edits {
            edit.anchor = Self::scoped_anchor(doc, &document, view, &edit.anchor)?.to_owned();
        }
        self.stage_scoped_edit(document, view, anchor, None, |snapshot, local, _| {
            snapshot
                .edit_run_format(local, edits)
                .map(|bytes| (bytes, Value::Null))
        })
    }

    pub fn preview_part_tables(
        &mut self,
        document: String,
        anchor: &str,
        mut edits: Vec<PhysicalTableChange>,
    ) -> Result<Value, ToolError> {
        let doc = self.document(&document)?;
        let view = PageView::PhysicalTables(PhysicalTableScope::All);
        let local = Self::scoped_anchor(doc, &document, view, anchor)?;
        for edit in &mut edits {
            edit.anchor = Self::scoped_anchor(doc, &document, view, &edit.anchor)?.to_owned();
        }
        self.check_preview_limit(0)?;
        let bytes = doc
            .snapshot
            .edit_part_tables(
                doc.physical_table_targets.get(local),
                |anchor| doc.physical_table_targets.get(anchor),
                edits,
            )
            .map_err(ToolError::from)?;
        self.retain_preview(document, bytes, Value::Null)
            .map_err(ToolError::from)
    }

    fn scoped_anchor<'a>(
        doc: &Document,
        document: &str,
        view: PageView,
        anchor: &'a str,
    ) -> Result<&'a str, String> {
        let (owner, local) = anchor
            .split_once('/')
            .ok_or_else(|| format!("invalid_anchor: inspect the {} view", view.key()))?;
        if owner != document {
            return Err("wrong_owner: use an anchor from the selected document".into());
        }
        match doc.view_anchors.get(local) {
            Some(issued_view) if *issued_view == view => Ok(local),
            Some(_) => Err(format!(
                "wrong_anchor_type: inspect the {} view",
                view.key()
            )),
            None if view == PageView::Revisions && doc.snapshot.has_issued_anchor(local) => {
                // Proposals use the same direct body targets as ordinary editing.
                // The revision adapter still checks the target subtype.
                Ok(local)
            }
            None if doc.snapshot.has_issued_anchor(local) => Err(format!(
                "wrong_anchor_type: inspect the {} view",
                view.key()
            )),
            None => Err(format!("invalid_anchor: inspect the {} view", view.key())),
        }
    }

    fn stage_scoped_edit(
        &mut self,
        document: String,
        view: PageView,
        anchor: &str,
        paired: Option<&str>,
        run: impl FnOnce(&Snapshot, &str, Option<&str>) -> Result<(Vec<u8>, Value), ScopedEditError>,
    ) -> Result<Value, ToolError> {
        let doc = self.document(&document)?;
        let local = Self::scoped_anchor(doc, &document, view, anchor)?;
        let paired = paired
            .map(|anchor| Self::scoped_anchor(doc, &document, view, anchor))
            .transpose()?;
        self.check_preview_limit(0)?;
        let (bytes, changes) = run(&doc.snapshot, local, paired)?;
        self.retain_preview(document, bytes, changes)
            .map_err(ToolError::from)
    }

    pub fn preview_edits(
        &mut self,
        document: String,
        mut edits: Vec<Edit>,
    ) -> Result<Value, ToolError> {
        let doc = self.document(&document)?;
        if edits.is_empty() {
            return Err(ToolError::from(
                "invalid_batch: supply at least one edit".to_owned(),
            ));
        }
        for (i, edit) in edits.iter_mut().enumerate() {
            if let Edit::Numbering { edit } = edit {
                match edit {
                    NumberingChange::Paragraph { anchor, .. } => {
                        *anchor = Self::scoped_numbering_paragraph(doc, &document, anchor, false)?
                            .to_owned();
                    }
                    NumberingChange::PhysicalParagraph { anchor, .. } => {
                        *anchor = Self::scoped_numbering_paragraph(doc, &document, anchor, true)?
                            .to_owned();
                    }
                    _ => {}
                }
                continue;
            }
            if let Edit::Style { edit } = edit {
                edit.scope(|anchor| {
                    *anchor =
                        Self::scoped_anchor(doc, &document, PageView::Styles, anchor)?.to_owned();
                    Ok(())
                })?;
                continue;
            }
            if let Edit::Control { edit } = edit {
                edit.scope(|anchor, is_site| {
                    let view = if is_site {
                        PageView::ControlSites
                    } else {
                        PageView::Controls
                    };
                    *anchor = Self::scoped_anchor(doc, &document, view, anchor)?.to_owned();
                    Ok(())
                })?;
                continue;
            }
            if let Edit::PairedRange { endpoints, .. } = edit {
                if let Some(endpoints) = endpoints {
                    for anchor in [&mut endpoints.first, &mut endpoints.last] {
                        *anchor =
                            Self::scoped_anchor(doc, &document, PageView::BodyInlines, anchor)?
                                .to_owned();
                    }
                }
                continue;
            }
            let view = match edit {
                Edit::HeaderImage { anchor, .. } if Self::is_default_header_slot(anchor) => {
                    Some(PageView::DefaultHeaders)
                }
                Edit::HeaderImage { .. } => Some(PageView::Stories),
                Edit::Block { .. } => Some(PageView::Sections),
                Edit::Field { .. } => Some(PageView::Fields),
                Edit::Inline { .. } => Some(PageView::BodyInlines),
                Edit::TableStructure { .. } => Some(PageView::BodyTables),
                _ => None,
            };
            let Some(anchor) = edit.anchor_slot() else {
                continue;
            };
            let (owner, local) = anchor
                .split_once('/')
                .ok_or_else(|| "invalid_anchor: use an inspected anchor".to_owned())?;
            if owner != document {
                return Err(
                    format!("wrong_owner: inspect the selected document for edit {i}").into(),
                );
            }
            let matches_view = view.is_some_and(|view| doc.view_anchors.get(local) == Some(&view));
            if !doc.snapshot.has_issued_anchor(local) && !matches_view {
                if doc.view_anchors.contains_key(local) {
                    return Err(format!(
                        "wrong_anchor_type: edit {i} requires an ordinary edit anchor"
                    )
                    .into());
                }
                return Err(
                    format!("invalid_anchor: inspect the document again for edit {i}").into(),
                );
            }
            anchor.drain(..owner.len() + 1);
        }
        self.check_preview_limit(0)?;
        let (bytes, changes) = doc.snapshot.preview(edits)?;
        self.retain_preview(document, bytes, changes)
            .map_err(ToolError::from)
    }

    fn retain_preview(
        &mut self,
        document: String,
        bytes: Vec<u8>,
        changes: Value,
    ) -> Result<Value, String> {
        self.check_preview_limit(bytes.len())?;
        let digest = hash(&bytes);
        let handle = format!("{document}/{}", Uuid::new_v4());
        let result = json!({"document":document,"preview":handle,"output_snapshot":digest,"changes":changes});
        self.documents
            .get_mut(&document)
            .expect("document still live")
            .previews
            .insert(handle, Preview { bytes, digest });
        Ok(result)
    }

    pub fn save_copy(
        &mut self,
        document: String,
        preview: String,
        output: String,
    ) -> Result<Value, String> {
        let doc = self.document(&document)?;
        if let Some((owner, _)) = preview.split_once('/')
            && owner != document
        {
            return Err("wrong_owner: use a preview from the selected document".into());
        }
        let selected = doc
            .previews
            .get(&preview)
            .ok_or("invalid_preview: create a preview in this document")?;
        let slot = path_slot(Path::new(&output))?;
        let absolute =
            std::path::absolute(&output).map_err(|e| format!("io: invalid path: {e}"))?;
        if self
            .documents
            .values()
            .any(|d| d.sources.contains(&slot) || d.sources.contains(&absolute))
        {
            return Err(
                "source_path: choose a new destination distinct from the open source".into(),
            );
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)
            .map_err(|e| {
                if e.kind() == ErrorKind::AlreadyExists {
                    "destination_exists: choose a new output path".to_string()
                } else {
                    format!("io: cannot create output: {e}")
                }
            })?;
        file.write_all(&selected.bytes)
            .and_then(|()| file.flush())
            .map_err(|e| format!("io: write failed; new copy may be incomplete: {e}"))?;
        Ok(json!({"output":output,"output_snapshot":selected.digest}))
    }

    pub fn close_document(&mut self, document: String) -> Result<Value, String> {
        self.documents.remove(&document).ok_or(UNKNOWN)?;
        Ok(json!({"closed":document}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::ToolKind;

    #[test]
    fn physical_inline_pages_keep_typed_values_and_bind_cursors_to_paragraphs() {
        use rocx::{
            Document, Footer, Header, HeaderFooterType, Hyperlink, HyperlinkTarget, Paragraph,
            RunFormat, Section, Table, TableCell, TableRow, TextRun,
        };

        let paragraph = |label: &str| {
            Paragraph::from_children(vec![
                TextRun::new(label.to_owned())
                    .with_format(RunFormat {
                        bold: Some(false),
                        ..RunFormat::default()
                    })
                    .into(),
                Hyperlink::new(
                    HyperlinkTarget::external("https://example.org").unwrap(),
                    vec![TextRun::new("link").into()],
                )
                .into(),
            ])
        };
        let table = |blocks| {
            Table::new(vec![
                TableRow::new(vec![TableCell::new(blocks).into()]).into(),
            ])
        };
        let section = Section::new(vec![
            table(vec![
                table(vec![Paragraph::new("nested").into()]).into(),
                paragraph("cell").into(),
                Paragraph::from_children(vec![]).into(),
            ])
            .into(),
        ])
        .with_headers(
            [(
                HeaderFooterType::Default,
                Header::new(vec![paragraph("header").into(), paragraph("next").into()]).into(),
            )]
            .into_iter()
            .collect(),
        )
        .with_footers(
            [(
                HeaderFooterType::Default,
                Footer::new(vec![paragraph("footer").into()]).into(),
            )]
            .into_iter()
            .collect(),
        );
        let bytes = futures_executor::block_on(
            Document::new(vec![section.into(), Section::new(vec![]).into()]).resolve(),
        )
        .unwrap()
        .to_docx()
        .unwrap();
        let mut session = Session::default();
        let other = session.register(bytes.clone(), Vec::new(), false).unwrap();
        let snapshot = session.register(bytes, Vec::new(), false).unwrap();
        let document = &snapshot["document"];
        let inspect = |session: &mut Session, document: &Value, view: Value| {
            ToolKind::Inspect
                .parse(
                    json!({"document":document, "view":view})
                        .as_object()
                        .unwrap()
                        .clone(),
                )
                .unwrap()
                .dispatch(session)
        };
        let stories = inspect(&mut session, document, json!({"kind":"stories"})).unwrap();
        let header = &stories["stories"][0]["owner"]["anchor"];
        let alias = &stories["stories"][6]["owner"]["anchor"];
        let footer = &stories["stories"][3]["owner"]["anchor"];
        assert_eq!(header, alias);
        let tables = inspect(&mut session, document, json!({"kind":"body-tables"})).unwrap();
        let body_table = &tables["body_tables"][0]["anchor"];
        let story_view = |anchor: &Value, block| json!({"kind":"story-inlines","anchor":anchor,"block_index":block,"limit":1});
        let cell_view = |block| {
            json!({"kind":"table-cell-inlines","anchor":body_table,
                   "row_index":0,"cell_index":0,"block_index":block,"limit":1})
        };
        let empty_header = inspect(&mut session, document, story_view(header, 0)).unwrap();
        assert_eq!(empty_header["inlines"], json!([]));
        let header_page = inspect(&mut session, document, story_view(header, 1)).unwrap();
        assert_eq!(
            header_page["inlines"][0]["items"],
            json!([{"kind":"text","text":"header"}]),
            "{header_page}"
        );
        assert_eq!(header_page["inlines"][0]["format"]["bold"], false);
        assert_eq!(header_page["inlines"][0]["inline_index"], 0);
        let next = inspect(&mut session, document, story_view(header, 2)).unwrap();
        assert_ne!(header_page["next_cursor"], next["next_cursor"]);
        assert_eq!(next["inlines"][0]["text"], "next");
        let footer_page = inspect(&mut session, document, story_view(footer, 1)).unwrap();
        assert_eq!(footer_page["inlines"][0]["text"], "footer");
        // The compiler adds empty paragraphs before a leading table and after every table.
        let cell_page = inspect(&mut session, document, cell_view(3)).unwrap();
        assert_eq!(cell_page["inlines"][0]["text"], "cell");
        let empty = inspect(&mut session, document, cell_view(2)).unwrap();
        assert_eq!(empty["inlines"], json!([]));
        assert!(empty["next_cursor"].is_null());
        assert!(
            inspect(&mut session, document, cell_view(1))
                .unwrap_err()
                .category
                == "inspection_failed"
        );
        assert!(
            inspect(
                &mut session,
                document,
                story_view(&stories["stories"][0]["anchor"], 0)
            )
            .unwrap_err()
            .category
                == "wrong_anchor_type"
        );
        for mut wrong_target in [story_view(header, 2), story_view(footer, 1), cell_view(3)] {
            wrong_target["cursor"] = header_page["next_cursor"].clone();
            assert!(
                inspect(&mut session, document, wrong_target)
                    .unwrap_err()
                    .category
                    == "invalid_cursor"
            );
        }
        let other_doc = &other["document"];
        let other_stories = inspect(&mut session, other_doc, json!({"kind":"stories"})).unwrap();
        let mut foreign_cursor = story_view(&other_stories["stories"][0]["owner"]["anchor"], 1);
        foreign_cursor["cursor"] = header_page["next_cursor"].clone();
        assert!(
            inspect(&mut session, other_doc, foreign_cursor)
                .unwrap_err()
                .category
                == "wrong_owner"
        );
        for (mut view, first) in [
            (story_view(alias, 1), header_page),
            (cell_view(3), cell_page),
        ] {
            view["cursor"] = first["next_cursor"].clone();
            let second = inspect(&mut session, document, view).unwrap();
            let link = &second["inlines"][0];
            assert_eq!(link["inline_index"], 1);
            assert_eq!(link["kind"], "hyperlink");
            assert!(link["relationship_id"].is_string());
            assert_eq!(link["children"][0]["items"][0]["text"], "link");
            assert!(second["next_cursor"].is_null());
        }
    }

    #[test]
    fn previews_require_issued_anchors_from_the_selected_document() {
        let mut session = Session::default();
        let create = |session: &mut Session| {
            session.create_document(vec!["text".into()]).unwrap()["document"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        let document = create(&mut session);
        let other = create(&mut session);
        let foreign = session.inspect_document(other).unwrap()["body"][0]["anchor"]
            .as_str()
            .unwrap()
            .to_owned();
        let insert = |anchor| Edit::InsertParagraph {
            anchor,
            text: "after".into(),
        };
        let guessed = format!("{document}/paragraph:0");
        for (anchor, category) in [
            ("no-owner".into(), "invalid_anchor"),
            (guessed.clone(), "invalid_anchor"),
            (foreign, "wrong_owner"),
        ] {
            let error = session
                .preview_edits(document.clone(), vec![insert(anchor)])
                .unwrap_err();
            assert_eq!(error.category, category);
        }

        let inspection = session.inspect_document(document.clone()).unwrap();
        let inspected = inspection["body"][0]["anchor"].as_str().unwrap();
        assert_eq!(inspected, guessed);
        for (anchor, category) in [
            (format!("{document}/paragraph:999"), "invalid_anchor"),
            (inspected.to_owned(), "wrong_anchor_type"),
        ] {
            let error = session
                .preview_edits(document.clone(), vec![Edit::RunRemove { anchor }])
                .unwrap_err();
            assert_eq!(error.category, category);
        }
        assert!(
            session
                .preview_edits(document.clone(), vec![insert(inspected.to_owned())])
                .is_ok()
        );
        assert_eq!(session.inspect_document(document).unwrap(), inspection);
    }

    #[test]
    fn rejected_scalar_boundaries_leave_the_same_handles_usable_for_retry() {
        let mut session = Session::default();
        let initial = session.create_document(vec!["A😀B".into()]).unwrap();
        let document = initial["document"].as_str().unwrap();
        let inspection = session.inspect_document(document.to_owned()).unwrap();
        let paragraph = &inspection["body"][0];
        let anchor = &paragraph["anchor"];
        let range = |start, end| json!({"kind":"text-range","anchor":anchor,"start":start,"end":end,"text":""});
        let insert = |offset, text: &str| json!({"kind":"insert-text","anchor":anchor,"offset":offset,"text":text});
        let preview = |session: &mut Session, edits: Value| {
            ToolKind::Preview
                .parse(
                    json!({"document":document,"edits":edits})
                        .as_object()
                        .unwrap()
                        .clone(),
                )
                .unwrap()
                .dispatch(session)
        };
        for (edits, detail) in [
            (json!([range(1, 1)]), "empty or reversed"),
            (json!([range(2, 1)]), "empty or reversed"),
            (json!([range(1, 4)]), "out-of-bounds"),
            (json!([insert(4, "x")]), "out-of-bounds"),
            (json!([insert(0, "")]), "nonempty text"),
            (json!([range(0, 2), insert(1, "x")]), "overlapping"),
            (json!([insert(2, "x"), insert(2, "y")]), "overlapping"),
        ] {
            let error = preview(&mut session, edits).unwrap_err();
            assert_eq!(error.category, "rejected_batch");
            assert!(error.message.contains(detail), "{error:?}");
        }
        assert_eq!(session.document(document).unwrap().previews.len(), 1);
        let other = session.create_document(vec!["other".into()]).unwrap();
        let foreign = session
            .inspect_document(other["document"].as_str().unwrap().to_owned())
            .unwrap();
        for (anchor, category) in [
            (&paragraph["runs"][0]["anchor"], "wrong_anchor_type"),
            (&foreign["body"][0]["anchor"], "wrong_owner"),
        ] {
            let error = preview(
                &mut session,
                json!([{"kind":"insert-text","anchor":anchor,"offset":0,"text":"x"}]),
            )
            .unwrap_err();
            assert_eq!(error.category, category);
        }
        let result = preview(&mut session, json!([insert(3, "!"), range(1, 2)])).unwrap();
        assert_eq!(result["changes"]["run_text"][0]["after"], "AB!");
        let retained = &session.document(document).unwrap().previews;
        assert_eq!(retained.len(), 2);
        assert_eq!(
            retained[initial["preview"].as_str().unwrap()].digest,
            initial["snapshot"]
        );
        assert_eq!(
            session.inspect_document(document.to_owned()).unwrap(),
            inspection
        );
    }

    #[test]
    fn created_documents_reserve_both_handles_or_neither() {
        let mut session = Session::default();
        for _ in 0..MAX_DOCUMENTS {
            session.create_document(Vec::new()).unwrap();
        }
        assert_eq!(session.create_document(Vec::new()).unwrap_err(), CAPACITY);
        let document = session.documents.keys().next().unwrap().clone();
        session.close_document(document).unwrap();
        let next = session.create_document(Vec::new()).unwrap();
        let doc = session
            .document(next["document"].as_str().unwrap())
            .unwrap();
        assert_eq!(doc.previews.len(), 1);
        assert!(doc.previews.contains_key(next["preview"].as_str().unwrap()));
    }

    #[test]
    fn rejected_preview_reservations_preserve_retained_results() {
        let mut session = Session::default();
        let initial = session.create_document(vec!["keep me".into()]).unwrap();
        let document = initial["document"].as_str().unwrap();
        assert_eq!(
            session.check_preview_limit(MAX_PREVIEW_BYTES).unwrap_err(),
            CAPACITY
        );
        let doc = session.documents.get_mut(document).unwrap();
        for count in 1..MAX_PREVIEWS {
            doc.previews.insert(
                count.to_string(),
                Preview {
                    bytes: Vec::new(),
                    digest: hash(&[]),
                },
            );
        }
        assert_eq!(session.create_document(Vec::new()).unwrap_err(), CAPACITY);
        assert_eq!(session.documents.len(), 1);
        let retained =
            &session.document(document).unwrap().previews[initial["preview"].as_str().unwrap()];
        assert_eq!(hash(&retained.bytes), initial["snapshot"]);
        session.close_document(document.to_string()).unwrap();
        session.create_document(Vec::new()).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn removed_source_and_its_opened_symlink_remain_reserved() {
        let dir = std::env::temp_dir().join(format!("rocx-mcp-{}", Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        let source = dir.join("source.docx");
        let alias = dir.join("alias.docx");
        fs::write(&source, Snapshot::create(vec![]).unwrap()).unwrap();
        std::os::unix::fs::symlink(&source, &alias).unwrap();
        let mut session = Session::default();
        session
            .open_document(alias.to_str().unwrap().into(), None)
            .unwrap();
        let new = session.create_document(Vec::new()).unwrap();
        fs::remove_file(&source).unwrap();
        fs::remove_file(&alias).unwrap();
        for output in [source, alias] {
            assert!(
                session
                    .save_copy(
                        new["document"].as_str().unwrap().into(),
                        new["preview"].as_str().unwrap().into(),
                        output.to_str().unwrap().into(),
                    )
                    .unwrap_err()
                    .starts_with("source_path:")
            );
            assert!(!output.exists());
        }
        fs::remove_dir(dir).unwrap();
    }
}
