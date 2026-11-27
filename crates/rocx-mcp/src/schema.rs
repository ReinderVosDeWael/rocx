pub(crate) mod blocks;
pub(crate) mod body_inlines;
pub(crate) mod controls;
mod entries;
pub(crate) mod entry_inlines;
pub(crate) mod fields;
pub(crate) mod layout;
mod limits;
pub(crate) mod notes;
pub(crate) mod numbering;
pub(crate) mod payloads;
pub(crate) mod resources;
pub(crate) mod revisions;
pub(crate) mod run_format;
pub(crate) mod stories;
pub(crate) mod styles;
pub(crate) mod tables;

use rmcp::{
    handler::server::common::schema_for_input,
    model::{JsonObject, Tool},
    schemars::{self, JsonSchema, Schema},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::{
    diagnostics::ToolError,
    session::{InlineSelector, PageView, Session},
};
pub(crate) use entries::PhysicalTableSelector;
use entries::{Operation, View};

#[derive(Clone, Copy)]
pub(crate) enum ToolKind {
    Create,
    Open,
    Inspect,
    Preview,
    Save,
    SaveInPlace,
    Close,
}

impl ToolKind {
    pub(crate) const ALL: [Self; 7] = [
        Self::Create,
        Self::Open,
        Self::Inspect,
        Self::Preview,
        Self::Save,
        Self::SaveInPlace,
        Self::Close,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Create => "create_document",
            Self::Open => "open_document",
            Self::Inspect => "inspect_document",
            Self::Preview => "preview_edits",
            Self::Save => "save_copy",
            Self::SaveInPlace => "save_in_place",
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
                 Returns a fresh document handle and the source SHA-256 without modifying the file. \
                 Optional limits override individual native import budgets as decimal strings. \
                 Example result: {\"document\":\"<document>\",\"snapshot\":\"<sha256>\"}.",
                schema_for_input::<Open>(),
            ),
            Self::Inspect => (
                "Inspect body paragraphs and runs, physical table cells, and default and first-page \
                  header slots. Returns reusable opaque anchors for candidate edits to this snapshot. \
                 A paragraph's scalar_length describes Unicode scalar coordinates for its plain-text \
                  projection, with null meaning none are advertised. It is not edit permission: \
                  preview_edits validates the actual target and batch. Omit view for this compact \
                   result or use {\"kind\":\"body\"} for body anchors without reading story parts. \
                   Use view {\"kind\":\"entries\"} for ordinary footnote, endnote, and comment \
                  definitions with entry anchors, note-references for direct body note sites and \
                  tracked revisions with {\"kind\":\"revisions\"}, or direct paragraph and section \
                  properties with {\"kind\":\"layout\"}. Use {\"kind\":\"sections\"} for native \
                  body ranges and boundary anchors, {\"kind\":\"stories\"} for all six \
                   header/footer slots, or {\"kind\":\"resources\"} for direct metadata and defaults. \
                   Use {\"kind\":\"linkable-stories\",\"story_kind\":\"header\"} (or \"footer\") \
                    for link source anchors from unreferenced parts; set include_referenced to \
                    true to also find parts already used by a section. \
                  Use fields, range-markers and body-inlines for original inline sites, format-runs for \
                   physical run targets, and body-tables or physical-tables for table edits. \
                   Physical-tables accepts a main, story, or entry scope; omit scope for all owners. \
                  Use controls and control-sites for main-story content control edits, or \
                  story-controls for read-only physical story controls. \
                  Body-inlines describes drawings; chart-series reads the series at one \
                  body-inlines anchor. Use entry-inlines for note and comment inline anchors, \
                  or styles for the physical style catalog and original style anchors. \
                  Use numbering-abstracts and numbering-instances for direct list catalogs, \
                  paragraph-numbering for one direct body assignment, or numbering-paragraphs \
                  for physical story, entry and body-cell assignments. \
                  The page limit defaults to 25 (range 1–100); \
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
                 revision previews return the native report projection, while entry and note \
                 reference edits return changes: null. Example scoped operation: {\"kind\":\"edit-entry\",\
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
            Self::SaveInPlace => (
                "Explicitly replace the source file opened by this document handle with the selected \
                 preview's retained bytes. Refuses in-memory documents, symbolic-link sources, and \
                 sources that no longer match the opened SHA-256. Reopen after saving to edit the \
                 new contents. Example result: \
                 {\"output\":\"/path/source.docx\",\"output_snapshot\":\"<sha256>\"}.",
                schema_for_input::<SaveInPlace>(),
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
            Self::SaveInPlace => parse(arguments).map(Call::SaveInPlace),
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
    SaveInPlace(SaveInPlace),
    Close(Document),
}

impl Call {
    pub(crate) fn dispatch(self, session: &mut Session) -> Result<Value, ToolError> {
        let legacy = match self {
            Self::Create(input) => session.create_document(input.paragraphs),
            Self::Open(input) => {
                return session.open_document(input.path, input.limits.map(Into::into));
            }
            Self::Inspect(input) => return dispatch_inspection(input, session),
            Self::Preview(Preview::Ordinary(input)) => {
                return session.preview_edits(input.document, input.edits);
            }
            Self::Preview(Preview::Scoped(input)) => match input.operation {
                Operation::EditEntry { anchor, edit } => {
                    return session.preview_entry(input.document, anchor, edit.into());
                }
                Operation::EditNoteReference {
                    anchor,
                    note_kind,
                    edit,
                } => {
                    return session.preview_note_reference(input.document, anchor, note_kind, edit);
                }
                Operation::EditRevision {
                    anchor,
                    paired_anchor,
                    edit,
                } => return session.preview_revision(input.document, anchor, paired_anchor, edit),
                Operation::EditLayout { edits } => {
                    return session.preview_layout(input.document, edits);
                }
                Operation::EditStoryText { anchor, edits }
                    if Session::is_default_header_owner(&anchor) =>
                {
                    return session.preview_default_header_text(input.document, &anchor, edits);
                }
                Operation::EditStoryText { anchor, edits } => {
                    return session.preview_story_text(input.document, &anchor, edits);
                }
                Operation::EditEntryInlines { anchor, edits } => {
                    return session.preview_entry_inlines(input.document, &anchor, edits);
                }
                Operation::EditStoryReference { anchor, edit } => {
                    return session.preview_story_reference(input.document, &anchor, edit);
                }
                Operation::EditStoryBlocks { anchor, edit } => {
                    return session.preview_story_blocks(input.document, &anchor, edit);
                }
                Operation::EditRunFormat { anchor, edits }
                    if Session::is_default_header_owner(&anchor) =>
                {
                    return session.preview_default_header_format(input.document, &anchor, edits);
                }
                Operation::EditRunFormat { anchor, edits } => {
                    return session.preview_run_format(input.document, &anchor, edits);
                }
                Operation::EditPartTables { anchor, edits } => {
                    return session.preview_part_tables(input.document, &anchor, edits);
                }
                Operation::NoOp {} => return session.preview_noop(input.document),
            },
            Self::Save(input) => session.save_copy(input.document, input.preview, input.output),
            Self::SaveInPlace(input) => session.save_in_place(input.document, input.preview),
            Self::Close(input) => session.close_document(input.document),
        };
        legacy.map_err(ToolError::from)
    }
}

fn dispatch_inspection(input: Inspect, session: &mut Session) -> Result<Value, ToolError> {
    match input.view {
        None => session.inspect_document(input.document),
        Some(View::Body) => session.inspect_body(input.document),
        Some(View::Entries { limit, cursor }) => {
            session.inspect_page(input.document, PageView::Entries, limit, cursor)
        }
        Some(View::Layout {
            scope,
            limit,
            cursor,
        }) => session.inspect_page(input.document, PageView::Layout(scope), limit, cursor),
        Some(View::NoteReferences { limit, cursor }) => {
            session.inspect_page(input.document, PageView::NoteReferences, limit, cursor)
        }
        Some(View::Revisions { limit, cursor }) => {
            session.inspect_page(input.document, PageView::Revisions, limit, cursor)
        }
        Some(View::Stories { limit, cursor }) => {
            session.inspect_page(input.document, PageView::Stories, limit, cursor)
        }
        Some(View::DefaultHeaders { limit, cursor }) => {
            session.inspect_page(input.document, PageView::DefaultHeaders, limit, cursor)
        }
        Some(View::LinkableStories {
            story_kind,
            include_referenced,
            limit,
            cursor,
        }) => session.inspect_page(
            input.document,
            PageView::LinkableStories(story_kind.into(), include_referenced),
            limit,
            cursor,
        ),
        Some(View::StoryInlines {
            anchor,
            block_index,
            limit,
            cursor,
        }) => session.inspect_inline_page(
            input.document,
            InlineSelector::Story {
                anchor: &anchor,
                block: block_index,
            },
            limit,
            cursor,
        ),
        Some(View::TableCellInlines {
            anchor,
            row_index,
            cell_index,
            block_index,
            limit,
            cursor,
        }) => session.inspect_inline_page(
            input.document,
            InlineSelector::TableCell {
                anchor: &anchor,
                row: row_index,
                cell: cell_index,
                block: block_index,
            },
            limit,
            cursor,
        ),
        Some(View::Sections { limit, cursor }) => {
            session.inspect_page(input.document, PageView::Sections, limit, cursor)
        }
        Some(View::FieldAt {
            anchor,
            inline_index,
        }) => session.inspect_field_at(input.document, anchor, inline_index),
        Some(View::Fields { limit, cursor }) => {
            session.inspect_page(input.document, PageView::Fields, limit, cursor)
        }
        Some(View::BodyInlines { limit, cursor }) => {
            session.inspect_page(input.document, PageView::BodyInlines, limit, cursor)
        }
        Some(View::InlineAt { inline_handle }) => {
            session.inspect_inline_at(input.document, inline_handle)
        }
        Some(View::ChartSeries { anchor }) => session.inspect_chart_series(input.document, anchor),
        Some(View::RangeMarkers { limit, cursor }) => {
            session.inspect_page(input.document, PageView::RangeMarkers, limit, cursor)
        }
        Some(View::Controls { limit, cursor }) => {
            session.inspect_page(input.document, PageView::Controls, limit, cursor)
        }
        Some(View::ControlSites { limit, cursor }) => {
            session.inspect_page(input.document, PageView::ControlSites, limit, cursor)
        }
        Some(View::StoryControls { limit, cursor }) => {
            session.inspect_page(input.document, PageView::StoryControls, limit, cursor)
        }
        Some(View::BodyTables { limit, cursor }) => {
            session.inspect_page(input.document, PageView::BodyTables, limit, cursor)
        }
        Some(View::FormatRuns { limit, cursor }) => {
            session.inspect_page(input.document, PageView::FormatRuns, limit, cursor)
        }
        Some(View::PhysicalTables {
            scope,
            limit,
            cursor,
        }) => session.inspect_physical_table_page(input.document, scope, limit, cursor),
        Some(View::EntryRuns { limit, cursor }) => {
            session.inspect_page(input.document, PageView::EntryRuns, limit, cursor)
        }
        Some(View::EntryInlines { limit, cursor }) => {
            session.inspect_page(input.document, PageView::EntryInlines, limit, cursor)
        }
        Some(View::Styles { limit, cursor }) => {
            session.inspect_page(input.document, PageView::Styles, limit, cursor)
        }
        Some(View::NumberingAbstracts { limit, cursor }) => {
            session.inspect_page(input.document, PageView::NumberingAbstracts, limit, cursor)
        }
        Some(View::NumberingInstances { limit, cursor }) => {
            session.inspect_page(input.document, PageView::NumberingInstances, limit, cursor)
        }
        Some(View::ParagraphNumbering { anchor }) => {
            session.inspect_paragraph_numbering(input.document, anchor)
        }
        Some(View::NumberingParagraphs {
            scope,
            limit,
            cursor,
        }) => session.inspect_page(
            input.document,
            PageView::NumberingParagraphs(scope),
            limit,
            cursor,
        ),
        Some(View::Resources { custom_property }) => {
            session.inspect_resources(input.document, custom_property)
        }
        Some(View::ResourceValue { query }) => {
            session.inspect_resource_value(input.document, query)
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
    /// Per-open archive read budgets. Omitted members use native defaults.
    #[serde(default, deserialize_with = "entries::present")]
    #[schemars(with = "limits::ImportLimits", skip_serializing_if = "Option::is_none")]
    limits: Option<limits::ImportLimits>,
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
pub(crate) struct SaveInPlace {
    /// An opaque live document handle from open_document.
    document: String,
    /// An opaque preview handle from preview_edits for the selected document.
    preview: String,
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
    /// Insert a paragraph with declared formatting and inlines after the anchor.
    InsertRichParagraph {
        anchor: String,
        paragraph: Box<payloads::ParagraphInput>,
    },
    /// Insert a table immediately after the anchor.
    InsertTable {
        /// Paragraph anchor.
        anchor: String,
        /// Nonempty rectangular matrix with one plain-text paragraph per cell.
        #[schemars(length(min = 1), inner(length(min = 1)))]
        rows: Vec<Vec<String>>,
    },
    /// Insert a declared table with formatted, optionally nested cell contents.
    InsertRichTable {
        anchor: String,
        table: Box<payloads::RichTableInput>,
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
    /// Append an inline image to an existing header or footer slot.
    #[schemars(transform = stories::require_image_payload)]
    HeaderImage {
        /// Slot anchor from compact inspection or stories, or `image_slot.anchor` from default-headers.
        anchor: String,
        mode: HeaderMode,
        /// Shorthand for an encoded image with intrinsic dimensions and default metadata.
        #[serde(default, deserialize_with = "entries::present")]
        #[schemars(with = "Vec<u8>", skip_serializing_if = "Option::is_none")]
        bytes: Option<Vec<u8>>,
        /// Rich inline image declaration. Supply this or `bytes`, not both.
        #[serde(default, deserialize_with = "entries::present")]
        #[schemars(
            with = "stories::StoryImageInput",
            skip_serializing_if = "Option::is_none"
        )]
        image: Option<Box<stories::StoryImageInput>>,
    },
    /// Set or clear a named document property or direct default in the ordinary batch.
    Resource { edit: resources::ResourceChange },
    /// Change a body block, direct properties or a section boundary.
    Block {
        anchor: String,
        edit: blocks::BlockChange,
    },
    /// Change a supported original main-story field.
    Field {
        anchor: String,
        edit: fields::FieldChange,
    },
    /// Insert, replace or change a direct body inline.
    Inline {
        anchor: String,
        edit: body_inlines::BodyInlineChange,
    },
    /// Create, move or remove a paired bookmark or comment range in the main story.
    PairedRange {
        /// Numeric range ID. For a comment, identifies one existing definition.
        #[schemars(range(min = 0, max = 4_294_967_295u32))]
        id: u32,
        /// Bookmark name. Omit for a comment range.
        #[serde(default, deserialize_with = "entries::present")]
        #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
        bookmark: Option<String>,
        /// Inclusive original endpoints from body-inlines. Omit to remove the pair.
        #[serde(default, deserialize_with = "entries::present")]
        #[schemars(with = "RangeEndpoints", skip_serializing_if = "Option::is_none")]
        endpoints: Option<RangeEndpoints>,
    },
    /// Change a direct body table in the ordinary mixed batch.
    TableStructure {
        anchor: String,
        edit: tables::BodyTableEdit,
    },
    /// Change a main-story content control using anchors from controls or control-sites.
    Control { edit: controls::ControlChange },
    /// Edit the relationship-selected style catalog.
    Style { edit: styles::StyleChange },
    /// Change a list definition, instance or paragraph assignment.
    Numbering { edit: numbering::NumberingChange },
}

impl Edit {
    pub(crate) fn anchor_slot(&mut self) -> Option<&mut String> {
        match self {
            Self::Alignment { anchor, .. }
            | Self::InsertParagraph { anchor, .. }
            | Self::InsertRichParagraph { anchor, .. }
            | Self::InsertTable { anchor, .. }
            | Self::InsertRichTable { anchor, .. }
            | Self::RunText { anchor, .. }
            | Self::RunRemove { anchor, .. }
            | Self::TextRange { anchor, .. }
            | Self::InsertText { anchor, .. }
            | Self::CellAlignment { anchor, .. }
            | Self::HeaderImage { anchor, .. }
            | Self::Block { anchor, .. }
            | Self::Field { anchor, .. }
            | Self::Inline { anchor, .. }
            | Self::TableStructure { anchor, .. } => Some(anchor),
            Self::Resource { .. }
            | Self::PairedRange { .. }
            | Self::Control { .. }
            | Self::Style { .. }
            | Self::Numbering { .. } => None,
        }
    }

    pub(crate) fn target_kind(&self) -> &'static str {
        match self {
            Self::Alignment { .. }
            | Self::InsertParagraph { .. }
            | Self::InsertRichParagraph { .. }
            | Self::InsertTable { .. }
            | Self::InsertRichTable { .. }
            | Self::TextRange { .. }
            | Self::InsertText { .. } => "paragraph",
            Self::RunText { .. } | Self::RunRemove { .. } => "run",
            Self::CellAlignment { .. } => "cell",
            Self::HeaderImage { .. } => "header",
            Self::Resource { .. }
            | Self::PairedRange { .. }
            | Self::Control { .. }
            | Self::Style { .. }
            | Self::Numbering { .. } => {
                unreachable!("edit has no single anchor")
            }
            Self::Block { .. }
            | Self::Field { .. }
            | Self::Inline { .. }
            | Self::TableStructure { .. } => unreachable!("domain adapter resolves its own target"),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(inline)]
pub(crate) struct RangeEndpoints {
    pub(crate) first: String,
    pub(crate) last: String,
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
    fn physical_inline_views_advertise_bounded_paragraph_selectors() {
        let inspect = ToolKind::Inspect.definition().input_schema;
        let variants = inspect["properties"]["view"]["oneOf"].as_array().unwrap();
        for (kind, indices) in [
            ("story-inlines", &["block_index"][..]),
            (
                "table-cell-inlines",
                &["row_index", "cell_index", "block_index"][..],
            ),
        ] {
            let view = variants
                .iter()
                .find(|view| view["properties"]["kind"]["const"] == kind)
                .unwrap();
            assert_eq!(view["additionalProperties"], false);
            let required = view["required"].as_array().unwrap();
            assert_eq!(required.len(), indices.len() + 2);
            for field in ["kind", "anchor"]
                .into_iter()
                .chain(indices.iter().copied())
            {
                assert!(required.contains(&json!(field)), "{kind} requires {field}");
            }
            for field in indices {
                assert_eq!(view["properties"][*field]["type"], "integer");
                assert_eq!(view["properties"][*field]["minimum"], 0);
            }
            assert_eq!(view["properties"]["limit"]["minimum"], 1);
            assert_eq!(view["properties"]["limit"]["maximum"], 100);
            assert_eq!(view["properties"]["cursor"]["type"], "string");
        }
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
                view.as_ref().and_then(|view| match view {
                    View::Entries { limit, cursor } => Some((*limit, cursor.as_deref())),
                    _ => None,
                }),
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
    fn discovery_has_seven_strict_tools_and_discriminated_edits() {
        let tools = ToolKind::ALL.map(ToolKind::definition);
        assert_eq!(
            tools.each_ref().map(|tool| tool.name.as_ref()),
            [
                "create_document",
                "open_document",
                "inspect_document",
                "preview_edits",
                "save_copy",
                "save_in_place",
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
            ("insert-rich-paragraph", &["paragraph"][..]),
            ("insert-table", &["rows"][..]),
            ("insert-rich-table", &["table"][..]),
            ("run-text", &["text"][..]),
            ("run-remove", &[][..]),
            ("text-range", &["start", "end", "text"][..]),
            ("insert-text", &["offset", "text"][..]),
            ("cell-alignment", &["alignment"][..]),
            ("header-image", &["mode"][..]),
            ("block", &["edit"][..]),
            ("field", &["edit"][..]),
            ("inline", &["edit"][..]),
            ("table-structure", &["edit"][..]),
        ];
        assert_eq!(variants.len(), required.len() + 5);
        for kind in ["resource", "control", "style", "numbering"] {
            let variant = variants
                .iter()
                .find(|variant| variant["properties"]["kind"]["const"] == kind)
                .unwrap();
            assert_eq!(variant["required"], json!(["kind", "edit"]));
            assert_eq!(variant["additionalProperties"], false);
        }
        let range = variants
            .iter()
            .find(|variant| variant["properties"]["kind"]["const"] == "paired-range")
            .unwrap();
        assert_eq!(range["required"], json!(["kind", "id"]));
        assert_eq!(range["additionalProperties"], false);
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
                assert_eq!(
                    variant["oneOf"],
                    json!([{"required": ["bytes"]}, {"required": ["image"]}])
                );
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
