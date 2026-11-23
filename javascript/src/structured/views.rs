use crate::{
    boundary as b,
    program::{block_handle, inline_handle},
};
use js_sys::Array;
use rocx::{
    CommentMetadata, ContentControlLock, ContentControlProperties, ContentControlText, FieldForm,
    FieldInstruction, FieldProperties, HyperlinkProperties, HyperlinkTarget, NoteTarget, RangeKind,
    ResolvedBlock, ResolvedComment, ResolvedCommentBody, ResolvedDocument, ResolvedInline,
    ResolvedNote, RevisionKind, TextRevision,
};
use wasm_bindgen::JsValue;

fn project<T: Clone>(values: &[T], view: fn(T) -> JsValue) -> JsValue {
    values.iter().cloned().map(view).collect::<Array>().into()
}

fn set_text(object: &JsValue, key: &str, text: Option<&str>) {
    if let Some(text) = text {
        b::set(object, key, &text.into());
    }
}

fn set_bool(object: &JsValue, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        b::set(object, key, &value.into());
    }
}

fn tagged(kind: &str, payload: &[(&str, JsValue)]) -> JsValue {
    let result = b::record(payload);
    b::set(&result, "kind", &kind.into());
    result
}

fn target_view(target: &HyperlinkTarget) -> JsValue {
    if let Some(name) = target.bookmark_name() {
        tagged("bookmark", &[("name", name.as_str().into())])
    } else {
        tagged("external", &[("url", target.as_str().into())])
    }
}

fn link_properties_view(properties: &HyperlinkProperties) -> JsValue {
    let result = b::record(&[]);
    set_text(&result, "tooltip", properties.tooltip.as_deref());
    set_bool(&result, "history", properties.history);
    result
}

fn instruction_view(instruction: &FieldInstruction) -> JsValue {
    match instruction {
        FieldInstruction::Page => tagged("page", &[]),
        FieldInstruction::NumPages => tagged("numPages", &[]),
        FieldInstruction::Reference(name) => tagged("reference", &[("name", name.as_str().into())]),
        FieldInstruction::TableOfContents => tagged("tableOfContents", &[]),
        FieldInstruction::Raw(_) => tagged("raw", &[("text", instruction.text().into())]),
    }
}

fn field_properties_view(properties: &FieldProperties) -> JsValue {
    let form = match properties.form {
        FieldForm::Simple => "simple",
        FieldForm::Complex => "complex",
    };
    let result = b::record(&[("form", form.into())]);
    set_bool(&result, "dirty", properties.dirty);
    set_bool(&result, "locked", properties.locked);
    result
}

fn note_target_view(target: NoteTarget) -> JsValue {
    tagged(target.kind.as_str(), &[("id", target.id.get().into())])
}

pub(crate) fn plain_comment_view(comment: &ResolvedComment) -> JsValue {
    let result = b::record(&[("text", comment.text.as_str().into())]);
    set_text(&result, "author", comment.author.as_deref());
    result
}

fn metadata_view(metadata: &CommentMetadata) -> JsValue {
    let result = b::record(&[]);
    set_text(&result, "author", metadata.author.as_deref());
    set_text(&result, "date", metadata.date.as_deref());
    set_text(&result, "initials", metadata.initials.as_deref());
    result
}

pub(super) fn comment_handle(body: ResolvedCommentBody) -> JsValue {
    let view = b::record(&[
        ("blocks", project(body.blocks(), block_handle)),
        ("metadata", metadata_view(body.metadata())),
    ]);
    b::handle("commentBody", body, view)
}

pub(super) fn note_handle(note: ResolvedNote) -> JsValue {
    let view = b::record(&[
        ("target", note_target_view(note.target())),
        ("blocks", project(note.blocks(), block_handle)),
    ]);
    b::handle("note", note, view)
}

fn range_view(range: &RangeKind) -> JsValue {
    match range {
        RangeKind::Bookmark(name) => tagged("bookmark", &[("name", name.as_str().into())]),
        RangeKind::Comment(comment) => {
            let view = plain_comment_view(comment);
            b::set(&view, "kind", &"comment".into());
            view
        }
        RangeKind::StructuredComment(body) => tagged(
            "structuredComment",
            &[("comment", comment_handle(body.clone()))],
        ),
    }
}

