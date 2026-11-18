//! Native conversion into resolved domain values.
use super::*;
pub(super) fn rust_error(error: Error) -> PyErr {
    if error.rule() == Some(Rule::UnknownStyle) {
        return PyKeyError::new_err(error.to_string());
    }

    fn io(error: &Error) -> Option<std::io::ErrorKind> {
        match error {
            Error::Io(e) => Some(e.kind()),
            Error::At { source, .. } => io(source),
            Error::Shared(source) => io(source),
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

use crate::{Error, Rule};
