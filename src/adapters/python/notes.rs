//! Direct conversion of resolved note/comment stories, without a second resolver.
use super::*;
pub(super) fn note_target(o: &Bound<'_, PyAny>) -> PyResult<crate::NoteTarget> {
    let kind = match field!(o, "kind", String).as_str() {
        "footnote" => crate::NoteKind::Footnote,
        "endnote" => crate::NoteKind::Endnote,
        _ => {
            return Err(rust_error(
                crate::Diagnostic::new(crate::Rule::InvalidContent, "unknown note kind")
                    .at("kind")
                    .into(),
            ));
        }
    };
    let id = o
        .getattr("id")?
        .extract::<u32>()
        .map_err(|e| conversion_error(e, "id"))?;
    Ok(crate::NoteTarget {
        kind,
        id: crate::NoteId::new(id),
    })
}
pub(super) fn note(o: &Bound<'_, PyAny>) -> PyResult<Option<crate::ResolvedNote>> {
    if kind(o)? != "Note" {
        return Err(PyTypeError::new_err("expected a Note definition"));
    }
    Ok(Some(crate::ResolvedNote {
        target: note_target(o)?,
        blocks: blocks(&o.getattr("children")?)?,
    }))
}
pub(super) fn structured_comment(o: &Bound<'_, PyAny>) -> PyResult<crate::ResolvedCommentBody> {
    if kind(o)? != "Comment" {
        return Err(PyTypeError::new_err("expected structured Comment content"));
    }
    Ok(crate::ResolvedCommentBody {
        blocks: blocks(&o.getattr("children")?)?,
        metadata: crate::CommentMetadata {
            author: field!(o, "author", Option<String>),
            date: field!(o, "date", Option<String>),
            initials: field!(o, "initials", Option<String>),
        },
    })
}
