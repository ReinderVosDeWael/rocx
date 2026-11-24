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
    #[pyo3(get)]
    blocks: Vec<Py<PyAny>>,
}

/// A direct header/footer paragraph with flat and structured inline views.
#[pyclass(name = "ImportedHeaderParagraph", frozen, module = "rocx.editor")]
struct PyImportedHeaderParagraph {
    #[pyo3(get)]
    inline_items: Vec<Py<PyImportedInline>>,
    #[pyo3(get)]
    inline_structure: Vec<Py<PyAny>>,
}

/// A section's effective reference to an existing header or footer part.
#[pyclass(name = "ImportedStory", frozen, module = "rocx.editor")]
struct PyImportedStory {
    #[pyo3(get)]
    anchor: Option<Py<PyHeaderAnchor>>,
    #[pyo3(get)]
    section_index: usize,
    #[pyo3(get)]
    kind: &'static str,
    #[pyo3(get)]
    variant: &'static str,
    #[pyo3(get)]
    part_name: String,
    #[pyo3(get)]
    inherited: bool,
    #[pyo3(get)]
    blocks: Vec<Py<PyAny>>,
}

#[pyclass(name = "ImportedEntry", frozen, module = "rocx.editor", get_all)]
struct PyImportedEntry {
    kind: &'static str,
    part_name: String,
    id: String,
    note_type: Option<String>,
    author: Option<String>,
    date: Option<String>,
    initials: Option<String>,
    blocks: Vec<Py<PyAny>>,
}

fn story_blocks<'a>(
    py: Python<'_>,
    blocks: &[editor::ImportedStoryBlock],
    structure: impl Fn(usize) -> Option<&'a [editor::ImportedInlineNode]>,
) -> PyResult<Vec<Py<PyAny>>> {
    blocks
        .iter()
        .enumerate()
        .map(|(index, block)| match block {
            editor::ImportedStoryBlock::Opaque(xml) => {
                Ok(Py::new(py, PyBodyOpaque { xml: xml.clone() })?.into_any())
            }
            editor::ImportedStoryBlock::Paragraph(items) => Ok(Py::new(
                py,
                PyImportedHeaderParagraph {
                    inline_items: items
                        .iter()
                        .map(|item| Py::new(py, PyImportedInline::from(item)))
                        .collect::<PyResult<_>>()?,
                    inline_structure: structure(index)
                        .expect("story paragraph structure")
                        .iter()
                        .map(|node| inline_object(py, node))
                        .collect::<PyResult<_>>()?,
                },
            )?
            .into_any()),
        })
        .collect()
}

#[pyclass(name = "RunTextEdit", frozen, module = "rocx.editor")]
struct PyRunTextEdit {
    inner: editor::RunTextEdit,
}

#[pymethods]
impl PyRunTextEdit {
    #[new]
    fn new(target: PyRef<'_, PyInlineAnchor>, text: String) -> Self {
        Self {
            inner: editor::RunTextEdit {
                target: target.inner.clone(),
                text,
            },
        }
    }
}

#[pyclass(name = "RunTextChange", frozen, module = "rocx.editor", get_all)]
struct PyRunTextChange {
    body_index: usize,
    inline_index: usize,
    before: String,
    after: String,
}

#[pyclass(name = "RunRemovalChange", frozen, module = "rocx.editor", get_all)]
struct PyRunRemovalChange {
    body_index: usize,
    inline_index: usize,
    before: String,
}

#[pyclass(name = "ImportedSection", frozen, module = "rocx.editor")]
struct PyImportedSection {
    #[pyo3(get)]
    body_start: usize,
    #[pyo3(get)]
    body_end: usize,
    #[pyo3(get)]
    section_type: Option<&'static str>,
    #[pyo3(get)]
    section_type_xml: Option<String>,
}

