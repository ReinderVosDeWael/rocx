//! Python objects over the imported-document editing core.
use std::sync::Mutex;

use pyo3::{
    IntoPyObjectExt,
    exceptions::{PyRuntimeError, PyTypeError, PyValueError},
    prelude::*,
    types::{PyBool, PyBytes, PyDict, PyList, PyModule},
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

#[pyclass(name = "SectionAnchor", frozen, module = "rocx.editor")]
struct PySectionAnchor {
    inner: editor::SectionAnchor,
}

#[pyclass(name = "LayoutEdit", frozen, module = "rocx.editor")]
struct PyLayoutEdit {
    inner: editor::LayoutEdit,
}

fn layout_value(o: &Bound<'_, PyAny>) -> PyResult<editor::LayoutValue> {
    use editor::LayoutValue::*;
    if o.is_instance_of::<PyBool>() {
        return Ok(Boolean(o.extract()?));
    }
    if let Ok(s) = o.extract::<String>() {
        return Ok(Token(s));
    }
    if let Ok(d) = o.cast::<PyDict>() {
        if d.keys().iter().any(|k| {
            !matches!(
                k.extract::<String>().as_deref(),
                Ok("size" | "space" | "color")
            )
        }) {
            return Err(PyValueError::new_err("unknown page border field"));
        }
        let num = |k: &str| {
            d.get_item(k)?
                .map(|v| {
                    if v.is_instance_of::<PyBool>() {
                        Err(PyValueError::new_err("expected integer"))
                    } else {
                        v.extract::<i32>()
                    }
                })
                .transpose()
        };
        return Ok(Border(editor::LayoutBorder {
            size: num("size")?,
            space: num("space")?,
            color: d.get_item("color")?.map(|v| v.extract()).transpose()?,
        }));
    }
    if let Ok(tabs) = o.cast::<PyList>() {
        return Ok(Tabs(
            tabs.iter()
                .map(|item| {
                    let d = item.cast::<PyDict>()?;
                    if d.keys().iter().any(|k| {
                        !matches!(
                            k.extract::<String>().as_deref(),
                            Ok("position" | "alignment" | "leader")
                        )
                    }) {
                        return Err(PyValueError::new_err("unknown tab stop field"));
                    }
                    let pos = d
                        .get_item("position")?
                        .ok_or_else(|| PyValueError::new_err("missing tab position"))?;
                    if pos.is_instance_of::<PyBool>() {
                        return Err(PyValueError::new_err("expected integer tab position"));
                    }
                    Ok(editor::LayoutTab {
                        position: pos.extract()?,
                        alignment: d
                            .get_item("alignment")?
                            .ok_or_else(|| PyValueError::new_err("missing tab alignment"))?
                            .extract()?,
                        leader: d.get_item("leader")?.map(|k| k.extract()).transpose()?,
                    })
                })
                .collect::<PyResult<_>>()?,
        ));
    }
    Ok(Integer(o.extract()?))
}
fn layout_field(s: &str) -> PyResult<editor::LayoutField> {
    editor::LayoutField::ALL
        .iter()
        .copied()
        .find(|f| f.python_name() == s)
        .ok_or_else(|| PyValueError::new_err("unknown layout field"))
}
#[pymethods]
impl PyLayoutEdit {
    #[staticmethod]
    fn paragraph(
        target: PyRef<'_, PyParagraphAnchor>,
        field: &str,
        value: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: editor::LayoutEdit::Paragraph(
                target.inner.clone(),
                layout_field(field)?,
                value.map(layout_value).transpose()?,
            ),
        })
    }
    #[staticmethod]
    fn section(
        target: PyRef<'_, PySectionAnchor>,
        field: &str,
        value: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: editor::LayoutEdit::Section(
                target.inner.clone(),
                layout_field(field)?,
                value.map(layout_value).transpose()?,
            ),
        })
    }
}
type LayoutMap = std::collections::BTreeMap<editor::LayoutField, editor::LayoutValue>;
fn layout_view<'py>(py: Python<'py>, values: &LayoutMap) -> PyResult<Bound<'py, PyDict>> {
    use editor::LayoutValue::*;
    let d = PyDict::new(py);
    for (f, v) in values {
        let k = f.python_name();
        match v {
            Token(s) => d.set_item(k, s)?,
            Boolean(b) => d.set_item(k, b)?,
            Integer(i) => d.set_item(k, i)?,
            Border(b) => {
                let edge = PyDict::new(py);
                if let Some(v) = b.size {
                    edge.set_item("size", v)?
                }
                if let Some(v) = b.space {
                    edge.set_item("space", v)?
                }
                if let Some(v) = &b.color {
                    edge.set_item("color", v)?
                }
                d.set_item(k, edge)?;
            }
            Tabs(stops) => {
                let list = PyList::empty(py);
                for t in stops {
                    let tab = PyDict::new(py);
                    tab.set_item("position", t.position)?;
                    tab.set_item("alignment", &t.alignment)?;
                    if let Some(l) = &t.leader {
                        tab.set_item("leader", l)?
                    }
                    list.append(tab)?;
                }
                d.set_item(k, list)?;
            }
        }
    }
    Ok(d)
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

#[pyclass(name = "TextRangeEdit", frozen, module = "rocx.editor")]
struct PyTextRangeEdit {
    inner: editor::TextRangeEdit,
}

#[pymethods]
impl PyTextRangeEdit {
    #[new]
    fn new(target: PyRef<'_, PyParagraphAnchor>, start: usize, end: usize, text: String) -> Self {
        Self {
            inner: editor::TextRangeEdit {
                target: target.inner.clone(),
                start,
                end,
                text,
            },
        }
    }
}

#[pyclass(name = "TextInsertion", frozen, module = "rocx.editor")]
struct PyTextInsertion {
    inner: editor::TextInsertion,
}

#[pymethods]
impl PyTextInsertion {
    #[new]
    fn new(target: PyRef<'_, PyParagraphAnchor>, position: usize, text: String) -> Self {
        Self {
            inner: editor::TextInsertion {
                target: target.inner.clone(),
                position,
                text,
            },
        }
    }
}

#[pyclass(name = "RunTextEdit", frozen, module = "rocx.editor")]
struct PyRunTextEdit {
    inner: editor::RunTextEdit,
}

#[pyclass(name = "ControlAnchor", frozen, module = "rocx.editor")]
struct PyControlAnchor {
    inner: editor::ControlAnchor,
}

