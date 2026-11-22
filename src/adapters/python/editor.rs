//! Python objects over the imported-document editing core.
use std::sync::Mutex;

use pyo3::{
    exceptions::{PyRuntimeError, PyTypeError, PyValueError},
    prelude::*,
    types::{PyBytes, PyModule},
};

use super::rust_error;
use crate::editor::{self, BodyBlock};

fn alignment_name(value: crate::Alignment) -> &'static str {
    match value {
        crate::Alignment::Left => "left",
        crate::Alignment::Center => "center",
        crate::Alignment::Right => "right",
        crate::Alignment::Justify => "justify",
        crate::Alignment::Distribute => "distribute",
        crate::Alignment::JustifyMedium => "justifyMedium",
        crate::Alignment::JustifyHigh => "justifyHigh",
        crate::Alignment::JustifyLow => "justifyLow",
        crate::Alignment::ThaiJustify => "thaiJustify",
    }
}

#[pyclass(name = "ParagraphAnchor", frozen, module = "rocx.editor")]
struct PyParagraphAnchor {
    inner: editor::ParagraphAnchor,
}

#[pyclass(name = "BodyParagraph", frozen, module = "rocx.editor")]
struct PyBodyParagraph {
    #[pyo3(get)]
    anchor: Py<PyParagraphAnchor>,
    #[pyo3(get)]
    text: String,
    #[pyo3(get)]
    alignment: Option<String>,
    #[pyo3(get)]
    alignment_xml: Option<String>,
    #[pyo3(get)]
    has_opaque_content: bool,
}

#[pyclass(name = "BodyTable", frozen, module = "rocx.editor")]
struct PyBodyTable {}

#[pyclass(name = "BodyOpaque", frozen, module = "rocx.editor")]
struct PyBodyOpaque {
    #[pyo3(get)]
    xml: String,
}

#[pyclass(name = "ParagraphAlignmentEdit", frozen, module = "rocx.editor")]
struct PyParagraphAlignmentEdit {
    target: editor::ParagraphAnchor,
    alignment: crate::Alignment,
}

#[pymethods]
impl PyParagraphAlignmentEdit {
    #[new]
    fn new(target: PyRef<'_, PyParagraphAnchor>, alignment: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            target: target.inner.clone(),
            alignment: super::alignment(alignment)?,
        })
    }
}

#[pyclass(name = "TableInsertion", frozen, module = "rocx.editor")]
struct PyTableInsertion {
    target: editor::ParagraphAnchor,
    table: crate::ResolvedTable,
}

#[pymethods]
impl PyTableInsertion {
    #[new]
    fn new(
        target: PyRef<'_, PyParagraphAnchor>,
        resolved_table: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let node_type = resolved_table
            .py()
            .import("rocx.resolution.snapshot")?
            .getattr("ResolvedNode")?;
        if !resolved_table.is_instance(&node_type)?
            || resolved_table
                .getattr("_native_kind")?
                .extract::<String>()?
                != "Table"
        {
            return Err(PyTypeError::new_err("expected a resolved Table"));
        }
        let table = super::tables::table(resolved_table)?
            .ok_or_else(|| PyValueError::new_err("table must not be empty"))?;
        if table.rows().iter().all(|row| row.cells().is_empty()) {
            return Err(PyValueError::new_err("table must not be empty"));
        }
        Ok(Self {
            target: target.inner.clone(),
            table,
        })
    }
}

#[pyclass(name = "AlignmentChange", frozen, module = "rocx.editor")]
struct PyAlignmentChange {
    #[pyo3(get)]
    body_index: usize,
    #[pyo3(get)]
    before: Option<String>,
    #[pyo3(get)]
    after: String,
}

#[pyclass(name = "TableInsertionChange", frozen, module = "rocx.editor")]
struct PyTableInsertionChange {
    #[pyo3(get)]
    after_body_index: usize,
    #[pyo3(get)]
    rows: usize,
    #[pyo3(get)]
    columns: usize,
}

