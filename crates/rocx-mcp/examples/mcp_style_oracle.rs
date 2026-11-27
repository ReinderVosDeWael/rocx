//! Direct Rust oracle for imported style editing.
use rocx::editor::{BodyBlock, EditBatch, ImportedDocument, ParagraphInsertion, StyleEdit};
use rocx::{
    Alignment, CharacterStyleDefinition, Color, Component, LineHeight, NumberingStyleDefinition,
    Paragraph, ParagraphFormat, ParagraphStyleDefinition, RunFormat, RunLanguage, TabAlignment,
    TabLeader, TabStop, TableSectionFormat, TableStyleDefinition, ThemeColor, Underline,
};

fn language() -> RunLanguage {
    RunLanguage {
        latin: Some("en-GB".into()),
        east_asia: Some("ja-JP".into()),
        bidi: Some("ar-SA".into()),
    }
}

fn tab() -> TabStop {
    TabStop {
        position: 36.,
        alignment: TabAlignment::Decimal,
        leader: Some(TabLeader::Dot),
    }
}

fn rich_region() -> TableSectionFormat {
    TableSectionFormat {
        background: Some([1, 2, 3]),
        bidi: Some(false),
        alignment: Some(Alignment::Center),
        spacing_before: Some(5.5),
        spacing_after: Some(2.),
        line_height: Some(LineHeight::AtLeast(12.5)),
        tab_stops: vec![tab()],
        bold: Some(false),
        italic: Some(true),
        underline: Some(true),
        font: Some("Arial".into()),
        font_size: Some(11.5),
        color: Some([10, 11, 12]),
        language: Some(language()),
        bold_complex_script: Some(true),
        italic_complex_script: Some(false),
        size_complex_script: Some(13.),
        rtl: Some(false),
        complex_script: Some(true),
        font_east_asia: Some("MS Gothic".into()),
        font_complex_script: Some("Tahoma".into()),
    }
}

fn table_with_region(region: &str) -> TableStyleDefinition {
    let table = TableStyleDefinition::default().with_name("Regional Table");
    let format = rich_region();
    match region {
        "wholeTable" => table.with_whole_table(format),
        "firstRow" => table.with_first_row(format),
        "lastRow" => table.with_last_row(format),
        "firstCol" => table.with_first_column(format),
        "lastCol" => table.with_last_column(format),
        "band1Horz" => table.with_banding_1_row(format),
        "band2Horz" => table.with_banding_2_row(format),
        "band1Vert" => table.with_banding_1_column(format),
        "band2Vert" => table.with_banding_2_column(format),
        "nwCell" => table.with_top_left_cell(format),
        "neCell" => table.with_top_right_cell(format),
        "swCell" => table.with_bottom_left_cell(format),
        "seCell" => table.with_bottom_right_cell(format),
        _ => panic!("unknown table region"),
    }
}

fn paragraph_style(mode: &str) -> ParagraphStyleDefinition {
    let line_height = match mode {
        "legacy" => None,
        "multiple" => Some(LineHeight::Multiple(1.25)),
        "exact" => Some(LineHeight::Exact(13.5)),
        "at-least" => Some(LineHeight::AtLeast(12.5)),
        _ => panic!("unknown line mode"),
    };
    let positive = matches!(mode, "legacy" | "exact");
    ParagraphStyleDefinition {
        name: "Detailed Paragraph".into(),
        base_style: Some("Destination".into()),
        bidi: Some(positive),
        alignment: Some(Alignment::Center),
        spacing_before: Some(5.5),
        spacing_after: Some(2.),
        line_spacing: (mode == "legacy").then_some(1.5),
        line_height,
        left_indent: Some(0.5),
        right_indent: Some(0.25),
        first_line_indent: Some(if positive { 0.125 } else { -0.125 }),
        keep_together: Some(positive),
        keep_with_next: Some(!positive),
        page_break_before: Some(positive),
        widow_control: Some(!positive),
        suppress_line_numbers: Some(positive),
        contextual_spacing: Some(!positive),
        mirror_indents: Some(positive),
        snap_to_grid: Some(!positive),
        tab_stops: vec![tab()],
        ..Default::default()
    }
}

