//! Checkout-only Python consumer of the same kernel.
use kernel::{
    Affinity, CaretMove, LayoutSession, LayoutSnapshot, OpenedPages, PageSpec, PhysicalSnapshot,
    TextPoint, VisualPosition, plain_section as project,
};
use pyo3::{
    exceptions::{PyIndexError, PyRuntimeError, PyValueError},
    prelude::*,
    types::PyBytes,
};
use std::sync::{Arc, Mutex, MutexGuard};

#[pyclass(name = "PageSpec", module = "rocx_layout", frozen, get_all)]
struct Spec {
    width: i32,
    height: i32,
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
    space_before: i32,
    space_after: i32,
}

#[pymethods]
impl Spec {
    #[new]
    #[pyo3(signature = (*, width, height, left=0, right=0, top=0, bottom=0, space_before=0, space_after=0))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        width: i32,
        height: i32,
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
        space_before: i32,
        space_after: i32,
    ) -> Self {
        Self {
            width,
            height,
            left,
            right,
            top,
            bottom,
            space_before,
            space_after,
        }
    }
}

impl From<&Spec> for PageSpec {
    fn from(s: &Spec) -> Self {
        Self {
            width: s.width,
            height: s.height,
            left: s.left,
            right: s.right,
            top: s.top,
            bottom: s.bottom,
            space_before: s.space_before,
            space_after: s.space_after,
        }
    }
}

#[pyfunction]
fn plain_section(docx: &[u8]) -> PyResult<(Vec<String>, Spec)> {
    let doc = rocx::editor::ImportedDocument::open(docx)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let (texts, s) = project(&doc).map_err(PyValueError::new_err)?;
    Ok((texts, s.into()))
}

impl From<PageSpec> for Spec {
    fn from(s: PageSpec) -> Self {
        Spec::new(
            s.width,
            s.height,
            s.left,
            s.right,
            s.top,
            s.bottom,
            s.space_before,
            s.space_after,
        )
    }
}

#[pyclass(name = "LayoutSession", module = "rocx_layout")]
struct Session(LayoutSession);

#[pymethods]
impl Session {
    #[new]
    #[pyo3(signature=(font,font_size,*,cache_budget=16777216))]
    fn new(font: Vec<u8>, font_size: i32, cache_budget: usize) -> PyResult<Self> {
        LayoutSession::with_cache_budget(font, font_size, cache_budget)
            .map(Self)
            .map_err(PyValueError::new_err)
    }
    #[getter]
    fn cache_stats(&self) -> crate::CacheStats {
        self.0.cache_stats().into()
    }
    fn clear_caches(&mut self) {
        self.0.clear_caches();
    }

    fn layout(
        &mut self,
        paragraphs: Vec<String>,
        width: i32,
        lines_per_page: usize,
    ) -> PyResult<Snapshot> {
        self.0
            .layout(&paragraphs, width, lines_per_page)
            .map(Snapshot)
            .map_err(PyValueError::new_err)
    }

    fn layout_pages(&mut self, paragraphs: Vec<String>, spec: &Spec) -> PyResult<Physical> {
        self.0
            .layout_pages(&paragraphs, spec.into())
            .map(Arc::new)
            .map(Physical)
            .map_err(PyValueError::new_err)
    }
    fn open_pages(&mut self, docx: &[u8]) -> PyResult<Opened> {
        self.0
            .open_pages(docx)
            .map(Mutex::new)
            .map(Opened)
            .map_err(PyValueError::new_err)
    }
}

#[pyclass(name = "PhysicalSnapshot", module = "rocx_layout", frozen)]
struct Physical(Arc<PhysicalSnapshot>);

#[pyclass(name = "VisualPosition", module = "rocx_layout", frozen)]
struct Position(VisualPosition);

#[pymethods]
impl Position {
    #[getter]
    fn paragraph(&self) -> usize {
        self.0.point.paragraph
    }
    #[getter]
    fn offset(&self) -> u32 {
        self.0.point.offset
    }
    #[getter]
    fn downstream(&self) -> bool {
        self.0.affinity == Affinity::Downstream
    }
    #[getter]
    fn preferred_x(&self) -> Option<i32> {
        self.0.preferred_x
    }
}

#[pyclass(name = "OpenedPages", module = "rocx_layout", frozen)]
struct Opened(Mutex<OpenedPages>);

impl Opened {
    fn inner(&self) -> PyResult<MutexGuard<'_, OpenedPages>> {
        self.0
            .lock()
            .map_err(|_| PyRuntimeError::new_err("document lock poisoned"))
    }
}

#[pymethods]
impl Opened {
    #[getter]
    fn geometry(&self) -> PyResult<Physical> {
        Ok(Physical(Arc::clone(self.inner()?.geometry())))
    }
    #[getter]
    fn spec(&self) -> PyResult<Spec> {
        Ok(self.inner()?.spec().into())
    }
    fn preview_insert<'py>(
        &self,
        py: Python<'py>,
        page: usize,
        x: i32,
        y: i32,
        text: &str,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self
            .inner()?
            .preview_insert(page, x, y, text)
            .map_err(PyValueError::new_err)?;
        Ok(PyBytes::new(py, &bytes))
    }
    fn preview_replace<'py>(
        &self,
        py: Python<'py>,
        anchor: (usize, u32),
        focus: (usize, u32),
        text: &str,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let point = |(paragraph, offset)| TextPoint { paragraph, offset };
        let bytes = self
            .inner()?
            .preview_replace(point(anchor), point(focus), text)
            .map_err(PyValueError::new_err)?;
        Ok(PyBytes::new(py, &bytes))
    }
}