#[pyclass(name = "EditPreview", module = "rocx.editor")]
struct PyEditPreview {
    inner: Mutex<editor::EditPreview>,
}

#[pymethods]
impl PyEditPreview {
    #[getter]
    fn insertions(&self) -> PyResult<Vec<PyTableInsertionChange>> {
        let preview = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Preview lock poisoned"))?;
        Ok(preview
            .insertions()
            .iter()
            .map(|change| PyTableInsertionChange {
                after_body_index: change.after_body_index,
                rows: change.rows,
                columns: change.columns,
            })
            .collect())
    }

    #[getter]
    fn changes(&self) -> PyResult<Vec<PyAlignmentChange>> {
        let preview = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Preview lock poisoned"))?;
        Ok(preview
            .changes()
            .iter()
            .map(|change| PyAlignmentChange {
                body_index: change.body_index,
                before: change.before.clone(),
                after: alignment_name(change.after).to_owned(),
            })
            .collect())
    }

    fn to_docx<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let preview = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Preview lock poisoned"))?;
        let bytes = preview.to_docx().map_err(rust_error)?;
        Ok(PyBytes::new(py, &bytes))
    }
}

#[pyclass(name = "ImportedDocument", module = "rocx.editor")]
struct PyImportedDocument {
    inner: Mutex<editor::ImportedDocument>,
}

#[pymethods]
impl PyImportedDocument {
    #[staticmethod]
    fn open(data: &Bound<'_, PyBytes>) -> PyResult<Self> {
        Ok(Self {
            inner: Mutex::new(editor::ImportedDocument::open(data.as_bytes()).map_err(rust_error)?),
        })
    }

    #[getter]
    fn body(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        doc.body()
            .iter()
            .map(|block| match block {
                BodyBlock::Paragraph(p) => Ok(Py::new(
                    py,
                    PyBodyParagraph {
                        anchor: Py::new(
                            py,
                            PyParagraphAnchor {
                                inner: p.anchor.clone(),
                            },
                        )?,
                        text: p.text.clone(),
                        alignment: p.alignment.map(|v| alignment_name(v).to_owned()),
                        alignment_xml: p.alignment_xml.clone(),
                        has_opaque_content: p.has_opaque_content,
                    },
                )?
                .into_any()),
                BodyBlock::Table => Ok(Py::new(py, PyBodyTable {})?.into_any()),
                BodyBlock::Opaque(xml) => {
                    Ok(Py::new(py, PyBodyOpaque { xml: xml.clone() })?.into_any())
                }
            })
            .collect()
    }

    #[pyo3(signature = (edits, *, table_insertions=None))]
    fn preview(
        &self,
        edits: Vec<PyRef<'_, PyParagraphAlignmentEdit>>,
        table_insertions: Option<Vec<PyRef<'_, PyTableInsertion>>>,
    ) -> PyResult<PyEditPreview> {
        let edits: Vec<_> = edits
            .iter()
            .map(|edit| editor::ParagraphAlignmentEdit {
                target: edit.target.clone(),
                alignment: edit.alignment,
            })
            .collect();
        let insertions: Vec<_> = table_insertions
            .unwrap_or_default()
            .iter()
            .map(|insertion| editor::TableInsertion {
                target: insertion.target.clone(),
                table: insertion.table.clone(),
            })
            .collect();
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(PyEditPreview {
            inner: Mutex::new(doc.preview_batch(&edits, &insertions).map_err(rust_error)?),
        })
    }
}

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyParagraphAnchor>()?;
    module.add_class::<PyBodyParagraph>()?;
    module.add_class::<PyBodyTable>()?;
    module.add_class::<PyBodyOpaque>()?;
    module.add_class::<PyParagraphAlignmentEdit>()?;
    module.add_class::<PyTableInsertion>()?;
    module.add_class::<PyAlignmentChange>()?;
    module.add_class::<PyTableInsertionChange>()?;
    module.add_class::<PyEditPreview>()?;
    module.add_class::<PyImportedDocument>()?;
    Ok(())
}
