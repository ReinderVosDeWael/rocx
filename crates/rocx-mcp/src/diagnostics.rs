use rocx::{Diagnostic, Error, Rule};
use serde_json::{Value, json};

/// A tool-level failure with optional native context, independent of its user-facing message.
#[derive(Debug)]
pub(crate) struct ToolError {
    pub category: String,
    pub message: String,
    native: Option<Error>,
}

impl ToolError {
    fn new(category: &str, message: String) -> Self {
        Self {
            category: category.into(),
            message,
            native: None,
        }
    }

    pub fn native(category: &str, error: Error, message: impl FnOnce(&Error) -> String) -> Self {
        Self {
            category: category.into(),
            message: message(&error),
            native: Some(error),
        }
    }

    pub fn response(&self, action: &str) -> Value {
        let mut response = json!({
            "category": self.category,
            "message": self.message,
            "action": action,
        });
        if let Some(error) = &self.native {
            if let Some(rule) = error.rule() {
                response["rule"] = json!(Rule::code(rule));
            }
            if let Some(path) = error.path() {
                response["path"] = json!(path);
            }
            if let Some(detail) = native_detail(error) {
                response["detail"] = json!(detail);
            }
            if let Some((character, byte_offset)) = xml_character(error) {
                response["character"] = json!(character);
                response["byte_offset"] = json!(byte_offset);
            }
        }
        response
    }
}

impl From<String> for ToolError {
    fn from(legacy: String) -> Self {
        // Only the existing MCP category/message convention is split here.
        match legacy.split_once(": ") {
            Some((category, message)) => Self::new(category, message.into()),
            None => Self::new("session_error", legacy),
        }
    }
}

fn native_detail(error: &Error) -> Option<&str> {
    match error {
        Error::Diagnostic(diagnostic) => Some(&diagnostic.message),
        Error::Validation(message) => Some(message),
        Error::At { source, .. } => native_detail(source),
        Error::Shared(source) => native_detail(source),
        Error::Producer(source) => source
            .downcast_ref::<Error>()
            .and_then(native_detail)
            .or_else(|| {
                source
                    .downcast_ref::<Diagnostic>()
                    .map(|diagnostic| diagnostic.message.as_str())
            }),
        _ => None,
    }
}

fn xml_character(error: &Error) -> Option<(char, usize)> {
    match error {
        Error::InvalidXmlCharacter {
            character,
            byte_offset,
        } => Some((*character, *byte_offset)),
        Error::At { source, .. } => xml_character(source),
        Error::Shared(source) => xml_character(source),
        Error::Producer(source) => source.downcast_ref::<Error>().and_then(xml_character),
        _ => None,
    }
}

/// A staged operation can reject a target or payload before calling the native editor.
#[derive(Debug)]
pub(crate) enum ScopedEditError {
    WrongAnchorType(String),
    Rejected(String),
    Native(Error),
}

impl From<Error> for ScopedEditError {
    fn from(error: Error) -> Self {
        Self::Native(error)
    }
}

impl From<&str> for ScopedEditError {
    fn from(message: &str) -> Self {
        Self::Rejected(message.into())
    }
}

impl From<String> for ScopedEditError {
    fn from(message: String) -> Self {
        Self::Rejected(message)
    }
}

impl From<ScopedEditError> for ToolError {
    fn from(error: ScopedEditError) -> Self {
        Self::scoped(error, "revise the operation and retry")
    }
}

impl ToolError {
    pub fn scoped(error: ScopedEditError, hint: &str) -> Self {
        match error {
            ScopedEditError::WrongAnchorType(message) => Self::new("wrong_anchor_type", message),
            ScopedEditError::Rejected(message) => {
                Self::new("rejected_batch", format!("{message}; {hint}"))
            }
            ScopedEditError::Native(error) => {
                Self::native("rejected_batch", error, |error| format!("{error}; {hint}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn wrapped_native_error_keeps_path_and_unformatted_detail() {
        let error = Error::Shared(Arc::new(Error::At {
            path: "edit.text".into(),
            source: Box::new(Error::Validation("invalid: empty".into())),
        }));
        let error = ToolError::native("rejected_batch", error, ToString::to_string);
        assert_eq!(
            error.response("Retry."),
            json!({
                "category": "rejected_batch",
                "message": "edit.text: invalid: empty",
                "action": "Retry.",
                "rule": "invalid_content",
                "path": "edit.text",
                "detail": "invalid: empty",
            })
        );
    }

    #[test]
    fn native_xml_error_preserves_character_and_byte_offset() {
        let error = Error::InvalidXmlCharacter {
            character: '\0',
            byte_offset: 0,
        };
        let error = ToolError::native("rejected_batch", error, ToString::to_string);
        assert_eq!(
            error.response("Retry."),
            json!({
                "category": "rejected_batch",
                "message": "invalid XML 1.0 character U+0000 at byte offset 0",
                "action": "Retry.",
                "rule": "invalid_xml",
                "character": "\0",
                "byte_offset": 0,
            })
        );
    }
}
