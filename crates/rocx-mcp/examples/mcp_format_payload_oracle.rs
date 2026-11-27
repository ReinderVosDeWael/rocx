//! Direct Rust oracle for modeled run and paragraph formatting declarations.
use rocx::{
    Alignment, Color, Component, LineHeight, Paragraph, ParagraphFormat, RunFormat, RunLanguage,
    TabAlignment, TabLeader, TabStop, TextRun, ThemeColor, Underline,
    editor::{BlockEdit, BodyBlock, EditBatch, ImportedDocument},
};

fn rich_runs() -> Vec<TextRun> {
    let mut runs = vec![
        TextRun::new("Rich run").with_format(RunFormat {
            style: Some("PayloadCharacter".into()),
            bold: Some(false),
            italic: Some(true),
            underline: Some(Underline::Double),
            font: Some("Latin Font".into()),
            size: Some(11.5),
            color: Some(Color::Theme {
                name: ThemeColor::Accent2,
                tint: Some(59),
                shade: None,
            }),
            superscript: Some(true),
            subscript: Some(false),
            strike: Some(true),
            all_caps: Some(false),
            small_caps: Some(true),
            snap_to_grid: Some(false),
            language: Some(RunLanguage {
                latin: Some("en-US".into()),
                east_asia: Some("ja-JP".into()),
                bidi: Some("ar-SA".into()),
            }),
            bold_complex_script: Some(true),
            italic_complex_script: Some(false),
            size_complex_script: Some(13.5),
            rtl: Some(false),
            complex_script: Some(true),
            font_east_asia: Some("East Asian Font".into()),
            font_complex_script: Some("Complex Font".into()),
        }),
        TextRun::new(" Subscript").with_format(RunFormat {
            color: Some(Color::Rgb([1, 128, 255])),
            superscript: Some(false),
            subscript: Some(true),
            ..Default::default()
        }),
        TextRun::new(" Automatic").with_format(RunFormat {
            color: Some(Color::Auto),
            ..Default::default()
        }),
    ];
    for (index, name) in [
        ThemeColor::Dark1,
        ThemeColor::Light1,
        ThemeColor::Dark2,
        ThemeColor::Light2,
        ThemeColor::Accent1,
        ThemeColor::Accent3,
        ThemeColor::Accent4,
        ThemeColor::Accent5,
        ThemeColor::Accent6,
        ThemeColor::Hyperlink,
        ThemeColor::FollowedHyperlink,
        ThemeColor::Background1,
        ThemeColor::Text1,
        ThemeColor::Background2,
        ThemeColor::Text2,
    ]
    .into_iter()
    .enumerate()
    {
        let underline = [
            Underline::None,
            Underline::Single,
            Underline::Dotted,
            Underline::Dash,
            Underline::Wave,
            Underline::Words,
        ]
        .get(index)
        .copied();
        runs.push(
            TextRun::new(format!(" Swatch {index}")).with_format(RunFormat {
                underline,
                color: Some(Color::Theme {
                    name,
                    tint: None,
                    shade: (index == 0).then_some(150),
                }),
                ..Default::default()
            }),
        );
    }
    runs
}

fn rich_paragraph() -> ParagraphFormat {
    let tab_stops = [
        (70., TabAlignment::End, None),
        (60., TabAlignment::Start, Some(TabLeader::MiddleDot)),
        (50., TabAlignment::Clear, None),
        (40., TabAlignment::Bar, Some(TabLeader::Heavy)),
        (30., TabAlignment::Decimal, Some(TabLeader::Underscore)),
        (20., TabAlignment::Right, Some(TabLeader::Hyphen)),
        (10., TabAlignment::Center, Some(TabLeader::Dot)),
        (-9.5, TabAlignment::Left, Some(TabLeader::None)),
    ]
    .map(|(position, alignment, leader)| TabStop {
        position,
        alignment,
        leader,
    });
    ParagraphFormat {
        bidi: Some(true),
        heading: None,
        style: Some("PayloadParagraph".into()),
        alignment: Some(Alignment::JustifyMedium),
        spacing_before: Some(4.25),
        spacing_after: Some(6.5),
        line_spacing: None,
        line_height: Some(LineHeight::AtLeast(12.25)),
        left_indent: Some(-3.5),
        right_indent: Some(8.25),
        first_line_indent: Some(-2.5),
        keep_together: Some(true),
        keep_with_next: Some(false),
        page_break_before: Some(true),
        widow_control: Some(false),
        suppress_line_numbers: Some(true),
        contextual_spacing: Some(false),
        mirror_indents: Some(true),
        snap_to_grid: Some(false),
        tab_stops: tab_stops.into(),
    }
}

fn main() -> rocx::Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        panic!("usage: mcp_format_payload_oracle SOURCE OUTPUT");
    };
    let document = ImportedDocument::open(&std::fs::read(source)?)?;
    let formats = [
        rich_paragraph(),
        ParagraphFormat {
            heading: Some(2),
            style: Some("PayloadParagraph".into()),
            alignment: Some(Alignment::Center),
            line_spacing: Some(1.25),
            first_line_indent: Some(3.5),
            ..Default::default()
        },
        ParagraphFormat {
            alignment: Some(Alignment::Justify),
            line_height: Some(LineHeight::Exact(13.5)),
            ..Default::default()
        },
        ParagraphFormat {
            alignment: Some(Alignment::Distribute),
            line_height: Some(LineHeight::Multiple(1.5)),
            ..Default::default()
        },
        ParagraphFormat::default().with_alignment(Alignment::Left),
        ParagraphFormat::default().with_alignment(Alignment::Right),
        ParagraphFormat::default().with_alignment(Alignment::JustifyHigh),
        ParagraphFormat::default().with_alignment(Alignment::JustifyLow),
        ParagraphFormat::default().with_alignment(Alignment::ThaiJustify),
    ];
    let mut blocks = Vec::new();
    for (index, format) in formats.into_iter().enumerate() {
        let BodyBlock::Paragraph(original) = &document.body()[index] else {
            panic!("expected fixture paragraph at {index}");
        };
        let runs = if index == 0 {
            rich_runs()
        } else {
            vec![TextRun::new(format!("Replacement {index}"))]
        };
        let paragraph = futures_executor::block_on(
            Paragraph::from_children(runs.into_iter().map(Into::into).collect())
                .with_format(format)
                .resolve(),
        )?
        .expect("unconditional fixture paragraph");
        blocks.push(BlockEdit::Replace(
            original.anchor.clone(),
            Box::new(paragraph),
        ));
    }
    let preview = document.preview_edits(EditBatch {
        blocks: &blocks,
        ..Default::default()
    })?;
    std::fs::write(output, preview.to_docx()?)?;
    Ok(())
}
