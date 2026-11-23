//! Independent Rust authoring of the same catalog as all_features.py.
use rocx::*;
use std::path::Path;
mod formatting;
mod stories;
mod tables;

pub const STAMP: &str = "2026-11-19T00:00:00Z";
const EDIT_LABEL: &str =
    "Imported editor: this paragraph changes from left to center in the edited catalog.";
pub fn p(text: impl Into<String>) -> Value<Block> {
    Paragraph::new(text.into()).into()
}
pub fn heading(text: &str) -> Value<Block> {
    Paragraph::new(text)
        .with_format(ParagraphFormat {
            heading: Some(1),
            page_break_before: Some(true),
            ..Default::default()
        })
        .into()
}
pub fn run(text: &str, format: RunFormat) -> Value<Block> {
    Paragraph::from_children(vec![TextRun::new(text).with_format(format).into()]).into()
}
pub fn cell(text: impl Into<String>) -> Value<TableCell> {
    TableCell::new(vec![p(text)]).into()
}
pub fn image(ext: &str, size: Option<ImageTransformation>) -> ImageRun {
    let bytes: &[u8] = match ext {
        "png" => include_bytes!("../../tests/fixtures/images/sample.png"),
        "jpg" => include_bytes!("../../tests/fixtures/images/sample.jpg"),
        "gif" => include_bytes!("../../tests/fixtures/images/sample.gif"),
        "bmp" => include_bytes!("../../tests/fixtures/images/sample.bmp"),
        "tiff" => include_bytes!("../../tests/fixtures/images/sample.tiff"),
        _ => include_bytes!("../../tests/fixtures/images/sample_zero_dpi.png"),
    };
    let mut image = ImageRun::new(bytes.to_vec());
    image.transformation = size.into();
    image.alt_text = Some(AltText {
        title: Some(format!("Image {ext}")),
        description: Some(format!("Accessible {ext} sample")),
        name: Some(format!("Sample {ext}")),
    })
    .into();
    image
}
pub fn metadata() -> Metadata {
    Metadata {
        creator: Some("DOCX inspection catalog".into()),
        title: Some("All features visual inspection".into()),
        subject: Some("Rust/Python byte parity".into()),
        description: Some("Labeled functionality samples; not a rendering guarantee".into()),
        keywords: Some("docx,visual,parity".into()),
        category: Some("Examples".into()),
        comments: Some("Core metadata comments".into()),
        version: Some("1.0".into()),
        custom_properties: [("Review status".into(), "Draft & review".into())].into(),
        custom_boolean_properties: [
            ("Review complete".into(), true),
            ("Published".into(), false),
        ]
        .into(),
        company: Some("Atlas & Co.".into()),
        manager: Some("Taylor".into()),
    }
}
pub async fn generate(output: &Path) -> Result<()> {
    // A real two-stage template edit exercises insertion and cross-run replacement.
    let seed = Paragraph::from_children(vec![
        TextRun::new("Template {{re").into(),
        TextRun::new("place}}: cross-run replacement.").into(),
    ]);
    let tagged_seed = Paragraph::from_children(vec![
        TextRun::new("Template plain-text control replacement: ").into(),
        InlineContentControl::new(vec![
            TextRun::new("old controlled value")
                .with_format(RunFormat::default().with_bold(true))
                .into(),
        ])
        .with_properties(
            ContentControlProperties::default()
                .with_tag("template-control")
                .with_lock(ContentControlLock::ControlLocked)
                .with_text(ContentControlText::Plain { multi_line: None }),
        )
        .into(),
    ]);
    let revision_seed = Paragraph::from_children(vec![
        TextRun::new("Template revision action: ").into(),
        TextRun::new("accepted template wording")
            .with_revision(Some(TextRevision {
                kind: RevisionKind::Insertion,
                author: "Template editor".into(),
                date: None,
            }))
            .into(),
        TextRun::new("removed template wording")
            .with_revision(Some(TextRevision {
                kind: RevisionKind::Deletion,
                author: "Template editor".into(),
                date: None,
            }))
            .into(),
    ]);
    let move_name = BookmarkName::new("TemplateMove")?;
    let move_seed = Paragraph::from_children(vec![
        TextRun::new("Template accepted move: ").into(),
        TextRun::new("moved template wording")
            .with_revision(Some(TextRevision {
                kind: RevisionKind::MoveSource(move_name.clone()),
                author: "Template editor".into(),
                date: Some(STAMP.into()),
            }))
            .into(),
        TextRun::new("destination: ").into(),
        TextRun::new("moved template wording")
            .with_revision(Some(TextRevision {
                kind: RevisionKind::MoveDestination(move_name),
                author: "Template editor".into(),
                date: Some(STAMP.into()),
            }))
            .into(),
    ]);
    let mut template_document = Document::new(vec![
        Section::new(vec![
            seed.into(),
            p("Template retained paragraph: insertion appears before this."),
            tagged_seed.into(),
            revision_seed.into(),
            move_seed.into(),
        ])
        .into(),
    ]);
    template_document.timestamp = Some(STAMP.into()).into();
    template_document.metadata = Metadata::default()
        .with_custom_properties(
            [
                ("Project".into(), "Atlas".into()),
                ("Review status".into(), "Pending".into()),
            ]
            .into(),
        )
        .with_custom_boolean_properties([("Review complete".into(), false)].into())
        .into();
    template_document.styles = vec![
        ParagraphStyleDefinition {
            name: "Catalog Existing".into(),
            bold: Some(true),
            ..Default::default()
        }
        .into(),
        CharacterStyleDefinition {
            name: "Catalog Existing Character".into(),
            format: RunFormat {
                bold: Some(true),
                ..Default::default()
            },
            ..Default::default()
        }
        .into(),
    ]
    .into();
    let template_bytes = template_document.resolve().await?.to_docx()?;
    let template_path = output.with_extension("template.docx");
    std::fs::write(&template_path, &template_bytes)?;

    let mut blocks = vec![
        p("ALL FEATURES — visual inspection catalog"),
        p(
            "Each sample names its requested feature. Field caches are placeholders; update fields in your viewer. Font fallback, pagination and conditional table-style activation depend on the viewer. Inspect comments, footnotes, endnotes and document properties too.",
        ),
        p(
            "Document defaults: Calibri 11 pt, after spacing 4 pt, 1.1 line height. See template replacement at the start and the retained template paragraph after topic 08.",
        ),
        p(
            "Template safety: this sample uses an unsigned template. Templates with an OPC signature origin are rejected before any output is written.",
        ),
        p(
            "Custom document properties: Project (Atlas) is retained; Review status changes to Draft & review.",
        ),
        p(
            "Boolean custom properties: Review complete changes from false to true; Published is explicitly false.",
        ),
        p(
            "Template compatibility: unsupported mandatory namespaces in edited WordprocessingML are rejected.",
        ),
        p("Extended document properties: Company is Atlas & Co.; Manager is Taylor."),
    ];
    formatting::blocks(&mut blocks);
    tables::blocks(&mut blocks);
    stories::blocks(&mut blocks)?;

    blocks.push(heading(
        "08 Authoring: ready, lazy, async, references, conditions and snapshot reuse",
    ));
    blocks.push(p("Ready value: this paragraph was supplied immediately."));
    blocks.push(
        Paragraph::new(Value::lazy(|| {
            "Lazy value: this paragraph was produced by a factory.".into()
        }))
        .into(),
    );
    blocks.push(
        Paragraph::new(Value::future(async {
            "Async value: this paragraph was produced by an awaitable.".into()
        }))
        .into(),
    );
    let shared = Property::new(
        "catalog.shared",
        Value::future(async { "Shared reference: computed once and used twice.".to_owned() }),
    );
    blocks.push(Paragraph::new(shared.reference()).into());
    blocks.push(Paragraph::new(shared.reference()).into());
    let cached = Deferred::new(Value::future(async {
        "Explicit Deferred: shared computation.".to_owned()
    }));
    blocks.push(Paragraph::new(cached).into());
    blocks.push(
        Paragraph::new("Condition true: this paragraph is visible.")
            .when(true)
            .into(),
    );
    blocks.push(
        Paragraph::new(Value::lazy(|| -> String {
            panic!("excluded producer must not execute")
        }))
        .when(false)
        .into(),
    );
    blocks.push(p(
        "Condition false: the excluded paragraph is absent and its producer did not execute.",
    ));
    let reusable = Document::new(vec![
        Section::new(vec![p(
            "Resolved-child reuse: immutable paragraph used twice.",
        )])
        .into(),
    ])
    .resolve()
    .await?;
    let reusable = reusable.sections()[0].blocks()[0].clone();
    blocks.extend([reusable.clone().into(), reusable.into()]);
    let inline_source = Document::new(vec![
        Section::new(vec![run(
            "Resolved-inline reuse: immutable run used twice.",
            RunFormat::default(),
        )])
        .into(),
    ])
    .resolve()
    .await?;
    let inline = inline_source.sections()[0].paragraphs()[0].runs()[0].clone();
    blocks.push(Paragraph::from_children(vec![inline.clone().into(), inline.into()]).into());

    blocks.push(p(
        "Header/footer distances: 720 and 360 twips from the page edges.",
    ));
    blocks.push(p(
        "Page borders: navy single top/bottom and gray dashed left/right; first page, offset from page, behind text.",
    ));
    let horizontal = PageBorder {
        style: BorderLine::Single,
        size: Some(12),
        space: Some(6),
        color: Some([32, 48, 96]),
    };
    let vertical = PageBorder {
        style: BorderLine::Dashed,
        size: Some(8),
        space: Some(4),
        color: Some([96, 96, 96]),
    };
    blocks.push(p(
        "Document page settings: mirrored margins enabled, top gutter explicitly off.",
    ));
    let mut first = Section::new(blocks);
    first.properties = Some(SectionProperties {
        page_size: Some(PageSize {
            width: Some(7772400),
            height: Some(10058400),
        }),
        page_orientation: Some(Orientation::Portrait),
        page_margins: Some(PageMargins {
            top: Some(914400),
            bottom: Some(914400),
            left: Some(914400),
            right: Some(914400),
            header: Some(457200),
            footer: Some(228600),
        }),
        page_borders: Some(PageBorders {
            offset_from: Some(PageBorderOffset::Page),
            display: Some(PageBorderDisplay::FirstPage),
            z_order: Some(PageBorderZOrder::Back),
            top: Some(horizontal.clone()),
            left: Some(vertical.clone()),
            bottom: Some(horizontal),
            right: Some(vertical),
        }),
        title_page: Some(true),
        bidi: Some(false),
        rtl_gutter: Some(false),
        ..Default::default()
    })
    .into();
    stories::headers(&mut first)?;
    let mut sections = vec![first.into()];
    for (i, (kind, label)) in [
        (SectionType::NextPage, "nextPage"),
        (SectionType::NextColumn, "nextColumn"),
        (SectionType::Continuous, "continuous"),
        (SectionType::EvenPage, "evenPage"),
        (SectionType::OddPage, "oddPage"),
    ]
    .into_iter()
    .enumerate()
    {
        let restart = [
            LineNumberRestart::NewPage,
            LineNumberRestart::NewSection,
            LineNumberRestart::Continuous,
        ]
        .get(i)
        .copied();
        let line_numbering_label = match restart {
            Some(value) => format!(
                "Line numbering: every 5 lines, start=0, distance=240 twips, restart={}.",
                value.xml()
            ),
            None if i == 4 => "Line numbering: clear previous section settings.".into(),
            None => "Line numbering: preserve previous section settings.".into(),
        };
        let grid_mode = [
            DocumentGridMode::Default,
            DocumentGridMode::Lines,
            DocumentGridMode::LinesAndChars,
            DocumentGridMode::SnapToChars,
        ]
        .get(i)
        .copied();
        let grid_label = match grid_mode {
            Some(mode) => format!(
                "Document grid: mode={}, line pitch=360 twips, character pitch delta=-4096.",
                mode.xml()
            ),
            None => "Document grid: preserve previous section settings.".into(),
        };
        let mut section = Section::new(vec![
            p(format!(
                "09 Section: break={label}; landscape; 2 columns with separator; page numbering lowerRoman starts at {}; vertical alignment={}; section bidi={}; rtl_gutter={}.",
                i + 1,
                ["top", "center", "bottom", "both", "top"][i],
                i % 2 == 1,
                i % 2 == 1
            )),
            p(line_numbering_label),
            Paragraph::new(
                "Paragraph layout: contextual spacing enabled, mirrored indents explicitly off.",
            )
            .with_format(
                ParagraphFormat::default()
                    .with_contextual_spacing(true)
                    .with_mirror_indents(false),
            )
            .into(),
            p(grid_label),
            Paragraph::from_children(vec![
                TextRun::new("Grid snapping: paragraph enabled, run explicitly off.")
                    .with_format(RunFormat::default().with_snap_to_grid(false))
                    .into(),
            ])
            .with_format(ParagraphFormat::default().with_snap_to_grid(true))
            .into(),
            Paragraph::new("Paragraph line numbers: suppressed.")
                .with_format(ParagraphFormat::default().with_suppress_line_numbers(true))
                .into(),
            Paragraph::new("Paragraph line numbers: suppression explicitly off.")
                .with_format(ParagraphFormat::default().with_suppress_line_numbers(false))
                .into(),
            p(
                "Section layout request: physical margins remain unchanged. Gutter placement does not create a gutter amount. This is the first column.",
            ),
            Paragraph::from_children(vec![
                Break::new(BreakType::Column).into(),
                TextRun::new("This follows a column break in the section sample.").into(),
            ])
            .into(),
        ]);
        section.properties = Some(SectionProperties {
            r#type: Some(kind),
            page_orientation: Some(Orientation::Landscape),
            columns: Some(Columns {
                count: Some(2),
                space: Some(360),
                separator: Some(true),
            }),
            line_numbering: if i == 4 {
                Some(LineNumbering::default().with_clear(true))
            } else {
                restart.map(|value| {
                    LineNumbering::default()
                        .with_count_by(5)
                        .with_start(0)
                        .with_distance(240)
                        .with_restart(value)
                })
            },
            document_grid: grid_mode.map(|mode| {
                DocumentGrid::default()
                    .with_mode(mode)
                    .with_line_pitch(360)
                    .with_character_pitch_delta(-4096)
            }),
            page_numbering: Some(PageNumbering {
                start: Some((i + 1) as u32),
                format: Some("lowerRoman".into()),
            }),
            vertical_align: Some(
                [
                    VerticalAlignment::Top,
                    VerticalAlignment::Center,
                    VerticalAlignment::Bottom,
                    VerticalAlignment::Both,
                    VerticalAlignment::Top,
                ][i],
            ),
            title_page: Some(false),
            bidi: Some(i % 2 == 1),
            rtl_gutter: Some(i % 2 == 1),
            ..Default::default()
        })
        .into();
        sections.push(section.into());
    }
    let mut document = Document::new(sections);
    document.timestamp = Some(STAMP.into()).into();
    document.metadata = metadata().into();
    document.comment_author = Some("Catalog reviewer".into()).into();
    document.defaults = Some(DocumentDefaults {
        run: Some(RunFormat {
            font: Some("Calibri".into()),
            size: Some(11.),
            ..Default::default()
        }),
        mirror_margins: Some(true),
        gutter_at_top: Some(false),
        track_revisions: Some(false),
        footnote_position: Some(FootnotePosition::BeneathText),
        endnote_position: Some(EndnotePosition::SectionEnd),
        footnote_numbering: Some(
            NoteNumbering::default()
                .with_format(NoteNumberFormat::LowerRoman)
                .with_start(3)
                .with_restart(NoteNumberRestart::EachPage),
        ),
        endnote_numbering: Some(
            NoteNumbering::default()
                .with_format(NoteNumberFormat::UpperLetter)
                .with_start(7)
                .with_restart(NoteNumberRestart::EachSection),
        ),
        paragraph: Some(ParagraphFormat {
            spacing_after: Some(4.),
            line_height: Some(LineHeight::Multiple(1.1)),
            ..Default::default()
        }),
    })
    .into();
    let mut styles = formatting::styles();
    styles.extend(tables::styles());
    styles.push(
        NumberingStyleDefinition {
            name: "Catalog Bullets".into(),
            numbering_id: 61,
        }
        .into(),
    );
    document.styles = styles.into();
    document.numbering = stories::numbering().into();
    document.notes = stories::notes().into();
    let snapshot = document.resolve().await?;
    let options = application::RenderOptions {
        timestamp: Some(STAMP.into()),
        profile: OutputProfile::Transitional,
        read_limits: opc::ReadLimits {
            max_parts: 10000,
            max_part_bytes: 67108864,
            max_expanded_bytes: 268435456,
        },
    };
    let template = DocumentTemplate {
        path: template_path,
        edits: TemplateEdits {
            replacements: vec![("{{replace}}".into(), "REPLACED".into())],
            content_control_replacements: [(
                "template-control".into(),
                "updated controlled value".into(),
            )]
            .into(),
            text_revision_action: Some(TextRevisionAction::Accept),
            paragraph_index: Some(1),
            scope: EditScope::SupportedStories,
        },
    };
    let bytes = snapshot.to_docx_with_template_and_options(&template, &options)?;
    assert_eq!(
        bytes,
        snapshot.to_docx_with_template_and_options(&template, &options)?
    );
    // Also exercise the pure caller-owned-byte compilation and inspectable plan.
    let plan =
        application::compile_with_edits(&snapshot, &template_bytes, &template.edits, &options)?;
    plan.validate()?;
    assert!(plan.parts().count() > 10);
    assert!(
        plan.part(&opc::PartName::new("/word/document.xml")?)
            .is_some()
    );
    assert_eq!(bytes, plan.to_bytes()?);
    snapshot.save_with_template_and_options(output, &template, &options)?;
    let imported = editor::ImportedDocument::open(&bytes)?;
    let targets: Vec<_> = imported
        .body()
        .iter()
        .filter_map(|block| match block {
            editor::BodyBlock::Paragraph(paragraph) if paragraph.text == EDIT_LABEL => {
                Some(paragraph)
            }
            _ => None,
        })
        .collect();
    assert_eq!(targets.len(), 1);
    assert_eq!(
        targets[0].inline_items,
        [editor::ImportedInline::Text(EDIT_LABEL.into())]
    );
    let mut table = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![run(
                "Imported editor table: bold label",
                RunFormat::default().with_bold(true),
            )])
            .into(),
            cell("Fixed-width second column"),
        ])
        .into(),
    ]);
    table.properties = TableProperties {
        column_widths: Some(vec![2160, 2160]),
        layout: Some(TableLayout::Fixed),
        ..Default::default()
    }
    .into();
    let table = table.await?.expect("nonempty showcase table");
    let preview = imported.preview_batch(
        &[editor::ParagraphAlignmentEdit {
            target: targets[0].anchor.clone(),
            alignment: Alignment::Center,
        }],
        &[editor::TableInsertion {
            target: targets[0].anchor.clone(),
            table,
        }],
    )?;
    let edited = preview.to_docx()?;
    // Typed physical-table readback from the edited original package.
    let reopened = editor::ImportedDocument::open(&edited)?;
    let after = preview.insertions()[0].after_body_index + 1;
    let editor::BodyBlock::Table(table) = &reopened.body()[after] else {
        panic!("expected inserted table")
    };
    let [editor::ImportedTableChild::Row(row)] = table.children.as_slice() else {
        panic!("expected row")
    };
    let editor::ImportedRowChild::Cell(cell) = &row.children[0] else {
        panic!("expected cell")
    };
    let editor::ImportedCellBlock::Paragraph(items) = &cell.blocks[0] else {
        panic!("expected paragraph")
    };
    assert_eq!(
        items,
        &[editor::ImportedInline::Text(
            "Imported editor table: bold label".into()
        )]
    );
    std::fs::write(output.with_extension("edited.docx"), edited)?;

    // Both previews start from the same snapshot: two sections share one header part.
    let mut first = Section::new(vec![p("Header image scope: first section.")]);
    first.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![p("Header image scope: shared source header.")]).into(),
    );
    let second = Section::new(vec![p(
        "Header image scope: second section shares the same header.",
    )]);
    let mut source = Document::new(vec![first.into(), second.into()]);
    source.timestamp = Some(STAMP.into()).into();
    let source = source.resolve().await?.to_docx()?;
    std::fs::write(output.with_extension("header-source.docx"), &source)?;
    let imported = editor::ImportedDocument::open(&source)?;
    assert_eq!(
        imported
            .sections()?
            .iter()
            .map(|section| (section.body_start, section.body_end))
            .collect::<Vec<_>>(),
        [(0, 2), (2, 3)]
    );
    let headers = imported.default_headers()?;
    assert_eq!(headers[0].part_name, headers[1].part_name);
    assert!(headers[1].blocks.iter().any(|block| matches!(block,
        editor::ImportedHeaderBlock::Paragraph(items)
        if items.iter().any(|item| matches!(item, editor::ImportedInline::Text(text) if text == "Header image scope: shared source header.")))));
    let target = &headers[1];
    let resolved = image("png", None).await?.expect("showcase image");
    for (mode, suffix) in [
        (editor::HeaderEditMode::Shared, "header-shared.docx"),
        (editor::HeaderEditMode::SectionLocal, "header-local.docx"),
    ] {
        let preview = imported.preview_header_image(&editor::HeaderImageEdit {
            target: target.anchor.clone(),
            mode,
            image: resolved.image().expect("resolved image").clone(),
        })?;
        std::fs::write(output.with_extension(suffix), preview.to_docx()?)?;
    }
    Ok(())
}