#[pyclass(name = "HeaderImageEdit", frozen, module = "rocx.editor")]
struct PyHeaderImageEdit {
    inner: editor::HeaderImageEdit,
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
        let mode = match mode {
            "shared" => editor::HeaderEditMode::Shared,
            "section_local" => editor::HeaderEditMode::SectionLocal,
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
            inner: editor::HeaderImageEdit {
                target: anchor.inner.clone(),
                image,
                mode,
            },
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

impl From<&editor::HeaderImageChange> for PyHeaderImageChange {
    fn from(change: &editor::HeaderImageChange) -> Self {
        Self {
            source_part: change.source_part.clone(),
            output_part: change.output_part.clone(),
            affected_sections: change.affected_sections.clone(),
            restored_section: change.restored_section,
            mode: match change.mode {
                editor::HeaderEditMode::Shared => "shared",
                editor::HeaderEditMode::SectionLocal => "section_local",
            }
            .to_owned(),
        }
    }
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
        Ok(preview.change().into())
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
    #[pyo3(get)]
    inline_items: Vec<Py<PyImportedInline>>,
    #[pyo3(get)]
    inline_structure: Vec<Py<PyAny>>,
}

#[pyclass(name = "InlineAnchor", frozen, eq, module = "rocx.editor")]
#[derive(PartialEq)]
struct PyInlineAnchor {
    inner: editor::InlineAnchor,
}

#[pyclass(name = "ImportedRun", frozen, module = "rocx.editor")]
struct PyImportedRun {
    #[pyo3(get)]
    anchor: Py<PyInlineAnchor>,
    #[pyo3(get)]
    items: Vec<Py<PyImportedInline>>,
}

#[pyclass(name = "ImportedHyperlink", frozen, module = "rocx.editor")]
struct PyImportedHyperlink {
    #[pyo3(get)]
    anchor: Py<PyInlineAnchor>,
    #[pyo3(get)]
    relationship_id: Option<String>,
    #[pyo3(get)]
    bookmark: Option<String>,
    #[pyo3(get)]
    children: Vec<Py<PyAny>>,
}

#[pyclass(name = "ImportedRevision", frozen, module = "rocx.editor")]
struct PyImportedRevision {
    #[pyo3(get)]
    anchor: Py<PyInlineAnchor>,
    #[pyo3(get)]
    kind: &'static str,
    #[pyo3(get)]
    id: Option<String>,
    #[pyo3(get)]
    author: Option<String>,
    #[pyo3(get)]
    date: Option<String>,
    #[pyo3(get)]
    children: Vec<Py<PyAny>>,
}

fn inline_object(py: Python<'_>, node: &editor::ImportedInlineNode) -> PyResult<Py<PyAny>> {
    use editor::ImportedInlineNode;
    let anchor = |inner: &editor::InlineAnchor| {
        Py::new(
            py,
            PyInlineAnchor {
                inner: inner.clone(),
            },
        )
    };
    Ok(match node {
        ImportedInlineNode::Run(run) => Py::new(
            py,
            PyImportedRun {
                anchor: anchor(&run.anchor)?,
                items: run
                    .items
                    .iter()
                    .map(|item| Py::new(py, PyImportedInline::from(item)))
                    .collect::<PyResult<_>>()?,
            },
        )?
        .into_any(),
        ImportedInlineNode::Hyperlink(link) => Py::new(
            py,
            PyImportedHyperlink {
                anchor: anchor(&link.anchor)?,
                relationship_id: link.relationship_id.clone(),
                bookmark: link.bookmark.clone(),
                children: link
                    .children
                    .iter()
                    .map(|child| inline_object(py, child))
                    .collect::<PyResult<_>>()?,
            },
        )?
        .into_any(),
        ImportedInlineNode::Revision(revision) => Py::new(
            py,
            PyImportedRevision {
                anchor: anchor(&revision.anchor)?,
                kind: revision.kind,
                id: revision.id.clone(),
                author: revision.author.clone(),
                date: revision.date.clone(),
                children: revision
                    .children
                    .iter()
                    .map(|child| inline_object(py, child))
                    .collect::<PyResult<_>>()?,
            },
        )?
        .into_any(),
        ImportedInlineNode::Opaque(xml) => {
            Py::new(py, PyBodyOpaque { xml: xml.clone() })?.into_any()
        }
    })
}

#[pyclass(name = "ImportedInline", frozen, module = "rocx.editor")]
struct PyImportedInline {
    #[pyo3(get)]
    kind: &'static str,
    #[pyo3(get)]
    value: Option<String>,
}

impl From<&editor::ImportedInline> for PyImportedInline {
    fn from(item: &editor::ImportedInline) -> Self {
        use crate::BreakType;
        use editor::ImportedInline::*;
        let (kind, value) = match item {
            Text(text) => ("text", Some(text.clone())),
            Tab => ("tab", None),
            Break(value) => (
                "break",
                Some(
                    match value {
                        BreakType::Line => "line",
                        BreakType::Page => "page",
                        BreakType::Column => "column",
                        BreakType::TextWrapping => "textWrapping",
                    }
                    .to_owned(),
                ),
            ),
            Opaque(name) => ("opaque", Some(name.clone())),
        };
        Self { kind, value }
    }
}

#[pyclass(name = "BodyTable", frozen, module = "rocx.editor")]
struct PyBodyTable {
    #[pyo3(get)]
    anchor: Py<PyTableAnchor>,
    #[pyo3(get)]
    children: Vec<Py<PyAny>>,
}

#[pyclass(name = "TableAnchor", frozen, module = "rocx.editor")]
struct PyTableAnchor {
    inner: editor::TableAnchor,
}

#[pyclass(name = "ImportedTableRow", frozen, module = "rocx.editor")]
struct PyImportedTableRow {
    #[pyo3(get)]
    children: Vec<Py<PyAny>>,
}

#[pyclass(name = "ImportedTableCell", frozen, module = "rocx.editor")]
struct PyImportedTableCell {
    #[pyo3(get)]
    blocks: Vec<Py<PyAny>>,
}

#[pyclass(name = "ImportedCellParagraph", frozen, module = "rocx.editor")]
struct PyImportedCellParagraph {
    #[pyo3(get)]
    inline_items: Vec<Py<PyImportedInline>>,
    #[pyo3(get)]
    inline_structure: Vec<Py<PyAny>>,
}

fn table_object(py: Python<'_>, table: &editor::ImportedTable) -> PyResult<Py<PyAny>> {
    use editor::{ImportedCellBlock, ImportedRowChild, ImportedTableChild};
    let children = table
        .children
        .iter()
        .map(|child| match child {
            ImportedTableChild::Opaque(xml) => {
                Ok(Py::new(py, PyBodyOpaque { xml: xml.clone() })?.into_any())
            }
            ImportedTableChild::Row(row) => {
                let children = row
                    .children
                    .iter()
                    .map(|child| match child {
                        ImportedRowChild::Opaque(xml) => {
                            Ok(Py::new(py, PyBodyOpaque { xml: xml.clone() })?.into_any())
                        }
                        ImportedRowChild::Cell(cell) => {
                            let blocks = cell
                                .blocks
                                .iter()
                                .enumerate()
                                .map(|(index, block)| match block {
                                    ImportedCellBlock::Opaque(xml) => {
                                        Ok(Py::new(py, PyBodyOpaque { xml: xml.clone() })?
                                            .into_any())
                                    }
                                    ImportedCellBlock::Paragraph(items) => Ok(Py::new(
                                        py,
                                        PyImportedCellParagraph {
                                            inline_items: items
                                                .iter()
                                                .map(|item| {
                                                    Py::new(py, PyImportedInline::from(item))
                                                })
                                                .collect::<PyResult<_>>()?,
                                            inline_structure: cell
                                                .inline_structure(index)
                                                .unwrap_or_default()
                                                .iter()
                                                .map(|node| inline_object(py, node))
                                                .collect::<PyResult<_>>()?,
                                        },
                                    )?
                                    .into_any()),
                                })
                                .collect::<PyResult<_>>()?;
                            Ok(Py::new(py, PyImportedTableCell { blocks })?.into_any())
                        }
                    })
                    .collect::<PyResult<_>>()?;
                Ok(Py::new(py, PyImportedTableRow { children })?.into_any())
            }
        })
        .collect::<PyResult<_>>()?;
    Ok(Py::new(
        py,
        PyBodyTable {
            anchor: Py::new(
                py,
                PyTableAnchor {
                    inner: table.anchor.clone(),
                },
            )?,
            children,
        },
    )?
    .into_any())
}

#[pyclass(name = "BodyOpaque", frozen, module = "rocx.editor")]
struct PyBodyOpaque {
    #[pyo3(get)]
    xml: String,
}

#[pyclass(name = "BodyContentControl", frozen, module = "rocx.editor")]
struct PyBodyContentControl {
    #[pyo3(get)]
    tag: Option<String>,
    #[pyo3(get)]
    alias: Option<String>,
    #[pyo3(get)]
    lock: Option<String>,
}

#[pyclass(name = "ParagraphAlignmentEdit", frozen, module = "rocx.editor")]
struct PyParagraphAlignmentEdit {
    target: editor::ParagraphAnchor,
    alignment: crate::Alignment,
}

#[pyclass(name = "CellAlignmentEdit", frozen, module = "rocx.editor")]
struct PyCellAlignmentEdit {
    inner: editor::CellAlignmentEdit,
}

#[pymethods]
impl PyCellAlignmentEdit {
    #[new]
    fn new(
        target: PyRef<'_, PyTableAnchor>,
        row_index: usize,
        cell_index: usize,
        alignment: &str,
    ) -> PyResult<Self> {
        let alignment = match alignment {
            "top" => crate::VerticalAlignment::Top,
            "center" => crate::VerticalAlignment::Center,
            "bottom" => crate::VerticalAlignment::Bottom,
            "both" => crate::VerticalAlignment::Both,
            _ => {
                return Err(PyValueError::new_err(
                    "alignment must be top, center, bottom or both",
                ));
            }
        };
        Ok(Self {
            inner: editor::CellAlignmentEdit {
                target: target.inner.clone(),
                row_index,
                cell_index,
                alignment,
            },
        })
    }
}

#[pyclass(name = "CellAlignmentChange", frozen, module = "rocx.editor", get_all)]
struct PyCellAlignmentChange {
    body_index: usize,
    row_index: usize,
    cell_index: usize,
    before: Option<String>,
    after: &'static str,
}

fn cell_alignment_name(value: crate::VerticalAlignment) -> &'static str {
    match value {
        crate::VerticalAlignment::Top => "top",
        crate::VerticalAlignment::Center => "center",
        crate::VerticalAlignment::Bottom => "bottom",
        crate::VerticalAlignment::Both => "both",
    }
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

#[pyclass(name = "ParagraphInsertion", frozen, module = "rocx.editor")]
struct PyParagraphInsertion {
    target: editor::ParagraphAnchor,
    paragraph: crate::ResolvedParagraph,
}

#[pymethods]
impl PyParagraphInsertion {
    #[new]
    fn new(
        target: PyRef<'_, PyParagraphAnchor>,
        resolved_paragraph: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let node_type = resolved_paragraph
            .py()
            .import("rocx.resolution.snapshot")?
            .getattr("ResolvedNode")?;
        if !resolved_paragraph.is_instance(&node_type)?
            || resolved_paragraph
                .getattr("_native_kind")?
                .extract::<String>()?
                != "Paragraph"
        {
            return Err(PyTypeError::new_err("expected a resolved Paragraph"));
        }
        Ok(Self {
            target: target.inner.clone(),
            paragraph: super::content::paragraph(resolved_paragraph)?,
        })
    }
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
    fn paragraph_insertions(&self) -> PyResult<Vec<usize>> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Preview lock poisoned"))?
            .paragraph_insertions()
            .to_vec())
    }
    #[getter]
    fn cell_alignment_changes(&self) -> PyResult<Vec<PyCellAlignmentChange>> {
        let preview = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Preview lock poisoned"))?;
        Ok(preview
            .cell_alignment_changes()
            .iter()
            .map(|c| PyCellAlignmentChange {
                body_index: c.body_index,
                row_index: c.row_index,
                cell_index: c.cell_index,
                before: c.before.clone(),
                after: cell_alignment_name(c.after),
            })
            .collect())
    }
    #[getter]
    fn run_removals(&self) -> PyResult<Vec<PyRunRemovalChange>> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Preview lock poisoned"))?
            .run_removals()
            .iter()
            .map(|c| PyRunRemovalChange {
                body_index: c.body_index,
                inline_index: c.inline_index,
                before: c.before.clone(),
            })
            .collect())
    }
    #[getter]
    fn run_text_changes(&self) -> PyResult<Vec<PyRunTextChange>> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Preview lock poisoned"))?
            .run_text_changes()
            .iter()
            .map(|c| PyRunTextChange {
                body_index: c.body_index,
                inline_index: c.inline_index,
                before: c.before.clone(),
                after: c.after.clone(),
            })
            .collect())
    }
    /// Header image changes in target section order.
    #[getter]
    fn header_changes(&self) -> PyResult<Vec<PyHeaderImageChange>> {
        let preview = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Preview lock poisoned"))?;
        Ok(preview.header_changes().iter().map(Into::into).collect())
    }

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

