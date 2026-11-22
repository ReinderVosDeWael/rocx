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

#[pyclass(name = "HeaderAnchor", frozen, module = "rocx.editor")]
struct PyHeaderAnchor {
    inner: editor::HeaderAnchor,
}

#[pyclass(name = "ImportedHeader", frozen, module = "rocx.editor")]
struct PyImportedHeader {
    #[pyo3(get)]
    anchor: Py<PyHeaderAnchor>,
    #[pyo3(get)]
    section_index: usize,
    #[pyo3(get)]
    part_name: String,
    #[pyo3(get)]
    inherited: bool,
}

#[pyclass(name = "HeaderImageEdit", frozen, module = "rocx.editor")]
struct PyHeaderImageEdit {
    target: editor::HeaderAnchor,
    image: crate::ResolvedImage,
    shared: bool,
}

#[pymethods]
impl PyHeaderImageEdit {
    #[new]
    #[pyo3(signature = (anchor, image, *, mode))]
    fn new(
        anchor: PyRef<'_, PyHeaderAnchor>,
        image: &Bound<'_, PyAny>,
        mode: &str,
    ) -> PyResult<Self> {
        let shared = match mode {
            "shared" => true,
            "section_local" => false,
            _ => {
                return Err(PyValueError::new_err(
                    "mode must be 'shared' or 'section_local'",
                ));
            }
        };
        let node_type = image
            .py()
            .import("rocx.resolution.snapshot")?
            .getattr("ResolvedNode")?;
        if !image.is_instance(&node_type)?
            || image.getattr("_native_kind")?.extract::<String>()? != "ImageRun"
        {
            return Err(PyTypeError::new_err("expected a resolved ImageRun"));
        }
        let Some(crate::ResolvedInline::Image { image, .. }) = super::content::inline(image)?
        else {
            return Err(PyTypeError::new_err("expected a resolved ImageRun"));
        };
        Ok(Self {
            target: anchor.inner.clone(),
            image,
            shared,
        })
    }
}

#[pyclass(name = "HeaderImageChange", frozen, module = "rocx.editor")]
struct PyHeaderImageChange {
    #[pyo3(get)]
    source_part: String,
    #[pyo3(get)]
    output_part: String,
    #[pyo3(get)]
    affected_sections: Vec<usize>,
    #[pyo3(get)]
    restored_section: Option<usize>,
    #[pyo3(get)]
    mode: String,
}

#[pyclass(name = "HeaderImagePreview", module = "rocx.editor")]
struct PyHeaderImagePreview {
    inner: Mutex<editor::HeaderImagePreview>,
}

#[pymethods]
impl PyHeaderImagePreview {
    #[getter]
    fn change(&self) -> PyResult<PyHeaderImageChange> {
        let preview = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Preview lock poisoned"))?;
        let change = preview.change();
        Ok(PyHeaderImageChange {
            source_part: change.source_part.clone(),
            output_part: change.output_part.clone(),
            affected_sections: change.affected_sections.clone(),
            restored_section: change.restored_section,
            mode: match &change.mode {
                editor::HeaderEditMode::Shared => "shared",
                editor::HeaderEditMode::SectionLocal => "section_local",
            }
            .to_owned(),
        })
    }

    fn to_docx<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let preview = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Preview lock poisoned"))?;
        Ok(PyBytes::new(py, &preview.to_docx().map_err(rust_error)?))
    }
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
    fn default_headers(&self, py: Python<'_>) -> PyResult<Vec<PyImportedHeader>> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        doc.default_headers()
            .map_err(rust_error)?
            .into_iter()
            .map(|header| {
                Ok(PyImportedHeader {
                    anchor: Py::new(
                        py,
                        PyHeaderAnchor {
                            inner: header.anchor,
                        },
                    )?,
                    section_index: header.section_index,
                    part_name: header.part_name,
                    inherited: header.inherited,
                })
            })
            .collect()
    }

    fn preview_header_image(
        &self,
        edit: PyRef<'_, PyHeaderImageEdit>,
    ) -> PyResult<PyHeaderImagePreview> {
        let edit = editor::HeaderImageEdit {
            target: edit.target.clone(),
            image: edit.image.clone(),
            mode: if edit.shared {
                editor::HeaderEditMode::Shared
            } else {
                editor::HeaderEditMode::SectionLocal
            },
        };
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(PyHeaderImagePreview {
            inner: Mutex::new(doc.preview_header_image(&edit).map_err(rust_error)?),
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
    module.add_class::<PyHeaderAnchor>()?;
    module.add_class::<PyImportedHeader>()?;
    module.add_class::<PyHeaderImageEdit>()?;
    module.add_class::<PyHeaderImageChange>()?;
    module.add_class::<PyHeaderImagePreview>()?;
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
