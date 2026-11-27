//! Independent native numbering transactions for the MCP fixtures.
use rocx::editor::{
    AbstractStart, BodyBlock, EditBatch, ImportedDocument, ImportedInlineNode, NumberingEdit as N,
    ResourceEdit, RunTextEdit, StyleEdit,
};
use rocx::{
    NumberingAlignment, NumberingDefinition, NumberingLevel, NumberingReference,
    NumberingStartOverride, NumberingStyleDefinition, NumberingSuffix, ParagraphFormat, RunFormat,
    TabAlignment, TabStop,
};

fn simple(level: u8, start: u32) -> NumberingLevel {
    NumberingLevel {
        level,
        start,
        format: "decimal".into(),
        text: format!("%{}.", level + 1),
        ..Default::default()
    }
}

fn rich() -> NumberingLevel {
    simple(0, 1)
        .with_label_format(RunFormat::default().with_bold(false).with_size(12.))
        .with_paragraph_format(
            ParagraphFormat::default()
                .with_left_indent(12.)
                .with_tab_stops(vec![TabStop {
                    position: 36.,
                    alignment: TabAlignment::Left,
                    leader: None,
                }]),
        )
        .with_suffix(NumberingSuffix::Space)
        .with_alignment(NumberingAlignment::Left)
        .with_is_legal(false)
        .with_paragraph_style("Normal")
}

fn replacement() -> NumberingLevel {
    simple(1, 3)
        .with_format("lowerRoman")
        .with_label_format(RunFormat::default().with_italic(true))
}

fn main() -> rocx::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let doc = ImportedDocument::open(&std::fs::read(args.next().unwrap()).unwrap())?;
    let action = args.next().unwrap().into_string().unwrap();
    let output = args.next().unwrap();
    if action == "inspect" {
        std::fs::write(
            output,
            serde_json::to_vec(&doc.numbering_catalog()?).unwrap(),
        )
        .unwrap();
        return Ok(());
    }
    let BodyBlock::Paragraph(p) = &doc.body()[0] else {
        panic!("expected first paragraph")
    };
    let paragraph = |value| N::Paragraph {
        target: p.anchor.clone(),
        value,
    };
    let mut run_text = Vec::new();
    let mut styles = Vec::new();
    let edits = match action.as_str() {
        "create" => {
            let BodyBlock::Paragraph(other) = &doc.body()[1] else {
                panic!("expected second paragraph")
            };
            let ImportedInlineNode::Run(run) = &other.inline_structure[0] else {
                panic!("expected run")
            };
            run_text.push(RunTextEdit {
                target: run.anchor.clone(),
                text: "Unrelated edited".into(),
            });
            vec![
                N::InstanceStart {
                    num_id: 90,
                    level: 1,
                    start: Some(6),
                },
                paragraph(Some(NumberingReference { id: 90, level: 1 })),
                N::InstanceLevel {
                    num_id: 90,
                    level: 1,
                    value: Some(Box::new(replacement())),
                },
                N::AddInstance {
                    num_id: 90,
                    abstract_id: 80,
                },
                N::AddAbstract {
                    abstract_id: 80,
                    levels: vec![rich(), simple(1, 1)],
                },
            ]
        }
        "relink" => vec![
            N::RelinkInstance {
                num_id: 90,
                abstract_id: 81,
            },
            N::ReplaceAbstract {
                abstract_id: 80,
                levels: vec![simple(0, 7)],
            },
            N::AddAbstract {
                abstract_id: 81,
                levels: vec![simple(0, 1), simple(1, 1)],
            },
            N::InstanceLevel {
                num_id: 90,
                level: 1,
                value: None,
            },
        ],
        "replace" => vec![
            N::AbstractLevel {
                abstract_id: 80,
                level: 1,
                value: Some(Box::new(simple(1, 8))),
            },
            N::ReplaceInstance(Box::new(NumberingDefinition {
                id: 90,
                abstract_id: Some(81),
                start_overrides: vec![NumberingStartOverride { level: 0, start: 2 }],
                level_overrides: vec![replacement()],
                ..Default::default()
            })),
        ],
        "remove" => vec![
            N::RemoveAbstract { abstract_id: 80 },
            N::RemoveAbstract { abstract_id: 81 },
            N::RemoveInstance { num_id: 90 },
            paragraph(None),
        ],
        "assign-alias" => vec![paragraph(Some(NumberingReference { id: 713, level: 0 }))],
        "reject-alias-level" | "reject-opaque-level" => vec![N::AbstractLevel {
            abstract_id: if action == "reject-alias-level" {
                711
            } else {
                710
            },
            level: 0,
            value: Some(Box::new(simple(0, 1))),
        }],
        "create-part" => {
            styles.push(StyleEdit::Add(
                NumberingStyleDefinition {
                    name: "First List Style".into(),
                    numbering_id: 90,
                }
                .into(),
            ));
            vec![
                N::AddInstance {
                    num_id: 90,
                    abstract_id: 80,
                },
                N::AddAbstract {
                    abstract_id: 80,
                    levels: vec![simple(0, 1)],
                },
                paragraph(Some(NumberingReference { id: 90, level: 0 })),
            ]
        }
        _ => panic!("unknown oracle action"),
    };
    let mut resources: Vec<_> = edits.into_iter().map(ResourceEdit::Numbering).collect();
    if action == "replace" {
        resources.push(ResourceEdit::AbstractStart(AbstractStart {
            abstract_id: 81,
            level: 0,
            start: None,
        }));
    }
    let preview = doc.preview_edits(EditBatch {
        resources: &resources,
        run_text: &run_text,
        styles: &styles,
        ..Default::default()
    });
    if matches!(
        action.as_str(),
        "reject-alias-level" | "reject-opaque-level"
    ) {
        let Err(error) = preview else {
            panic!("native numbering guard accepted {action}")
        };
        std::fs::write(output, error.to_string()).unwrap();
        return Ok(());
    }
    let preview = preview?;
    std::fs::write(output, preview.to_docx()?).unwrap();
    Ok(())
}