#[pyclass(name = "ImportedControl", frozen, get_all, module = "rocx.editor")]
struct PyImportedControl {
    anchor: Py<PyControlAnchor>,
    context: &'static str,
    id: Option<String>,
    tag: Option<String>,
    alias: Option<String>,
    lock: Option<String>,
    bound: bool,
}

#[pyclass(name = "ControlEdit", frozen, module = "rocx.editor")]
struct PyControlEdit {
    inner: editor::ControlEdit,
}

#[pymethods]
impl PyControlEdit {
    #[new]
    #[pyo3(signature = (target, *, tag=None, alias=None, text=None))]
    fn new(
        target: PyRef<'_, PyControlAnchor>,
        tag: Option<String>,
        alias: Option<String>,
        text: Option<String>,
    ) -> Self {
        Self {
            inner: editor::ControlEdit {
                target: target.inner.clone(),
                tag,
                alias,
                text,
            },
        }
    }
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
    anchor: Py<PySectionAnchor>,
    layout: LayoutMap,
    #[pyo3(get)]
    flags: std::collections::BTreeMap<&'static str, bool>,
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
    layout: LayoutMap,
    #[pyo3(get)]
    flags: std::collections::BTreeMap<&'static str, bool>,
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

#[pymethods]
impl PyBodyParagraph {
    #[getter]
    fn layout<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        layout_view(py, &self.layout)
    }
}
#[pymethods]
impl PyImportedSection {
    #[getter]
    fn layout<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        layout_view(py, &self.layout)
    }
}

#[pyclass(name = "ImportedRun", frozen, module = "rocx.editor")]
struct PyImportedRun {
    direct_format: crate::RunFormat,
    #[pyo3(get)]
    has_opaque_formatting: bool,
    #[pyo3(get)]
    bold: Option<bool>,
    #[pyo3(get)]
    italic: Option<bool>,
    #[pyo3(get)]
    strike: Option<bool>,
    #[pyo3(get)]
    hidden: Option<bool>,
    #[pyo3(get)]
    anchor: Py<PyInlineAnchor>,
    #[pyo3(get)]
    items: Vec<Py<PyImportedInline>>,
}

#[pymethods]
impl PyImportedRun {
    /// An independent dictionary of directly modeled fields; no effective styles.
    #[getter]
    fn format<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let f = &self.direct_format;
        let out = PyDict::new(py);
        macro_rules! scalars { ($($field:ident),*) => {$(
            if let Some(v) = &f.$field { out.set_item(stringify!($field),v)?; }
        )*}; }
        scalars!(
            style,
            bold,
            italic,
            font,
            size,
            superscript,
            subscript,
            strike,
            all_caps,
            small_caps,
            snap_to_grid,
            bold_complex_script,
            italic_complex_script,
            size_complex_script,
            rtl,
            complex_script,
            font_east_asia,
            font_complex_script
        );
        if let Some(u) = f.underline {
            out.set_item("underline", u.xml())?;
        }
        if let Some(color) = f.color {
            match color {
                crate::Color::Auto => out.set_item("color", "auto")?,
                crate::Color::Rgb([r, g, b]) => out.set_item("color", (r, g, b))?,
                crate::Color::Theme { name, tint, shade } => {
                    let value = PyDict::new(py);
                    value.set_item("theme", name.xml())?;
                    if let Some(v) = tint {
                        value.set_item("tint", v)?;
                    }
                    if let Some(v) = shade {
                        value.set_item("shade", v)?;
                    }
                    out.set_item("color", value)?;
                }
            }
        }
        if let Some(lang) = &f.language {
            out.set_item(
                "language",
                py.import("rocx.declarative")?
                    .getattr("RunLanguage")?
                    .call1((
                        lang.latin.as_deref(),
                        lang.east_asia.as_deref(),
                        lang.bidi.as_deref(),
                    ))?,
            )?;
        }
        Ok(out)
    }
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
                direct_format: run.format.clone(),
                has_opaque_formatting: run.has_opaque_formatting,
                bold: run.bold,
                italic: run.italic,
                strike: run.strike,
                hidden: run.hidden,
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
            NoteReference { kind, id } => (
                match kind {
                    crate::NoteKind::Footnote => "footnoteReference",
                    crate::NoteKind::Endnote => "endnoteReference",
                },
                Some(id.clone()),
            ),
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
    properties: editor::DirectTableProperties,
    #[pyo3(get)]
    layout: Option<String>,
    #[pyo3(get)]
    grid_widths: Vec<Option<u32>>,
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
    properties: editor::DirectTableProperties,
    #[pyo3(get)]
    children: Vec<Py<PyAny>>,
}