#[pyclass(name = "ImportedStyle", module = "rocx.editor", get_all)]
struct PyImportedStyle {
    id: Option<String>,
    kind: &'static str,
    kind_xml: Option<String>,
    name: Option<String>,
    based_on: Option<String>,
}

#[pyclass(name = "ImportedStyles", module = "rocx.editor", get_all)]
struct PyImportedStyles {
    part_name: String,
    styles: Vec<Py<PyImportedStyle>>,
}

#[pymethods]
impl PyImportedDocument {
    #[getter]
    fn styles(&self, py: Python<'_>) -> PyResult<Option<PyImportedStyles>> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        doc.styles()
            .map_err(rust_error)?
            .map(|styles| {
                let items = styles
                    .styles
                    .into_iter()
                    .map(|style| {
                        let kind_xml = match &style.kind {
                            editor::StyleKind::Unsupported(value) => Some(value.clone()),
                            editor::StyleKind::Missing => None,
                            kind => Some(kind.as_str().to_owned()),
                        };
                        Py::new(
                            py,
                            PyImportedStyle {
                                id: style.id,
                                kind: style.kind.as_str(),
                                kind_xml,
                                name: style.name,
                                based_on: style.based_on,
                            },
                        )
                    })
                    .collect::<PyResult<Vec<_>>>()?;
                Ok(PyImportedStyles {
                    part_name: styles.part_name,
                    styles: items,
                })
            })
            .transpose()
    }
    #[staticmethod]
    fn open(data: &Bound<'_, PyBytes>) -> PyResult<Self> {
        Ok(Self {
            inner: Mutex::new(editor::ImportedDocument::open(data.as_bytes()).map_err(rust_error)?),
        })
    }

