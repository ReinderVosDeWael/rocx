//! Thin PyO3 conversion layer. Python handles asyncio; Rust owns document rendering.
use crate::*;
use pyo3::{
    exceptions::{
        PyFileNotFoundError, PyKeyError, PyOSError, PyPermissionError, PyTypeError, PyValueError,
    },
    prelude::*,
    types::{PyAny, PyBytes, PyDict, PyModule},
};

macro_rules! field {
    ($o:expr,$name:expr,$t:ty) => {
        $o.getattr($name)?.extract::<$t>()?
    };
}
macro_rules! dict_field {
    ($d:expr,$name:expr,$t:ty) => {
        $d.get_item($name)?
            .map(|v| v.extract::<Option<$t>>())
            .transpose()?
            .flatten()
    };
}
fn kind(o: &Bound<'_, PyAny>) -> PyResult<String> {
    o.getattr("_native_kind")
        .and_then(|v| v.extract())
        .map_err(|_| PyTypeError::new_err("Expected a declarative document component"))
}
fn active(o: &Bound<'_, PyAny>) -> PyResult<bool> {
    o.getattr("_active")?.extract()
}
fn optional_record<T>(
    o: &Bound<'_, PyAny>,
    name: &str,
    parse: impl FnOnce(&Bound<'_, PyAny>) -> PyResult<T>,
) -> PyResult<Option<T>> {
    let v = o.getattr(name)?;
    if v.is_none() {
        Ok(None)
    } else {
        Ok(Some(parse(&v)?))
    }
}
fn optional_enum<T>(
    o: &Bound<'_, PyAny>,
    name: &str,
    parse: impl FnOnce(&Bound<'_, PyAny>) -> PyResult<T>,
) -> PyResult<Option<T>> {
    optional_record(o, name, parse)
}
fn list<T>(
    o: &Bound<'_, PyAny>,
    parse: impl Fn(&Bound<'_, PyAny>) -> PyResult<Option<T>>,
) -> PyResult<Vec<T>> {
    if o.is_none() {
        return Ok(Vec::new());
    }
    let mut values = Vec::new();
    for v in o.try_iter()? {
        if let Some(v) = parse(&v?)? {
            values.push(v);
        }
    }
    Ok(values)
}
fn alignment(o: &Bound<'_, PyAny>) -> PyResult<Alignment> {
    if let Ok(v) = o.extract::<i64>() {
        return match v {
            0 => Ok(Alignment::Left),
            1 => Ok(Alignment::Center),
            2 => Ok(Alignment::Right),
            3 => Ok(Alignment::Justify),
            4 => Ok(Alignment::Distribute),
            5 => Ok(Alignment::JustifyMedium),
            7 => Ok(Alignment::JustifyHigh),
            8 => Ok(Alignment::JustifyLow),
            9 => Ok(Alignment::ThaiJustify),
            _ => Err(PyValueError::new_err("Invalid paragraph alignment")),
        };
    }
    match o.extract::<String>()?.as_str() {
        "left" => Ok(Alignment::Left),
        "center" => Ok(Alignment::Center),
        "right" => Ok(Alignment::Right),
        "both" | "justify" => Ok(Alignment::Justify),
        _ => Err(PyValueError::new_err("Invalid paragraph alignment")),
    }
}

fn breaktype(o: &Bound<'_, PyAny>) -> PyResult<BreakType> {
    match o.extract::<String>()?.as_str() {
        "line" => Ok(BreakType::Line),
        "textWrapping" => Ok(BreakType::TextWrapping),
        "page" => Ok(BreakType::Page),
        "column" => Ok(BreakType::Column),
        _ => Err(PyValueError::new_err("Invalid BreakType value")),
    }
}
fn orientation(o: &Bound<'_, PyAny>) -> PyResult<Orientation> {
    match o.extract::<String>()?.as_str() {
        "portrait" => Ok(Orientation::Portrait),
        "landscape" => Ok(Orientation::Landscape),
        _ => Err(PyValueError::new_err("Invalid Orientation value")),
    }
}
fn headerfootertype(o: &Bound<'_, PyAny>) -> PyResult<HeaderFooterType> {
    match o.extract::<String>()?.as_str() {
        "default" => Ok(HeaderFooterType::Default),
        "first" => Ok(HeaderFooterType::First),
        "even" => Ok(HeaderFooterType::Even),
        _ => Err(PyValueError::new_err("Invalid HeaderFooterType value")),
    }
}
fn verticalalignment(o: &Bound<'_, PyAny>) -> PyResult<VerticalAlignment> {
    match o.extract::<String>()?.as_str() {
        "top" => Ok(VerticalAlignment::Top),
        "center" => Ok(VerticalAlignment::Center),
        "bottom" => Ok(VerticalAlignment::Bottom),
        "both" => Ok(VerticalAlignment::Both),
        _ => Err(PyValueError::new_err("Invalid VerticalAlignment value")),
    }
}
fn sectiontype(o: &Bound<'_, PyAny>) -> PyResult<SectionType> {
    match o.extract::<String>()?.as_str() {
        "nextPage" => Ok(SectionType::NextPage),
        "nextColumn" => Ok(SectionType::NextColumn),
        "continuous" => Ok(SectionType::Continuous),
        "evenPage" => Ok(SectionType::EvenPage),
        "oddPage" => Ok(SectionType::OddPage),
        _ => Err(PyValueError::new_err("Invalid SectionType value")),
    }
}
fn verticalmerge(o: &Bound<'_, PyAny>) -> PyResult<VerticalMerge> {
    match o.extract::<String>()?.as_str() {
        "restart" => Ok(VerticalMerge::Restart),
        "continue" => Ok(VerticalMerge::Continue),
        _ => Err(PyValueError::new_err("Invalid VerticalMerge value")),
    }
}
fn tablelayout(o: &Bound<'_, PyAny>) -> PyResult<TableLayout> {
    match o.extract::<String>()?.as_str() {
        "fixed" => Ok(TableLayout::Fixed),
        "autofit" => Ok(TableLayout::Autofit),
        _ => Err(PyValueError::new_err("Invalid TableLayout value")),
    }
}
fn borderside(o: &Bound<'_, PyAny>) -> PyResult<BorderSide> {
    match o.extract::<String>()?.as_str() {
        "top" => Ok(BorderSide::Top),
        "left" => Ok(BorderSide::Left),
        "bottom" => Ok(BorderSide::Bottom),
        "right" => Ok(BorderSide::Right),
        "insideH" => Ok(BorderSide::InsideH),
        "insideV" => Ok(BorderSide::InsideV),
        _ => Err(PyValueError::new_err("Invalid BorderSide value")),
    }
}
fn cellborderside(o: &Bound<'_, PyAny>) -> PyResult<CellBorderSide> {
    match o.extract::<String>()?.as_str() {
        "top" => Ok(CellBorderSide::Top),
        "bottom" => Ok(CellBorderSide::Bottom),
        "start" => Ok(CellBorderSide::Start),
        "end" => Ok(CellBorderSide::End),
        _ => Err(PyValueError::new_err("Invalid CellBorderSide value")),
    }
}
fn borderline(o: &Bound<'_, PyAny>) -> PyResult<BorderLine> {
    match o.extract::<String>()?.as_str() {
        "single" => Ok(BorderLine::Single),
        "dashed" => Ok(BorderLine::Dashed),
        _ => Err(PyValueError::new_err("Invalid BorderLine value")),
    }
}
fn pagesize(o: &Bound<'_, PyAny>) -> PyResult<PageSize> {
    let d = o.cast::<PyDict>()?;
    Ok(PageSize {
        width: dict_field!(d, "width", i64),
        height: dict_field!(d, "height", i64),
    })
}
fn pagemargins(o: &Bound<'_, PyAny>) -> PyResult<PageMargins> {
    let d = o.cast::<PyDict>()?;
    Ok(PageMargins {
        top: dict_field!(d, "top", i64),
        bottom: dict_field!(d, "bottom", i64),
        left: dict_field!(d, "left", i64),
        right: dict_field!(d, "right", i64),
    })
}
fn pagenumbering(o: &Bound<'_, PyAny>) -> PyResult<PageNumbering> {
    let d = o.cast::<PyDict>()?;
    Ok(PageNumbering {
        start: dict_field!(d, "start", u32),
        format: dict_field!(d, "format", String),
    })
}
fn columns(o: &Bound<'_, PyAny>) -> PyResult<Columns> {
    let d = o.cast::<PyDict>()?;
    Ok(Columns {
        count: dict_field!(d, "count", u32),
        space: dict_field!(d, "space", u32),
        separator: dict_field!(d, "separator", bool),
    })
}
fn imagetransformation(o: &Bound<'_, PyAny>) -> PyResult<ImageTransformation> {
    let d = o.cast::<PyDict>()?;
    Ok(ImageTransformation {
        width: dict_field!(d, "width", f64),
        height: dict_field!(d, "height", f64),
    })
}
fn alttext(o: &Bound<'_, PyAny>) -> PyResult<AltText> {
    let d = o.cast::<PyDict>()?;
    Ok(AltText {
        title: dict_field!(d, "title", String),
        description: dict_field!(d, "description", String),
        name: dict_field!(d, "name", String),
    })
}
fn paragraphstyledefinition(o: &Bound<'_, PyAny>) -> PyResult<ParagraphStyleDefinition> {
    Ok(ParagraphStyleDefinition {
        name: field!(o, "name", String),
        base_style: field!(o, "base_style", Option<String>),
        next_paragraph_style: field!(o, "next_paragraph_style", Option<String>),
        bold: field!(o, "bold", Option<bool>),
        italic: field!(o, "italic", Option<bool>),
        underline: field!(o, "underline", Option<bool>),
        font: field!(o, "font", Option<String>),
        font_size: field!(o, "font_size", Option<f64>),
        color: field!(o, "color", Option<[u8; 3]>),
        alignment: optional_enum(o, "alignment", alignment)?,
        spacing_before: field!(o, "spacing_before", Option<f64>),
        spacing_after: field!(o, "spacing_after", Option<f64>),
        line_spacing: field!(o, "line_spacing", Option<f64>),
        left_indent: field!(o, "left_indent", Option<f64>),
        right_indent: field!(o, "right_indent", Option<f64>),
        first_line_indent: field!(o, "first_line_indent", Option<f64>),
        keep_together: field!(o, "keep_together", Option<bool>),
        keep_with_next: field!(o, "keep_with_next", Option<bool>),
        page_break_before: field!(o, "page_break_before", Option<bool>),
        widow_control: field!(o, "widow_control", Option<bool>),
    })
}
fn tablesectionformat(o: &Bound<'_, PyAny>) -> PyResult<TableSectionFormat> {
    Ok(TableSectionFormat {
        bold: field!(o, "bold", Option<bool>),
        italic: field!(o, "italic", Option<bool>),
        underline: field!(o, "underline", Option<bool>),
        font: field!(o, "font", Option<String>),
        font_size: field!(o, "font_size", Option<f64>),
        color: field!(o, "color", Option<[u8; 3]>),
        background: field!(o, "background", Option<[u8; 3]>),
        alignment: optional_enum(o, "alignment", alignment)?,
        spacing_before: field!(o, "spacing_before", Option<f64>),
        spacing_after: field!(o, "spacing_after", Option<f64>),
    })
}
fn tableborder(o: &Bound<'_, PyAny>) -> PyResult<TableBorder> {
    Ok(TableBorder {
        side: borderside(&o.getattr("side")?)?,
        sz: field!(o, "sz", u32),
        color: field!(o, "color", [u8; 3]),
        val: borderline(&o.getattr("val")?)?,
    })
}
fn cellborder(o: &Bound<'_, PyAny>) -> PyResult<CellBorder> {
    Ok(CellBorder {
        side: cellborderside(&o.getattr("side")?)?,
        sz: field!(o, "sz", u32),
        color: field!(o, "color", [u8; 3]),
        val: borderline(&o.getattr("val")?)?,
    })
}
fn tablestyledefinition(o: &Bound<'_, PyAny>) -> PyResult<TableStyleDefinition> {
    Ok(TableStyleDefinition {
        name: field!(o, "name", String),
        base_style: field!(o, "base_style", Option<String>),
        whole_table: optional_record(o, "whole_table", tablesectionformat)?,
        first_row: optional_record(o, "first_row", tablesectionformat)?,
        last_row: optional_record(o, "last_row", tablesectionformat)?,
        first_column: optional_record(o, "first_column", tablesectionformat)?,
        last_column: optional_record(o, "last_column", tablesectionformat)?,
        banding_1_row: optional_record(o, "banding_1_row", tablesectionformat)?,
        banding_2_row: optional_record(o, "banding_2_row", tablesectionformat)?,
        banding_1_column: optional_record(o, "banding_1_column", tablesectionformat)?,
        banding_2_column: optional_record(o, "banding_2_column", tablesectionformat)?,
        top_left_cell: optional_record(o, "top_left_cell", tablesectionformat)?,
        top_right_cell: optional_record(o, "top_right_cell", tablesectionformat)?,
        bottom_left_cell: optional_record(o, "bottom_left_cell", tablesectionformat)?,
        bottom_right_cell: optional_record(o, "bottom_right_cell", tablesectionformat)?,
    })
}
fn sectionproperties(o: &Bound<'_, PyAny>) -> PyResult<SectionProperties> {
    Ok(SectionProperties {
        page_size: optional_record(o, "page_size", pagesize)?,
        page_orientation: optional_enum(o, "page_orientation", orientation)?,
        page_margins: optional_record(o, "page_margins", pagemargins)?,
        page_numbering: optional_record(o, "page_numbering", pagenumbering)?,
        columns: optional_record(o, "columns", columns)?,
        vertical_align: optional_enum(o, "vertical_align", verticalalignment)?,
        title_page: field!(o, "title_page", Option<bool>),
        r#type: optional_enum(o, "type", sectiontype)?,
    })
}
fn paragraph(o: &Bound<'_, PyAny>) -> PyResult<Paragraph> {
    let text = o.getattr("text")?;
    let children = o.getattr("children")?;
    let mut result = match (text.is_none(), children.is_none()) {
        (false, true) => Paragraph::new(text.extract::<String>()?),
        (true, false) => Paragraph::from_children(list(&children, inline)?),
        _ => {
            return Err(PyValueError::new_err(
                "Paragraph requires exactly one of text or children",
            ));
        }
    };
    let heading = field!(o, "heading", Option<i64>);
    if let Some(level) = heading {
        if !(0..=9).contains(&level) {
            return Err(PyKeyError::new_err(format!(
                "no style with name 'Heading {level}'"
            )));
        }
    }
    result.heading = heading.map(|n| n as u8);
    result.style = field!(o, "style", Option<String>);
    result.alignment = optional_enum(o, "alignment", alignment)?;
    result.spacing_before = field!(o, "spacing_before", Option<f64>);
    result.spacing_after = field!(o, "spacing_after", Option<f64>);
    result.line_spacing = field!(o, "line_spacing", Option<f64>);
    result.left_indent = field!(o, "left_indent", Option<f64>);
    result.right_indent = field!(o, "right_indent", Option<f64>);
    result.first_line_indent = field!(o, "first_line_indent", Option<f64>);
    result.keep_together = field!(o, "keep_together", Option<bool>);
    result.keep_with_next = field!(o, "keep_with_next", Option<bool>);
    result.page_break_before = field!(o, "page_break_before", Option<bool>);
    result.widow_control = field!(o, "widow_control", Option<bool>);
    result.comment_text = field!(o, "comment_text", Option<String>).map(Value::ready);
    result.comment_author = field!(o, "comment_author", Option<String>).map(Value::ready);
    Ok(result)
}
fn textrun(o: &Bound<'_, PyAny>) -> PyResult<TextRun> {
    let text = field!(o, "text", String);
    let mut result = TextRun::new(text);
    result.bold = field!(o, "bold", Option<bool>);
    result.italic = field!(o, "italic", Option<bool>);
    result.underline = field!(o, "underline", Option<bool>);
    result.font = field!(o, "font", Option<String>);
    result.size = field!(o, "size", Option<f64>);
    result.color = field!(o, "color", Option<[u8; 3]>);
    result.superscript = field!(o, "superscript", Option<bool>);
    result.subscript = field!(o, "subscript", Option<bool>);
    result.strike = field!(o, "strike", Option<bool>);
    result.all_caps = field!(o, "all_caps", Option<bool>);
    result.small_caps = field!(o, "small_caps", Option<bool>);
    result.comment_text = field!(o, "comment_text", Option<String>).map(Value::ready);
    result.comment_author = field!(o, "comment_author", Option<String>).map(Value::ready);
    Ok(result)
}

fn inline(o: &Bound<'_, PyAny>) -> PyResult<Option<Value<Inline>>> {
    if !active(o)? {
        return Ok(None);
    }
    let value = match kind(o)?.as_str() {
        "TextRun" => Inline::TextRun(textrun(o)?),
        "Tab" => Inline::Tab(Tab::default()),
        "Break" => Inline::Break(Break::new(breaktype(&o.getattr("type")?)?)),
        "ImageRun" => {
            let data = o.getattr("data")?;
            let source = if let Ok(bytes) = data.cast::<PyBytes>() {
                ImageData::Bytes(bytes.as_bytes().to_vec())
            } else {
                ImageData::Path(data.extract::<std::path::PathBuf>()?)
            };
            let mut image = ImageRun::new(source);
            image.r#type = field!(o, "type", Option<String>);
            image.transformation = optional_record(o, "transformation", imagetransformation)?;
            image.alt_text = optional_record(o, "alt_text", alttext)?;
            Inline::ImageRun(image)
        }
        other => {
            return Err(PyTypeError::new_err(format!(
                "Unsupported inline component {other}"
            )));
        }
    };
    Ok(Some(Value::ready(value)))
}
fn block(o: &Bound<'_, PyAny>) -> PyResult<Option<Value<Block>>> {
    if !active(o)? {
        return Ok(None);
    }
    Ok(Some(Value::ready(match kind(o)?.as_str() {
        "Paragraph" => Block::Paragraph(paragraph(o)?),
        "Table" => Block::Table(table(o)?),
        other => {
            return Err(PyTypeError::new_err(format!(
                "Unsupported block component {other}"
            )));
        }
    })))
}
fn blocks(o: &Bound<'_, PyAny>) -> PyResult<BlockChildren> {
    list(o, block)
}
fn table(o: &Bound<'_, PyAny>) -> PyResult<Table> {
    Ok(Table {
        rows: Value::ready(list(&o.getattr("rows")?, |r| {
            if !active(r)? {
                Ok(None)
            } else {
                Ok(Some(Value::ready(tablerow(r)?)))
            }
        })?),
        column_widths: field!(o, "column_widths", Option<Vec<u32>>),
        layout: optional_enum(o, "layout", tablelayout)?,
        style: field!(o, "style", Option<String>),
        borders: list(&o.getattr("borders")?, |v| Ok(Some(tableborder(v)?)))?,
        ..Table::default()
    })
}
fn tablerow(o: &Bound<'_, PyAny>) -> PyResult<TableRow> {
    Ok(TableRow::new(list(&o.getattr("children")?, |c| {
        if !active(c)? {
            Ok(None)
        } else {
            Ok(Some(Value::ready(tablecell(c)?)))
        }
    })?))
}
fn tablecell(o: &Bound<'_, PyAny>) -> PyResult<TableCell> {
    Ok(TableCell {
        children: blocks(&o.getattr("children")?)?.into(),
        grid_span: field!(o, "grid_span", Option<u32>),
        vmerge: optional_enum(o, "vmerge", verticalmerge)?,
        borders: list(&o.getattr("borders")?, |v| Ok(Some(cellborder(v)?)))?,
        background_color: field!(o, "background_color", Option<[u8; 3]>),
        vertical_alignment: optional_enum(o, "vertical_alignment", verticalalignment)?,
        ..TableCell::default()
    })
}
fn section(o: &Bound<'_, PyAny>) -> PyResult<Section> {
    let mut section = Section::new(blocks(&o.getattr("children")?)?);
    section.properties = optional_record(o, "properties", sectionproperties)?;
    for name in ["headers", "footers"] {
        let value = o.getattr(name)?;
        if value.is_none() {
            continue;
        }
        for (k, v) in value.cast::<PyDict>()? {
            let key = headerfootertype(&k)?;
            if !active(&v)? {
                continue;
            }
            let children = blocks(&v.getattr("children")?)?;
            if name == "headers" {
                section.headers.insert(key, Header::new(children).into());
            } else {
                section.footers.insert(key, Footer::new(children).into());
            }
        }
    }
    Ok(section)
}
fn document(o: &Bound<'_, PyAny>) -> PyResult<Document> {
    let values = o.getattr("sections")?;
    let mut sections = Vec::new();
    let mut last_active = true;
    for value in values.try_iter()? {
        let value = value?;
        last_active = active(&value)?;
        if last_active {
            sections.push(Value::ready(section(&value)?));
        }
    }
    // Match the pinned Python packer's section count when its last declaration is excluded.
    if !last_active && !sections.is_empty() {
        sections.push(Section::default().into());
    }
    let mut doc = Document::new(sections);

    doc.creator = field!(o, "creator", Option<String>);
    doc.title = field!(o, "title", Option<String>);
    doc.subject = field!(o, "subject", Option<String>);
    doc.description = field!(o, "description", Option<String>);
    doc.keywords = field!(o, "keywords", Option<String>);
    doc.category = field!(o, "category", Option<String>);
    doc.comments = field!(o, "comments", Option<String>);
    doc.version = field!(o, "version", Option<String>);
    doc.comment_author = field!(o, "comment_author", Option<String>);

    doc.styles = list(&o.getattr("styles")?, |v| {
        Ok(Some(match kind(v)?.as_str() {
            "ParagraphStyleDefinition" => paragraphstyledefinition(v)?.into(),
            "TableStyleDefinition" => tablestyledefinition(v)?.into(),
            _ => return Err(PyTypeError::new_err("Unknown style definition")),
        }))
    })?;
    // CMI stores its numbering dictionary without applying it. Built-in list styles
    // are retained in the shared default Word template, exactly as upstream.
    Ok(doc)
}
fn template(o: &Bound<'_, PyAny>) -> PyResult<DocumentTemplate> {
    let mut template = DocumentTemplate::new(field!(o, "path", std::path::PathBuf));
    template.paragraph_index = field!(o, "paragraph_index", Option<usize>);
    let replacements = o.getattr("replacements")?;
    if !replacements.is_none() {
        for (k, v) in replacements.cast::<PyDict>()? {
            template.replacements.push((k.extract()?, v.extract()?));
        }
    }
    Ok(template)
}
fn rust_error(error: Error) -> PyErr {
    if matches!(&error,Error::Validation(message) if message.starts_with("unknown style")) {
        return PyKeyError::new_err(error.to_string());
    }

    fn io(error: &Error) -> Option<std::io::ErrorKind> {
        match error {
            Error::Io(e) => Some(e.kind()),
            Error::At { source, .. } => io(source),
            _ => None,
        }
    }
    match io(&error) {
        Some(std::io::ErrorKind::NotFound) => PyFileNotFoundError::new_err(error.to_string()),
        Some(std::io::ErrorKind::PermissionDenied) => PyPermissionError::new_err(error.to_string()),
        Some(_) => PyOSError::new_err(error.to_string()),
        None => PyValueError::new_err(error.to_string()),
    }
}
#[pyfunction(signature=(declaration, template_value=None))]
fn render(
    py: Python<'_>,
    declaration: &Bound<'_, PyAny>,
    template_value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyBytes>> {
    let doc = document(declaration)?;
    let template = template_value
        .filter(|v| !v.is_none())
        .map(template)
        .transpose()?;
    let bytes = py
        .detach(move || {
            let resolved = futures_executor::block_on(doc.resolve())?;
            match template {
                Some(t) => resolved.to_docx_with_template(&t),
                None => resolved.to_docx(),
            }
        })
        .map_err(rust_error)?;
    Ok(PyBytes::new(py, &bytes).unbind())
}
#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(self::render, module)?)?;
    module.add("BACKEND", "rust")?;
    Ok(())
}