#[pyclass(name = "ImportedTableCell", frozen, module = "rocx.editor")]
struct PyImportedTableCell {
    #[pyo3(get)]
    properties: editor::DirectTableProperties,
    #[pyo3(get)]
    grid_span: Option<u32>,
    #[pyo3(get)]
    vertical_merge: Option<String>,
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
                            Ok(Py::new(
                                py,
                                PyImportedTableCell {
                                    properties: cell.properties.clone(),
                                    blocks,
                                    grid_span: cell.grid_span,
                                    vertical_merge: cell.vertical_merge.clone(),
                                },
                            )?
                            .into_any())
                        }
                    })
                    .collect::<PyResult<_>>()?;
                Ok(Py::new(
                    py,
                    PyImportedTableRow {
                        children,
                        properties: row.properties.clone(),
                    },
                )?
                .into_any())
            }
        })
        .collect::<PyResult<_>>()?;
    Ok(Py::new(
        py,
        PyBodyTable {
            properties: table.properties.clone(),
            layout: table.layout.map(|v| v.xml().to_owned()),
            grid_widths: table.grid_widths.clone(),
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

#[pyclass(name = "TableStructureEdit", frozen, module = "rocx.editor")]
struct PyTableStructureEdit {
    inner: editor::TableStructureEdit,
}
fn table_site(
    row: Option<usize>,
    cell: Option<usize>,
    column: Option<usize>,
) -> PyResult<editor::TableSite> {
    use editor::TableSite::*;
    Ok(match (row, cell, column) {
        (None, None, None) => Table,
        (Some(r), None, None) => Row(r),
        (Some(r), Some(c), None) => Cell(r, c),
        (None, None, Some(c)) => Column(c),
        _ => return Err(PyValueError::new_err("invalid table site coordinates")),
    })
}
#[pymethods]
impl PyTableStructureEdit {
    #[staticmethod]
    #[pyo3(signature=(target,field,source,*,row=None,cell=None,column=None))]
    fn patch_property(
        target: PyRef<'_, PyTableAnchor>,
        field: String,
        source: Option<&Bound<'_, PyAny>>,
        row: Option<usize>,
        cell: Option<usize>,
        column: Option<usize>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: editor::TableStructureEdit {
                target: target.inner.clone(),
                operation: editor::TableOperation::Property {
                    site: table_site(row, cell, column)?,
                    field,
                    source: source.map(resolved_block_table).transpose()?.map(Box::new),
                },
            },
        })
    }
    #[staticmethod]
    #[pyo3(signature=(target,source,*,row=None,cell=None))]
    fn replace_node(
        target: PyRef<'_, PyTableAnchor>,
        source: &Bound<'_, PyAny>,
        row: Option<usize>,
        cell: Option<usize>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: editor::TableStructureEdit {
                target: target.inner.clone(),
                operation: editor::TableOperation::ReplaceNode {
                    site: table_site(row, cell, None)?,
                    source: Box::new(resolved_block_table(source)?),
                },
            },
        })
    }
    #[staticmethod]
    fn remove_table(target: PyRef<'_, PyTableAnchor>) -> Self {
        Self {
            inner: editor::TableStructureEdit {
                target: target.inner.clone(),
                operation: editor::TableOperation::RemoveTable,
            },
        }
    }
    #[staticmethod]
    #[pyo3(signature = (target, value))]
    fn layout(
        target: PyRef<'_, PyTableAnchor>,
        value: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: editor::TableStructureEdit {
                target: target.inner.clone(),
                operation: editor::TableOperation::Layout(
                    value.map(super::values::tablelayout).transpose()?,
                ),
            },
        })
    }
    #[staticmethod]
    fn insert_row(target: PyRef<'_, PyTableAnchor>, index: usize) -> Self {
        Self {
            inner: editor::TableStructureEdit {
                target: target.inner.clone(),
                operation: editor::TableOperation::InsertRow(index),
            },
        }
    }
    #[staticmethod]
    fn remove_row(target: PyRef<'_, PyTableAnchor>, index: usize) -> Self {
        Self {
            inner: editor::TableStructureEdit {
                target: target.inner.clone(),
                operation: editor::TableOperation::RemoveRow(index),
            },
        }
    }
    #[staticmethod]
    fn insert_column(target: PyRef<'_, PyTableAnchor>, index: usize) -> Self {
        Self {
            inner: editor::TableStructureEdit {
                target: target.inner.clone(),
                operation: editor::TableOperation::InsertColumn(index),
            },
        }
    }
    #[staticmethod]
    fn remove_column(target: PyRef<'_, PyTableAnchor>, index: usize) -> Self {
        Self {
            inner: editor::TableStructureEdit {
                target: target.inner.clone(),
                operation: editor::TableOperation::RemoveColumn(index),
            },
        }
    }
    #[staticmethod]
    fn insert_cell(target: PyRef<'_, PyTableAnchor>, row: usize, cell: usize) -> Self {
        Self {
            inner: editor::TableStructureEdit {
                target: target.inner.clone(),
                operation: editor::TableOperation::InsertCell { row, cell },
            },
        }
    }
    #[staticmethod]
    fn remove_cell(target: PyRef<'_, PyTableAnchor>, row: usize, cell: usize) -> Self {
        Self {
            inner: editor::TableStructureEdit {
                target: target.inner.clone(),
                operation: editor::TableOperation::RemoveCell { row, cell },
            },
        }
    }
    #[staticmethod]
    fn replace_cell(
        target: PyRef<'_, PyTableAnchor>,
        row: usize,
        cell: usize,
        blocks: Vec<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: editor::TableStructureEdit {
                target: target.inner.clone(),
                operation: editor::TableOperation::ReplaceCell {
                    row,
                    cell,
                    blocks: blocks
                        .iter()
                        .map(|block| {
                            let cls = block
                                .py()
                                .import("rocx.resolution.snapshot")?
                                .getattr("ResolvedNode")?;
                            if !block.is_instance(&cls)? {
                                return Err(PyTypeError::new_err("expected a resolved cell block"));
                            }
                            super::content::block(block)?
                                .ok_or_else(|| PyValueError::new_err("empty cell block"))
                        })
                        .collect::<PyResult<_>>()?,
                },
            },
        })
    }
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

#[pyclass(name = "InlineEdit", frozen, module = "rocx.editor")]
struct PyInlineEdit {
    inner: editor::InlineEdit,
}

fn inline_payload(node: &Bound<'_, PyAny>) -> PyResult<crate::ResolvedInline> {
    if !node.is_instance(
        &node
            .py()
            .import("rocx.resolution.snapshot")?
            .getattr("ResolvedNode")?,
    )? {
        return Err(PyTypeError::new_err("expected resolved inline"));
    }
    super::content::inline(node)?.ok_or_else(|| PyTypeError::new_err("expected resolved inline"))
}

#[pymethods]
impl PyInlineEdit {
    #[staticmethod]
    #[pyo3(signature = (target, *, clear=None, **properties))]
    fn patch_format(
        target: PyRef<'_, PyInlineAnchor>,
        clear: Option<Vec<String>>,
        properties: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let py = target.py();
        let kwargs = match properties {
            Some(d) => d.copy()?,
            None => PyDict::new(py),
        };
        let style = kwargs
            .get_item("style")?
            .map(|v| v.extract::<String>())
            .transpose()?;
        if style.is_some() {
            kwargs.del_item("style")?;
        }
        let defaults = py
            .import("rocx.declarative")?
            .getattr("RunDefaults")?
            .call((), Some(&kwargs))?;
        let mut set = super::values::run_format(&defaults)?;
        set.style = style;
        let clear = clear
            .unwrap_or_default()
            .iter()
            .map(|name| {
                let mut camel = String::new();
                for piece in name.split('_') {
                    if camel.is_empty() {
                        camel.push_str(piece);
                    } else {
                        let mut chars = piece.chars();
                        if let Some(c) = chars.next() {
                            camel.push(c.to_ascii_uppercase());
                            camel.push_str(chars.as_str());
                        }
                    }
                }
                editor::RunFormatField::parse(&camel).ok_or_else(|| {
                    PyValueError::new_err(format!("unknown run format field {name}"))
                })
            })
            .collect::<PyResult<_>>()?;
        Ok(Self {
            inner: editor::InlineEdit {
                target: target.inner.clone(),
                action: editor::InlineAction::PatchFormat {
                    set: Box::new(set),
                    clear,
                },
            },
        })
    }
    #[staticmethod]
    fn insert_after(
        target: PyRef<'_, PyInlineAnchor>,
        inline: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: editor::InlineEdit {
                target: target.inner.clone(),
                action: editor::InlineAction::InsertAfter(inline_payload(inline)?),
            },
        })
    }
    #[staticmethod]
    fn replace(target: PyRef<'_, PyInlineAnchor>, inline: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: editor::InlineEdit {
                target: target.inner.clone(),
                action: editor::InlineAction::Replace(inline_payload(inline)?),
            },
        })
    }
    #[staticmethod]
    fn set_bold(target: PyRef<'_, PyInlineAnchor>, bold: bool) -> Self {
        Self {
            inner: editor::InlineEdit {
                target: target.inner.clone(),
                action: editor::InlineAction::SetBold(bold),
            },
        }
    }

    #[staticmethod]
    #[pyo3(signature = (target, property, value))]
    fn set_toggle(
        target: PyRef<'_, PyInlineAnchor>,
        property: &str,
        value: Option<bool>,
    ) -> PyResult<Self> {
        let prop = editor::RunToggle::parse(property)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("unknown run toggle"))?;
        Ok(Self {
            inner: editor::InlineEdit {
                target: target.inner.clone(),
                action: editor::InlineAction::SetToggle(prop, value),
            },
        })
    }
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
        Ok(Self {
            target: target.inner.clone(),
            paragraph: resolved_block_paragraph(resolved_paragraph)?,
        })
    }
}