    #[getter]
    fn sections(&self) -> PyResult<Vec<PyImportedSection>> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(doc
            .sections()
            .map_err(rust_error)?
            .into_iter()
            .map(|section| PyImportedSection {
                body_start: section.body_start,
                body_end: section.body_end,
                section_type: section.section_type.map(crate::SectionType::xml),
                section_type_xml: section.section_type_xml,
            })
            .collect())
    }

    #[getter]
    fn default_headers(&self, py: Python<'_>) -> PyResult<Vec<PyImportedHeader>> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        doc.default_headers()
            .map_err(rust_error)?
            .iter()
            .map(|header| {
                Ok(PyImportedHeader {
                    anchor: Py::new(
                        py,
                        PyHeaderAnchor {
                            inner: header.anchor.clone(),
                        },
                    )?,
                    section_index: header.section_index,
                    part_name: header.part_name.clone(),
                    inherited: header.inherited,
                    blocks: story_blocks(py, &header.blocks, |index| {
                        header.inline_structure(index)
                    })?,
                })
            })
            .collect()
    }

    /// Effective header/footer references in section, kind, and variant order.
    /// Absent slots are omitted; page selection settings do not filter the view.
    #[getter]
    fn stories(&self, py: Python<'_>) -> PyResult<Vec<PyImportedStory>> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        doc.stories()
            .map_err(rust_error)?
            .into_iter()
            .map(|story| {
                Ok(PyImportedStory {
                    anchor: story
                        .anchor
                        .clone()
                        .map(|inner| Py::new(py, PyHeaderAnchor { inner }))
                        .transpose()?,
                    section_index: story.section_index,
                    kind: story.kind.as_str(),
                    variant: story.variant.xml(),
                    part_name: story.part_name.clone(),
                    inherited: story.inherited,
                    blocks: story_blocks(py, &story.blocks, |index| story.inline_structure(index))?,
                })
            })
            .collect()
    }

    #[getter]
    fn entry_stories(&self, py: Python<'_>) -> PyResult<Vec<PyImportedEntry>> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        doc.entry_stories()
            .map_err(rust_error)?
            .iter()
            .map(|e| {
                Ok(PyImportedEntry {
                    kind: e.kind.as_str(),
                    part_name: e.part_name.clone(),
                    id: e.id.clone(),
                    note_type: e.note_type.clone(),
                    author: e.author.clone(),
                    date: e.date.clone(),
                    initials: e.initials.clone(),
                    blocks: story_blocks(py, e.blocks(), |i| e.inline_structure(i))?,
                })
            })
            .collect()
    }

    fn preview_header_image(
        &self,
        edit: PyRef<'_, PyHeaderImageEdit>,
    ) -> PyResult<PyHeaderImagePreview> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(PyHeaderImagePreview {
            inner: Mutex::new(doc.preview_header_image(&edit.inner).map_err(rust_error)?),
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
                        inline_items: p
                            .inline_items
                            .iter()
                            .map(|item| Py::new(py, PyImportedInline::from(item)))
                            .collect::<PyResult<_>>()?,
                        inline_structure: p
                            .inline_structure
                            .iter()
                            .map(|node| inline_object(py, node))
                            .collect::<PyResult<_>>()?,
                    },
                )?
                .into_any()),
                BodyBlock::Table(table) => table_object(py, table),
                BodyBlock::ContentControl(control) => Ok(Py::new(
                    py,
                    PyBodyContentControl {
                        tag: control.tag.clone(),
                        alias: control.alias.clone(),
                        lock: control.lock.clone(),
                    },
                )?
                .into_any()),
                BodyBlock::Opaque(xml) => {
                    Ok(Py::new(py, PyBodyOpaque { xml: xml.clone() })?.into_any())
                }
            })
            .collect()
    }

    /// Read a body or default-header run or hyperlink from this snapshot.
    fn inline_at(&self, py: Python<'_>, target: PyRef<'_, PyInlineAnchor>) -> PyResult<Py<PyAny>> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        inline_object(py, doc.inline_at(&target.inner).map_err(rust_error)?)
    }

    /// Stage all supplied edits in one transaction against the original snapshot.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (edits, *, table_insertions=None, header_images=None, run_text=None, run_removals=None, paragraph_insertions=None, cell_alignments=None))]
    fn preview(
        &self,
        edits: Vec<PyRef<'_, PyParagraphAlignmentEdit>>,
        table_insertions: Option<Vec<PyRef<'_, PyTableInsertion>>>,
        header_images: Option<Vec<PyRef<'_, PyHeaderImageEdit>>>,
        run_text: Option<Vec<PyRef<'_, PyRunTextEdit>>>,
        run_removals: Option<Vec<PyRef<'_, PyInlineAnchor>>>,
        paragraph_insertions: Option<Vec<PyRef<'_, PyParagraphInsertion>>>,
        cell_alignments: Option<Vec<PyRef<'_, PyCellAlignmentEdit>>>,
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
        let header_images: Vec<_> = header_images
            .unwrap_or_default()
            .iter()
            .map(|edit| edit.inner.clone())
            .collect();
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(PyEditPreview {
            inner: Mutex::new(
                doc.preview_mixed(
                    &edits,
                    &insertions,
                    &header_images,
                    &run_text
                        .unwrap_or_default()
                        .iter()
                        .map(|e| e.inner.clone())
                        .collect::<Vec<_>>(),
                    &paragraph_insertions
                        .unwrap_or_default()
                        .iter()
                        .map(|e| editor::ParagraphInsertion {
                            target: e.target.clone(),
                            paragraph: e.paragraph.clone(),
                        })
                        .collect::<Vec<_>>(),
                    &cell_alignments
                        .unwrap_or_default()
                        .iter()
                        .map(|e| e.inner.clone())
                        .collect::<Vec<_>>(),
                    &run_removals
                        .unwrap_or_default()
                        .iter()
                        .map(|anchor| anchor.inner.clone())
                        .collect::<Vec<_>>(),
                )
                .map_err(rust_error)?,
            ),
        })
    }
}

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyHeaderAnchor>()?;
    module.add_class::<PyImportedHeader>()?;
    module.add_class::<PyImportedHeaderParagraph>()?;
    module.add_class::<PyImportedStory>()?;
    module.add_class::<PyImportedEntry>()?;
    module.add_class::<PyImportedStyle>()?;
    module.add_class::<PyImportedStyles>()?;
    module.add_class::<PyRunTextEdit>()?;
    module.add_class::<PyRunTextChange>()?;
    module.add_class::<PyRunRemovalChange>()?;
    module.add_class::<PyImportedSection>()?;
    module.add_class::<PyHeaderImageEdit>()?;
    module.add_class::<PyHeaderImageChange>()?;
    module.add_class::<PyHeaderImagePreview>()?;
    module.add_class::<PyParagraphAnchor>()?;
    module.add_class::<PyBodyParagraph>()?;
    module.add_class::<PyImportedInline>()?;
    module.add_class::<PyInlineAnchor>()?;
    module.add_class::<PyImportedRun>()?;
    module.add_class::<PyImportedHyperlink>()?;
    module.add_class::<PyImportedRevision>()?;
    module.add_class::<PyBodyTable>()?;
    module.add_class::<PyTableAnchor>()?;
    module.add_class::<PyCellAlignmentEdit>()?;
    module.add_class::<PyCellAlignmentChange>()?;
    module.add_class::<PyImportedTableRow>()?;
    module.add_class::<PyImportedTableCell>()?;
    module.add_class::<PyImportedCellParagraph>()?;
    module.add_class::<PyBodyOpaque>()?;
    module.add_class::<PyBodyContentControl>()?;
    module.add_class::<PyParagraphAlignmentEdit>()?;
    module.add_class::<PyTableInsertion>()?;
    module.add_class::<PyParagraphInsertion>()?;
    module.add_class::<PyAlignmentChange>()?;
    module.add_class::<PyTableInsertionChange>()?;
    module.add_class::<PyEditPreview>()?;
    module.add_class::<PyImportedDocument>()?;
    Ok(())
}
