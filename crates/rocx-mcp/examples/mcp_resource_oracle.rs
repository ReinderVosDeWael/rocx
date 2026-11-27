//! Inspect and edit independent SDK acceptance fixtures through native Rust, not MCP.
use std::{env, fs, path::Path};

use rocx::Alignment;
use rocx::editor::{
    BodyBlock, CustomValue, DefaultField, DefaultTab, DefaultValue, EditBatch, ImportedDocument,
    MetadataField, ResourceEdit, TextInsertion,
};
use serde_json::{Value, json};

fn inspect(doc: &ImportedDocument, query: &str, fields: &[std::ffi::OsString]) -> Value {
    fn outcome<T>(result: rocx::Result<T>, project: impl FnOnce(T) -> Value) -> Value {
        match result {
            Ok(value) => json!({ "value": project(value) }),
            Err(error) => json!({ "error": error.to_string() }),
        }
    }

    match query {
        "metadata" => fields
            .iter()
            .map(|field| {
                let key = field.to_str().unwrap();
                let value = doc.metadata_field(key.parse::<MetadataField>().unwrap());
                (key.to_owned(), outcome(value, |value| json!(value)))
            })
            .collect::<serde_json::Map<_, _>>()
            .into(),
        "custom-property" => fields
            .iter()
            .map(|field| {
                let name = field.to_str().unwrap();
                let value = outcome(doc.custom_property(name), |value| {
                    value.map_or(Value::Null, |value| match value {
                        CustomValue::Boolean(value) => json!(value),
                        CustomValue::Text(value) => json!(value),
                    })
                });
                (name.to_owned(), value)
            })
            .collect::<serde_json::Map<_, _>>()
            .into(),
        "default-alignment" => outcome(doc.default_alignment(), |value| {
            json!(value.map(|alignment| match alignment {
                Alignment::Left => "left",
                Alignment::Center => "center",
                Alignment::Right => "right",
                Alignment::Justify => "justify",
                Alignment::Distribute => "distribute",
                Alignment::JustifyMedium => "justify-medium",
                Alignment::JustifyHigh => "justify-high",
                Alignment::JustifyLow => "justify-low",
                Alignment::ThaiJustify => "thai-justify",
            }))
        }),
        "document-defaults" => outcome(doc.document_defaults(), |values| {
            values
                .into_iter()
                .map(|(field, value)| {
                    let value = match value {
                        DefaultValue::Bool(value) => json!(value),
                        DefaultValue::Integer(value) => json!(value),
                        DefaultValue::Text(value) => json!(value),
                        DefaultValue::Tabs(tabs) => json!(
                            tabs.into_iter()
                                .map(|tab| json!({
                                    "position_twips": tab.position_twips,
                                    "alignment": tab.alignment,
                                    "leader": tab.leader,
                                }))
                                .collect::<Vec<_>>()
                        ),
                    };
                    (field.key().to_owned(), value)
                })
                .collect::<serde_json::Map<_, _>>()
                .into()
        }),
        _ => panic!("unknown inspection: {query}"),
    }
}