fn resolved_block_paragraph(
    resolved_paragraph: &Bound<'_, PyAny>,
) -> PyResult<crate::ResolvedParagraph> {
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
    super::content::paragraph(resolved_paragraph)
}

#[pyclass(name = "BlockEdit", frozen, module = "rocx.editor")]
struct PyBlockEdit {
    inner: editor::BlockEdit,
}

#[pymethods]
impl PyBlockEdit {
    #[staticmethod]
    fn split_section(target: PyRef<'_, PySectionAnchor>, offset: usize) -> Self {
        Self {
            inner: editor::BlockEdit::SplitSection(target.inner.clone(), offset),
        }
    }
    #[staticmethod]
    fn merge_section(target: PyRef<'_, PySectionAnchor>) -> Self {
        Self {
            inner: editor::BlockEdit::MergeSection(target.inner.clone()),
        }
    }
    #[staticmethod]
    fn replace(
        target: PyRef<'_, PyParagraphAnchor>,
        paragraph: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: editor::BlockEdit::Replace(
                target.inner.clone(),
                Box::new(resolved_block_paragraph(paragraph)?),
            ),
        })
    }
    #[staticmethod]
    fn remove(target: PyRef<'_, PyParagraphAnchor>) -> Self {
        Self {
            inner: editor::BlockEdit::Remove(target.inner.clone()),
        }
    }
    #[staticmethod]
    fn replace_section(
        target: PyRef<'_, PyParagraphAnchor>,
        blocks: Vec<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: editor::BlockEdit::ReplaceSection(
                target.inner.clone(),
                blocks
                    .iter()
                    .map(|block| {
                        if matches!(
                            block
                                .getattr("_native_kind")
                                .and_then(|v| v.extract::<String>())
                                .as_deref(),
                            Ok("Table")
                        ) {
                            resolved_block_table(block).map(crate::ResolvedBlock::Table)
                        } else {
                            resolved_block_paragraph(block).map(crate::ResolvedBlock::Paragraph)
                        }
                    })
                    .collect::<PyResult<_>>()?,
            ),
        })
    }
    #[staticmethod]
    fn keep_with_next(target: PyRef<'_, PyParagraphAnchor>, value: bool) -> Self {
        Self {
            inner: editor::BlockEdit::KeepWithNext(target.inner.clone(), value),
        }
    }
    #[staticmethod]
    fn paragraph_flag(
        target: PyRef<'_, PyParagraphAnchor>,
        flag: &str,
        value: Option<bool>,
    ) -> PyResult<Self> {
        let flag = editor::ParagraphFlag::ALL
            .iter()
            .find(|f| f.python_name() == flag)
            .ok_or_else(|| PyValueError::new_err("unknown paragraph flag"))?;
        Ok(Self {
            inner: editor::BlockEdit::ParagraphFlag(target.inner.clone(), *flag, value),
        })
    }
    #[staticmethod]
    fn section_flag(
        target: PyRef<'_, PyParagraphAnchor>,
        flag: &str,
        value: Option<bool>,
    ) -> PyResult<Self> {
        let flag = editor::SectionFlag::ALL
            .iter()
            .find(|f| f.python_name() == flag)
            .ok_or_else(|| PyValueError::new_err("unknown section flag"))?;
        Ok(Self {
            inner: editor::BlockEdit::SectionFlag(target.inner.clone(), *flag, value),
        })
    }
    #[staticmethod]
    fn section_type(
        target: PyRef<'_, PyParagraphAnchor>,
        value: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: editor::BlockEdit::SectionType(
                target.inner.clone(),
                value.map(super::sectiontype).transpose()?,
            ),
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
        Ok(Self {
            target: target.inner.clone(),
            table: resolved_block_table(resolved_table)?,
        })
    }
}

fn story_blocks_payload(blocks: Vec<Bound<'_, PyAny>>) -> PyResult<Vec<crate::ResolvedBlock>> {
    blocks
        .iter()
        .map(|b| {
            if super::kind(b)? == "Table" {
                resolved_block_table(b).map(crate::ResolvedBlock::Table)
            } else {
                resolved_block_paragraph(b).map(crate::ResolvedBlock::Paragraph)
            }
        })
        .collect()
}

fn resolved_block_table(resolved_table: &Bound<'_, PyAny>) -> PyResult<crate::ResolvedTable> {
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
    Ok(table)
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
    fn inline_changes(&self) -> PyResult<Vec<(usize, usize)>> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Preview lock poisoned"))?
            .inline_changes()
            .to_vec())
    }
    #[getter]
    fn block_changes(&self) -> PyResult<Vec<usize>> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Preview lock poisoned"))?
            .block_changes()
            .to_vec())
    }
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

#[pyclass(name = "StyleAnchor", frozen, module = "rocx.editor")]
struct PyStyleAnchor {
    inner: editor::StyleAnchor,
}

#[pyclass(name = "StyleEdit", frozen, module = "rocx.editor")]
struct PyStyleEdit {
    inner: editor::StyleEdit,
}

