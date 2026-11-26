//! Edit independent SDK acceptance fixtures through native Rust, not MCP.
use std::{env, fs, path::Path};

use rocx::Alignment;
use rocx::editor::{
    BodyBlock, CustomValue, DefaultField, DefaultTab, DefaultValue, EditBatch, ImportedDocument,
    MetadataField, ResourceEdit, TextInsertion,
};

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

fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let [output, source, case] = args.as_slice() else {
        panic!("usage: mcp_resource_oracle OUTPUT SOURCE mixed|direct");
    };
    fs::write(output, edit(Path::new(source), case.to_str().unwrap())).unwrap();
}