pub(crate) fn content_control_properties_view(properties: &ContentControlProperties) -> JsValue {
    let text = match &properties.text {
        ContentControlText::Rich => tagged("rich", &[]),
        ContentControlText::Plain { multi_line } => {
            let text = tagged("plain", &[]);
            set_bool(&text, "multiLine", *multi_line);
            text
        }
        ContentControlText::DropDown { items, selected } => {
            let items: Array = items
                .iter()
                .map(|item| {
                    b::record(&[
                        ("displayText", item.display_text.as_str().into()),
                        ("value", item.value.as_str().into()),
                    ])
                })
                .collect();
            tagged(
                "dropDown",
                &[
                    ("items", items.into()),
                    ("selected", selected.as_str().into()),
                ],
            )
        }
    };
    let result = b::record(&[("text", text)]);
    set_text(&result, "alias", properties.alias.as_deref());
    set_text(&result, "tag", properties.tag.as_deref());
    set_text(
        &result,
        "lock",
        properties.lock.map(|lock| match lock {
            ContentControlLock::Unlocked => "unlocked",
            ContentControlLock::ControlLocked => "controlLocked",
            ContentControlLock::ContentLocked => "contentLocked",
            ContentControlLock::ControlAndContentLocked => "controlAndContentLocked",
        }),
    );
    if let Some(id) = properties.id {
        b::set(&result, "id", &id.into());
    }
    result
}

pub(crate) fn text_revision_view(revision: &TextRevision) -> JsValue {
    let kind = match &revision.kind {
        RevisionKind::Insertion => tagged("insertion", &[]),
        RevisionKind::Deletion => tagged("deletion", &[]),
        RevisionKind::MoveSource(name) => tagged("moveSource", &[("name", name.as_str().into())]),
        RevisionKind::MoveDestination(name) => {
            tagged("moveDestination", &[("name", name.as_str().into())])
        }
    };
    let result = b::record(&[("kind", kind), ("author", revision.author.as_str().into())]);
    set_text(&result, "date", revision.date.as_deref());
    result
}

pub(super) fn inline_view(inline: &ResolvedInline) -> Option<JsValue> {
    let view = match inline {
        ResolvedInline::Hyperlink(link) => tagged(
            "hyperlink",
            &[
                ("target", target_view(link.target())),
                ("children", project(link.children(), inline_handle)),
                ("properties", link_properties_view(link.properties())),
            ],
        ),
        ResolvedInline::Field(field) => tagged(
            "field",
            &[
                ("instruction", instruction_view(field.instruction())),
                ("instructionText", field.instruction().text().into()),
                ("children", project(field.children(), inline_handle)),
                ("properties", field_properties_view(field.properties())),
            ],
        ),
        ResolvedInline::ContentControl(control) => tagged(
            "contentControl",
            &[
                ("children", project(control.children(), inline_handle)),
                (
                    "properties",
                    content_control_properties_view(control.properties()),
                ),
            ],
        ),
        ResolvedInline::NoteReference(target) => {
            tagged("noteReference", &[("target", note_target_view(*target))])
        }
        ResolvedInline::RangeStart { id, kind } => tagged(
            "rangeStart",
            &[("id", id.as_str().into()), ("range", range_view(kind))],
        ),
        ResolvedInline::RangeEnd(id) => tagged("rangeEnd", &[("id", id.as_str().into())]),
        ResolvedInline::Text { text, .. } => tagged("text", &[("text", text.as_str().into())]),
        _ => return None,
    };
    b::set(&view, "displayText", &inline.text().as_ref().into());
    crate::text_layout::inline_attributes(inline, &view);
    Some(b::handle("inline", inline.clone(), view))
}

pub(super) fn block_view(block: &ResolvedBlock) -> Option<JsValue> {
    let ResolvedBlock::ContentControl(control) = block else {
        return None;
    };
    let view = tagged(
        "contentControl",
        &[
            ("children", project(control.children(), block_handle)),
            (
                "properties",
                content_control_properties_view(control.properties()),
            ),
        ],
    );
    Some(b::handle("blockControl", block.clone(), view))
}

pub(super) fn document_view(document: &ResolvedDocument, view: &JsValue) {
    b::set(view, "notes", &project(document.notes(), note_handle));
}
