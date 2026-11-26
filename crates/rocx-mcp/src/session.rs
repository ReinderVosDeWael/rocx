use crate::{editor::Snapshot, schema::Edit};
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
    sources: Vec<PathBuf>,
    previews: HashMap<String, Preview>,
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

fn scope_anchors(value: &mut Value, owner: &str) {
    match value {
        Value::Object(fields) => {
            for (name, field) in fields {
                if name == "anchor" {
                    if let Value::String(local) = field {
                        *local = format!("{owner}/{local}");
                    }
                } else {
                    scope_anchors(field, owner);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| scope_anchors(v, owner)),
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
    ) -> Result<Value, String> {
        let snapshot = Snapshot::open(&bytes).map_err(|e| format!("invalid_document_data: {e}"))?;
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
                sources,
                previews,
            },
        );
        Ok(response)
    }

    pub fn open_document(&mut self, path: String) -> Result<Value, String> {
        self.check_document_limit()?;
        let slot = path_slot(Path::new(&path))?;
        let absolute = std::path::absolute(&path).map_err(|e| format!("io: invalid path: {e}"))?;
        let resolved =
            fs::canonicalize(&path).map_err(|e| format!("io: cannot open source: {e}"))?;
        let bytes = fs::read(&resolved).map_err(|e| format!("io: cannot read source: {e}"))?;
        self.register(bytes, vec![absolute, slot, resolved], false)
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
        scope_anchors(&mut result, &document);
        result["document"] = json!(document);
        Ok(result)
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
            let anchor = edit.anchor_slot();
            let (owner, local) = anchor
                .split_once('/')
                .ok_or("invalid_anchor: use an inspected anchor")?;
            if owner != document {
                return Err(format!(
                    "wrong_owner: inspect the selected document for edit {i}"
                ));
            }
            if !doc.snapshot.has_issued_anchor(local) {
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
            .open_document(alias.to_str().unwrap().into())
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
