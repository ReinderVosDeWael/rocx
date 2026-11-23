use super::{bookmark, comment, located, unknown};
use crate::{
    boundary::{self as b, Failure, Pass, Record},
    host::Wire,
};
use rocx::{
    Comment, CommentMetadata, ContentControlListItem, ContentControlLock, ContentControlProperties,
    ContentControlText, Deferred, FieldForm, FieldInstruction, FieldProperties,
    HyperlinkProperties, HyperlinkTarget, NoteId, NoteKind, NoteTarget, RangeId, RangeKind,
    RangeStart, ResolvedComment, ResolvedCommentBody, Result, RevisionKind, TextRevision,
};

pub(super) fn tag(i: &Wire, path: &str) -> Result<String> {
    b::string(
        &b::member(&i.js(), "kind", path)?,
        &b::child_path(path, "kind"),
    )
}

pub(super) fn expect_kind(r: &Record, expected: &str) -> Result<()> {
    if string(r, "kind")? != expected {
        return Err(Failure::invalid(
            &r.path("kind"),
            format!("expected `{expected}`"),
        ));
    }
    Ok(())
}

pub(super) fn string(r: &Record, key: &str) -> Result<String> {
    b::string(&r.get(key)?, &r.path(key))
}

fn optional<T>(
    r: &Record,
    key: &str,
    decode: impl FnOnce(&Wire, &str) -> Result<T>,
) -> Result<Option<T>> {
    let value = r.get(key)?;
    if value.js().is_undefined() {
        Ok(None)
    } else {
        decode(&value, &r.path(key)).map(Some)
    }
}

fn bool_value(i: &Wire, path: &str) -> Result<bool> {
    i.js()
        .as_bool()
        .ok_or_else(|| Failure::invalid(path, "expected a boolean"))
}

fn u32_value(i: &Wire, path: &str) -> Result<u32> {
    u32::try_from(b::u64_number(i, path)?).map_err(|_| Failure::invalid(path, "outside u32 bounds"))
}

pub(super) fn hyperlink_target(i: &Wire, _: &Pass, path: &str) -> Result<HyperlinkTarget> {
    match tag(i, path)?.as_str() {
        "external" => {
            let r = Record::new(i, path, &["kind", "url"])?;
            located(
                HyperlinkTarget::external(string(&r, "url")?),
                &r.path("url"),
            )
        }
        "bookmark" => {
            let r = Record::new(i, path, &["kind", "name"])?;
            Ok(HyperlinkTarget::bookmark(bookmark(
                &r.get("name")?,
                &r.path("name"),
            )?))
        }
        _ => Err(Failure::invalid(
            &b::child_path(path, "kind"),
            "unknown hyperlink target",
        )),
    }
}

pub(super) fn hyperlink_properties(i: &Wire, _: &Pass, path: &str) -> Result<HyperlinkProperties> {
    let r = Record::new(i, path, &["tooltip", "history"])?;
    Ok(HyperlinkProperties {
        tooltip: optional(&r, "tooltip", b::string)?,
        history: optional(&r, "history", bool_value)?,
    })
}

pub(super) fn instruction(i: &Wire, _: &Pass, path: &str) -> Result<FieldInstruction> {
    let kind = tag(i, path)?;
    Ok(match kind.as_str() {
        "reference" => {
            let r = Record::new(i, path, &["kind", "name"])?;
            FieldInstruction::Reference(bookmark(&r.get("name")?, &r.path("name"))?)
        }
        "raw" => {
            let r = Record::new(i, path, &["kind", "text"])?;
            located(FieldInstruction::raw(string(&r, "text")?), &r.path("text"))?
        }
        kind => {
            let r = Record::new(i, path, &["kind"])?;
            match kind {
                "page" => FieldInstruction::Page,
                "numPages" => FieldInstruction::NumPages,
                "tableOfContents" => FieldInstruction::TableOfContents,
                _ => return Err(unknown(&r, "kind", "field instruction")),
            }
        }
    })
}

pub(super) fn field_properties(i: &Wire, _: &Pass, path: &str) -> Result<FieldProperties> {
    let r = Record::new(i, path, &["form", "dirty", "locked"])?;
    let form = match optional(&r, "form", b::string)?.as_deref() {
        None | Some("simple") => FieldForm::Simple,
        Some("complex") => FieldForm::Complex,
        _ => return Err(unknown(&r, "form", "field form")),
    };
    Ok(FieldProperties {
        form,
        dirty: optional(&r, "dirty", bool_value)?,
        locked: optional(&r, "locked", bool_value)?,
    })
}

pub(super) fn note_target(i: &Wire, _: &Pass, path: &str) -> Result<NoteTarget> {
    let r = Record::new(i, path, &["kind", "id"])?;
    let kind = match string(&r, "kind")?.as_str() {
        "footnote" => NoteKind::Footnote,
        "endnote" => NoteKind::Endnote,
        _ => return Err(unknown(&r, "kind", "note kind")),
    };
    Ok(NoteTarget {
        kind,
        id: NoteId::new(u32_value(&r.get("id")?, &r.path("id"))?),
    })
}

pub(crate) fn comment_metadata(i: &Wire, _: &Pass, path: &str) -> Result<CommentMetadata> {
    let r = Record::new(i, path, &["author", "date", "initials"])?;
    Ok(CommentMetadata {
        author: optional(&r, "author", b::string)?,
        date: optional(&r, "date", b::string)?,
        initials: optional(&r, "initials", b::string)?,
    })
}

pub(crate) fn plain_comment(i: &Wire, path: &str) -> Result<ResolvedComment> {
    let r = Record::new(i, path, &["text", "author"])?;
    comment_fields(&r)
}

