use crate::{
    editor::Snapshot,
    schema::{
        Edit,
        layout::LayoutChange,
        notes::{NoteChange, NoteKind},
        resources::ResourceQuery,
        revisions::RevisionChange,
        run_format::RunFormatChange,
        stories::{StoryBlockChange, StoryReferenceChange, StoryTextChange},
        tables::PhysicalTableChange,
    },
};
use rocx::opc::ReadLimits;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
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
    cursors: HashMap<String, (PageView, usize)>,
    sources: Vec<PathBuf>,
    previews: HashMap<String, Preview>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageView {
    BodyInlines,
    RangeMarkers,
    BodyTables,
    Entries,
    EntryRuns,
    Fields,
    FormatRuns,
    Layout,
    NoteReferences,
    PhysicalTables,
    Revisions,
    Sections,
    Stories,
}

impl PageView {
    fn key(self) -> &'static str {
        match self {
            Self::BodyInlines => "body-inlines",
            Self::RangeMarkers => "range-markers",
            Self::BodyTables => "body-tables",
            Self::Entries => "entries",
            Self::EntryRuns => "entry-runs",
            Self::Fields => "fields",
            Self::FormatRuns => "format-runs",
            Self::Layout => "layout",
            Self::NoteReferences => "note-references",
            Self::PhysicalTables => "physical-tables",
            Self::Revisions => "revisions",
            Self::Sections => "sections",
            Self::Stories => "stories",
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
        limits: Option<ReadLimits>,
    ) -> Result<Value, String> {
        let snapshot = match limits {
            Some(limits) => Snapshot::open_with_limits(&bytes, limits),
            None => Snapshot::open(&bytes),
        }
        .map_err(|e| format!("invalid_document_data: {e}"))?;
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
    ) -> Result<Value, String> {
        self.check_document_limit()?;
        let slot = path_slot(Path::new(&path))?;
        let absolute = std::path::absolute(&path).map_err(|e| format!("io: invalid path: {e}"))?;
        let resolved =
            fs::canonicalize(&path).map_err(|e| format!("io: cannot open source: {e}"))?;
        let bytes = fs::read(&resolved).map_err(|e| format!("io: cannot read source: {e}"))?;
        self.register(bytes, vec![absolute, slot, resolved], false, limits)
    }

    pub fn create_document(&mut self, paragraphs: Vec<String>) -> Result<Value, String> {
        self.check_document_limit()?;
        let bytes = Snapshot::create(paragraphs).map_err(|e| format!("invalid_paragraphs: {e}"))?;
        self.register(bytes, Vec::new(), true, None)
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
        let (mut rows, next) = match view {
            PageView::BodyInlines => doc.snapshot.body_inline_page(start, limit),
            PageView::RangeMarkers => doc.snapshot.range_marker_page(start, limit),
            PageView::BodyTables => doc.snapshot.body_table_page(start, limit),
            PageView::Entries => doc.snapshot.entry_page(start, limit),
            PageView::EntryRuns => doc.snapshot.entry_run_page(start, limit),
            PageView::Fields => doc.snapshot.field_page(start, limit),
            PageView::FormatRuns => doc.snapshot.format_run_page(start, limit),
            PageView::Layout => doc.snapshot.layout_page(start, limit),
            PageView::NoteReferences => doc.snapshot.note_reference_page(start, limit),
            PageView::PhysicalTables => doc.snapshot.physical_table_page(start, limit),
            PageView::Revisions => doc.snapshot.revision_page(start, limit),
            PageView::Sections => doc.snapshot.section_page(start, limit),
            PageView::Stories => doc.snapshot.story_page(start, limit),
        }
        .map_err(|e| format!("inspection_failed: {e}"))?;
        for row in &mut rows {
            scope_anchors(row, &document, &mut |local| {
                doc.view_anchors.insert(local.to_owned(), view);
            });
        }
        let next_cursor = next.map(|n| {
            let local = format!("{}:{n}", view.key());
            doc.cursors.insert(local.clone(), (view, n));
            format!("{document}/{local}")
        });
        let field = match view {
            PageView::BodyInlines => "body_inlines",
            PageView::RangeMarkers => "range_markers",
            PageView::BodyTables => "body_tables",
            PageView::FormatRuns => "format_runs",
            PageView::NoteReferences => "note_references",
            PageView::EntryRuns => "entry_runs",
            PageView::PhysicalTables => "physical_tables",
            _ => view.key(),
        };
        Ok(json!({"document":document, (field):rows, "next_cursor":next_cursor}))
    }