#[pymethods]
impl PyStyleEdit {
    #[staticmethod]
    #[pyo3(signature=(target, field, value, *, region=None))]
    fn field(
        target: PyRef<'_, PyStyleAnchor>,
        field: String,
        value: Option<String>,
        region: Option<String>,
    ) -> Self {
        Self {
            inner: editor::StyleEdit::Field {
                target: target.inner.clone(),
                field,
                value,
                region,
            },
        }
    }
    #[staticmethod]
    #[pyo3(signature=(target, region, format))]
    fn region(
        target: PyRef<'_, PyStyleAnchor>,
        region: String,
        format: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: editor::StyleEdit::Region {
                target: target.inner.clone(),
                region,
                format: format
                    .map(super::values::tablesectionformat)
                    .transpose()?
                    .map(Box::new),
            },
        })
    }
    #[staticmethod]
    fn add(definition: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: editor::StyleEdit::Add(super::style_definition(definition)?),
        })
    }
    #[staticmethod]
    #[pyo3(signature=(target, *, bold=None, italic=None))]
    fn patch(target: PyRef<'_, PyStyleAnchor>, bold: Option<bool>, italic: Option<bool>) -> Self {
        Self {
            inner: editor::StyleEdit::Patch {
                target: target.inner.clone(),
                bold,
                italic,
            },
        }
    }
    #[staticmethod]
    fn replace(target: PyRef<'_, PyStyleAnchor>, definition: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: editor::StyleEdit::Replace {
                target: target.inner.clone(),
                definition: super::style_definition(definition)?,
            },
        })
    }
    #[staticmethod]
    fn remove(target: PyRef<'_, PyStyleAnchor>) -> Self {
        Self {
            inner: editor::StyleEdit::Remove(target.inner.clone()),
        }
    }
}

#[pyclass(name = "ImportedStyle", module = "rocx.editor", get_all)]
struct PyImportedStyle {
    id: Option<String>,
    kind: &'static str,
    kind_xml: Option<String>,
    name: Option<String>,
    based_on: Option<String>,
    next_style: Option<String>,
    linked_style: Option<String>,
    property_xml: Vec<String>,
}

#[pyclass(name = "ImportedStyles", module = "rocx.editor", get_all)]
struct PyImportedStyles {
    part_name: String,
    styles: Vec<Py<PyImportedStyle>>,
}

#[pyclass(name = "AbstractStart", module = "rocx.editor", frozen, get_all)]
struct PyAbstractStart {
    abstract_id: u32,
    level: u8,
    start: Option<u32>,
}

#[pyclass(
    name = "ImportedNumberingInstance",
    module = "rocx.editor",
    frozen,
    get_all
)]
struct PyImportedNumberingInstance {
    num_id: u32,
    abstract_id: u32,
    /// Pairs of (level, direct starting value) in source order.
    start_overrides: Vec<(u8, u32)>,
    replacement_levels: Vec<u32>,
}

#[pyclass(name = "ResourceEdit", module = "rocx.editor", frozen)]
struct PyResourceEdit {
    inner: editor::ResourceEdit,
}

#[pymethods]
impl PyResourceEdit {
    #[staticmethod]
    #[pyo3(signature=(field,value))]
    fn document_default(field: &str, value: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        use editor::{DefaultField, DefaultValue as V};
        let field = DefaultField::from_key(field)
            .ok_or_else(|| PyValueError::new_err("unknown document default"))?;
        let value = value
            .map(|v| -> PyResult<_> {
                if let Ok(b) = v.extract::<bool>() {
                    Ok(V::Bool(b))
                } else if let Ok(n) = v.extract::<i64>() {
                    Ok(V::Integer(n))
                } else if let Ok(s) = v.extract::<String>() {
                    Ok(V::Text(s))
                } else {
                    Ok(V::Tabs(
                        v.extract::<Vec<(i32, String, Option<String>)>>()?
                            .into_iter()
                            .map(|(position_twips, alignment, leader)| editor::DefaultTab {
                                position_twips,
                                alignment,
                                leader,
                            })
                            .collect(),
                    ))
                }
            })
            .transpose()?;
        Ok(Self {
            inner: editor::ResourceEdit::Default(field, value),
        })
    }
    #[staticmethod]
    #[pyo3(signature=(field, value))]
    fn metadata(field: &str, value: Option<String>) -> PyResult<Self> {
        Ok(Self {
            inner: editor::ResourceEdit::Metadata(field.parse().map_err(rust_error)?, value),
        })
    }
    #[staticmethod]
    #[pyo3(signature=(name, value))]
    fn custom_property(name: String, value: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let value = value
            .map(|v| {
                if v.is_instance_of::<pyo3::types::PyBool>() {
                    Ok(editor::CustomValue::Boolean(v.extract()?))
                } else if v.is_instance_of::<pyo3::types::PyString>() {
                    Ok(editor::CustomValue::Text(v.extract()?))
                } else {
                    Err(PyTypeError::new_err("expected a string, boolean or None"))
                }
            })
            .transpose()?;
        Ok(Self {
            inner: editor::ResourceEdit::CustomProperty(name, value),
        })
    }
    #[staticmethod]
    #[pyo3(signature=(value))]
    fn core_title(value: Option<String>) -> Self {
        Self {
            inner: editor::ResourceEdit::CoreTitle(value),
        }
    }
    #[staticmethod]
    #[pyo3(signature=(value))]
    fn core_subject(value: Option<String>) -> Self {
        Self {
            inner: editor::ResourceEdit::CoreSubject(value),
        }
    }
    #[staticmethod]
    #[pyo3(signature=(value))]
    fn default_alignment(value: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        Ok(Self {
            inner: editor::ResourceEdit::DefaultAlignment(value.map(super::alignment).transpose()?),
        })
    }
    #[staticmethod]
    #[pyo3(signature=(abstract_id, level, start))]
    fn abstract_start(abstract_id: u32, level: u8, start: Option<u32>) -> Self {
        Self {
            inner: editor::ResourceEdit::AbstractStart(editor::AbstractStart {
                abstract_id,
                level,
                start,
            }),
        }
    }
    #[staticmethod]
    fn add_abstract(abstract_id: u32, levels: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: editor::ResourceEdit::Numbering(editor::NumberingEdit::AddAbstract {
                abstract_id,
                levels: super::list(levels, super::numbering_level)?,
            }),
        })
    }
    #[staticmethod]
    #[pyo3(signature=(num_id,level,value))]
    fn instance_level(num_id: u32, level: u8, value: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        Ok(Self {
            inner: editor::ResourceEdit::Numbering(editor::NumberingEdit::InstanceLevel {
                num_id,
                level,
                value: value
                    .map(super::numbering_level)
                    .transpose()?
                    .flatten()
                    .map(Box::new),
            }),
        })
    }
    #[staticmethod]
    fn remove_abstract(abstract_id: u32) -> Self {
        Self {
            inner: editor::ResourceEdit::Numbering(editor::NumberingEdit::RemoveAbstract {
                abstract_id,
            }),
        }
    }
    #[staticmethod]
    fn add_instance(num_id: u32, abstract_id: u32) -> Self {
        Self {
            inner: editor::ResourceEdit::Numbering(editor::NumberingEdit::AddInstance {
                num_id,
                abstract_id,
            }),
        }
    }
    #[staticmethod]
    fn remove_instance(num_id: u32) -> Self {
        Self {
            inner: editor::ResourceEdit::Numbering(editor::NumberingEdit::RemoveInstance {
                num_id,
            }),
        }
    }
    #[staticmethod]
    #[pyo3(signature=(num_id,level,start))]
    fn instance_start(num_id: u32, level: u8, start: Option<u32>) -> Self {
        Self {
            inner: editor::ResourceEdit::Numbering(editor::NumberingEdit::InstanceStart {
                num_id,
                level,
                start,
            }),
        }
    }
    #[staticmethod]
    #[pyo3(signature=(target,value))]
    fn paragraph_numbering(target: PyRef<'_, PyParagraphAnchor>, value: Option<(u32, u8)>) -> Self {
        Self {
            inner: editor::ResourceEdit::Numbering(editor::NumberingEdit::Paragraph {
                target: target.inner.clone(),
                value: value.map(|(id, level)| crate::NumberingReference { id, level }),
            }),
        }
    }
}

