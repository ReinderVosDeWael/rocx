use rmcp::{
    handler::server::common::schema_for_input,
    model::{JsonObject, Tool},
    schemars::{self, JsonSchema},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;

use crate::session::Session;

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
                 preview_edits validates the actual target and batch. \
                 Example result excerpt: {\"document\":\"<document>\",\"body\":[{\"type\":\"paragraph\",\
                 \"anchor\":\"<paragraph anchor>\",\"text\":\"Example\",\"scalar_length\":7}],\"headers\":[]}.",
                schema_for_input::<Document>(),
            ),
            Self::Preview => (
                "Validate and encode an ordered batch of edits against the original snapshot. \
                 Use anchors from inspect_document. Returns a fresh preview handle, output SHA-256, \
                 and change report without writing a file or invalidating earlier previews. \
                 Example result excerpt: {\"document\":\"<document>\",\"preview\":\"<preview>\",\
                 \"output_snapshot\":\"<sha256>\"}.",
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
                let input: Preview = parse(arguments)?;
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
                Ok(Call::Preview(input))
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
    Inspect(Document),
    Preview(Preview),
    Save(Save),
    Close(Document),
}

impl Call {
    pub(crate) fn dispatch(self, session: &mut Session) -> Result<Value, String> {
        match self {
            Self::Create(input) => session.create_document(input.paragraphs),
            Self::Open(input) => session.open_document(input.path),
            Self::Inspect(input) => session.inspect_document(input.document),
            Self::Preview(input) => session.preview_edits(input.document, input.edits),
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
pub(crate) struct Preview {
    /// The live document whose anchors the edits use.
    document: String,
    /// Ordered edits against the original snapshot.
    #[schemars(length(min = 1))]
    edits: Vec<Edit>,
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

    fn preview(edits: Value) -> Result<Call, String> {
        ToolKind::Preview.parse(
            json!({"document": "doc", "edits": edits})
                .as_object()
                .unwrap()
                .clone(),
        )
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