    pub fn preview_entry(
        &mut self,
        document: String,
        anchor: String,
        edit: rocx::editor::EntryEdit,
    ) -> Result<Value, String> {
        self.preview_scoped(
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
    ) -> Result<Value, String> {
        self.preview_scoped(
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
    ) -> Result<Value, String> {
        self.preview_scoped(
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
    ) -> Result<Value, String> {
        let doc = self.document(&document)?;
        for edit in &mut edits {
            edit.anchor =
                Self::scoped_anchor(doc, &document, PageView::Layout, &edit.anchor)?.to_owned();
        }
        self.check_preview_limit(0)?;
        let (bytes, changes) = doc
            .snapshot
            .preview_layout(edits)
            .map_err(|e| format!("rejected_batch: {e}; revise the layout edits and retry"))?;
        self.retain_preview(document, bytes, changes)
    }

    pub fn preview_story_text(
        &mut self,
        document: String,
        anchor: &str,
        mut edits: Vec<StoryTextChange>,
    ) -> Result<Value, String> {
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
        self.preview_scoped(document, view, anchor, None, |snapshot, local, _| {
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
    ) -> Result<Value, String> {
        let doc = self.document(&document)?;
        if let StoryReferenceChange::Link { anchor } = &mut edit {
            *anchor = Self::scoped_anchor(doc, &document, PageView::Stories, anchor)?.to_owned();
        }
        self.preview_scoped(
            document,
            PageView::Stories,
            anchor,
            None,
            |snapshot, local, _| {
                snapshot
                    .edit_story_reference(local, edit)
                    .map(|bytes| (bytes, Value::Null))
            },
        )
    }

    pub fn preview_noop(&mut self, document: String) -> Result<Value, String> {
        let doc = self.document(&document)?;
        self.check_preview_limit(0)?;
        let (bytes, changes) = doc
            .snapshot
            .preview_noop()
            .map_err(|e| format!("rejected_batch: {e}"))?;
        self.retain_preview(document, bytes, changes)
    }

    pub fn preview_story_blocks(
        &mut self,
        document: String,
        anchor: &str,
        edit: StoryBlockChange,
    ) -> Result<Value, String> {
        self.preview_scoped(
            document,
            PageView::Stories,
            anchor,
            None,
            |snapshot, local, _| {
                snapshot
                    .edit_story_blocks(local, edit)
                    .map(|bytes| (bytes, Value::Null))
            },
        )
    }

    pub fn preview_run_format(
        &mut self,
        document: String,
        anchor: &str,
        mut edits: Vec<RunFormatChange>,
    ) -> Result<Value, String> {
        let doc = self.document(&document)?;
        let view = PageView::FormatRuns;
        Self::scoped_anchor(doc, &document, view, anchor)?;
        for edit in &mut edits {
            edit.anchor = Self::scoped_anchor(doc, &document, view, &edit.anchor)?.to_owned();
        }
        self.preview_scoped(document, view, anchor, None, |snapshot, local, _| {
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
    ) -> Result<Value, String> {
        let doc = self.document(&document)?;
        let view = PageView::PhysicalTables;
        Self::scoped_anchor(doc, &document, view, anchor)?;
        for edit in &mut edits {
            edit.anchor = Self::scoped_anchor(doc, &document, view, &edit.anchor)?.to_owned();
        }
        self.preview_scoped(document, view, anchor, None, |snapshot, local, _| {
            snapshot
                .edit_part_tables(local, edits)
                .map(|bytes| (bytes, Value::Null))
        })
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

    fn preview_scoped(
        &mut self,
        document: String,
        view: PageView,
        anchor: &str,
        paired: Option<&str>,
        run: impl FnOnce(&Snapshot, &str, Option<&str>) -> Result<(Vec<u8>, Value), String>,
    ) -> Result<Value, String> {
        let doc = self.document(&document)?;
        let local = Self::scoped_anchor(doc, &document, view, anchor)?;
        let paired = paired
            .map(|anchor| Self::scoped_anchor(doc, &document, view, anchor))
            .transpose()?;
        self.check_preview_limit(0)?;
        let (bytes, changes) = run(&doc.snapshot, local, paired).map_err(|e| {
            if e.starts_with("wrong_anchor_type: ") {
                e
            } else {
                format!("rejected_batch: {e}; revise the operation and retry")
            }
        })?;
        self.retain_preview(document, bytes, changes)
    }

    pub fn preview_edits(
        &mut self,
        document: String,
        mut edits: Vec<Edit>,
    ) -> Result<Value, String> {
        let doc = self.document(&document)?;
        if edits.is_empty() {
            return Err("invalid_batch: supply at least one edit".into());
        }
        for (i, edit) in edits.iter_mut().enumerate() {
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
                .ok_or("invalid_anchor: use an inspected anchor")?;
            if owner != document {
                return Err(format!(
                    "wrong_owner: inspect the selected document for edit {i}"
                ));
            }
            let matches_view = view.is_some_and(|view| doc.view_anchors.get(local) == Some(&view));
            if !doc.snapshot.has_issued_anchor(local) && !matches_view {
                if doc.view_anchors.contains_key(local) {
                    return Err(format!(
                        "wrong_anchor_type: edit {i} requires an ordinary edit anchor"
                    ));
                }
                return Err(format!(
                    "invalid_anchor: inspect the document again for edit {i}"
                ));
            }
            anchor.drain(..owner.len() + 1);
        }
        self.check_preview_limit(0)?;
        let (bytes, changes) = doc.snapshot.preview(edits).map_err(|e| {
            if e.starts_with("wrong_anchor_type:") {
                e
            } else {
                format!("rejected_batch: {e}; revise the edits and retry")
            }
        })?;
        self.retain_preview(document, bytes, changes)
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
            ("no-owner".into(), "invalid_anchor:"),
            (guessed.clone(), "invalid_anchor:"),
            (foreign, "wrong_owner:"),
        ] {
            let error = session
                .preview_edits(document.clone(), vec![insert(anchor)])
                .unwrap_err();
            assert!(error.starts_with(category), "{error}");
        }

        let inspection = session.inspect_document(document.clone()).unwrap();
        let inspected = inspection["body"][0]["anchor"].as_str().unwrap();
        assert_eq!(inspected, guessed);
        for (anchor, category) in [
            (format!("{document}/paragraph:999"), "invalid_anchor:"),
            (inspected.to_owned(), "wrong_anchor_type:"),
        ] {
            let error = session
                .preview_edits(document.clone(), vec![Edit::RunRemove { anchor }])
                .unwrap_err();
            assert!(error.starts_with(category), "{error}");
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
            assert!(error.starts_with("rejected_batch:"), "{error}");
            assert!(error.contains(detail), "{error}");
        }
        assert_eq!(session.document(document).unwrap().previews.len(), 1);
        let other = session.create_document(vec!["other".into()]).unwrap();
        let foreign = session
            .inspect_document(other["document"].as_str().unwrap().to_owned())
            .unwrap();
        for (anchor, category) in [
            (&paragraph["runs"][0]["anchor"], "wrong_anchor_type:"),
            (&foreign["body"][0]["anchor"], "wrong_owner:"),
        ] {
            let error = preview(
                &mut session,
                json!([{"kind":"insert-text","anchor":anchor,"offset":0,"text":"x"}]),
            )
            .unwrap_err();
            assert!(error.starts_with(category), "{error}");
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