fn comment_fields(r: &Record) -> Result<ResolvedComment> {
    Ok(ResolvedComment {
        text: string(r, "text")?,
        author: optional(r, "author", b::string)?,
    })
}

pub(super) fn range_id(i: &Wire, _: &Pass, path: &str) -> Result<RangeId> {
    located(RangeId::new(b::string(i, path)?), path)
}

fn range_value(i: &Wire, path: &str) -> Result<RangeKind> {
    match tag(i, path)?.as_str() {
        "bookmark" => {
            let r = Record::new(i, path, &["kind", "name"])?;
            bookmark(&r.get("name")?, &r.path("name")).map(RangeKind::Bookmark)
        }
        "comment" => {
            let r = Record::new(i, path, &["kind", "text", "author"])?;
            comment_fields(&r).map(RangeKind::Comment)
        }
        "structuredComment" => {
            let r = Record::new(i, path, &["kind", "comment"])?;
            let body = b::borrow::<ResolvedCommentBody>(&r.get("comment")?, &r.path("comment"))?;
            Ok(RangeKind::StructuredComment((*body).clone()))
        }
        _ => Err(Failure::invalid(
            &b::child_path(path, "kind"),
            "unknown range kind",
        )),
    }
}

pub(super) fn resolved_range(i: &Wire, path: &str) -> Result<RangeKind> {
    range_value(i, path)
}

fn range_declaration(i: &Wire, pass: &Pass, path: &str) -> Result<Option<Comment>> {
    if tag(i, path)? != "structuredComment" {
        return Ok(None);
    }
    let r = Record::new(i, path, &["kind", "comment"])?;
    let input = r.get("comment")?;
    let path = r.path("comment");
    if b::borrow::<ResolvedCommentBody>(&input, &path).is_ok() {
        Ok(None)
    } else {
        comment(&input, pass, &path).map(Some)
    }
}

pub(super) async fn range(
    i: &Wire,
    pass: &Pass,
    path: &str,
    id: Deferred<RangeId>,
) -> Result<RangeKind> {
    if let Some(comment) = range_declaration(i, pass, path)? {
        // The outer marker resolves the id and guard. Use the native
        // structured comment field without resolving the id a second time.
        RangeStart::structured_comment(id, comment)
            .kind
            .resolve()
            .await
    } else {
        range_value(i, path)
    }
}

fn control_text(i: &Wire, path: &str) -> Result<ContentControlText> {
    Ok(match tag(i, path)?.as_str() {
        "rich" => {
            Record::new(i, path, &["kind"])?;
            ContentControlText::Rich
        }
        "plain" => {
            let r = Record::new(i, path, &["kind", "multiLine"])?;
            ContentControlText::Plain {
                multi_line: optional(&r, "multiLine", bool_value)?,
            }
        }
        "dropDown" => {
            let r = Record::new(i, path, &["kind", "items", "selected"])?;
            let items = b::list(&r.get("items")?, &r.path("items"))?
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let r = Record::new(
                        item,
                        &format!("{}[{index}]", r.path("items")),
                        &["displayText", "value"],
                    )?;
                    Ok(ContentControlListItem {
                        display_text: string(&r, "displayText")?,
                        value: string(&r, "value")?,
                    })
                })
                .collect::<Result<_>>()?;
            ContentControlText::DropDown {
                items,
                selected: string(&r, "selected")?,
            }
        }
        _ => {
            return Err(Failure::invalid(
                &b::child_path(path, "kind"),
                "unknown content control text kind",
            ));
        }
    })
}

pub(crate) fn content_control_properties(
    i: &Wire,
    _: &Pass,
    path: &str,
) -> Result<ContentControlProperties> {
    let r = Record::new(i, path, &["text", "alias", "tag", "lock", "id"])?;
    let lock = optional(&r, "lock", |i, path| {
        Ok(match b::string(i, path)?.as_str() {
            "unlocked" => ContentControlLock::Unlocked,
            "controlLocked" => ContentControlLock::ControlLocked,
            "contentLocked" => ContentControlLock::ContentLocked,
            "controlAndContentLocked" => ContentControlLock::ControlAndContentLocked,
            _ => return Err(Failure::invalid(path, "unknown content control lock")),
        })
    })?;
    Ok(ContentControlProperties {
        text: optional(&r, "text", control_text)?.unwrap_or_default(),
        alias: optional(&r, "alias", b::string)?,
        tag: optional(&r, "tag", b::string)?,
        lock,
        id: optional(&r, "id", u32_value)?,
    })
}

fn revision_kind(i: &Wire, path: &str) -> Result<RevisionKind> {
    let kind = tag(i, path)?;
    Ok(match kind.as_str() {
        "insertion" | "deletion" => {
            Record::new(i, path, &["kind"])?;
            if kind == "insertion" {
                RevisionKind::Insertion
            } else {
                RevisionKind::Deletion
            }
        }
        "moveSource" | "moveDestination" => {
            let r = Record::new(i, path, &["kind", "name"])?;
            let name = bookmark(&r.get("name")?, &r.path("name"))?;
            if kind == "moveSource" {
                RevisionKind::MoveSource(name)
            } else {
                RevisionKind::MoveDestination(name)
            }
        }
        _ => {
            return Err(Failure::invalid(
                &b::child_path(path, "kind"),
                "unknown revision kind",
            ));
        }
    })
}

pub(crate) fn text_revision(i: &Wire, _: &Pass, path: &str) -> Result<TextRevision> {
    let r = Record::new(i, path, &["kind", "author", "date"])?;
    Ok(TextRevision {
        kind: revision_kind(&r.get("kind")?, &r.path("kind"))?,
        author: string(&r, "author")?,
        date: optional(&r, "date", b::string)?,
    })
}