fn rich_paragraph_text(style: ParagraphStyleDefinition) -> ParagraphStyleDefinition {
    style
        .with_bold(false)
        .with_italic(true)
        .with_underline(true)
        .with_font("Arial")
        .with_font_size(11.5)
        .with_color([10, 11, 12])
        .with_language(language())
        .with_bold_complex_script(true)
        .with_italic_complex_script(false)
        .with_size_complex_script(13.)
        .with_rtl(false)
        .with_complex_script(true)
        .with_font_east_asia("MS Gothic")
        .with_font_complex_script("Tahoma")
}

fn replacement_paragraph() -> ParagraphStyleDefinition {
    rich_paragraph_text(ParagraphStyleDefinition {
        name: "Destination".into(),
        base_style: Some("Normal".into()),
        next_paragraph_style: Some("Memo Style".into()),
        linked_style: Some("Swap Style".into()),
        ..paragraph_style("at-least")
    })
}

fn inspect(doc: &ImportedDocument) -> serde_json::Value {
    use rocx::editor::StyleKind;
    use serde_json::json;

    match doc.styles() {
        Ok(Some(catalog)) => {
            let styles: Vec<_> = catalog
                .styles
                .into_iter()
                .map(|row| {
                    let unsupported = match &row.kind {
                        StyleKind::Unsupported(value) => Some(value),
                        _ => None,
                    };
                    json!({
                        "id": row.id, "kind": row.kind.as_str(), "unsupported_kind": unsupported,
                        "name": row.name, "based_on": row.based_on, "next_style": row.next_style,
                        "linked_style": row.linked_style, "property_xml": row.property_xml,
                    })
                })
                .collect();
            json!({"part_name": catalog.part_name, "styles": styles})
        }
        Ok(None) => json!({"part_name": null, "styles": []}),
        Err(err) => json!({"error": err.to_string()}),
    }
}

fn direct_fields() -> Vec<(&'static str, &'static str, &'static str)> {
    let mut fields = vec![
        ("MemoStyle", "basedOn", "Destination"),
        ("MemoStyle", "next", "Destination"),
        ("MemoStyle", "link", "SwapStyle"),
        ("MemoStyle", "jc", "right"),
        ("MemoStyle", "tab.-40.leader", "middleDot"),
        ("MemoStyle", "tab.-40.val", "end"),
        ("Destination", "ind.hanging", "80"),
        ("Destination", "ind.hangingChars", "50"),
    ];
    fields.extend(
        [
            "b",
            "i",
            "bCs",
            "iCs",
            "rtl",
            "cs",
            "strike",
            "caps",
            "smallCaps",
            "vanish",
            "keepNext",
            "keepLines",
            "pageBreakBefore",
            "widowControl",
            "bidi",
            "contextualSpacing",
            "mirrorIndents",
            "snapToGrid",
            "suppressLineNumbers",
        ]
        .into_iter()
        .enumerate()
        .map(|(i, key)| ("MemoStyle", key, if i % 2 == 0 { "0" } else { "1" })),
    );
    fields.extend(
        [
            ("rFonts.ascii", "Arial"),
            ("rFonts.hAnsi", "Calibri"),
            ("rFonts.eastAsia", "MS Gothic"),
            ("rFonts.cs", "Tahoma"),
            ("rFonts.asciiTheme", "majorAscii"),
            ("rFonts.hAnsiTheme", "minorHAnsi"),
            ("rFonts.eastAsiaTheme", "majorEastAsia"),
            ("rFonts.cstheme", "minorBidi"),
            ("lang.val", "en-GB"),
            ("lang.eastAsia", "ja-JP"),
            ("lang.bidi", "ar-SA"),
            ("sz.val", "23"),
            ("szCs.val", "27"),
            ("vertAlign.val", "superscript"),
            ("runSnapToGrid.val", "0"),
            ("color.themeColor", "accent2"),
            ("color.themeTint", "3A"),
            ("color.themeShade", "C4"),
            ("color.val", "012345"),
            ("u.color", "654321"),
            ("u.themeColor", "accent5"),
            ("u.themeTint", "59"),
            ("u.themeShade", "A6"),
            ("u.val", "wavyDouble"),
            ("spacing.before", "110"),
            ("spacing.after", "40"),
            ("spacing.beforeLines", "50"),
            ("spacing.afterLines", "125"),
            ("spacing.beforeAutospacing", "0"),
            ("spacing.afterAutospacing", "1"),
            ("spacing.line", "270"),
            ("spacing.lineRule", "exact"),
            ("ind.left", "144"),
            ("ind.right", "288"),
            ("ind.start", "-72"),
            ("ind.end", "36"),
            ("ind.leftChars", "100"),
            ("ind.rightChars", "200"),
            ("ind.startChars", "-50"),
            ("ind.endChars", "25"),
            ("ind.firstLine", "180"),
            ("ind.firstLineChars", "75"),
        ]
        .into_iter()
        .map(|(key, value)| ("MemoStyle", key, value)),
    );
    fields.extend(
        [
            ("shd.fill", "102030"),
            ("shd.color", "405060"),
            ("shd.themeColor", "accent1"),
            ("shd.themeFill", "accent6"),
            ("shd.themeTint", "33"),
            ("shd.themeShade", "CC"),
            ("shd.themeFillTint", "44"),
            ("shd.themeFillShade", "BB"),
            ("shd.val", "pct25"),
        ]
        .into_iter()
        .map(|(key, value)| ("Matrix", key, value)),
    );
    fields
}