fn edit(source: &Path, case: &str) -> Vec<u8> {
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let (resources, insertions) = match case {
        "mixed" => {
            let BodyBlock::Paragraph(paragraph) = &doc.body()[0] else {
                panic!("expected fixture paragraph");
            };
            (
                vec![
                    ResourceEdit::CoreTitle(Some("MCP & Rust".into())),
                    ResourceEdit::CoreSubject(None),
                    ResourceEdit::DefaultAlignment(Some(Alignment::Justify)),
                    ResourceEdit::Default(DefaultField::Bold, Some(DefaultValue::Bool(false))),
                    ResourceEdit::Default(
                        DefaultField::BeforeTwips,
                        Some(DefaultValue::Integer(288)),
                    ),
                    ResourceEdit::Default(
                        DefaultField::LineRule,
                        Some(DefaultValue::Text("atLeast".into())),
                    ),
                    ResourceEdit::Default(
                        DefaultField::TabStops,
                        Some(DefaultValue::Tabs(vec![
                            DefaultTab {
                                position_twips: 480,
                                alignment: "left".into(),
                                leader: None,
                            },
                            DefaultTab {
                                position_twips: 960,
                                alignment: "decimal".into(),
                                leader: Some("dot".into()),
                            },
                        ])),
                    ),
                    ResourceEdit::Default(
                        DefaultField::TrackRevisions,
                        Some(DefaultValue::Bool(false)),
                    ),
                    ResourceEdit::Metadata(MetadataField::Comments, Some("Review 😀".into())),
                    ResourceEdit::Metadata(MetadataField::Company, None),
                    ResourceEdit::CustomProperty(
                        "Reviewed".into(),
                        Some(CustomValue::Boolean(false)),
                    ),
                ],
                vec![TextInsertion {
                    target: paragraph.anchor.clone(),
                    position: 4,
                    text: " & added".into(),
                }],
            )
        }
        "legacy" => (
            vec![
                ResourceEdit::CoreTitle(None),
                ResourceEdit::CoreSubject(Some("Résumé 世界 😀".into())),
                ResourceEdit::DefaultAlignment(None),
            ],
            vec![],
        ),
        "direct" => (
            vec![
                ResourceEdit::Default(DefaultField::Bold, None),
                ResourceEdit::Default(
                    DefaultField::Justification,
                    Some(DefaultValue::Text("start".into())),
                ),
                ResourceEdit::Default(
                    DefaultField::BeforeTwips,
                    Some(DefaultValue::Integer(u32::MAX.into())),
                ),
                ResourceEdit::Default(
                    DefaultField::LeftTwips,
                    Some(DefaultValue::Integer(i32::MIN.into())),
                ),
                ResourceEdit::CustomProperty("Reviewed".into(), None),
                ResourceEdit::CustomProperty(
                    "Empty".into(),
                    Some(CustomValue::Text(String::new())),
                ),
            ],
            vec![],
        ),
        _ => panic!("unknown resource fixture case: {case}"),
    };
    doc.preview_edits(EditBatch {
        resources: &resources,
        text_insertions: &insertions,
        ..EditBatch::default()
    })
    .unwrap()
    .to_docx()
    .unwrap()
}

fn write_modeled(
    source: &Path,
    values: &Path,
    phase: &str,
    metadata_fields: &[std::ffi::OsString],
) -> Vec<u8> {
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let values = ImportedDocument::open(&fs::read(values).unwrap()).unwrap();
    let set = match phase {
        "set" => true,
        "clear" => false,
        _ => panic!("unknown modeled write phase: {phase}"),
    };
    let mut edits: Vec<_> = values
        .document_defaults()
        .unwrap()
        .into_iter()
        .map(|(field, value)| ResourceEdit::Default(field, set.then_some(value)))
        .collect();
    edits.extend(metadata_fields.iter().map(|field| {
        let field = field.to_str().unwrap().parse::<MetadataField>().unwrap();
        let value = values.metadata_field(field).unwrap().unwrap();
        ResourceEdit::Metadata(field, set.then_some(value))
    }));
    doc.preview_edits(EditBatch {
        resources: &edits,
        ..EditBatch::default()
    })
    .unwrap()
    .to_docx()
    .unwrap()
}

fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [command, source, query, fields @ ..] if command == "inspect" => {
            let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
            println!("{}", inspect(&doc, query.to_str().unwrap(), fields));
        }
        [command, output, source, values, phase, fields @ ..] if command == "write-modeled" => {
            fs::write(
                output,
                write_modeled(
                    Path::new(source),
                    Path::new(values),
                    phase.to_str().unwrap(),
                    fields,
                ),
            )
            .unwrap();
        }
        [output, source, case] => {
            fs::write(output, edit(Path::new(source), case.to_str().unwrap())).unwrap();
        }
        _ => panic!(
            "usage: mcp_resource_oracle OUTPUT SOURCE mixed|legacy|direct \
             or mcp_resource_oracle inspect SOURCE QUERY [FIELD ...] \
             or mcp_resource_oracle write-modeled OUTPUT SOURCE VALUES set|clear [FIELD ...]"
        ),
    }
}