#[pymethods]
impl Physical {
    #[getter]
    fn page_count(&self) -> usize {
        self.0.rows.len()
    }

    fn page_json(&self, index: usize) -> PyResult<String> {
        let rows = self
            .0
            .rows
            .get(index)
            .ok_or_else(|| PyIndexError::new_err("unknown page"))?;
        let rows: Vec<_> = rows
            .iter()
            .map(|r| (r, &self.0.layout.paragraphs[r.paragraph][r.line]))
            .collect();
        serde_json::to_string(&rows).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn hit_test(&self, page: usize, x: i32, y: i32) -> Option<(usize, u32)> {
        self.0.hit_test(page, x, y)
    }

    fn hit_position(&self, page: usize, x: i32, y: i32) -> Option<Position> {
        self.0.hit_position(page, x, y).map(Position)
    }
    #[pyo3(signature=(paragraph,offset,*,downstream=false,preferred_x=None))]
    fn visual_position(
        &self,
        paragraph: usize,
        offset: u32,
        downstream: bool,
        preferred_x: Option<i32>,
    ) -> Option<Position> {
        self.0
            .visual_position(
                TextPoint { paragraph, offset },
                if downstream {
                    Affinity::Downstream
                } else {
                    Affinity::Upstream
                },
                preferred_x,
            )
            .map(Position)
    }
    fn move_caret(&self, position: &Position, direction: &str) -> PyResult<Option<Position>> {
        use CaretMove::*;
        let motion = match direction {
            "left" => Left,
            "right" => Right,
            "up" => Up,
            "down" => Down,
            "line_start" => LineStart,
            "line_end" => LineEnd,
            _ => {
                return Err(PyValueError::new_err(
                    "expected left, right, up, down, line_start or line_end",
                ));
            }
        };
        Ok(self.0.move_caret(&position.0, motion).map(Position))
    }

    #[pyo3(signature = (paragraph, offset, *, downstream=false))]
    fn caret_box(
        &self,
        paragraph: usize,
        offset: u32,
        downstream: bool,
    ) -> Option<crate::CaretBox> {
        self.0
            .caret_box(
                TextPoint { paragraph, offset },
                if downstream {
                    Affinity::Downstream
                } else {
                    Affinity::Upstream
                },
            )
            .map(Into::into)
    }

    fn selection_rects(
        &self,
        anchor: (usize, u32),
        focus: (usize, u32),
    ) -> Option<Vec<crate::SelectionRect>> {
        let point = |(paragraph, offset)| TextPoint { paragraph, offset };
        self.0
            .selection_rects(point(anchor), point(focus))
            .map(|rows| rows.into_iter().map(Into::into).collect())
    }

    #[getter]
    fn reuse(&self) -> (usize, usize, usize) {
        let r = self.0.layout.reuse;
        (r.shaped_hits, r.line_hits, r.page_hits)
    }
}

#[pyclass(name = "LayoutSnapshot", module = "rocx_layout", frozen)]
struct Snapshot(LayoutSnapshot);

#[pymethods]
impl Snapshot {
    #[getter]
    fn page_count(&self) -> usize {
        self.0.pages.len()
    }

    /// Page placements paired with the geometry of the referenced line.
    fn page_json(&self, index: usize) -> PyResult<String> {
        let page = self
            .0
            .pages
            .get(index)
            .ok_or_else(|| PyIndexError::new_err("unknown page"))?;
        let rows: Vec<_> = page
            .lines
            .iter()
            .map(|p| (p, &self.0.paragraphs[p.paragraph][p.line]))
            .collect();
        serde_json::to_string(&rows).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn hit_row(&self, page: usize, row: usize, x: i32) -> Option<(usize, u32)> {
        self.0.hit_row(page, row, x)
    }

    #[getter]
    fn reuse(&self) -> (usize, usize, usize) {
        let r = self.0.reuse;
        (r.shaped_hits, r.line_hits, r.page_hits)
    }
}

// Python value records mirror the small immutable geometry/telemetry records;
// the native kernel has no extension-module dependency or build flags.
macro_rules! record {
    ($name:ident {$($field:ident:$ty:ty),+}) => {
        #[pyclass(module="rocx_layout",frozen,get_all)]
        struct $name { $( $field:$ty, )+ }
        impl From<kernel::$name> for $name {
            fn from(value:kernel::$name)->Self { Self { $($field:value.$field,)+ } }
        }
    };
}
record!(CaretBox {
    page: usize,
    row: usize,
    x: i32,
    top: i32,
    bottom: i32
});
record!(SelectionRect {
    page: usize,
    row: usize,
    left: i32,
    right: i32,
    top: i32,
    bottom: i32
});
record!(CacheStats {
    paragraphs: usize,
    wrap_variants: usize,
    estimated_bytes: usize,
    budget_bytes: usize
});

#[pymodule]
fn rocx_layout(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Session>()?;
    m.add_class::<Snapshot>()?;
    m.add_class::<Spec>()?;
    m.add_class::<Physical>()?;
    m.add_class::<Opened>()?;
    m.add_class::<Position>()?;
    m.add_class::<crate::CaretBox>()?;
    m.add_class::<crate::SelectionRect>()?;
    m.add_class::<crate::CacheStats>()?;
    m.add_function(wrap_pyfunction!(plain_section, m)?)?;
    Ok(())
}