#[pymethods]
impl PyImportedDocument {
    fn style_anchor(&self, id: &str) -> PyResult<PyStyleAnchor> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(PyStyleAnchor {
            inner: doc.style_anchor(id).map_err(rust_error)?,
        })
    }
    fn metadata_field(&self, field: &str) -> PyResult<Option<String>> {
        self.inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?
            .metadata_field(field.parse().map_err(rust_error)?)
            .map_err(rust_error)
    }
    fn custom_property(&self, py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
        match self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?
            .custom_property(name)
            .map_err(rust_error)?
        {
            Some(editor::CustomValue::Text(s)) => s.into_py_any(py),
            Some(editor::CustomValue::Boolean(v)) => v.into_py_any(py),
            None => Ok(py.None()),
        }
    }
    #[getter]
    fn core_title(&self) -> PyResult<Option<String>> {
        self.inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?
            .core_title()
            .map_err(rust_error)
    }
    #[getter]
    fn core_subject(&self) -> PyResult<Option<String>> {
        self.inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?
            .core_subject()
            .map_err(rust_error)
    }
    #[getter]
    fn default_alignment(&self) -> PyResult<Option<&'static str>> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?
            .default_alignment()
            .map_err(rust_error)?
            .map(alignment_name))
    }
    #[getter]
    fn document_defaults<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        use editor::DefaultValue as V;
        let values = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?
            .document_defaults()
            .map_err(rust_error)?;
        let dict = pyo3::types::PyDict::new(py);
        for (f, v) in values {
            match v {
                V::Bool(b) => dict.set_item(f.key(), b)?,
                V::Integer(i) => dict.set_item(f.key(), i)?,
                V::Text(s) => dict.set_item(f.key(), s)?,
                V::Tabs(t) => dict.set_item(
                    f.key(),
                    t.into_iter()
                        .map(|t| (t.position_twips, t.alignment, t.leader))
                        .collect::<Vec<_>>(),
                )?,
            }
        }
        Ok(dict)
    }
    #[getter]
    fn abstract_starts(&self) -> PyResult<Vec<PyAbstractStart>> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?
            .abstract_starts()
            .map_err(rust_error)?
            .into_iter()
            .map(|s| PyAbstractStart {
                abstract_id: s.abstract_id,
                level: s.level,
                start: s.start,
            })
            .collect())
    }
    #[getter]
    fn numbering_instances(&self) -> PyResult<Vec<PyImportedNumberingInstance>> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?
            .numbering_instances()
            .map_err(rust_error)?
            .into_iter()
            .map(|n| PyImportedNumberingInstance {
                num_id: n.num_id,
                replacement_levels: n.replacement_levels.into_iter().map(u32::from).collect(),
                abstract_id: n.abstract_id,
                start_overrides: n
                    .start_overrides
                    .into_iter()
                    .map(|o| (o.level, o.start))
                    .collect(),
            })
            .collect())
    }
    fn paragraph_numbering(
        &self,
        target: PyRef<'_, PyParagraphAnchor>,
    ) -> PyResult<Option<(u32, u8)>> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?
            .paragraph_numbering(&target.inner)
            .map_err(rust_error)?
            .map(|r| (r.id, r.level)))
    }
    #[getter]
    fn controls(&self, py: Python<'_>) -> PyResult<Vec<PyImportedControl>> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        doc.controls()
            .iter()
            .map(|c| {
                Ok(PyImportedControl {
                    anchor: Py::new(
                        py,
                        PyControlAnchor {
                            inner: c.anchor.clone(),
                        },
                    )?,
                    context: c.context.as_str(),
                    id: c.id.clone(),
                    tag: c.tag.clone(),
                    alias: c.alias.clone(),
                    lock: c.lock.clone(),
                    bound: c.bound,
                })
            })
            .collect()
    }
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
                                next_style: style.next_style,
                                linked_style: style.linked_style,
                                property_xml: style.property_xml,
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
    fn sections(&self, py: Python<'_>) -> PyResult<Vec<PyImportedSection>> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        doc.sections()
            .map_err(rust_error)?
            .into_iter()
            .map(|section| {
                Ok(PyImportedSection {
                    layout: section.layout,
                    anchor: Py::new(
                        py,
                        PySectionAnchor {
                            inner: section.anchor,
                        },
                    )?,
                    flags: section
                        .flags
                        .iter()
                        .map(|(f, v)| (f.python_name(), *v))
                        .collect(),
                    body_start: section.body_start,
                    body_end: section.body_end,
                    section_type: section.section_type.map(crate::SectionType::xml),
                    section_type_xml: section.section_type_xml,
                })
            })
            .collect()
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

    #[pyo3(signature = (owner_part, target, kind, *, id=None, text=None))]
    fn edit_note_reference<'py>(
        &self,
        py: Python<'py>,
        owner_part: &str,
        target: PyRef<'_, PyInlineAnchor>,
        kind: &str,
        id: Option<u32>,
        text: Option<String>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        use editor::NoteReferenceEdit::*;
        let edit = match (kind, id, text) {
            ("create", Some(id), Some(text)) => Create { id, text },
            ("delete", None, None) => Delete,
            ("detach", None, None) => Detach,
            ("attach", Some(id), None) => Attach(id),
            ("retarget", Some(id), None) => Retarget(id),
            ("renumber", Some(id), None) => Renumber(id),
            _ => {
                return Err(PyValueError::new_err(
                    "invalid note reference edit arguments",
                ));
            }
        };
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(PyBytes::new(
            py,
            &doc.edit_note_reference(owner_part, &target.inner, edit)
                .map_err(rust_error)?,
        ))
    }

    #[pyo3(signature = (owner_part, id, kind, *, index=None, value=None))]
    fn edit_entry<'py>(
        &self,
        py: Python<'py>,
        owner_part: &str,
        id: &str,
        kind: &str,
        index: Option<usize>,
        value: Option<String>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        use editor::EntryEdit::*;
        let edit = match (kind, index, value) {
            ("insertParagraph", Some(index), Some(text)) => InsertParagraph { index, text },
            ("replaceParagraph", Some(index), Some(text)) => ReplaceParagraph { index, text },
            ("removeParagraph", Some(index), None) => RemoveParagraph(index),
            ("deleteUnusedNote", None, None) => DeleteUnusedNote,
            ("author", None, Some(s)) => CommentAuthor(s),
            ("initials", None, Some(s)) => CommentInitials(Some(s)),
            ("clearInitials", None, None) => CommentInitials(None),
            ("date", None, Some(s)) => CommentDate(Some(s)),
            ("clearDate", None, None) => CommentDate(None),
            _ => return Err(PyValueError::new_err("invalid entry edit arguments")),
        };
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(PyBytes::new(
            py,
            &doc.edit_entry(owner_part, id, edit).map_err(rust_error)?,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (section_index, story_kind, variant, action, *, blocks=None, part_name=None))]
    fn edit_story_reference<'py>(
        &self,
        py: Python<'py>,
        section_index: usize,
        story_kind: &str,
        variant: &Bound<'_, PyAny>,
        action: &str,
        blocks: Option<Vec<Bound<'_, PyAny>>>,
        part_name: Option<String>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        use editor::{SectionStoryKind, StoryReferenceEdit::*};
        let kind = match story_kind {
            "header" => SectionStoryKind::Header,
            "footer" => SectionStoryKind::Footer,
            _ => return Err(PyValueError::new_err("expected header or footer")),
        };
        let variant = super::values::headerfootertype(variant)?;
        let edit = match (action, blocks, part_name) {
            ("create", Some(b), None) => Create(story_blocks_payload(b)?),
            ("link", None, Some(p)) => Link(p),
            ("remove", None, None) => Remove,
            _ => {
                return Err(PyValueError::new_err(
                    "expected create with blocks, link with part_name, or remove",
                ));
            }
        };
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(PyBytes::new(
            py,
            &doc.edit_story_reference(section_index, kind, variant, edit)
                .map_err(rust_error)?,
        ))
    }

    fn edit_story_blocks<'py>(
        &self,
        py: Python<'py>,
        target: PyRef<'_, PyHeaderAnchor>,
        mode: &str,
        index: usize,
        remove: usize,
        blocks: Vec<Bound<'_, PyAny>>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let mode = match mode {
            "shared" => editor::HeaderEditMode::Shared,
            "section_local" => editor::HeaderEditMode::SectionLocal,
            _ => return Err(PyValueError::new_err("expected shared or section_local")),
        };
        let blocks = story_blocks_payload(blocks)?;
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(PyBytes::new(
            py,
            &doc.edit_story_blocks(&target.inner, mode, index, remove, &blocks)
                .map_err(rust_error)?,
        ))
    }

    fn edit_entry_inlines<'py>(
        &self,
        py: Python<'py>,
        owner_part: &str,
        edits: Vec<PyRef<'_, PyInlineEdit>>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        let edits = edits.iter().map(|e| e.inner.clone()).collect::<Vec<_>>();
        Ok(PyBytes::new(
            py,
            &doc.edit_entry_inlines(owner_part, &edits)
                .map_err(rust_error)?,
        ))
    }

    fn edit_story_text<'py>(
        &self,
        py: Python<'py>,
        owner_part: &str,
        edits: Vec<PyRef<'_, PyRunTextEdit>>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        let edits = edits.iter().map(|e| e.inner.clone()).collect::<Vec<_>>();
        Ok(PyBytes::new(
            py,
            &doc.edit_story_text(owner_part, &edits)
                .map_err(rust_error)?,
        ))
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
                        layout: p.layout.clone(),
                        flags: p.flags.iter().map(|(f, v)| (f.python_name(), *v)).collect(),
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

    /// Track insertion after, replacement of, or deletion of one body run.
    #[pyo3(signature = (target, kind, author, *, text=None, date=None))]
    fn preview_tracked_text(
        &self,
        target: PyRef<'_, PyInlineAnchor>,
        kind: &str,
        author: &str,
        text: Option<String>,
        date: Option<&str>,
    ) -> PyResult<PyEditPreview> {
        use editor::TrackedText::*;
        let edit = match (kind, text) {
            ("insertAfter", Some(t)) => InsertAfter(t),
            ("replace", Some(t)) => Replace(t),
            ("delete", None) => Delete,
            _ => {
                return Err(PyValueError::new_err(
                    "expected insertAfter/replace with text or delete without text",
                ));
            }
        };
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(PyEditPreview {
            inner: Mutex::new(
                doc.preview_tracked_text(&target.inner, edit, author, date)
                    .map_err(rust_error)?,
            ),
        })
    }

    fn preview_layout(&self, edits: Vec<PyRef<'_, PyLayoutEdit>>) -> PyResult<PyEditPreview> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        let edits = edits.iter().map(|e| e.inner.clone()).collect::<Vec<_>>();
        Ok(PyEditPreview {
            inner: Mutex::new(doc.preview_layout(&edits).map_err(rust_error)?),
        })
    }

    fn preview_tracked_move(
        &self,
        target: PyRef<'_, PyInlineAnchor>,
        after: PyRef<'_, PyInlineAnchor>,
        name: &str,
        author: &str,
        date: &str,
    ) -> PyResult<PyEditPreview> {
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(PyEditPreview {
            inner: Mutex::new(
                doc.preview_tracked_move(&target.inner, &after.inner, name, author, date)
                    .map_err(rust_error)?,
            ),
        })
    }

    #[pyo3(signature = (target, action, *, paired=None))]
    fn preview_revision(
        &self,
        target: PyRef<'_, PyInlineAnchor>,
        action: &str,
        paired: Option<PyRef<'_, PyInlineAnchor>>,
    ) -> PyResult<PyEditPreview> {
        let action = match action {
            "accept" => crate::TextRevisionAction::Accept,
            "reject" => crate::TextRevisionAction::Reject,
            _ => return Err(PyValueError::new_err("expected accept or reject")),
        };
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(PyEditPreview {
            inner: Mutex::new(
                doc.preview_revision(&target.inner, action, paired.as_ref().map(|a| &a.inner))
                    .map_err(rust_error)?,
            ),
        })
    }

    /// Stage all supplied edits in one transaction against the original snapshot.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (edits, *, table_insertions=None, header_images=None, run_text=None, run_removals=None, paragraph_insertions=None, cell_alignments=None, text_ranges=None, text_insertions=None, block_edits=None, inline_edits=None, table_structure=None, style_edits=None, resources=None, controls=None))]
    fn preview(
        &self,
        edits: Vec<PyRef<'_, PyParagraphAlignmentEdit>>,
        table_insertions: Option<Vec<PyRef<'_, PyTableInsertion>>>,
        header_images: Option<Vec<PyRef<'_, PyHeaderImageEdit>>>,
        run_text: Option<Vec<PyRef<'_, PyRunTextEdit>>>,
        run_removals: Option<Vec<PyRef<'_, PyInlineAnchor>>>,
        paragraph_insertions: Option<Vec<PyRef<'_, PyParagraphInsertion>>>,
        cell_alignments: Option<Vec<PyRef<'_, PyCellAlignmentEdit>>>,
        text_ranges: Option<Vec<PyRef<'_, PyTextRangeEdit>>>,
        text_insertions: Option<Vec<PyRef<'_, PyTextInsertion>>>,
        block_edits: Option<Vec<PyRef<'_, PyBlockEdit>>>,
        inline_edits: Option<Vec<PyRef<'_, PyInlineEdit>>>,
        table_structure: Option<Vec<PyRef<'_, PyTableStructureEdit>>>,
        style_edits: Option<Vec<PyRef<'_, PyStyleEdit>>>,
        resources: Option<Vec<PyRef<'_, PyResourceEdit>>>,
        controls: Option<Vec<PyRef<'_, PyControlEdit>>>,
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
        macro_rules! inner_edits {
            ($items:ident) => {
                $items
                    .unwrap_or_default()
                    .iter()
                    .map(|e| e.inner.clone())
                    .collect::<Vec<_>>()
            };
        }
        let paragraphs: Vec<_> = paragraph_insertions
            .unwrap_or_default()
            .iter()
            .map(|e| editor::ParagraphInsertion {
                target: e.target.clone(),
                paragraph: e.paragraph.clone(),
            })
            .collect();
        let doc = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("Document lock poisoned"))?;
        Ok(PyEditPreview {
            inner: Mutex::new(
                doc.preview_edits(editor::EditBatch {
                    alignments: &edits,
                    tables: &insertions,
                    header_images: &inner_edits!(header_images),
                    run_text: &inner_edits!(run_text),
                    paragraphs: &paragraphs,
                    cells: &inner_edits!(cell_alignments),
                    run_removals: &inner_edits!(run_removals),
                    ranges: &inner_edits!(text_ranges),
                    text_insertions: &inner_edits!(text_insertions),
                    blocks: &inner_edits!(block_edits),
                    inline_edits: &inner_edits!(inline_edits),
                    structural: &inner_edits!(table_structure),
                    styles: &inner_edits!(style_edits),
                    resources: &inner_edits!(resources),
                    controls: &inner_edits!(controls),
                })
                .map_err(rust_error)?,
            ),
        })
    }
}

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyAbstractStart>()?;
    module.add_class::<PyImportedNumberingInstance>()?;
    module.add_class::<PyResourceEdit>()?;
    module.add_class::<PyStyleAnchor>()?;
    module.add_class::<PyStyleEdit>()?;
    module.add_class::<PyControlAnchor>()?;
    module.add_class::<PyControlEdit>()?;
    module.add_class::<PyImportedControl>()?;
    module.add_class::<PyBlockEdit>()?;
    module.add_class::<PyHeaderAnchor>()?;
    module.add_class::<PyImportedHeader>()?;
    module.add_class::<PyImportedHeaderParagraph>()?;
    module.add_class::<PyImportedStory>()?;
    module.add_class::<PyImportedEntry>()?;
    module.add_class::<PyImportedStyle>()?;
    module.add_class::<PyImportedStyles>()?;
    module.add_class::<PyTextRangeEdit>()?;
    module.add_class::<PyTextInsertion>()?;
    module.add_class::<PyRunTextEdit>()?;
    module.add_class::<PyRunTextChange>()?;
    module.add_class::<PyRunRemovalChange>()?;
    module.add_class::<PyImportedSection>()?;
    module.add_class::<PyHeaderImageEdit>()?;
    module.add_class::<PyHeaderImageChange>()?;
    module.add_class::<PyHeaderImagePreview>()?;
    module.add_class::<PyParagraphAnchor>()?;
    module.add_class::<PySectionAnchor>()?;
    module.add_class::<PyLayoutEdit>()?;
    module.add_class::<PyBodyParagraph>()?;
    module.add_class::<PyImportedInline>()?;
    module.add_class::<PyInlineAnchor>()?;
    module.add_class::<PyImportedRun>()?;
    module.add_class::<PyImportedHyperlink>()?;
    module.add_class::<PyImportedRevision>()?;
    module.add_class::<PyBodyTable>()?;
    module.add_class::<PyTableAnchor>()?;
    module.add_class::<PyTableStructureEdit>()?;
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
    module.add_class::<PyInlineEdit>()?;
    module.add_class::<PyAlignmentChange>()?;
    module.add_class::<PyTableInsertionChange>()?;
    module.add_class::<PyEditPreview>()?;
    module.add_class::<PyImportedDocument>()?;
    Ok(())
}
