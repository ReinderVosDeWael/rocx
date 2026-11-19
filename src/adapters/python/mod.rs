//! Thin PyO3 conversion layer. Python handles asyncio; Rust owns document rendering.
use pyo3::{
    exceptions::{
        PyFileNotFoundError, PyKeyError, PyOSError, PyPermissionError, PyTypeError, PyValueError,
    },
    prelude::*,
    types::{PyAny, PyBytes, PyModule},
};

macro_rules! field {
    ($o:expr,$name:expr,$t:ty) => {
        $o.getattr($name)?.extract::<$t>()?
    };
}
macro_rules! dict_field {
    ($d:expr,$name:expr,$t:ty) => {
        $d.call_method1("get", ($name,))?.extract::<Option<$t>>()?
    };
}
fn items<'py>(o: &Bound<'py, PyAny>) -> PyResult<Vec<(Bound<'py, PyAny>, Bound<'py, PyAny>)>> {
    o.call_method0("items")?
        .try_iter()?
        .map(|item| {
            let item = item?;
            Ok((item.get_item(0)?, item.get_item(1)?))
        })
        .collect()
}
fn kind(o: &Bound<'_, PyAny>) -> PyResult<String> {
    o.getattr("_native_kind")
        .and_then(|v| v.extract())
        .map_err(|_| PyTypeError::new_err("Expected a declarative document component"))
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
mod values;
use values::*;
mod content;
use content::*;
mod tables;
use tables::*;
mod stories;
use stories::*;
mod document;
mod notes;
use document::*;
use notes::*;
mod template;
use template::*;
mod errors;
use errors::*;
#[pyfunction(signature=(declaration, template_value=None, options=None))]
fn render(
    py: Python<'_>,
    declaration: &Bound<'_, PyAny>,
    template_value: Option<&Bound<'_, PyAny>>,
    options: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyBytes>> {
    let doc = document(declaration).map_err(|e| conversion_error(e, "document"))?;
    doc.validate().map_err(|error| rust_error(error.into()))?;
    let options = render_options(options).map_err(|e| conversion_error(e, "options"))?;
    let template = template_value
        .filter(|v| !v.is_none())
        .map(template)
        .transpose()
        .map_err(|e| conversion_error(e, "template"))?;
    let bytes = py
        .detach(move || match template {
            Some(t) => doc.to_docx_with_template_and_options(&t, &options),
            None => doc.to_docx_with_options(&options),
        })
        .map_err(rust_error)?;
    Ok(PyBytes::new(py, &bytes).unbind())
}

fn render_options(
    options: Option<&Bound<'_, PyAny>>,
) -> PyResult<crate::application::RenderOptions> {
    let Some(options) = options.filter(|o| !o.is_none()) else {
        return Ok(Default::default());
    };
    let profile = match field!(options, "profile", String).as_str() {
        "transitional" => crate::OutputProfile::Transitional,
        "strict" => crate::OutputProfile::Strict,
        _ => {
            return Err(rust_error(
                crate::Diagnostic::new(crate::Rule::UnsupportedProfile, "unknown output profile")
                    .at("options.profile")
                    .into(),
            ));
        }
    };
    let limits = options.getattr("read_limits")?;
    Ok(crate::application::RenderOptions {
        profile,
        timestamp: field!(options, "timestamp", Option<String>),
        read_limits: crate::opc::ReadLimits {
            max_parts: field!(&limits, "max_parts", usize),
            max_part_bytes: field!(&limits, "max_part_bytes", u64),
            max_expanded_bytes: field!(&limits, "max_expanded_bytes", u64),
        },
    })
}
#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(self::render, module)?)?;
    module.add("BACKEND", "rust")?;
    Ok(())
}
