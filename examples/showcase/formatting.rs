use super::*;

pub fn blocks(out: &mut BlockChildren) {
    out.push(heading("01 Paragraphs and headings"));
    for level in 0..=9 {
        out.push(
            Paragraph::new(format!(
                "This is a Heading {level}{}",
                if level == 0 {
                    " (heading=0: no heading style)"
                } else {
                    ""
                }
            ))
            .with_format(ParagraphFormat {
                heading: Some(level),
                ..Default::default()
            })
            .into(),
        );
    }
    for (alignment, name) in [
        (Alignment::Left, "left"),
        (Alignment::Center, "center"),
        (Alignment::Right, "right"),
        (Alignment::Justify, "justify"),
        (Alignment::Distribute, "distribute"),
        (Alignment::JustifyMedium, "justifyMedium"),
        (Alignment::JustifyHigh, "justifyHigh"),
        (Alignment::JustifyLow, "justifyLow"),
        (Alignment::ThaiJustify, "thaiJustify"),
    ] {
        out.push(Paragraph::new(format!("Paragraph alignment={name}: The quick brown fox jumps over the lazy dog. Ελληνικά العربية 日本語." )).with_format(ParagraphFormat { alignment: Some(alignment), ..Default::default() }).into());
    }
    for (height, name) in [
        (LineHeight::Multiple(1.5), "multiple 1.5"),
        (LineHeight::Exact(20.), "exact 20 pt"),
        (LineHeight::AtLeast(18.), "at_least 18 pt"),
    ] {
        out.push(
            Paragraph::from_children(vec![
                TextRun::new(format!("Line height={name}; before=6 pt; after=8 pt")).into(),
                Break::default().into(),
                TextRun::new("Second line makes the line-height request visible.").into(),
            ])
            .with_format(ParagraphFormat {
                line_height: Some(height),
                spacing_before: Some(6.),
                spacing_after: Some(8.),
                ..Default::default()
            })
            .into(),
        );
    }
    out.push(Paragraph::new("Legacy line_spacing=1.25; left=18 pt, right=12 pt, hanging=6 pt; keep lines/next and widow control enabled.").with_format(ParagraphFormat { line_spacing: Some(1.25), left_indent: Some(18.), right_indent: Some(12.), first_line_indent: Some(-6.), keep_together: Some(true), keep_with_next: Some(true), widow_control: Some(true), ..Default::default() }).into());
    out.push(Paragraph::new("First-line indent=18 pt; keep lines/next, page-break-before and widow control explicitly false.").with_format(ParagraphFormat { first_line_indent: Some(18.), keep_together: Some(false), keep_with_next: Some(false), page_break_before: Some(false), widow_control: Some(false), bidi: Some(false), ..Default::default() }).into());
    for (a, name) in [
        (TabAlignment::Left, "left"),
        (TabAlignment::Center, "center"),
        (TabAlignment::Right, "right"),
        (TabAlignment::Decimal, "decimal"),
        (TabAlignment::Bar, "bar"),
        (TabAlignment::Clear, "clear"),
        (TabAlignment::Start, "start"),
        (TabAlignment::End, "end"),
    ] {
        out.push(
            Paragraph::from_children(vec![
                TextRun::new(format!("Tab {name}")).into(),
                Tab::default().into(),
                TextRun::new("123.45").into(),
            ])
            .with_format(ParagraphFormat {
                tab_stops: vec![TabStop {
                    position: 216.,
                    alignment: a,
                    leader: None,
                }],
                ..Default::default()
            })
            .into(),
        );
    }
    for (leader, name) in [
        (TabLeader::None, "none"),
        (TabLeader::Dot, "dot"),
        (TabLeader::Hyphen, "hyphen"),
        (TabLeader::Underscore, "underscore"),
        (TabLeader::Heavy, "heavy"),
        (TabLeader::MiddleDot, "middle_dot"),
    ] {
        out.push(
            Paragraph::from_children(vec![
                TextRun::new(format!("Tab leader={name}")).into(),
                Tab::default().into(),
                TextRun::new("at 288 pt").into(),
            ])
            .with_format(ParagraphFormat {
                tab_stops: vec![
                    TabStop {
                        position: 288.,
                        alignment: TabAlignment::Right,
                        leader: Some(leader),
                    },
                    TabStop {
                        position: -12.,
                        alignment: TabAlignment::Clear,
                        leader: None,
                    },
                ],
                ..Default::default()
            })
            .into(),
        );
    }
    out.push(heading("02 Run formatting, colors, scripts and whitespace"));
    for (name, format) in [
        (
            "Bold=true",
            RunFormat {
                bold: Some(true),
                ..Default::default()
            },
        ),
        (
            "Italic=true",
            RunFormat {
                italic: Some(true),
                ..Default::default()
            },
        ),
        (
            "Strike=true",
            RunFormat {
                strike: Some(true),
                ..Default::default()
            },
        ),
        (
            "ALL CAPS=true",
            RunFormat {
                all_caps: Some(true),
                ..Default::default()
            },
        ),
        (
            "Small caps=true",
            RunFormat {
                small_caps: Some(true),
                ..Default::default()
            },
        ),
        (
            "Superscript=true",
            RunFormat {
                superscript: Some(true),
                ..Default::default()
            },
        ),
        (
            "Subscript=true",
            RunFormat {
                subscript: Some(true),
                ..Default::default()
            },
        ),
        (
            "Arial 16 pt, RGB 803020",
            RunFormat {
                font: Some("Arial".into()),
                size: Some(16.),
                color: Some([128, 48, 32].into()),
                ..Default::default()
            },
        ),
        (
            "Automatic color",
            RunFormat {
                color: Some(Color::Auto),
                ..Default::default()
            },
        ),
    ] {
        out.push(run(name, format));
    }
    for (u, name) in [
        (Underline::None, "none"),
        (Underline::Single, "single"),
        (Underline::Double, "double"),
        (Underline::Dotted, "dotted"),
        (Underline::Dash, "dash"),
        (Underline::Wave, "wave"),
        (Underline::Words, "words"),
    ] {
        out.push(run(
            &format!("Underline={name}: words with spaces"),
            RunFormat {
                underline: Some(u),
                ..Default::default()
            },
        ));
    }
    for (color, name) in [
        (ThemeColor::Dark1, "dark1"),
        (ThemeColor::Light1, "light1"),
        (ThemeColor::Dark2, "dark2"),
        (ThemeColor::Light2, "light2"),
        (ThemeColor::Accent1, "accent1"),
        (ThemeColor::Accent2, "accent2"),
        (ThemeColor::Accent3, "accent3"),
        (ThemeColor::Accent4, "accent4"),
        (ThemeColor::Accent5, "accent5"),
        (ThemeColor::Accent6, "accent6"),
        (ThemeColor::Hyperlink, "hyperlink"),
        (ThemeColor::FollowedHyperlink, "followedHyperlink"),
        (ThemeColor::Background1, "background1"),
        (ThemeColor::Text1, "text1"),
        (ThemeColor::Background2, "background2"),
        (ThemeColor::Text2, "text2"),
    ] {
        out.push(
            Paragraph::from_children(vec![
                TextRun::new(format!(
                    "Theme {name} (light colors may be invisible on white): "
                ))
                .into(),
                TextRun::new("colored text")
                    .with_format(RunFormat {
                        color: Some(Color::Theme {
                            name: color,
                            tint: Some(32),
                            shade: Some(224),
                        }),
                        ..Default::default()
                    })
                    .into(),
            ])
            .into(),
        );
    }
    out.push(run("Explicit false overrides: bold, italic, strike, caps, small caps, superscript, subscript and underline disabled.",RunFormat { bold: Some(false), italic: Some(false), strike: Some(false), all_caps: Some(false), small_caps: Some(false), superscript: Some(false), subscript: Some(false), underline: Some(Underline::None), ..Default::default() }));
    out.push(
        Paragraph::from_children(vec![
            TextRun::new("  Preserved leading and trailing spaces  ").into(),
            TextRun::new("").into(),
            TextRun::new("Unicode: café Ελληνικά 日本語 العربية; literal tab\tand newline\nend.")
                .into(),
        ])
        .into(),
    );
    out.push(
        Paragraph::from_children(vec![
            TextRun::new("Run language/script: en-US / ja-JP / ar-SA; العربية 日本語")
                .with_format(RunFormat {
                    language: Some(RunLanguage {
                        latin: Some("en-US".into()),
                        east_asia: Some("ja-JP".into()),
                        bidi: Some("ar-SA".into()),
                    }),
                    bold_complex_script: Some(true),
                    italic_complex_script: Some(false),
                    size_complex_script: Some(14.),
                    rtl: Some(true),
                    complex_script: Some(true),
                    font_east_asia: Some("Yu Mincho".into()),
                    font_complex_script: Some("Arial".into()),
                    ..Default::default()
                })
                .into(),
        ])
        .with_format(ParagraphFormat {
            bidi: Some(true),
            ..Default::default()
        })
        .into(),
    );
    out.push(run("Run RTL=false, complex_script=false, bCs=false, iCs=true; independent from paragraph direction.", RunFormat { rtl: Some(false), complex_script: Some(false), bold_complex_script: Some(false), italic_complex_script: Some(true), ..Default::default() }));
    out.push(heading(
        "03 Styles, inheritance, links and direct overrides",
    ));
    out.push(
        Paragraph::from_children(vec![
            TextRun::new("Paragraph style Catalog Child (forward base/next/link); ").into(),
            TextRun::new("character style Catalog Character with bold=false override")
                .with_format(RunFormat {
                    style: Some("Catalog Character".into()),
                    bold: Some(false),
                    ..Default::default()
                })
                .into(),
        ])
        .with_format(ParagraphFormat {
            style: Some("Catalog Child".into()),
            ..Default::default()
        })
        .into(),
    );
    out.push(Paragraph::from_children(vec![TextRun::new("Updated template paragraph/character styles: original bold retained, italic added.").with_format(RunFormat { style:Some("Catalog Existing Character".into()), ..Default::default() }).into()]).with_format(ParagraphFormat { style:Some("Catalog Existing".into()), ..Default::default() }).into());
}

