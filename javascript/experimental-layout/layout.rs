use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use js_sys::{Array, Int32Array, JsString, Reflect, Uint8Array, Uint32Array};
use rocx::editor::ImportedDocument;
use rocx_layout::{
    Affinity, CaretMove, LayoutSession, LayoutSnapshot, OpenedPages, PageSpec, PhysicalSnapshot,
    Row, TextPoint, VisualPosition,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

const LINE_STRIDE: usize = 9;
const GLYPH_STRIDE: usize = 5;
const CARET_STRIDE: usize = 2;
const ROW_STRIDE: usize = 6;
static LAST_PHYSICAL_SNAPSHOT_ID: AtomicU32 = AtomicU32::new(0);

fn next_physical_snapshot_id() -> Result<u32, JsError> {
    let mut previous = LAST_PHYSICAL_SNAPSHOT_ID.load(Ordering::Relaxed);
    loop {
        let next = previous.checked_add(1).ok_or_else(|| {
            JsError::new("physical snapshot IDs exhausted; reload the Wasm module")
        })?;
        match LAST_PHYSICAL_SNAPSHOT_ID.compare_exchange_weak(
            previous,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return Ok(next),
            Err(observed) => previous = observed,
        }
    }
}

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = include_str!("page.d.ts");

/// Extract the supported plain, single-section input from DOCX bytes.
#[wasm_bindgen(js_name = plainSection, unchecked_return_type = "ExperimentalLayoutSection")]
pub fn plain_section(bytes: &[u8]) -> Result<JsValue, JsError> {
    let doc = ImportedDocument::open(bytes).map_err(|e| JsError::new(&e.to_string()))?;
    let (paragraphs, spec) = rocx_layout::plain_section(&doc).map_err(js_error)?;
    to_js(&Section {
        paragraphs,
        spec: spec.into(),
    })
}

/// Layout session for one supplied font face and size (in 1/64 pt).
#[wasm_bindgen]
pub struct ExperimentalLayoutSession {
    session: LayoutSession,
    revision: u32,
}

#[wasm_bindgen]
impl ExperimentalLayoutSession {
    #[wasm_bindgen(constructor)]
    pub fn new(
        font: Vec<u8>,
        #[wasm_bindgen(unchecked_param_type = "number")] font_size: JsValue,
    ) -> Result<ExperimentalLayoutSession, JsError> {
        let size = integer(&font_size, "fontSize", 1, i32::MAX.into())? as i32;
        let session = LayoutSession::new(font, size).map_err(js_error)?;
        Ok(Self {
            session,
            revision: 0,
        })
    }

    #[wasm_bindgen(js_name = withCacheBudget)]
    pub fn with_cache_budget(
        font: Vec<u8>,
        #[wasm_bindgen(unchecked_param_type = "number")] font_size: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] budget: JsValue,
    ) -> Result<ExperimentalLayoutSession, JsError> {
        let size = integer(&font_size, "fontSize", 1, i32::MAX.into())? as i32;
        let budget = integer(&budget, "budget", 0, u32::MAX.into())? as usize;
        Ok(Self {
            session: LayoutSession::with_cache_budget(font, size, budget).map_err(js_error)?,
            revision: 0,
        })
    }

    #[wasm_bindgen(getter,js_name=cacheStats,unchecked_return_type="ExperimentalLayoutCacheStats")]
    pub fn cache_stats(&self) -> Result<JsValue, JsError> {
        let s = self.session.cache_stats();
        to_js(&CacheStats {
            paragraphs: s.paragraphs,
            wrap_variants: s.wrap_variants,
            estimated_bytes: s.estimated_bytes,
            budget_bytes: s.budget_bytes,
        })
    }

    #[wasm_bindgen(js_name=clearCaches)]
    pub fn clear_caches(&mut self) {
        self.session.clear_caches();
    }

    /// Lay out plain paragraphs, retaining the resulting geometry in Wasm.
    #[wasm_bindgen]
    pub fn layout(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "readonly string[]")] paragraphs: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] width: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] lines_per_page: JsValue,
    ) -> Result<ExperimentalLayoutSnapshot, JsError> {
        let width = integer(&width, "width", 1, i32::MAX.into())? as i32;
        let lines = integer(&lines_per_page, "linesPerPage", 1, u32::MAX.into())? as usize;
        let paragraphs = read_paragraphs(paragraphs)?;
        let revision = self.next_revision()?;
        let snapshot = self
            .session
            .layout(&paragraphs, width, lines)
            .map_err(js_error)?;
        self.revision = revision;
        Ok(ExperimentalLayoutSnapshot { snapshot, revision })
    }

    /// Lay out plain paragraphs using physical page dimensions and font metrics.
    #[wasm_bindgen(js_name = layoutPages)]
    pub fn layout_pages(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "readonly string[]")] paragraphs: JsValue,
        #[wasm_bindgen(unchecked_param_type = "ExperimentalLayoutPageSpec")] spec: JsValue,
    ) -> Result<ExperimentalPhysicalLayoutSnapshot, JsError> {
        let spec = read_page_spec(&spec)?;
        let paragraphs = read_paragraphs(paragraphs)?;
        let revision = self.next_revision()?;
        let snapshot = self
            .session
            .layout_pages(&paragraphs, spec)
            .map_err(js_error)?;
        let snapshot_id = next_physical_snapshot_id()?;
        self.revision = revision;
        Ok(ExperimentalPhysicalLayoutSnapshot {
            snapshot: Arc::new(snapshot),
            revision,
            snapshot_id,
        })
    }

    /// Open a DOCX and bind its immutable geometry and edit previews to one revision.
    #[wasm_bindgen(js_name = openPages)]
    pub fn open_pages(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Uint8Array")] bytes: JsValue,
    ) -> Result<ExperimentalOpenedPages, JsError> {
        let bytes = bytes
            .dyn_into::<Uint8Array>()
            .map_err(|_| JsError::new("bytes must be a Uint8Array"))?;
        let revision = self.next_revision()?;
        let opened = self.session.open_pages(&bytes.to_vec()).map_err(js_error)?;
        let snapshot_id = next_physical_snapshot_id()?;
        self.revision = revision;
        Ok(ExperimentalOpenedPages {
            opened,
            revision,
            snapshot_id,
        })
    }
}

