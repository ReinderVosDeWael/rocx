mod entries;

use rmcp::{
    handler::server::common::schema_for_input,
    model::{JsonObject, Tool},
    schemars::{self, JsonSchema, Schema},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::session::Session;
use entries::{Operation, View};

#[derive(Clone, Copy)]
pub(crate) enum ToolKind {
    Create,
    Open,
    Inspect,
    Preview,
    Save,
    Close,
}

impl ToolKind {
    pub(crate) const ALL: [Self; 6] = [
        Self::Create,
        Self::Open,
        Self::Inspect,
        Self::Preview,
        Self::Save,
        Self::Close,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Create => "create_document",
            Self::Open => "open_document",
            Self::Inspect => "inspect_document",
            Self::Preview => "preview_edits",
            Self::Save => "save_copy",
            Self::Close => "close_document",
        }
    }

    pub(crate) fn definition(self) -> Tool {
        let (description, schema) = match self {
            Self::Create => (
                "Create an in-memory DOCX from plain paragraphs. Omit paragraphs or pass [] \
                 to start with one empty paragraph. Returns a document handle, source SHA-256, \
                 and an initial preview handle. No file is written. Example result: \
                 {\"document\":\"<document>\",\"snapshot\":\"<sha256>\",\
                 \"preview\":\"<preview>\",\"output_snapshot\":\"<sha256>\"}.",
                schema_for_input::<Create>(),
            ),
            Self::Open => (
                "Open a DOCX on the server host. Relative paths use the server launch directory. \
                 Returns a fresh document handle and the source SHA-256. The source is never modified. \
                 Example result: {\"document\":\"<document>\",\"snapshot\":\"<sha256>\"}.",
                schema_for_input::<Open>(),
            ),
            Self::Inspect => (
                "Inspect body paragraphs and runs, physical table cells, and default and first-page \
                  header slots. Returns reusable opaque anchors for candidate edits to this snapshot. \
                 A paragraph's scalar_length describes Unicode scalar coordinates for its plain-text \
                  projection, with null meaning none are advertised. It is not edit permission: \
                  preview_edits validates the actual target and batch. Omit view for this compact \
                  result. Use view {\"kind\":\"entries\"} for ordinary footnote, endnote, and comment \
                  definitions with entry anchors. The page limit defaults to 25 (range 1–100); \
                  pass the returned next_cursor as view.cursor for the same document, with the same \
                  or a different limit. Omit the cursor to restart. Example compact result excerpt: \
                  {\"document\":\"<document>\",\"body\":[\
                  {\"type\":\"paragraph\",\"anchor\":\"<paragraph anchor>\",\"text\":\"Example\",\
                  \"scalar_length\":7}],\"headers\":[]}.",
                schema_for_input::<Inspect>(),
            ),
            Self::Preview => (
                "Validate and encode edits against the original snapshot. Pass exactly one of \
                 edits (an ordered ordinary batch) or operation (a single scoped edit). Use anchors \
                 from inspect_document; edit-entry requires an entry definition anchor from the \
                 entries view. Returns a fresh preview handle and output SHA-256 without writing \
                 a file or invalidating earlier previews. Ordinary batches return a change report; \
                 edit-entry returns changes: null. Example scoped operation: {\"kind\":\"edit-entry\",\
                 \"anchor\":\"<entry anchor>\",\"edit\":{\"kind\":\"comment-initials\",\"value\":null}}.",
                schema_for_input::<Preview>(),
            ),
            Self::Save => (
                "Save the retained bytes of a selected preview to a new path on the server host. \
                 Relative paths use the server launch directory. Does not overwrite files or replay edits. \
                 Returns the output path and selected preview SHA-256. Example result: \
                 {\"output\":\"copy.docx\",\"output_snapshot\":\"<sha256>\"}.",
                schema_for_input::<Save>(),
            ),
            Self::Close => (
                "Close the document and release its snapshot, anchors, and previews. \
                 Example acknowledgment: {\"closed\":\"<document>\"}.",
                schema_for_input::<Document>(),
            ),
        };
        Tool::new(
            self.name(),
            description,
            schema.expect("tool arguments are object schemas"),
        )
    }

    pub(crate) fn parse(self, arguments: JsonObject) -> Result<Call, String> {
        let arguments = Value::Object(arguments);
        match self {
            Self::Create => parse(arguments).map(Call::Create),
            Self::Open => parse(arguments).map(Call::Open),
            Self::Inspect => parse(arguments).map(Call::Inspect),
            Self::Preview => {
                if arguments.get("operation").is_some() {
                    if arguments.get("edits").is_some() {
                        return Err("pass exactly one of edits or operation".into());
                    }
                    return parse(arguments).map(Preview::Scoped).map(Call::Preview);
                }
                let input: OrdinaryPreview = parse(arguments)?;
                if input.edits.is_empty() {
                    return Err("edits must contain at least one edit".into());
                }
                for (index, edit) in input.edits.iter().enumerate() {
                    if let Edit::InsertTable { rows, .. } = edit
                        && !rows.first().is_some_and(|first| {
                            !first.is_empty() && rows.iter().all(|row| row.len() == first.len())
                        })
                    {
                        return Err(format!(
                            "edits[{index}].rows must be nonempty and rectangular"
                        ));
                    }
                }
                Ok(Call::Preview(Preview::Ordinary(input)))
            }
            Self::Save => parse(arguments).map(Call::Save),
            Self::Close => parse(arguments).map(Call::Close),
        }
    }
}