fn main() -> rocx::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let doc = ImportedDocument::open(&std::fs::read(args.next().unwrap()).unwrap())?;
    let action = args.next().unwrap().into_string().unwrap();
    if action == "inspect" {
        println!("{}", inspect(&doc));
        return Ok(());
    }
    let output = args.next().unwrap();
    let anchor = |id| doc.style_anchor(id).unwrap();
    let field = |id, key: &str, value: Option<&str>| StyleEdit::Field {
        target: anchor(id),
        region: None,
        field: key.into(),
        value: value.map(str::to_owned),
    };
    let region = TableSectionFormat::default()
        .with_bold(false)
        .with_italic(true)
        .with_alignment(Alignment::Center)
        .with_background([1, 2, 3]);
    let mut paragraphs = Vec::new();
    let (action, key) = action.split_once(':').unwrap_or((&action, ""));
    let styles = match action {
        "mixed" => {
            let BodyBlock::Paragraph(p) = &doc.body()[0] else {
                panic!("expected fixture paragraph")
            };
            let paragraph = futures_executor::block_on(
                Paragraph::new("Staged body")
                    .with_format(ParagraphFormat::default().with_style("Staged Paragraph"))
                    .resolve(),
            )?
            .unwrap();
            paragraphs.push(ParagraphInsertion {
                target: p.anchor.clone(),
                paragraph,
            });
            vec![
                StyleEdit::Add(
                    ParagraphStyleDefinition::default()
                        .with_name("Staged Paragraph")
                        .with_next_paragraph_style("Destination")
                        .with_keep_with_next(true)
                        .with_line_height(LineHeight::Exact(14.))
                        .with_bold(false)
                        .with_color([9, 8, 7])
                        .into(),
                ),
                StyleEdit::Add(
                    TableStyleDefinition::default()
                        .with_name("Staged Table")
                        .with_banding_2_column(region.clone())
                        .into(),
                ),
                StyleEdit::Add(
                    NumberingStyleDefinition {
                        name: "Staged List".into(),
                        numbering_id: 1,
                    }
                    .into(),
                ),
                StyleEdit::Replace {
                    target: anchor("SwapStyle"),
                    definition: CharacterStyleDefinition::default()
                        .with_name("Swap Style")
                        .with_format(RunFormat::default().with_bold(true).with_font("Arial"))
                        .into(),
                },
                StyleEdit::Patch {
                    target: anchor("MemoStyle"),
                    bold: Some(false),
                    italic: Some(true),
                },
                StyleEdit::Field {
                    target: anchor("Destination"),
                    region: None,
                    field: "jc".into(),
                    value: Some("center".into()),
                },
                StyleEdit::Region {
                    target: anchor("Matrix"),
                    region: "firstRow".into(),
                    format: Some(Box::new(region)),
                },
                StyleEdit::Remove(anchor("Spare")),
            ]
        }
        "clear" => vec![
            StyleEdit::Region {
                target: anchor("Matrix"),
                region: "firstRow".into(),
                format: None,
            },
            StyleEdit::Field {
                target: anchor("Destination"),
                region: None,
                field: "jc".into(),
                value: None,
            },
        ],
        "retarget" => vec![StyleEdit::Retarget {
            target: anchor("MemoStyle"),
            replacement: anchor("Destination"),
        }],
        "create" => vec![StyleEdit::Add(
            ParagraphStyleDefinition::default()
                .with_name("First Style")
                .into(),
        )],
        "region-set" => vec![
            StyleEdit::Add(table_with_region(key).into()),
            StyleEdit::Region {
                target: anchor("Matrix"),
                region: key.into(),
                format: Some(Box::new(rich_region())),
            },
        ],
        "region-fields" => [
            ("rFonts.cs", Some("Arial Unicode MS")),
            ("tab.720.leader", None),
            ("shd.fill", Some("A0B1C2")),
        ]
        .into_iter()
        .map(|(field, value)| StyleEdit::Field {
            target: anchor("Matrix"),
            region: Some(key.into()),
            field: field.into(),
            value: value.map(str::to_owned),
        })
        .collect(),
        "region-clear" => vec![StyleEdit::Region {
            target: anchor("Matrix"),
            region: key.into(),
            format: None,
        }],
        "paragraph-format" => vec![StyleEdit::Add(paragraph_style(key).into())],
        "replace-rich" => vec![
            StyleEdit::Replace {
                target: anchor("Destination"),
                definition: replacement_paragraph().into(),
            },
            StyleEdit::Replace {
                target: anchor("Spare"),
                definition: TableStyleDefinition::default()
                    .with_name("Spare")
                    .with_base_style("Matrix")
                    .with_whole_table(rich_region())
                    .with_first_row(rich_region())
                    .into(),
            },
        ],
        "reject-line-spacing-conflict" => {
            let mut definition = replacement_paragraph();
            definition.line_spacing = Some(1.5);
            if key == "add" {
                definition.name = "Conflicting Paragraph".into();
                vec![StyleEdit::Add(definition.into())]
            } else {
                assert_eq!(key, "replace");
                vec![StyleEdit::Replace {
                    target: anchor("Destination"),
                    definition: definition.into(),
                }]
            }
        }
        "reject-replacement-identity" => vec![StyleEdit::Replace {
            target: anchor("Destination"),
            definition: replacement_paragraph()
                .with_name("Renamed Paragraph")
                .into(),
        }],
        "reject-live-replacement" => vec![StyleEdit::Replace {
            target: anchor("MemoStyle"),
            definition: replacement_paragraph()
                .with_name("Memo Style")
                .with_next_paragraph_style("Normal")
                .into(),
        }],
        "style-definition-properties" => vec![
            StyleEdit::Add(
                CharacterStyleDefinition::default()
                    .with_name("Editorial Character")
                    .with_base_style("Default Paragraph Font")
                    .with_linked_style("Memo Style")
                    .with_format(
                        RunFormat::default()
                            .with_bold(true)
                            .with_italic(false)
                            .with_underline(Underline::Dash)
                            .with_font("Georgia")
                            .with_size(9.5)
                            .with_color(Color::Theme {
                                name: ThemeColor::Accent3,
                                tint: None,
                                shade: Some(0xA4),
                            })
                            .with_superscript(true)
                            .with_strike(false)
                            .with_all_caps(true)
                            .with_small_caps(false)
                            .with_snap_to_grid(false)
                            .with_language(language())
                            .with_bold_complex_script(false)
                            .with_italic_complex_script(true)
                            .with_size_complex_script(14.)
                            .with_rtl(true)
                            .with_complex_script(false)
                            .with_font_east_asia("Yu Gothic")
                            .with_font_complex_script("Tahoma"),
                    )
                    .into(),
            ),
            StyleEdit::Add(
                rich_paragraph_text(
                    ParagraphStyleDefinition::default()
                        .with_name("Editorial Paragraph")
                        .with_linked_style("Swap Style"),
                )
                .into(),
            ),
            StyleEdit::Add(
                TableStyleDefinition::default()
                    .with_name("Editorial Table")
                    .with_base_style("Matrix")
                    .into(),
            ),
        ],
        "direct-fields-set" | "direct-fields-clear" => direct_fields()
            .into_iter()
            .map(|(id, key, value)| {
                field(id, key, (action == "direct-fields-set").then_some(value))
            })
            .collect(),
        "reject-style-cycle" => vec![field("Destination", "basedOn", Some("MemoStyle"))],
        "reject-required-style-val" => vec![field("MemoStyle", "color.val", None)],
        _ => panic!("unknown oracle action"),
    };
    let preview = doc.preview_edits(EditBatch {
        styles: &styles,
        paragraphs: &paragraphs,
        ..Default::default()
    });
    if action.starts_with("reject-") {
        let Err(error) = preview else {
            panic!("native style guard accepted {action}")
        };
        std::fs::write(output, error.to_string()).unwrap();
    } else {
        std::fs::write(output, preview?.to_docx()?).unwrap();
    }
    Ok(())
}
