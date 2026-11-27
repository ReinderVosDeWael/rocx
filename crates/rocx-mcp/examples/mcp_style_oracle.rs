//! Direct Rust oracle for imported style editing.
use rocx::editor::{BodyBlock, EditBatch, ImportedDocument, ParagraphInsertion, StyleEdit};
use rocx::{
    Alignment, CharacterStyleDefinition, Component, LineHeight, NumberingStyleDefinition,
    Paragraph, ParagraphFormat, ParagraphStyleDefinition, RunFormat, RunLanguage, TabAlignment,
    TabLeader, TabStop, TableSectionFormat, TableStyleDefinition,
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
        _ => panic!("unknown oracle action"),
    };
    let preview = doc.preview_edits(EditBatch {
        styles: &styles,
        paragraphs: &paragraphs,
        ..Default::default()
    })?;
    std::fs::write(output, preview.to_docx()?).unwrap();
    Ok(())
}