pub fn styles() -> Vec<StyleDefinition> {
    vec![
        ParagraphStyleDefinition {
            name: "Catalog Child".into(),
            base_style: Some("Catalog Base".into()),
            next_paragraph_style: Some("Normal".into()),
            linked_style: Some("Catalog Character".into()),
            line_height: Some(LineHeight::AtLeast(16.)),
            tab_stops: vec![TabStop {
                position: 72.,
                ..Default::default()
            }],
            bidi: Some(false),
            ..Default::default()
        }
        .into(),
        CharacterStyleDefinition {
            name: "Catalog Character".into(),
            base_style: Some("Catalog Character Base".into()),
            linked_style: Some("Catalog Child".into()),
            format: RunFormat {
                bold: Some(true),
                italic: Some(true),
                color: Some([48, 64, 144].into()),
                ..Default::default()
            },
        }
        .into(),
        ParagraphStyleDefinition {
            name: "Catalog Base".into(),
            base_style: Some("Normal".into()),
            font: Some("Arial".into()),
            font_size: Some(12.),
            bold: Some(true),
            italic: Some(false),
            underline: Some(false),
            color: Some([24, 64, 96]),
            alignment: Some(Alignment::Left),
            spacing_before: Some(6.),
            spacing_after: Some(6.),
            left_indent: Some(0.25),
            right_indent: Some(0.125),
            first_line_indent: Some(-0.125),
            keep_together: Some(true),
            keep_with_next: Some(false),
            page_break_before: Some(false),
            widow_control: Some(true),
            language: Some(RunLanguage {
                latin: Some("en-US".into()),
                ..Default::default()
            }),
            bold_complex_script: Some(false),
            italic_complex_script: Some(false),
            size_complex_script: Some(12.),
            rtl: Some(false),
            complex_script: Some(false),
            font_east_asia: Some("Yu Mincho".into()),
            font_complex_script: Some("Arial".into()),
            ..Default::default()
        }
        .into(),
        CharacterStyleDefinition {
            name: "Catalog Character Base".into(),
            format: RunFormat {
                font: Some("Calibri".into()),
                size: Some(12.),
                ..Default::default()
            },
            ..Default::default()
        }
        .into(),
        ParagraphStyleDefinition {
            name: "Catalog Existing".into(),
            italic: Some(true),
            ..Default::default()
        }
        .into(),
        CharacterStyleDefinition {
            name: "Catalog Existing Character".into(),
            format: RunFormat {
                italic: Some(true),
                ..Default::default()
            },
            ..Default::default()
        }
        .into(),
    ]
}
