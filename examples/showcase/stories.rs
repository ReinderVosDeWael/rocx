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
    out.push(p("Numbering label formatting: abstract level 0 requests bold; replacement level 0 requests bold off and italic on."));
    out.push(p("Numbering level paragraph properties: abstract level 0 requests 24 pt left and 12 pt hanging; replacement level 0 requests 36 pt left."));
    out.push(p("Numbering level style links: abstract level 0 links to Heading 1; replacement level 0 links to Heading 2."));
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
    let mut shared = Paragraph::new(
        "Shared abstract numbering: numId=62 reuses the bullet levels of numId=61 with its own start override.",
    );
    shared.numbering = Some(NumberingReference { id: 62, level: 0 }).into();
    out.push(shared.into());
    out.push(p(
        "Numbering style: Catalog Bullets links concrete numId=61 to abstract definition 100.",
    ));
    let mut inherited =
        Paragraph::new("Inherited numbering style: numId=63 gets its levels from Catalog Bullets.");
    inherited.numbering = Some(NumberingReference { id: 63, level: 0 }).into();
    out.push(inherited.into());
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
                    ..Default::default()
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
    out.push(p(
        "Note numbering: lower-Roman footnotes start at 3 each page; uppercase-letter endnotes start at 7 each section. Footnotes are requested beneath text and endnotes at section end.",
    ));
    out.push(p(
        "Tracked text revisions: inserted text and deleted text retain review metadata. Tracking future edits is explicitly off.",
    ));
    out.push(
        Paragraph::from_children(vec![
            TextRun::new("Current: ").into(),
            TextRun::new("inserted wording")
                .with_revision(Some(TextRevision {
                    kind: RevisionKind::Insertion,
                    author: "Catalog editor".into(),
                    date: Some("2026-11-20T00:00:00Z".into()),
                }))
                .into(),
            TextRun::new("old wording")
                .with_revision(Some(TextRevision {
                    kind: RevisionKind::Deletion,
                    author: "Catalog editor".into(),
                    date: None,
                }))
                .into(),
        ])
        .into(),
    );
    out.push(p("Inline content control: rich text, alias Review caption, tag review-caption, ID 5001, content locked."));
    out.push(
        Paragraph::from_children(vec![
            TextRun::new("Review: ").into(),
            InlineContentControl::new(vec![
                TextRun::new("controlled caption")
                    .with_format(RunFormat::default().with_bold(true))
                    .into(),
            ])
            .with_properties(
                ContentControlProperties::default()
                    .with_alias("Review caption")
                    .with_tag("review-caption")
                    .with_id(5001)
                    .with_lock(ContentControlLock::ContentLocked),
            )
            .into(),
        ])
        .into(),
    );
    out.push(p(
        "Block content control: a controlled paragraph and table share an alias, tag and ID 5002.",
    ));
    out.push(
        BlockContentControl::new(vec![
            p("Controlled block paragraph"),
            Table::new(vec![
                TableRow::new(vec![cell("Controlled table cell")]).into(),
            ])
            .into(),
        ])
        .with_properties(
            ContentControlProperties::default()
                .with_alias("Review block")
                .with_tag("review-block")
                .with_id(5002)
                .with_lock(ContentControlLock::ControlLocked),
        )
        .into(),
    );
    out.push(p("Plain-text control: one run, multiline enabled."));
    out.push(
        Paragraph::from_children(vec![
            InlineContentControl::new(vec![TextRun::new("first line\nsecond line").into()])
                .with_properties(
                    ContentControlProperties::default()
                        .with_tag("plain-review")
                        .with_text(ContentControlText::Plain {
                            multi_line: Some(true),
                        }),
                )
                .into(),
        ])
        .into(),
    );
    out.push(p(
        "Drop-down control: Review status offers Pending and Accepted, with Accepted selected.",
    ));
    out.push(
        Paragraph::from_children(vec![
            InlineContentControl::new(vec![TextRun::new("Accepted").into()])
                .with_properties(
                    ContentControlProperties::default()
                        .with_alias("Review status")
                        .with_tag("review-status")
                        .with_text(ContentControlText::DropDown {
                            items: vec![
                                ContentControlListItem {
                                    display_text: "Pending".into(),
                                    value: "pending".into(),
                                },
                                ContentControlListItem {
                                    display_text: "Accepted".into(),
                                    value: "accepted".into(),
                                },
                            ],
                            selected: "accepted".into(),
                        }),
                )
                .into(),
        ])
        .into(),
    );
    out.push(p(
        "Tracked text move: CatalogMove pairs a source and a destination.",
    ));
    let move_name = BookmarkName::new("CatalogMove")?;
    for (caption, kind) in [
        ("Move source: ", RevisionKind::MoveSource(move_name.clone())),
        (
            "Move destination: ",
            RevisionKind::MoveDestination(move_name),
        ),
    ] {
        out.push(
            Paragraph::from_children(vec![
                TextRun::new(caption).into(),
                TextRun::new("relocated wording")
                    .with_revision(Some(TextRevision {
                        kind,
                        author: "Catalog editor".into(),
                        date: Some("2026-11-20T00:00:00Z".into()),
                    }))
                    .into(),
            ])
            .into(),
        );
    }
    out.push(p(
        "Table content controls: one row and its cell have separate tags.",
    ));
    out.push(
        Table::new(vec![
            TableRow::new(vec![cell("Uncontrolled table cell")]).into(),
            TableRow::new(vec![
                TableCell::new(vec![p("Controlled row and cell")])
                    .with_properties(
                        TableCellProperties::default().with_content_control(
                            ContentControlProperties::default()
                                .with_tag("review-cell")
                                .with_id(5004),
                        ),
                    )
                    .into(),
            ])
            .with_properties(
                TableRowProperties::default().with_content_control(
                    ContentControlProperties::default()
                        .with_tag("review-row")
                        .with_id(5003),
                ),
            )
            .into(),
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
                ..Default::default()
            }),
        ),
        (
            "jpg",
            "JPEG height-only 36 pt (aspect retained)",
            Some(ImageTransformation {
                width: None,
                height: Some(36.),
                ..Default::default()
            }),
        ),
        (
            "gif",
            "GIF both dimensions 48 x 24 pt",
            Some(ImageTransformation {
                width: Some(48.),
                height: Some(24.),
                ..Default::default()
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
                            ..Default::default()
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
    out.push(p("Image crop and rotation: 10% left, 5% top, 20% right and 15% bottom; 30 degrees clockwise, horizontal flip on and vertical flip off."));
    out.push(
        Paragraph::from_children(vec![
            image(
                "png",
                Some(
                    ImageTransformation::default()
                        .with_width(60.)
                        .with_height(40.)
                        .with_rotation(30.)
                        .with_flip_horizontal(true)
                        .with_flip_vertical(false)
                        .with_crop(
                            ImageCrop::default()
                                .with_left(10.)
                                .with_top(5.)
                                .with_right(20.)
                                .with_bottom(15.),
                        ),
                ),
            )
            .into(),
        ])
        .into(),
    );
    out.push(p("Floating picture: column-relative horizontal offset 250 pt, page-relative bottom alignment, square wrapping on the largest side; gaps top 0, bottom 2, left 3, right 4 pt. Anchor locked; in-cell layout and overlap off."));
    out.push(
        Paragraph::from_children(vec![
            image(
                "png",
                Some(
                    ImageTransformation::default().with_width(42.).with_anchor(
                        ImageAnchor::default()
                            .with_horizontal_reference(ImageAnchorReference::Column)
                            .with_x(250.)
                            .with_vertical_alignment(ImageAnchorVerticalAlignment::Bottom)
                            .with_wrap(ImageWrap::Square)
                            .with_wrap_side(ImageWrapSide::Largest)
                            .with_distance_top(0.)
                            .with_distance_bottom(2.)
                            .with_distance_left(3.)
                            .with_distance_right(4.)
                            .with_relative_height(4)
                            .with_locked(true)
                            .with_layout_in_cell(false)
                            .with_allow_overlap(false),
                    ),
                ),
            )
            .into(),
        ])
        .into(),
    );
    out.push(p(
        "Floating picture: tight text wrapping around an authored triangular contour.",
    ));
    out.push(
        Paragraph::from_children(vec![
            image(
                "png",
                Some(
                    ImageTransformation::default().with_width(42.).with_anchor(
                        ImageAnchor::default()
                            .with_x(360.)
                            .with_y(160.)
                            .with_wrap(ImageWrap::Tight)
                            .with_wrap_polygon(vec![(0, 0), (21600, 21600), (0, 21600)]),
                    ),
                ),
            )
            .into(),
        ])
        .into(),
    );
    out.push(p(
        "Inline DrawingML shapes: a blue rectangle with text and a green ellipse with a dark outline.",
    ));
    out.push(p("Shape text style: blue rectangle, 14.25 pt white Arial text, bold explicitly false, italic true, centered horizontally and bottom anchored. Blank middle line preserved; ellipse has no text body."));
    out.push(
        Paragraph::from_children(vec![
            ShapeRun::new(
                ShapeProperties::default()
                    .with_width(96.)
                    .with_height(64.)
                    .with_fill([32, 80, 176])
                    .with_text("Blue\n\nText")
                    .with_text_format(
                        ShapeTextFormat::default()
                            .with_font_size(14.25)
                            .with_bold(false)
                            .with_italic(true)
                            .with_font("Arial")
                            .with_color([255, 255, 255])
                            .with_alignment(ShapeTextAlignment::Center)
                            .with_vertical_alignment(ShapeTextVerticalAlignment::Bottom),
                    ),
            )
            .into(),
            ShapeRun::new(
                ShapeProperties::default()
                    .with_width(44.)
                    .with_height(32.)
                    .with_geometry(ShapeGeometry::Ellipse)
                    .with_fill([32, 144, 80])
                    .with_outline(ShapeOutline {
                        width: 1.5,
                        color: [16, 48, 32],
                    }),
            )
            .into(),
        ])
        .into(),
    );
    out.push(p("Standalone inline text box: 144 by 48 points, plain multiline DrawingML text, Arial 12 pt, bold explicitly false; no geometric fill or outline, no rich WordprocessingML story."));
    out.push(
        Paragraph::from_children(vec![
            ShapeRun::new(
                ShapeProperties::default()
                    .with_geometry(ShapeGeometry::TextBox)
                    .with_width(144.)
                    .with_height(48.)
                    .with_text("Standalone\nText box")
                    .with_text_format(
                        ShapeTextFormat::default()
                            .with_font("Arial")
                            .with_font_size(12.)
                            .with_bold(false),
                    ),
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
        ..Default::default()
    })
    .into();
    let mut async_image = ImageRun::deferred(Value::future(async {
        ImageData::Bytes(include_bytes!("../../tests/fixtures/images/sample.png").to_vec())
    }));
    async_image.transformation = Value::lazy(|| {
        Some(ImageTransformation {
            width: None,
            height: Some(24.),
            ..Default::default()
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
    let bullet_levels = vec![
        NumberingLevel::default()
            .with_start(1)
            .with_format("bullet")
            .with_text("•"),
    ];
    vec![
        NumberingDefinition {
            abstract_id: None,
            inherit_style: None,
            id: 60,
            levels: (0..9)
                .map(|level| NumberingLevel {
                    label_format: (level == 0).then(|| RunFormat::default().with_bold(true)),
                    paragraph_style: (level == 0).then(|| "Heading 1".into()),
                    paragraph_format: (level == 0).then(|| {
                        ParagraphFormat::default()
                            .with_left_indent(24.)
                            .with_first_line_indent(-12.)
                    }),
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
                    label_format: None,
                    paragraph_style: None,
                    paragraph_format: None,
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
                    label_format: Some(RunFormat::default().with_bold(false).with_italic(true)),
                    paragraph_style: Some("Heading 2".into()),
                    paragraph_format: Some(ParagraphFormat::default().with_left_indent(36.)),
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
        NumberingDefinition::default()
            .with_id(61)
            .with_abstract_id(100)
            .with_levels(bullet_levels.clone()),
        NumberingDefinition::default()
            .with_id(62)
            .with_abstract_id(100)
            .with_levels(bullet_levels)
            .with_start_overrides(vec![NumberingStartOverride { level: 0, start: 4 }]),
        NumberingDefinition::default()
            .with_id(63)
            .with_abstract_id(101)
            .with_inherit_style("Catalog Bullets"),
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
                            ..Default::default()
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
                            ..Default::default()
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