impl ExperimentalLayoutSession {
    fn next_revision(&self) -> Result<u32, JsError> {
        self.revision
            .checked_add(1)
            .ok_or_else(|| JsError::new("layout revision exhausted; create a new session"))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CacheStats {
    paragraphs: usize,
    wrap_variants: usize,
    estimated_bytes: usize,
    budget_bytes: usize,
}

/// Immutable result. Call `free()` from JavaScript when no longer needed.
#[wasm_bindgen]
pub struct ExperimentalLayoutSnapshot {
    snapshot: LayoutSnapshot,
    revision: u32,
}

#[wasm_bindgen]
impl ExperimentalLayoutSnapshot {
    #[wasm_bindgen(getter)]
    pub fn revision(&self) -> u32 {
        self.revision
    }

    #[wasm_bindgen(getter, js_name = pageCount)]
    pub fn page_count(&self) -> usize {
        self.snapshot.pages.len()
    }

    #[wasm_bindgen(getter, unchecked_return_type = "ExperimentalLayoutReuse")]
    pub fn reuse(&self) -> Result<JsValue, JsError> {
        page_reuse(&self.snapshot)
    }

    /// Copy geometry for one zero-based page. Missing pages return undefined.
    #[wasm_bindgen(unchecked_return_type = "ExperimentalLayoutPage | undefined")]
    pub fn page(
        &self,
        #[wasm_bindgen(unchecked_param_type = "number")] index: JsValue,
    ) -> Result<JsValue, JsError> {
        let index = integer(&index, "page", 0, u32::MAX.into())? as u32;
        page_geometry(&self.snapshot, self.revision, index, None)
    }

    /// Find a scalar caret on a visual row; no hit returns undefined.
    #[wasm_bindgen(js_name = hitRow, unchecked_return_type = "ExperimentalLayoutHit | undefined")]
    pub fn hit_row(
        &self,
        #[wasm_bindgen(unchecked_param_type = "number")] page: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] row: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] x: JsValue,
    ) -> Result<JsValue, JsError> {
        let page = integer(&page, "page", 0, u32::MAX.into())? as usize;
        let row = integer(&row, "row", 0, u32::MAX.into())? as usize;
        let x = coordinate(&x, "x")?;
        hit(self.revision, self.snapshot.hit_row(page, row, x))
    }
}

/// Immutable physical result. Call `free()` from JavaScript when no longer needed.
#[wasm_bindgen]
pub struct ExperimentalPhysicalLayoutSnapshot {
    snapshot: Arc<PhysicalSnapshot>,
    revision: u32,
    snapshot_id: u32,
}

#[wasm_bindgen]
impl ExperimentalPhysicalLayoutSnapshot {
    #[wasm_bindgen(getter)]
    pub fn revision(&self) -> u32 {
        self.revision
    }

    #[wasm_bindgen(getter, js_name = pageCount)]
    pub fn page_count(&self) -> usize {
        self.snapshot.layout.pages.len()
    }

    #[wasm_bindgen(getter, unchecked_return_type = "ExperimentalLayoutReuse")]
    pub fn reuse(&self) -> Result<JsValue, JsError> {
        page_reuse(&self.snapshot.layout)
    }

    /// Copy geometry for one zero-based page. Missing pages return undefined.
    #[wasm_bindgen(unchecked_return_type = "ExperimentalPhysicalLayoutPage | undefined")]
    pub fn page(
        &self,
        #[wasm_bindgen(unchecked_param_type = "number")] index: JsValue,
    ) -> Result<JsValue, JsError> {
        let index = integer(&index, "page", 0, u32::MAX.into())? as u32;
        let rows = self
            .snapshot
            .rows
            .get(index as usize)
            .map(|rows| rows.as_slice());
        page_geometry(&self.snapshot.layout, self.revision, index, rows)
    }

    /// Find the nearest scalar caret at page-relative coordinates.
    #[wasm_bindgen(js_name = hitTest, unchecked_return_type = "ExperimentalLayoutHit | undefined")]
    pub fn hit_test(
        &self,
        #[wasm_bindgen(unchecked_param_type = "number")] page: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] x: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] y: JsValue,
    ) -> Result<JsValue, JsError> {
        let page = integer(&page, "page", 0, u32::MAX.into())? as usize;
        let x = coordinate(&x, "x")?;
        let y = coordinate(&y, "y")?;
        hit(self.revision, self.snapshot.hit_test(page, x, y))
    }

    /// Hit a visual caret, retaining the side of a soft wrap.
    #[wasm_bindgen(js_name = hitPosition, unchecked_return_type = "ExperimentalLayoutVisualPosition | undefined")]
    pub fn hit_position(
        &self,
        #[wasm_bindgen(unchecked_param_type = "number")] page: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] x: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] y: JsValue,
    ) -> Result<JsValue, JsError> {
        let page = integer(&page, "page", 0, u32::MAX.into())? as usize;
        let x = coordinate(&x, "x")?;
        let y = coordinate(&y, "y")?;
        self.write_position(self.snapshot.hit_position(page, x, y))
    }

    /// Move a caret within this snapshot, retaining preferred x on vertical moves.
    #[wasm_bindgen(js_name = moveCaret, unchecked_return_type = "ExperimentalLayoutVisualPosition | undefined")]
    pub fn move_caret(
        &self,
        #[wasm_bindgen(unchecked_param_type = "ExperimentalLayoutVisualPosition")]
        position: JsValue,
        #[wasm_bindgen(unchecked_param_type = "ExperimentalLayoutCaretMove")] direction: JsValue,
    ) -> Result<JsValue, JsError> {
        check_record(&position, "position")?;
        let token = |name| integer(&get_field(&position, name)?, name, 1, u32::MAX.into());
        if token("revision")? as u32 != self.revision {
            return Err(JsError::new("stale caret position revision"));
        }
        if token("snapshotId")? as u32 != self.snapshot_id {
            return Err(JsError::new("caret position belongs to another snapshot"));
        }
        let direction = match direction.as_string().as_deref() {
            Some("left") => CaretMove::Left,
            Some("right") => CaretMove::Right,
            Some("up") => CaretMove::Up,
            Some("down") => CaretMove::Down,
            Some("lineStart") => CaretMove::LineStart,
            Some("lineEnd") => CaretMove::LineEnd,
            _ => return Err(JsError::new("invalid caret move direction")),
        };
        let point = read_point(&get_field(&position, "point")?, "point")?;
        let affinity = read_affinity(&get_field(&position, "affinity")?)?;
        let preferred_x = get_field(&position, "preferredX")?;
        let preferred_x = if preferred_x.is_undefined() {
            None
        } else {
            Some(coordinate(&preferred_x, "preferredX")?)
        };
        let moved = self
            .snapshot
            .visual_position(point, affinity, preferred_x)
            .and_then(|position| self.snapshot.move_caret(&position, direction));
        self.write_position(moved)
    }

    /// Look up a page-local caret box for a paragraph-relative scalar point.
    #[wasm_bindgen(js_name = caretBox, unchecked_return_type = "ExperimentalLayoutCaretBox | undefined")]
    pub fn caret_box(
        &self,
        #[wasm_bindgen(unchecked_param_type = "ExperimentalLayoutTextPoint")] point: JsValue,
        #[wasm_bindgen(unchecked_param_type = "ExperimentalLayoutAffinity")] affinity: JsValue,
    ) -> Result<JsValue, JsError> {
        let point = read_point(&point, "point")?;
        let affinity = read_affinity(&affinity)?;
        self.snapshot
            .caret_box(point, affinity)
            .map_or(Ok(JsValue::UNDEFINED), |value| to_js(&value))
    }

    /// Find page-local selection rectangles between two scalar points.
    #[wasm_bindgen(js_name = selectionRects, unchecked_return_type = "readonly ExperimentalLayoutSelectionRect[] | undefined")]
    pub fn selection_rects(
        &self,
        #[wasm_bindgen(unchecked_param_type = "ExperimentalLayoutTextPoint")] anchor: JsValue,
        #[wasm_bindgen(unchecked_param_type = "ExperimentalLayoutTextPoint")] focus: JsValue,
    ) -> Result<JsValue, JsError> {
        let anchor = read_point(&anchor, "anchor")?;
        let focus = read_point(&focus, "focus")?;
        self.snapshot
            .selection_rects(anchor, focus)
            .map_or(Ok(JsValue::UNDEFINED), |value| to_js(&value))
    }
}

impl ExperimentalPhysicalLayoutSnapshot {
    fn write_position(&self, position: Option<VisualPosition>) -> Result<JsValue, JsError> {
        let Some(position) = position else {
            return Ok(JsValue::UNDEFINED);
        };
        to_js(&VersionedPosition {
            revision: self.revision,
            snapshot_id: self.snapshot_id,
            point: position.point,
            affinity: match position.affinity {
                Affinity::Upstream => "upstream",
                Affinity::Downstream => "downstream",
            },
            preferred_x: position.preferred_x,
        })
    }
}

/// DOCX source with immutable layout. Call `free()` when no longer needed.
#[wasm_bindgen]
pub struct ExperimentalOpenedPages {
    opened: OpenedPages,
    revision: u32,
    snapshot_id: u32,
}

#[wasm_bindgen]
impl ExperimentalOpenedPages {
    #[wasm_bindgen(getter)]
    pub fn revision(&self) -> u32 {
        self.revision
    }

    #[wasm_bindgen(getter, unchecked_return_type = "ExperimentalLayoutPageSpec")]
    pub fn spec(&self) -> Result<JsValue, JsError> {
        to_js(&SectionSpec::from(self.opened.spec()))
    }

    /// Return a separately owned handle sharing this opening's immutable geometry.
    #[wasm_bindgen]
    pub fn geometry(&self) -> ExperimentalPhysicalLayoutSnapshot {
        ExperimentalPhysicalLayoutSnapshot {
            snapshot: Arc::clone(self.opened.geometry()),
            revision: self.revision,
            snapshot_id: self.snapshot_id,
        }
    }

    /// Insert text at the scalar hit in this opening. Returns a new DOCX.
    #[wasm_bindgen(js_name = previewInsert)]
    pub fn preview_insert(
        &self,
        #[wasm_bindgen(unchecked_param_type = "number")] page: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] x: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] y: JsValue,
        #[wasm_bindgen(unchecked_param_type = "string")] text: JsValue,
    ) -> Result<Vec<u8>, JsError> {
        let page = integer(&page, "page", 0, u32::MAX.into())? as usize;
        let x = coordinate(&x, "x")?;
        let y = coordinate(&y, "y")?;
        let text = edit_text(text)?;
        self.opened
            .preview_insert(page, x, y, &text)
            .map_err(js_error)
    }

    /// Replace a nonempty selection in one editable paragraph; empty text deletes.
    #[wasm_bindgen(js_name = previewReplace)]
    pub fn preview_replace(
        &self,
        #[wasm_bindgen(unchecked_param_type = "ExperimentalLayoutTextPoint")] anchor: JsValue,
        #[wasm_bindgen(unchecked_param_type = "ExperimentalLayoutTextPoint")] focus: JsValue,
        #[wasm_bindgen(unchecked_param_type = "string")] text: JsValue,
    ) -> Result<Vec<u8>, JsError> {
        let anchor = read_point(&anchor, "anchor")?;
        let focus = read_point(&focus, "focus")?;
        let text = edit_text(text)?;
        self.opened
            .preview_replace(anchor, focus, &text)
            .map_err(js_error)
    }
}

fn page_geometry(
    snapshot: &LayoutSnapshot,
    revision: u32,
    index: u32,
    rows: Option<&[Row]>,
) -> Result<JsValue, JsError> {
    let Some(page) = snapshot.pages.get(index as usize) else {
        return Ok(JsValue::UNDEFINED);
    };
    let mut lines = Vec::with_capacity(page.lines.len() * LINE_STRIDE);
    let mut glyphs = Vec::new();
    let mut carets = Vec::new();
    for placement in page.lines.iter() {
        let line = &snapshot.paragraphs[placement.paragraph][placement.line];
        lines.extend([
            placement.paragraph as u32,
            placement.line as u32,
            line.start,
            line.end,
            line.width as u32,
            (glyphs.len() / GLYPH_STRIDE) as u32,
            line.glyphs.len() as u32,
            (carets.len() / CARET_STRIDE) as u32,
            line.carets.len() as u32,
        ]);
        for glyph in &line.glyphs {
            glyphs.extend([
                glyph.id,
                glyph.x as u32,
                glyph.y as u32,
                glyph.advance as u32,
                glyph.cluster,
            ]);
        }
        for caret in &line.carets {
            carets.extend([caret.offset, caret.x as u32]);
        }
    }
    let rows = rows.map(|rows| {
        let mut words = Vec::with_capacity(rows.len() * ROW_STRIDE);
        for row in rows {
            words.extend([
                row.paragraph as i32,
                row.line as i32,
                row.x,
                row.top,
                row.baseline,
                row.bottom,
            ]);
        }
        RowBuffer(Int32Array::from(words.as_slice()))
    });
    to_js(&Page {
        revision,
        index,
        lines: Uint32Array::from(lines.as_slice()),
        glyphs: Uint32Array::from(glyphs.as_slice()),
        carets: Uint32Array::from(carets.as_slice()),
        rows,
    })
}

fn hit(revision: u32, hit: Option<(usize, u32)>) -> Result<JsValue, JsError> {
    match hit {
        Some((paragraph, offset)) => to_js(&Hit {
            revision,
            paragraph,
            offset,
        }),
        None => Ok(JsValue::UNDEFINED),
    }
}

fn page_reuse(snapshot: &LayoutSnapshot) -> Result<JsValue, JsError> {
    let reuse = snapshot.reuse;
    to_js(&Reuse {
        shaped_hits: reuse.shaped_hits,
        line_hits: reuse.line_hits,
        page_hits: reuse.page_hits,
    })
}

#[derive(Serialize)]
struct Page {
    revision: u32,
    index: u32,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    lines: Uint32Array,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    glyphs: Uint32Array,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    carets: Uint32Array,
    #[serde(skip_serializing_if = "Option::is_none")]
    rows: Option<RowBuffer>,
}

#[derive(Serialize)]
#[serde(transparent)]
struct RowBuffer(#[serde(with = "serde_wasm_bindgen::preserve")] Int32Array);

#[derive(Serialize)]
struct Hit {
    revision: u32,
    paragraph: usize,
    offset: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionedPosition {
    revision: u32,
    snapshot_id: u32,
    point: TextPoint,
    affinity: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    preferred_x: Option<i32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Reuse {
    shaped_hits: usize,
    line_hits: usize,
    page_hits: usize,
}

#[derive(Serialize)]
struct Section {
    paragraphs: Vec<String>,
    spec: SectionSpec,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SectionSpec {
    width: i32,
    height: i32,
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
    space_before: i32,
    space_after: i32,
}

impl From<PageSpec> for SectionSpec {
    fn from(spec: PageSpec) -> Self {
        Self {
            width: spec.width,
            height: spec.height,
            left: spec.left,
            right: spec.right,
            top: spec.top,
            bottom: spec.bottom,
            space_before: spec.space_before,
            space_after: spec.space_after,
        }
    }
}

fn edit_text(value: JsValue) -> Result<String, JsError> {
    if !value.is_string() {
        return Err(JsError::new("text must be a string"));
    }
    let text: JsString = value.unchecked_into();
    if !text.is_well_formed() {
        return Err(JsError::new("text: malformed UTF-16 (unpaired surrogate)"));
    }
    Ok(String::from(text))
}

fn read_paragraphs(value: JsValue) -> Result<Vec<String>, JsError> {
    if !Array::is_array(&value) {
        return Err(JsError::new("paragraphs must be an array of strings"));
    }
    let input: Array = value.unchecked_into();
    input
        .iter()
        .enumerate()
        .map(|(index, text)| {
            if !text.is_string() {
                return Err(JsError::new(&format!("paragraph {index} must be a string")));
            }
            let text: JsString = text.unchecked_into();
            if !text.is_well_formed() {
                return Err(JsError::new(&format!(
                    "paragraph {index}: malformed UTF-16 (unpaired surrogate)"
                )));
            }
            Ok(String::from(text))
        })
        .collect()
}

fn read_page_spec(spec: &JsValue) -> Result<PageSpec, JsError> {
    check_record(spec, "page spec")?;
    let field = |name| coordinate(&get_field(spec, name)?, name);
    Ok(PageSpec {
        width: field("width")?,
        height: field("height")?,
        left: field("left")?,
        right: field("right")?,
        top: field("top")?,
        bottom: field("bottom")?,
        space_before: field("spaceBefore")?,
        space_after: field("spaceAfter")?,
    })
}

fn read_point(point: &JsValue, name: &str) -> Result<TextPoint, JsError> {
    check_record(point, name)?;
    let field = |name| integer(&get_field(point, name)?, name, 0, u32::MAX.into());
    Ok(TextPoint {
        paragraph: field("paragraph")? as usize,
        offset: field("offset")? as u32,
    })
}

fn read_affinity(affinity: &JsValue) -> Result<Affinity, JsError> {
    match affinity.as_string().as_deref() {
        Some("upstream") => Ok(Affinity::Upstream),
        Some("downstream") => Ok(Affinity::Downstream),
        _ => Err(JsError::new("affinity must be 'upstream' or 'downstream'")),
    }
}

fn check_record(value: &JsValue, name: &str) -> Result<(), JsError> {
    if !value.is_object() || Array::is_array(value) {
        return Err(JsError::new(&format!("{name} must be an object")));
    }
    Ok(())
}

fn get_field(value: &JsValue, name: &str) -> Result<JsValue, JsError> {
    Reflect::get(value, &JsValue::from_str(name))
        .map_err(|_| JsError::new(&format!("failed to read {name}")))
}

fn coordinate(value: &JsValue, name: &str) -> Result<i32, JsError> {
    Ok(integer(value, name, i32::MIN.into(), i32::MAX.into())? as i32)
}

fn integer(value: &JsValue, name: &str, min: i64, max: i64) -> Result<i64, JsError> {
    value
        .as_f64()
        .filter(|n| (min as f64..=max as f64).contains(n) && n.fract() == 0.0)
        .map(|n| n as i64)
        .ok_or_else(|| JsError::new(&format!("{name} must be an integer in {min}..={max}")))
}

fn to_js(value: &impl Serialize) -> Result<JsValue, JsError> {
    serde_wasm_bindgen::to_value(value).map_err(|error| JsError::new(&error.to_string()))
}

fn js_error(message: String) -> JsError {
    JsError::new(&message)
}
