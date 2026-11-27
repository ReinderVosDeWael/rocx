//! Keep click-to-edit operations bound to the same immutable DOCX that
//! supplied the displayed text. A successful preview produces a new document.
use rocx::editor::{BodyBlock, EditBatch, ImportedDocument, TextInsertion, TextRangeEdit};
use std::sync::Arc;

use crate::{Affinity, LayoutSession, PageSpec, PhysicalSnapshot, TextPoint};

/// Geometry and source for one opening of an imported package. Cloning the
/// geometry does not reopen the DOCX or transfer its edit anchors.
pub struct OpenedPages {
    source: ImportedDocument,
    spec: PageSpec,
    geometry: Arc<PhysicalSnapshot>,
}

impl LayoutSession {
    /// Open a supported DOCX with its direct paragraph alignment and spacing.
    pub fn open_pages(&mut self, bytes: &[u8]) -> Result<OpenedPages, String> {
        let source = ImportedDocument::open(bytes).map_err(|e| e.to_string())?;
        let (text, spec, styles) = crate::import::section_flow(&source)?;
        let geometry = Arc::new(self.layout_flow_pages(&text, spec, &styles)?);
        Ok(OpenedPages {
            source,
            spec,
            geometry,
        })
    }
}

impl OpenedPages {
    pub fn spec(&self) -> PageSpec {
        self.spec
    }
    pub fn geometry(&self) -> &Arc<PhysicalSnapshot> {
        &self.geometry
    }

    /// Produce DOCX bytes by inserting at a hit on this specific snapshot.
    /// The underlying editor checks whether this paragraph is editable.
    pub fn preview_insert(
        &self,
        page: usize,
        x: i32,
        y: i32,
        text: &str,
    ) -> Result<Vec<u8>, String> {
        let (index, offset) = self
            .geometry
            .hit_test(page, x, y)
            .ok_or("no text at page point")?;
        let Some(BodyBlock::Paragraph(p)) = self.source.body().get(index) else {
            return Err("source paragraph not found".into());
        };
        let operation = TextInsertion {
            target: p.anchor.clone(),
            position: offset as usize,
            text: text.to_owned(),
        };
        self.source
            .preview_edits(EditBatch {
                text_insertions: std::slice::from_ref(&operation),
                ..Default::default()
            })
            .and_then(|preview| preview.to_docx())
            .map_err(|e| e.to_string())
    }

    /// Replace or delete a selection within one editable paragraph. Endpoints
    /// count source scalars and must be valid visual caret stops in this
    /// snapshot. Multi-paragraph selections can be drawn but not replaced.
    pub fn preview_replace(
        &self,
        anchor: TextPoint,
        focus: TextPoint,
        text: &str,
    ) -> Result<Vec<u8>, String> {
        if anchor.paragraph != focus.paragraph || anchor == focus {
            return Err("expected a nonempty selection in one paragraph".into());
        }
        let (start, end) = if anchor < focus {
            (anchor, focus)
        } else {
            (focus, anchor)
        };
        for endpoint in [start, end] {
            self.geometry
                .caret_box(endpoint, Affinity::Upstream)
                .ok_or("selection endpoint is not a caret stop")?;
        }
        let Some(BodyBlock::Paragraph(p)) = self.source.body().get(start.paragraph) else {
            return Err("source paragraph not found".into());
        };
        let operation = TextRangeEdit {
            target: p.anchor.clone(),
            start: start.offset as usize,
            end: end.offset as usize,
            text: text.to_owned(),
        };
        self.source
            .preview_edits(EditBatch {
                ranges: std::slice::from_ref(&operation),
                ..Default::default()
            })
            .and_then(|preview| preview.to_docx())
            .map_err(|e| e.to_string())
    }
}