fn parse<T: DeserializeOwned>(arguments: Value) -> Result<T, String> {
    serde_json::from_value(arguments).map_err(|error| error.to_string())
}

pub(crate) enum Call {
    Create(Create),
    Open(Open),
    Inspect(Inspect),
    Preview(Preview),
    Save(Save),
    Close(Document),
}

impl Call {
    pub(crate) fn dispatch(self, session: &mut Session) -> Result<Value, String> {
        match self {
            Self::Create(input) => session.create_document(input.paragraphs),
            Self::Open(input) => session.open_document(input.path),
            Self::Inspect(input) => match input.view {
                None => session.inspect_document(input.document),
                Some(View::Entries { limit, cursor }) => {
                    session.inspect_entries(input.document, limit, cursor)
                }
            },
            Self::Preview(Preview::Ordinary(input)) => {
                session.preview_edits(input.document, input.edits)
            }
            Self::Preview(Preview::Scoped(input)) => match input.operation {
                Operation::EditEntry { anchor, edit } => {
                    session.preview_entry(input.document, anchor, edit.into())
                }
            },
            Self::Save(input) => session.save_copy(input.document, input.preview, input.output),
            Self::Close(input) => session.close_document(input.document),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Create {
    /// Plain text for each paragraph. Defaults to one empty paragraph.
    #[serde(default)]
    paragraphs: Vec<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Open {
    /// Source DOCX path on the server host.
    path: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Document {
    /// An opaque live document handle from create_document or open_document.
    document: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Inspect {
    /// An opaque live document handle from create_document or open_document.
    document: String,
    /// Omit for compact body and header inspection.
    #[serde(default, deserialize_with = "entries::present")]
    #[schemars(with = "View", skip_serializing_if = "Option::is_none")]
    view: Option<View>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Save {
    /// The live owner of the preview.
    document: String,
    /// An opaque preview handle from create_document or preview_edits.
    preview: String,
    /// Path of the new DOCX copy. Must not exist.
    output: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(inline)]
pub(crate) struct OrdinaryPreview {
    /// The live document whose anchors the edits use.
    document: String,
    /// Ordered edits against the original snapshot.
    #[schemars(length(min = 1))]
    edits: Vec<Edit>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(inline)]
pub(crate) struct ScopedPreview {
    /// The live document whose anchors the edits use.
    document: String,
    /// A single operation that is not part of an ordinary batch.
    operation: Operation,
}

#[derive(JsonSchema)]
#[schemars(untagged, transform = preview_input_schema)]
pub(crate) enum Preview {
    Ordinary(OrdinaryPreview),
    Scoped(ScopedPreview),
}

// Keep the strict MCP root discoverable as an object. Its only properties are
// document and the two mode selectors; exactly two keys requires one mode.
fn preview_input_schema(schema: &mut Schema) {
    let branches = schema.remove("anyOf").expect("untagged input union");
    let mut properties = JsonObject::new();
    for branch in branches.as_array().expect("input union branches") {
        properties.extend(
            branch["properties"]
                .as_object()
                .expect("object branch properties")
                .clone(),
        );
    }
    schema.insert("type".into(), json!("object"));
    schema.insert("additionalProperties".into(), json!(false));
    schema.insert("required".into(), json!(["document"]));
    schema.insert("properties".into(), Value::Object(properties));
    schema.insert("minProperties".into(), json!(2));
    schema.insert("maxProperties".into(), json!(2));
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(inline)]
pub(crate) enum Edit {
    /// Set direct alignment of a body paragraph.
    Alignment {
        /// Paragraph anchor.
        anchor: String,
        alignment: ParagraphAlignment,
    },
    /// Insert a plain paragraph immediately after the anchor.
    InsertParagraph {
        /// Paragraph anchor.
        anchor: String,
        text: String,
    },
    /// Insert a table immediately after the anchor.
    InsertTable {
        /// Paragraph anchor.
        anchor: String,
        /// Nonempty rectangular matrix with one plain-text paragraph per cell.
        #[schemars(length(min = 1), inner(length(min = 1)))]
        rows: Vec<Vec<String>>,
    },
    /// Replace the text of a supported direct run.
    RunText {
        /// Run anchor.
        anchor: String,
        /// Replacement text with no tabs or line breaks.
        text: String,
    },
    /// Remove a supported direct run.
    RunRemove {
        /// Run anchor.
        anchor: String,
    },
    /// Replace or delete a nonempty range in a plain direct body paragraph.
    TextRange {
        /// Paragraph anchor.
        anchor: String,
        /// Inclusive original Unicode scalar offset.
        start: usize,
        /// Exclusive original Unicode scalar offset; must exceed start.
        end: usize,
        /// Replacement text. An empty string deletes the range.
        text: String,
    },
    /// Insert nonempty text at an original Unicode scalar boundary in a plain direct body paragraph.
    InsertText {
        /// Paragraph anchor.
        anchor: String,
        /// Original Unicode scalar offset, including paragraph end.
        offset: usize,
        /// Inserted text with no tabs or line breaks.
        text: String,
    },
    /// Set vertical alignment of a physical table cell.
    CellAlignment {
        /// Cell anchor.
        anchor: String,
        alignment: CellAlignment,
    },
    /// Append an inline image to a header slot.
    HeaderImage {
        /// Header slot anchor.
        anchor: String,
        mode: HeaderMode,
        /// Complete encoded image as byte values.
        bytes: Vec<u8>,
    },
}

impl Edit {
    pub(crate) fn anchor_slot(&mut self) -> &mut String {
        match self {
            Self::Alignment { anchor, .. }
            | Self::InsertParagraph { anchor, .. }
            | Self::InsertTable { anchor, .. }
            | Self::RunText { anchor, .. }
            | Self::RunRemove { anchor, .. }
            | Self::TextRange { anchor, .. }
            | Self::InsertText { anchor, .. }
            | Self::CellAlignment { anchor, .. }
            | Self::HeaderImage { anchor, .. } => anchor,
        }
    }

    pub(crate) fn target_kind(&self) -> &'static str {
        match self {
            Self::Alignment { .. }
            | Self::InsertParagraph { .. }
            | Self::InsertTable { .. }
            | Self::TextRange { .. }
            | Self::InsertText { .. } => "paragraph",
            Self::RunText { .. } | Self::RunRemove { .. } => "run",
            Self::CellAlignment { .. } => "cell",
            Self::HeaderImage { .. } => "header",
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(inline)]
pub(crate) enum ParagraphAlignment {
    Left,
    Center,
    Right,
    Justify,
    Distribute,
    JustifyMedium,
    JustifyHigh,
    JustifyLow,
    ThaiJustify,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(inline)]
pub(crate) enum CellAlignment {
    Top,
    Center,
    Bottom,
    Both,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(inline)]
pub(crate) enum HeaderMode {
    Shared,
    SectionLocal,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn request(tool: ToolKind, input: Value) -> Result<Call, String> {
        tool.parse(input.as_object().unwrap().clone())
    }

    fn preview(edits: Value) -> Result<Call, String> {
        request(
            ToolKind::Preview,
            json!({"document": "doc", "edits": edits}),
        )
    }

    #[test]
    fn typed_forms_advertise_strict_selectors_and_exclusive_preview_shapes() {
        let inspect = ToolKind::Inspect.definition().input_schema;
        assert_eq!(inspect["required"], json!(["document"]));
        let view = &inspect["properties"]["view"]["oneOf"][0];
        assert_eq!(view["additionalProperties"], false, "{view}");
        assert_eq!(view["required"], json!(["kind"]));
        assert_eq!(view["properties"]["kind"]["const"], "entries");
        let limit = &view["properties"]["limit"];
        assert_eq!(limit["minimum"], 1);
        assert_eq!(limit["maximum"], 100);
        assert_eq!(limit["default"], 25);
        assert_eq!(view["properties"]["cursor"]["type"], "string");

        let preview = ToolKind::Preview.definition().input_schema;
        assert_eq!(preview["required"], json!(["document"]));
        assert_eq!(preview["minProperties"], 2);
        assert_eq!(preview["maxProperties"], 2);
        let properties = preview["properties"].as_object().unwrap();
        assert_eq!(properties.len(), 3);
        assert!(
            ["document", "edits", "operation"]
                .iter()
                .all(|field| properties.contains_key(*field))
        );

        let operation = &preview["properties"]["operation"]["oneOf"][0];
        assert_eq!(operation["additionalProperties"], false, "{operation}");
        assert_eq!(operation["required"], json!(["kind", "anchor", "edit"]));
        assert_eq!(operation["properties"]["kind"]["const"], "edit-entry");
        let variants = operation["properties"]["edit"]["oneOf"].as_array().unwrap();
        let required = [
            ("delete-unused-note", &[][..]),
            ("insert-paragraph", &["index", "text"][..]),
            ("replace-paragraph", &["index", "text"][..]),
            ("remove-paragraph", &["index"][..]),
            ("comment-author", &["value"][..]),
            ("comment-initials", &["value"][..]),
            ("comment-date", &["value"][..]),
        ];
        assert_eq!(variants.len(), required.len());
        for (kind, fields) in required {
            let variant = variants
                .iter()
                .find(|variant| variant["properties"]["kind"]["const"] == kind)
                .unwrap();
            assert_eq!(variant["additionalProperties"], false);
            let required = variant["required"].as_array().unwrap();
            assert_eq!(required.len(), fields.len() + 1);
            for field in ["kind"].into_iter().chain(fields.iter().copied()) {
                assert!(required.contains(&json!(field)), "{kind} requires {field}");
            }
            if fields.contains(&"index") {
                assert_eq!(variant["properties"]["index"]["type"], "integer");
                assert_eq!(variant["properties"]["index"]["minimum"], 0);
            }
            match kind {
                "comment-author" => assert_eq!(variant["properties"]["value"]["type"], "string"),
                "comment-initials" | "comment-date" => {
                    assert_eq!(
                        variant["properties"]["value"]["type"],
                        json!(["string", "null"])
                    );
                }
                _ => {}
            }
        }
    }

    #[test]
    fn inspection_omits_view_or_selects_a_strict_bounded_page() {
        let valid = [
            (json!({"document":"doc"}), None),
            (
                json!({"document":"doc","view":{"kind":"entries"}}),
                Some((25, None)),
            ),
            (
                json!({"document":"doc","view":{"kind":"entries","limit":1}}),
                Some((1, None)),
            ),
            (
                json!({"document":"doc","view":{"kind":"entries","limit":100,"cursor":"cursor"}}),
                Some((100, Some("cursor"))),
            ),
        ];
        for (input, expected) in valid {
            let Ok(Call::Inspect(Inspect { document, view })) = request(ToolKind::Inspect, input)
            else {
                panic!("valid inspection was rejected");
            };
            assert_eq!(document, "doc");
            assert_eq!(
                view.as_ref()
                    .map(|View::Entries { limit, cursor }| (*limit, cursor.as_deref())),
                expected
            );
        }
        for view in [
            json!(null),
            json!({}),
            json!({"kind":"unsupported"}),
            json!({"kind":"entries","limit":0}),
            json!({"kind":"entries","limit":101}),
            json!({"kind":"entries","limit":1.5}),
            json!({"kind":"entries","limit":null}),
            json!({"kind":"entries","cursor":null}),
            json!({"kind":"entries","cursor":1}),
            json!({"kind":"entries","unknown":true}),
        ] {
            let input = json!({"document":"doc","view":view});
            assert!(
                request(ToolKind::Inspect, input.clone()).is_err(),
                "accepted {input}"
            );
        }
        assert!(request(ToolKind::Inspect, json!({"document":"doc","limit":25})).is_err());
    }

    fn scoped(edit: Value) -> Value {
        json!({"document":"doc","operation":{"kind":"edit-entry","anchor":"entry","edit":edit}})
    }

    #[test]
    fn scoped_wire_edits_convert_to_native_payloads() {
        use rocx::editor::EntryEdit;

        for (input, matches) in [
            (
                json!({"kind":"delete-unused-note"}),
                (|edit| matches!(edit, EntryEdit::DeleteUnusedNote)) as fn(EntryEdit) -> bool,
            ),
            (
                json!({"kind":"insert-paragraph","index":0,"text":"insert"}),
                |edit| matches!(edit, EntryEdit::InsertParagraph { index: 0, text } if text == "insert"),
            ),
            (
                json!({"kind":"replace-paragraph","index":1,"text":"replace"}),
                |edit| matches!(edit, EntryEdit::ReplaceParagraph { index: 1, text } if text == "replace"),
            ),
            (json!({"kind":"remove-paragraph","index":2}), |edit| {
                matches!(edit, EntryEdit::RemoveParagraph(2))
            }),
            (
                json!({"kind":"comment-author","value":"Author"}),
                |edit| matches!(edit, EntryEdit::CommentAuthor(value) if value == "Author"),
            ),
            (
                json!({"kind":"comment-initials","value":""}),
                |edit| matches!(edit, EntryEdit::CommentInitials(Some(value)) if value.is_empty()),
            ),
            (json!({"kind":"comment-initials","value":null}), |edit| {
                matches!(edit, EntryEdit::CommentInitials(None))
            }),
            (
                json!({"kind":"comment-date","value":"2025-01-01T00:00:00Z"}),
                |edit| matches!(edit, EntryEdit::CommentDate(Some(value)) if value == "2025-01-01T00:00:00Z"),
            ),
            (json!({"kind":"comment-date","value":null}), |edit| {
                matches!(edit, EntryEdit::CommentDate(None))
            }),
        ] {
            let Ok(Call::Preview(Preview::Scoped(ScopedPreview {
                document,
                operation: Operation::EditEntry { anchor, edit },
            }))) = request(ToolKind::Preview, scoped(input.clone()))
            else {
                panic!("rejected {input}");
            };
            assert_eq!(document, "doc");
            assert_eq!(anchor, "entry");
            assert!(matches(edit.into()), "incorrect conversion for {input}");
        }
    }

    #[test]
    fn scoped_requests_reject_mixed_forms_and_invalid_fields() {
        let edit = json!({"kind":"delete-unused-note"});
        let operation = scoped(edit.clone())["operation"].clone();
        for input in [
            json!({"document":"doc"}),
            json!({"document":"doc","operation":null}),
            json!({"document":"doc","operation":operation,"edits":null}),
            json!({"document":"doc","operation":null,"edits":[{"kind":"run-remove","anchor":"r"}]}),
            json!({"document":"doc","operation":operation,"extra":false}),
            json!({"document":"doc","operation":{"kind":"edit-entry","anchor":"e","edit":edit,"extra":null}}),
            json!({"document":"doc","operation":{"kind":"edit-entry","edit":edit}}),
            json!({"document":"doc","operation":{"kind":"edit-entry","anchor":"e"}}),
            json!({"document":"doc","operation":{"kind":"unsupported","anchor":"e","edit":edit}}),
            scoped(json!({"kind":"delete-unused-note","index":0})),
            scoped(json!({"kind":"insert-paragraph","index":-1,"text":"text"})),
            scoped(json!({"kind":"replace-paragraph","index":0})),
            scoped(json!({"kind":"remove-paragraph","index":0,"text":"extra"})),
            scoped(json!({"kind":"comment-author","value":null})),
            scoped(json!({"kind":"comment-initials"})),
            scoped(json!({"kind":"comment-date"})),
            scoped(json!({"kind":"comment-date","value":1})),
        ] {
            assert!(
                request(ToolKind::Preview, input.clone()).is_err(),
                "accepted {input}"
            );
        }
    }

    #[test]
    fn discovery_has_six_strict_tools_and_nine_discriminated_edits() {
        let tools = ToolKind::ALL.map(ToolKind::definition);
        assert_eq!(
            tools.each_ref().map(|tool| tool.name.as_ref()),
            [
                "create_document",
                "open_document",
                "inspect_document",
                "preview_edits",
                "save_copy",
                "close_document"
            ]
        );
        assert!(tools.iter().all(|tool| {
            tool.input_schema["type"] == "object"
                && tool.input_schema["additionalProperties"] == false
                && tool
                    .description
                    .as_ref()
                    .is_some_and(|text| text.contains("Example"))
        }));
        let edits = &tools[3].input_schema["properties"]["edits"];
        assert_eq!(edits["minItems"], 1);
        let variants = edits["items"]["oneOf"].as_array().unwrap();
        let required = [
            ("alignment", &["alignment"][..]),
            ("insert-paragraph", &["text"][..]),
            ("insert-table", &["rows"][..]),
            ("run-text", &["text"][..]),
            ("run-remove", &[][..]),
            ("text-range", &["start", "end", "text"][..]),
            ("insert-text", &["offset", "text"][..]),
            ("cell-alignment", &["alignment"][..]),
            ("header-image", &["mode", "bytes"][..]),
        ];
        assert_eq!(variants.len(), required.len());
        for (kind, extra) in required {
            let variant = variants
                .iter()
                .find(|variant| variant["properties"]["kind"]["const"] == kind)
                .unwrap_or_else(|| panic!("missing schema for {kind}"));
            assert_eq!(variant["additionalProperties"], false);
            let fields = variant["required"].as_array().unwrap();
            assert_eq!(fields.len(), extra.len() + 2);
            for field in ["kind", "anchor"].into_iter().chain(extra.iter().copied()) {
                assert!(fields.contains(&json!(field)), "{kind} requires {field}");
            }
            for field in ["start", "end", "offset"]
                .into_iter()
                .filter(|field| extra.contains(field))
            {
                assert_eq!(variant["properties"][field]["type"], "integer");
                assert_eq!(variant["properties"][field]["minimum"], 0);
            }
            if kind == "header-image" {
                let byte = &variant["properties"]["bytes"]["items"];
                assert_eq!(byte["type"], "integer");
                assert_eq!(byte["minimum"], 0);
                assert_eq!(byte["maximum"], 255);
            }
        }
    }

    #[test]
    fn edit_inputs_accept_only_the_supported_shapes_and_values() {
        assert!(
            preview(json!([
                {"kind":"alignment","anchor":"p","alignment":"thai-justify"},
                {"kind":"insert-paragraph","anchor":"p","text":"After"},
                {"kind":"insert-table","anchor":"p","rows":[["A","B"],["C","D"]]},
                {"kind":"run-text","anchor":"r","text":"Updated"},
                {"kind":"run-remove","anchor":"r"},
                {"kind":"text-range","anchor":"p","start":0,"end":1,"text":""},
                {"kind":"insert-text","anchor":"p","offset":0,"text":"New"},
                {"kind":"cell-alignment","anchor":"c","alignment":"both"},
                {"kind":"header-image","anchor":"h","mode":"section-local","bytes":[0,255]}
            ]))
            .is_ok()
        );
        assert!(preview(json!([])).is_err());
        for bad in [
            json!({"kind":"metadata","anchor":"p"}),
            json!({"kind":"run-remove"}),
            json!({"kind":"run-remove","anchor":0}),
            json!({"kind":"run-remove","anchor":"r","text":"unused"}),
            json!({"kind":"text-range","anchor":"p","start":-1,"end":1,"text":""}),
            json!({"kind":"text-range","anchor":"p","start":0,"end":1.5,"text":""}),
            json!({"kind":"text-range","anchor":"p","start":0,"end":1}),
            json!({"kind":"insert-text","anchor":"p","offset":"0","text":"New"}),
            json!({"kind":"insert-text","anchor":"p","offset":0,"end":1,"text":"New"}),
            json!({"kind":"alignment","anchor":"p","alignment":"top"}),
            json!({"kind":"cell-alignment","anchor":"c","alignment":"left"}),
            json!({"kind":"insert-table","anchor":"p","rows":[]}),
            json!({"kind":"insert-table","anchor":"p","rows":[[]]}),
            json!({"kind":"insert-table","anchor":"p","rows":[["A"],["B","C"]]}),
            json!({"kind":"header-image","anchor":"h","mode":"shared","bytes":[256]}),
            json!({"kind":"header-image","anchor":"h","mode":"shared","bytes":[-1]}),
            json!({"kind":"header-image","anchor":"h","mode":"shared","bytes":[1.5]}),
            json!({"kind":"header-image","anchor":"h","mode":"local","bytes":[0]}),
        ] {
            assert!(preview(json!([bad])).is_err(), "accepted {bad}");
        }
    }

    #[test]
    fn create_omits_paragraphs_but_save_never_accepts_edits() {
        let Ok(Call::Create(input)) = ToolKind::Create.parse(JsonObject::new()) else {
            panic!("create without arguments is valid");
        };
        assert!(input.paragraphs.is_empty());
        assert!(
            ToolKind::Save
                .parse(
                    json!({"document":"doc","preview":"preview","output":"copy.docx","edits":[]})
                        .as_object()
                        .unwrap()
                        .clone()
                )
                .is_err()
        );
    }
}
