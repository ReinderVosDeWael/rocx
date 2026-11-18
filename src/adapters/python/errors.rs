//! Native conversion into resolved domain values.
use super::*;
pub(super) fn rust_error(error: Error) -> PyErr {
    fn io(error: &Error) -> Option<std::io::ErrorKind> {
        match error {
            Error::Io(e) => Some(e.kind()),
            Error::At { source, .. } => io(source),
            Error::Shared(source) => io(source),
            _ => None,
        }
    }
    let exception = if error.rule() == Some(Rule::UnknownStyle) {
        PyKeyError::new_err(error.to_string())
    } else {
        match io(&error) {
            Some(std::io::ErrorKind::NotFound) => PyFileNotFoundError::new_err(error.to_string()),
            Some(std::io::ErrorKind::PermissionDenied) => {
                PyPermissionError::new_err(error.to_string())
            }
            Some(_) => PyOSError::new_err(error.to_string()),
            None => PyValueError::new_err(error.to_string()),
        }
    };
    if let Err(attachment) = Python::attach(|py| {
        use std::error::Error as _;
        let value = exception.value(py);
        value.setattr("rule", error.rule().map(Rule::code))?;
        value.setattr("location", error.path())?;
        let cause = error.source().map(ToString::to_string);
        value.setattr("cause", &cause)?;
        if let Some(cause) = cause {
            exception.set_cause(py, Some(pyo3::exceptions::PyRuntimeError::new_err(cause)));
        }
        Ok::<_, PyErr>(())
    }) {
        return attachment;
    }
    exception
}

use crate::{Error, Rule};

pub(super) fn conversion_error(error: PyErr, location: &str) -> PyErr {
    let attached = Python::attach(|py| -> PyResult<()> {
        let value = error.value(py);
        if !value.hasattr("rule")? {
            value.setattr("rule", Rule::InvalidContent.code())?;
        }
        if !value.hasattr("location")? {
            value.setattr("location", location)?;
        }
        if !value.hasattr("cause")? {
            value.setattr("cause", error.cause(py).map(|cause| cause.to_string()))?;
        }
        Ok(())
    });
    match attached {
        Ok(()) => error,
        Err(attachment) => attachment,
    }
}
