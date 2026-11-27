//! Independent native numbering transactions for the MCP fixtures.
use rocx::editor::{
    AbstractStart, BodyBlock, EditBatch, ImportedDocument, ImportedInlineNode, NumberingEdit as N,
    ResourceEdit, RunTextEdit, StyleEdit,
};
use rocx::{
    Alignment, Color, LineHeight, NumberingAlignment, NumberingDefinition, NumberingLevel,
    NumberingReference, NumberingStartOverride, NumberingStyleDefinition, NumberingSuffix,
    ParagraphFormat, RunFormat, RunLanguage, TabAlignment, TabLeader, TabStop, ThemeColor,
    Underline,
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

fn level_variants() -> Vec<NumberingLevel> {
    use NumberingAlignment::*;
    use NumberingSuffix::*;

    vec![
        simple(0, 1).with_alignment(Left).with_suffix(Tab),
        simple(1, 2)
            .with_alignment(Center)
            .with_suffix(Space)
            .with_restart(0)
            .with_is_legal(true),
        simple(2, 3)
            .with_alignment(Right)
            .with_suffix(Nothing)
            .with_restart(1)
            .with_is_legal(false),
        simple(3, 4).with_alignment(Start).with_paragraph_format(
            ParagraphFormat::default()
                .with_line_spacing(1.25)
                .with_tab_stops(vec![TabStop {
                    position: 18.,
                    alignment: TabAlignment::End,
                    leader: None,
                }]),
        ),
    ]
}

fn detailed_override() -> NumberingLevel {
    simple(2, 10)
        .with_format("upperLetter")
        .with_text("§ %3)")
        .with_alignment(NumberingAlignment::End)
        .with_suffix(NumberingSuffix::Nothing)
        .with_restart(2)
        .with_is_legal(true)
        .with_paragraph_style("heading 1")
        .with_label_format(
            RunFormat::default()
                .with_bold(false)
                .with_italic(true)
                .with_font("Arial")
                .with_size(11.5)
                .with_color(Color::Theme {
                    name: ThemeColor::Accent4,
                    tint: Some(36),
                    shade: None,
                })
                .with_underline(Underline::Wave)
                .with_subscript(true)
                .with_strike(false)
                .with_all_caps(false)
                .with_small_caps(true)
                .with_snap_to_grid(false)
                .with_bold_complex_script(true)
                .with_italic_complex_script(false)
                .with_size_complex_script(13.)
                .with_rtl(false)
                .with_complex_script(true)
                .with_font_east_asia("MS Gothic")
                .with_font_complex_script("Tahoma")
                .with_language(RunLanguage {
                    latin: Some("en-GB".into()),
                    east_asia: Some("ja-JP".into()),
                    bidi: Some("ar-SA".into()),
                }),
        )
        .with_paragraph_format(
            ParagraphFormat::default()
                .with_alignment(Alignment::JustifyHigh)
                .with_bidi(true)
                .with_spacing_before(5.5)
                .with_spacing_after(2.)
                .with_line_height(LineHeight::Exact(12.5))
                .with_left_indent(24.)
                .with_right_indent(6.)
                .with_first_line_indent(-12.)
                .with_keep_together(false)
                .with_keep_with_next(true)
                .with_page_break_before(false)
                .with_widow_control(false)
                .with_suppress_line_numbers(true)
                .with_contextual_spacing(false)
                .with_mirror_indents(true)
                .with_snap_to_grid(false)
                .with_tab_stops(vec![TabStop {
                    position: 36.,
                    alignment: TabAlignment::Decimal,
                    leader: Some(TabLeader::MiddleDot),
                }]),
        )
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
    if action == "inspect-instances" {
        let instances: Vec<_> = doc
            .numbering_instances()?
            .into_iter()
            .map(|instance| {
                serde_json::json!({
                    "num_id": instance.num_id,
                    "abstract_id": instance.abstract_id,
                    "start_overrides": instance.start_overrides.into_iter().map(|entry|
                        serde_json::json!({"level": entry.level, "start": entry.start})
                    ).collect::<Vec<_>>(),
                    "replacement_levels": instance.replacement_levels,
                })
            })
            .collect();
        std::fs::write(output, serde_json::to_vec(&instances).unwrap()).unwrap();
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
                abstract_id: Some(80),
                start_overrides: vec![NumberingStartOverride { level: 0, start: 2 }],
                level_overrides: vec![replacement()],
                ..Default::default()
            })),
        ],
        "replace-empty" => vec![N::ReplaceInstance(Box::new(NumberingDefinition {
            id: 90,
            abstract_id: Some(80),
            ..Default::default()
        }))],
        "remove" => vec![
            N::RemoveAbstract { abstract_id: 80 },
            N::RemoveAbstract { abstract_id: 81 },
            N::RemoveInstance { num_id: 90 },
            paragraph(None),
        ],
        "assign-alias" => vec![paragraph(Some(NumberingReference { id: 713, level: 0 }))],
        "level-fields-set" => vec![
            N::AddAbstract {
                abstract_id: 80,
                levels: level_variants(),
            },
            N::AddAbstract {
                abstract_id: 81,
                levels: vec![
                    simple(0, 1),
                    simple(8, 5).with_alignment(NumberingAlignment::End),
                ],
            },
            N::AddInstance {
                num_id: 90,
                abstract_id: 80,
            },
            N::InstanceStart {
                num_id: 90,
                level: 2,
                start: Some(12),
            },
            N::InstanceStart {
                num_id: 90,
                level: 3,
                start: Some(14),
            },
            N::InstanceLevel {
                num_id: 90,
                level: 2,
                value: Some(Box::new(detailed_override())),
            },
            N::InstanceLevel {
                num_id: 90,
                level: 3,
                value: Some(Box::new(
                    simple(3, 20).with_label_format(
                        RunFormat::default()
                            .with_superscript(true)
                            .with_color(Color::Auto),
                    ),
                )),
            },
        ],
        "level-fields-clear" => vec![
            N::AbstractLevel {
                abstract_id: 81,
                level: 8,
                value: None,
            },
            N::InstanceStart {
                num_id: 90,
                level: 2,
                start: None,
            },
            N::InstanceLevel {
                num_id: 90,
                level: 3,
                value: None,
            },
        ],
        "reject-override-level-index" => vec![N::InstanceLevel {
            num_id: 90,
            level: 2,
            value: Some(Box::new(simple(3, 1))),
        }],
        "reject-override-run-style" => vec![N::InstanceLevel {
            num_id: 90,
            level: 2,
            value: Some(Box::new(simple(2, 1).with_label_format(
                RunFormat::default().with_style("DefaultParagraphFont"),
            ))),
        }],
        "reject-override-paragraph-heading" => vec![N::InstanceLevel {
            num_id: 90,
            level: 2,
            value: Some(Box::new(
                simple(2, 1).with_paragraph_format(ParagraphFormat::default().with_heading(1)),
            )),
        }],
        "reject-alias-level" | "reject-opaque-level" => vec![N::AbstractLevel {
            abstract_id: if action == "reject-alias-level" {
                711
            } else {
                710
            },
            level: 0,
            value: Some(Box::new(simple(0, 1))),
        }],
        "edit-owner-level" => vec![N::AbstractLevel {
            abstract_id: 710,
            level: 0,
            value: Some(Box::new(simple(0, 9))),
        }],
        "reject-owning-relink" => vec![N::RelinkInstance {
            num_id: 712,
            abstract_id: 711,
        }],
        "reject-lost-abstract-level" | "reject-opaque-abstract" => vec![N::ReplaceAbstract {
            abstract_id: 710,
            levels: vec![simple(u8::from(action == "reject-lost-abstract-level"), 1)],
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
        "reject-replace-numbering-style" => {
            styles.push(StyleEdit::Replace {
                target: doc.style_anchor("FirstListStyle")?,
                definition: NumberingStyleDefinition {
                    name: "First List Style".into(),
                    numbering_id: 90,
                }
                .into(),
            });
            Vec::new()
        }
        _ => panic!("unknown oracle action"),
    };
    let mut resources: Vec<_> = edits.into_iter().map(ResourceEdit::Numbering).collect();
    if action == "level-fields-clear" {
        resources.push(ResourceEdit::AbstractStart(AbstractStart {
            abstract_id: 80,
            level: 0,
            start: Some(9),
        }));
    } else if action == "replace" {
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
    if action.starts_with("reject-") {
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
