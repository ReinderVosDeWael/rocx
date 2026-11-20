use super::*;
fn link(label: &str) -> Result<Value<Inline>> {
    Ok(Hyperlink::new(
        HyperlinkTarget::external("https://example.com/inspection?q=docx#sample")?,
        vec![
            TextRun::new(label)
                .with_format(RunFormat {
                    color: Some(Color::Theme {
                        name: ThemeColor::Hyperlink,
                        tint: None,
                        shade: None,
                    }),
                    underline: Some(Underline::Single),
                    ..Default::default()
                })
                .into(),
        ],
    )
    .with_properties(HyperlinkProperties {
        tooltip: Some("External link; never fetched by the library".into()),
        history: Some(true),
    })
    .into())
}
pub fn blocks(out: &mut BlockChildren) -> Result<()> {
    out.push(heading(
        "06 Lists, hyperlinks, bookmarks, comments, notes and cached fields",
    ));
    out.push(p("Concrete numbering-level replacements (numId=60): level 0 uses lowerLetter, text %1), replacement start=3 plus separate startOverride=7; level 1 uses upperRoman, text (%2), start=4. Abstract formats/starts are unchanged. The viewer computes the resulting labels."));
    out.push(p("Legal numbering requests: abstract level 2 and replacement level 1 enable decimal display of referenced levels; replacement level 0 explicitly disables it. Other levels omit the request. numFmt tokens remain unchanged; the viewer renders labels."));
    out.push(p("Numbering suffix requests: abstract level 2 explicitly uses tab; replacement level 0 uses space; replacement level 1 uses nothing. Other levels omit suff (consumer default: tab). Separators are not inserted into paragraph text by the library."));
    out.push(p("Level restart requests: abstract level 2 and replacement level 0 use 0 (never restart); replacement level 1 uses trigger 1 (one-based first level). Other levels omit lvlRestart (restart after the preceding level). This is independent of concrete startOverride; the viewer calculates counters."));
    out.push(p("Numbering-label alignment: abstract levels 0..4 request left, center, right, start, end; replacement levels 0/1 request end/start. Remaining levels omit lvlJc (direction-dependent consumer default). Paragraph text alignment is independent; the viewer positions generated labels."));
    for level in 0..9 {
        let mut paragraph = Paragraph::new(format!(
            "Numbered list: concrete numId=60 level={level}; abstract start=1; concrete level 0 restart=7, level 8 restart=0. Labels are computed by your viewer."
        ));
        paragraph.numbering = Some(NumberingReference { id: 60, level }).into();
        out.push(paragraph.into());
    }
    let mut bullet = Paragraph::new("Bullet list: numId=61, literal bullet label.");
    bullet.numbering = Some(NumberingReference { id: 61, level: 0 }).into();
    out.push(bullet.into());
    out.push(
        Paragraph::from_children(vec![link(
            "External hyperlink: example.com (tooltip and history=true)",
        )?])
        .into(),
    );
    let name = BookmarkName::new("CatalogBookmark")?;
    out.push(
        Paragraph::from_children(vec![
            Hyperlink::new(
                HyperlinkTarget::bookmark(name.clone()),
                vec![TextRun::new("Internal hyperlink: jump to CatalogBookmark").into()],
            )
            .with_properties(HyperlinkProperties {
                history: Some(false),
                ..Default::default()
            })
            .into(),
        ])
        .into(),
    );
    let overlap = BookmarkName::new("OverlappingBookmark")?;
    let review = RangeId::new("cross-paragraph-review")?;
    out.push(
        Paragraph::from_children(vec![
            RangeStart::bookmark(name.clone()).into(),
            TextRun::new("Bookmark CatalogBookmark starts here. ").into(),
            RangeStart::bookmark(overlap.clone()).into(),
            RangeStart::comment(
                review.clone(),
                ResolvedComment {
                    text: "Cross-paragraph plain comment".into(),
                    author: Some("Range reviewer".into()),
                },
            )
            .into(),
            TextRun::new("Overlapping bookmark and comment begin.").into(),
        ])
        .into(),
    );
    out.push(
        Paragraph::from_children(vec![
            TextRun::new("Cross-paragraph bookmark ends; overlapping bookmark continues. ").into(),
            RangeEnd::bookmark(&name).into(),
            RangeEnd::bookmark(&overlap).into(),
            RangeEnd::new(review).into(),
        ])
        .into(),
    );
    let structured = RangeId::new("structured-review")?;
    let comment = Comment::new(vec![
        p("Structured comment: paragraph, table, image and hyperlink."),
        Table::new(vec![TableRow::new(vec![cell("Comment table")]).into()]).into(),
        Paragraph::from_children(vec![
            image(
                "png",
                Some(ImageTransformation {
                    width: Some(18.),
                    height: None,
                }),
            )
            .into(),
            link("Comment external link")?,
        ])
        .into(),
    ])
    .with_metadata(CommentMetadata {
        author: Some("Structured reviewer".into()),
        date: Some(STAMP.into()),
        initials: Some("SR".into()),
    });
    out.push(
        Paragraph::from_children(vec![
            RangeStart::structured_comment(structured.clone(), comment).into(),
            TextRun::new("Structured comment anchor: inspect the comment pane.").into(),
            RangeEnd::new(structured).into(),
        ])
        .into(),
    );
    let mut convenience = Paragraph::from_children(vec![
        TextRun {
            text: "Run comment convenience anchor".into(),
            comment_text: Some("Run convenience comment".into()),
            comment_author: Some("Run reviewer".into()),
            ..Default::default()
        }
        .into(),
    ]);
    convenience.comment_text = Some("Paragraph convenience comment".into());
    convenience.comment_author = Some("Paragraph reviewer".into());
    out.push(convenience.into());
    out.push(
        Paragraph::from_children(vec![
            TextRun::new("Footnote reference").into(),
            NoteReference::footnote(NoteId::new(0)).into(),
            TextRun::new("; endnote reference").into(),
            NoteReference::endnote(NoteId::new(0)).into(),
        ])
        .into(),
    );
    for (instruction, label) in [
        (FieldInstruction::Page, "PAGE"),
        (FieldInstruction::NumPages, "NUMPAGES"),
        (FieldInstruction::Reference(name), "REF CatalogBookmark"),
        (FieldInstruction::TableOfContents, "TOC"),
        (FieldInstruction::raw(" DATE ")?, "raw DATE"),
    ] {
        for (form, form_label) in [
            (FieldForm::Simple, "simple"),
            (FieldForm::Complex, "complex"),
        ] {
            out.push(
                Paragraph::from_children(vec![
                    TextRun::new(format!("Field {form_label} {label}: ")).into(),
                    Field::new(
                        instruction.clone(),
                        vec![TextRun::new(format!("[{label} cached placeholder]")).into()],
                    )
                    .with_properties(FieldProperties {
                        form,
                        dirty: Some(true),
                        locked: Some(false),
                    })
                    .into(),
                ])
                .into(),
            );
        }
    }
    out.push(
        Paragraph::from_children(vec![
            TextRun::new("Nested cached field (outer complex locked=true): ").into(),
            Field::new(
                FieldInstruction::raw(" IF 1 = 1 ")?,
                vec![
                    Field::new(
                        FieldInstruction::Page,
                        vec![TextRun::new("[nested PAGE cache]").into()],
                    )
                    .into(),
                ],
            )
            .with_properties(FieldProperties {
                form: FieldForm::Complex,
                dirty: Some(false),
                locked: Some(true),
            })
            .into(),
        ])
        .into(),
    );
    out.push(heading("07 Inline images and explicit breaks"));
    for (ext, label, size) in [
        (
            "png",
            "PNG width-only 48 pt (aspect retained)",
            Some(ImageTransformation {
                width: Some(48.),
                height: None,
            }),
        ),
        (
            "jpg",
            "JPEG height-only 36 pt (aspect retained)",
            Some(ImageTransformation {
                width: None,
                height: Some(36.),
            }),
        ),
        (
            "gif",
            "GIF both dimensions 48 x 24 pt",
            Some(ImageTransformation {
                width: Some(48.),
                height: Some(24.),
            }),
        ),
        ("bmp", "BMP native density-derived size", None),
        ("tiff", "TIFF native density-derived size", None),
        ("zero", "PNG zero-DPI fallback size", None),
    ] {
        out.push(p(format!(
            "Image: {label}; accessibility title/description/name authored."
        )));
        out.push(Paragraph::from_children(vec![image(ext, size).into()]).into());
    }
    out.push(
        Paragraph::from_children(vec![
            Hyperlink::new(
                HyperlinkTarget::external("https://example.com/image")?,
                vec![
                    image(
                        "png",
                        Some(ImageTransformation {
                            width: Some(24.),
                            height: None,
                        }),
                    )
                    .into(),
                    Tab::default().into(),
                    TextRun::new("Linked image, tab and line break").into(),
                    Break::default().into(),
                    TextRun::new("Same PNG bytes are deduplicated in the package.").into(),
                ],
            )
            .into(),
        ])
        .into(),
    );
    out.push(p("Image from filesystem path (filename preserved), followed by async byte data with deferred size/alt text."));
    let mut path_image = ImageRun::from_path(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/images/sample.png"),
    );
    path_image.r#type = Some("png".into());
    path_image.transformation = Some(ImageTransformation {
        width: Some(24.),
        height: None,
    })
    .into();
    let mut async_image = ImageRun::deferred(Value::future(async {
        ImageData::Bytes(include_bytes!("../../tests/fixtures/images/sample.png").to_vec())
    }));
    async_image.transformation = Value::lazy(|| {
        Some(ImageTransformation {
            width: None,
            height: Some(24.),
        })
    });
    async_image.alt_text = Value::future(async {
        Some(AltText {
            description: Some("Deferred image accessibility".into()),
            ..Default::default()
        })
    });
    out.push(Paragraph::from_children(vec![path_image.into(), async_image.into()]).into());
    for (kind, name) in [
        (BreakType::Line, "line"),
        (BreakType::TextWrapping, "textWrapping"),
        (BreakType::Page, "page"),
        (BreakType::Column, "column"),
    ] {
        out.push(
            Paragraph::from_children(vec![
                TextRun::new(format!("Before explicit {name} break")).into(),
                Break::new(kind).into(),
                TextRun::new(format!("After explicit {name} break")).into(),
            ])
            .into(),
        );
    }
    Ok(())
}
pub fn numbering() -> Vec<NumberingDefinition> {
    vec![
        NumberingDefinition {
            id: 60,
            levels: (0..9)
                .map(|level| NumberingLevel {
                    level,
                    start: 1,
                    format: if level == 1 { "lowerRoman" } else { "decimal" }.into(),
                    text: format!("%{}.", level + 1),
                    is_legal: if level == 2 { Some(true) } else { None },
                    suffix: if level == 2 {
                        Some(NumberingSuffix::Tab)
                    } else {
                        None
                    },
                    restart: if level == 2 { Some(0) } else { None },
                    alignment: match level {
                        0 => Some(NumberingAlignment::Left),
                        1 => Some(NumberingAlignment::Center),
                        2 => Some(NumberingAlignment::Right),
                        3 => Some(NumberingAlignment::Start),
                        4 => Some(NumberingAlignment::End),
                        _ => None,
                    },
                })
                .collect(),
            start_overrides: vec![
                NumberingStartOverride { level: 0, start: 7 },
                NumberingStartOverride { level: 8, start: 0 },
            ],
            level_overrides: vec![
                NumberingLevel {
                    level: 1,
                    start: 4,
                    format: "upperRoman".into(),
                    text: "(%2)".into(),
                    is_legal: Some(true),
                    suffix: Some(NumberingSuffix::Nothing),
                    restart: Some(1),
                    alignment: Some(NumberingAlignment::Start),
                },
                NumberingLevel {
                    level: 0,
                    start: 3,
                    format: "lowerLetter".into(),
                    text: "%1)".into(),
                    is_legal: Some(false),
                    suffix: Some(NumberingSuffix::Space),
                    restart: Some(0),
                    alignment: Some(NumberingAlignment::End),
                },
            ],
        },
        NumberingDefinition {
            id: 61,
            levels: vec![NumberingLevel {
                level: 0,
                start: 1,
                format: "bullet".into(),
                text: "•".into(),
                is_legal: None,
                suffix: None,
                restart: None,
                alignment: None,
            }],
            ..Default::default()
        },
    ]
}
pub fn notes() -> Vec<Value<Note>> {
    vec![
        Note::footnote(
            NoteId::new(0),
            vec![
                p("Footnote body: rich paragraph, table, image and external hyperlink."),
                Table::new(vec![TableRow::new(vec![cell("Footnote table")]).into()]).into(),
                Paragraph::from_children(vec![
                    image(
                        "png",
                        Some(ImageTransformation {
                            width: Some(18.),
                            height: None,
                        }),
                    )
                    .into(),
                    link("Footnote external link").unwrap(),
                ])
                .into(),
            ],
        )
        .into(),
        Note::endnote(
            NoteId::new(0),
            vec![p(
                "Endnote body: this is an endnote, distinct from footnote id=0.",
            )],
        )
        .into(),
    ]
}
pub fn headers(section: &mut Section) -> Result<()> {
    for (kind, name) in [
        (HeaderFooterType::Default, "default"),
        (HeaderFooterType::First, "first"),
        (HeaderFooterType::Even, "even"),
    ] {
        section.headers.insert(
            kind,
            Header::new(vec![
                Paragraph::from_children(vec![
                    TextRun::new(format!("Header {name}: ")).into(),
                    link("owner-local external hyperlink")?,
                    image(
                        "png",
                        Some(ImageTransformation {
                            width: Some(12.),
                            height: None,
                        }),
                    )
                    .into(),
                ])
                .into(),
            ])
            .into(),
        );
        section.footers.insert(
            kind,
            Footer::new(vec![
                Paragraph::from_children(vec![
                    TextRun::new(format!("Footer {name}: PAGE ")).into(),
                    Field::new(FieldInstruction::Page, vec![TextRun::new("1").into()]).into(),
                    TextRun::new(" / NUMPAGES ").into(),
                    Field::new(FieldInstruction::NumPages, vec![TextRun::new("1").into()]).into(),
                ])
                .into(),
            ])
            .into(),
        );
    }
    Ok(())
}
