//! Independent Rust authoring of the same catalog as Python and JavaScript.
use rocx::*;
use std::path::Path;
mod formatting;
mod javascript;
mod stories;
mod tables;

pub const STAMP: &str = "2026-11-19T00:00:00Z";
const EDIT_LABEL: &str =
    "Imported editor: this paragraph changes from left to center in the edited catalog.";
const INLINE_LABEL: &str = "Imported editor inline structure: ";
const INLINE_LINK_RUNS: [&str; 2] = ["jump to ", "CatalogBookmark"];
const HEADER_INLINE_LABEL: &str = "Default-header inline structure: ";
const HEADER_LINK_RUNS: [&str; 2] = ["shared ", "link"];
const HEADER_BATCH_LABEL: &str = "Combined editor batch: this paragraph is centered and followed by a table in both image previews.";
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
        subject: Some("Rust/Python/JavaScript byte parity".into()),
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
    blocks.extend(javascript::blocks());

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
    let styles = imported.styles()?.expect("catalog styles part");
    let styled = imported.preview_with_styles(
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[
            editor::StyleEdit::Add(
                ParagraphStyleDefinition::default()
                    .with_name("Catalog Imported Child")
                    .with_base_style("Catalog Base")
                    .into(),
            ),
            editor::StyleEdit::Patch {
                target: imported.style_anchor("CatalogBase")?,
                bold: None,
                italic: Some(true),
            },
            editor::StyleEdit::Field {
                target: imported.style_anchor("Regionfirst_row")?,
                region: Some("firstRow".into()),
                field: "b".into(),
                value: None,
            },
            editor::StyleEdit::Region {
                target: imported.style_anchor("Regionlast_row")?,
                region: "lastRow".into(),
                format: Some(Box::new(TableSectionFormat {
                    italic: Some(true),
                    ..Default::default()
                })),
            },
            editor::StyleEdit::Add(
                NumberingStyleDefinition {
                    name: "Catalog Imported List".into(),
                    numbering_id: 60,
                }
                .into(),
            ),
        ],
    )?;
    assert!(
        editor::ImportedDocument::open(&styled.to_docx()?)?
            .styles()?
            .unwrap()
            .styles
            .iter()
            .any(|s| s.id.as_deref() == Some("CatalogImportedChild")
                && s.based_on.as_deref() == Some("CatalogBase"))
    );
    assert!(
        styles
            .styles
            .iter()
            .any(|style| style.name.as_deref() == Some("Catalog Bullets")
                && style.kind == editor::StyleKind::Numbering
                && style.id.is_some())
    );
    let revisions: Vec<_> = imported
        .body()
        .iter()
        .filter_map(|block| match block {
            editor::BodyBlock::Paragraph(p) => Some(p.inline_structure.iter()),
            _ => None,
        })
        .flatten()
        .filter_map(|node| match node {
            editor::ImportedInlineNode::Revision(r)
                if r.author.as_deref() == Some("Catalog editor") =>
            {
                Some(r)
            }
            _ => None,
        })
        .collect();
    for (kind, text) in [
        ("insertion", "inserted wording"),
        ("deletion", "old wording"),
    ] {
        assert!(
            revisions.iter().any(|revision| revision.kind == kind
                && matches!(revision.children.as_slice(), [editor::ImportedInlineNode::Run(run)]
                if run.items == [editor::ImportedInline::Text(text.into())] &&
                matches!(imported.inline_at(&run.anchor), Ok(editor::ImportedInlineNode::Run(_)))))
        );
    }
    let entries = imported.entry_stories()?;
    for kind in [editor::EntryKind::Footnote, editor::EntryKind::Endnote] {
        assert!(entries.iter().any(|e| e.kind == kind && e.id == "0"));
    }
    let review = entries
        .iter()
        .find(|e| e.author.as_deref() == Some("Structured reviewer"))
        .unwrap();
    assert_eq!(review.kind, editor::EntryKind::Comment);
    assert_eq!(review.initials.as_deref(), Some("SR"));
    let Some([editor::ImportedInlineNode::Run(review_run)]) = review.inline_structure(0) else {
        panic!("expected structured comment label")
    };
    assert!(
        matches!(imported.inline_at(&review_run.anchor)?, editor::ImportedInlineNode::Run(found)
        if matches!(found.items.first(), Some(editor::ImportedInline::Text(text)) if text.starts_with("Structured comment:")))
    );
    let mut entries_output = bytes.clone();
    for kind in [
        editor::EntryKind::Footnote,
        editor::EntryKind::Endnote,
        editor::EntryKind::Comment,
    ] {
        let source = editor::ImportedDocument::open(&entries_output)?;
        let e = source
            .entry_stories()?
            .iter()
            .find(|e| {
                e.kind == kind
                    && if kind == editor::EntryKind::Comment {
                        e.author.as_deref() == Some("Structured reviewer")
                    } else {
                        e.id == "0"
                    }
            })
            .unwrap();
        let run = e
            .inline_structure(0)
            .unwrap()
            .iter()
            .find_map(|node| match node {
                Run(r) if matches!(r.items.as_slice(), [editor::ImportedInline::Text(_)]) => {
                    Some(r)
                }
                _ => None,
            })
            .unwrap();
        let label = format!("Edited {} entry", kind.as_str());
        let text_bytes = source.edit_story_text(
            &e.part_name,
            &[editor::RunTextEdit {
                target: run.anchor.clone(),
                text: label.clone(),
            }],
        )?;
        let source = editor::ImportedDocument::open(&text_bytes)?;
        let e = source
            .entry_stories()?
            .iter()
            .find(|other| other.kind == kind && other.id == e.id)
            .unwrap();
        let run = e.inline_structure(0).unwrap().iter().find_map(|node| match node {
            Run(r) if matches!(r.items.as_slice(), [editor::ImportedInline::Text(text)] if text == &label) => Some(r),
            _ => None,
        }).unwrap();
        let link = Hyperlink::new(
            HyperlinkTarget::external("https://example.com/entry-review")?,
            vec![TextRun::new("Review link").into()],
        );
        let payload = Document::new(vec![
            Section::new(vec![Paragraph::from_children(vec![link.into()]).into()]).into(),
        ])
        .resolve()
        .await?;
        let ResolvedBlock::Paragraph(p) = &payload.sections()[0].blocks()[0] else {
            unreachable!()
        };
        entries_output = source.edit_entry_inlines(
            &e.part_name,
            &[editor::InlineEdit {
                target: run.anchor.clone(),
                action: editor::InlineAction::InsertAfter(p.runs()[0].clone()),
            }],
        )?;
    }
    std::fs::write(output.with_extension("entry-text.docx"), entries_output)?;
    let owner = &entries
        .iter()
        .find(|e| e.kind == editor::EntryKind::Footnote)
        .unwrap()
        .part_name;
    let target = imported
        .body()
        .iter()
        .find_map(|b| match b {
            editor::BodyBlock::Paragraph(p)
                if p.text == "Footnote reference; endnote reference" =>
            {
                p.inline_structure.first()
            }
            _ => None,
        })
        .unwrap();
    let Run(first) = target else { unreachable!() };
    let created = imported.edit_note_reference(
        owner,
        &first.anchor,
        editor::NoteReferenceEdit::Create {
            id: 47,
            text: "Lifecycle note".into(),
        },
    )?;
    let appended = editor::ImportedDocument::open(&created)?.edit_entry(
        owner,
        "47",
        editor::EntryEdit::InsertParagraph {
            index: 1,
            text: "Second note paragraph".into(),
        },
    )?;
    let lifecycle = editor::ImportedDocument::open(&appended)?.edit_entry(
        &review.part_name,
        &review.id,
        editor::EntryEdit::CommentInitials(Some("ED".into())),
    )?;
    std::fs::write(output.with_extension("entry-edits.docx"), lifecycle)?;
    let inline_label = format!("{INLINE_LABEL}{}", INLINE_LINK_RUNS.concat());
    assert!(imported.body().iter().any(|block| matches!(
        block, editor::BodyBlock::ContentControl(control)
            if control.tag.as_deref() == Some("review-block")
                && control.alias.as_deref() == Some("Review block")
                && control.lock.as_deref() == Some("sdtLocked")
    )));
    let (paragraph_index, paragraph) = imported
        .body()
        .iter()
        .enumerate()
        .find_map(|(index, block)| match block {
            editor::BodyBlock::Paragraph(paragraph) if paragraph.text == inline_label => {
                Some((index, paragraph))
            }
            _ => None,
        })
        .expect("labeled inline structure");
    use editor::ImportedInlineNode::{Hyperlink as Link, Run};
    let [Run(label), Link(link)] = paragraph.inline_structure.as_slice() else {
        panic!("expected a standalone run and hyperlink");
    };
    let replacement = "Imported editor: replaced this run by its anchor. ";
    let paragraph_payload = Document::new(vec![
        Section::new(vec![
            Paragraph::new("Imported editor: inserted paragraph.").into(),
        ])
        .into(),
    ])
    .resolve()
    .await?;
    let ResolvedBlock::Paragraph(inserted_paragraph) = &paragraph_payload.sections()[0].blocks()[0]
    else {
        panic!("expected paragraph payload");
    };
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
    assert_eq!(
        imported.core_title()?.as_deref(),
        Some("All features visual inspection")
    );
    assert_eq!(imported.default_alignment()?, None);
    assert_eq!(
        imported.document_defaults()?[&editor::DefaultField::SizeHalfPoints],
        editor::DefaultValue::Integer(22)
    );
    assert!(
        imported
            .abstract_starts()?
            .iter()
            .any(|s| (s.abstract_id, s.level, s.start) == (100, 0, Some(1)))
    );
    let controls = imported.controls();
    for (context, id) in [
        ("inline", "5001"),
        ("block", "5002"),
        ("row", "5003"),
        ("cell", "5004"),
    ] {
        assert!(
            controls
                .iter()
                .any(|c| c.context.as_str() == context && c.id.as_deref() == Some(id))
        );
    }
    let caption = controls
        .iter()
        .find(|c| c.id.as_deref() == Some("5001"))
        .unwrap();
    let preview = imported.preview_edits(editor::EditBatch {
        styles: &[editor::StyleEdit::Add(
            rocx::NumberingStyleDefinition {
                name: "Reviewed List".into(),
                numbering_id: 9049,
            }
            .into(),
        )],
        run_text: &[editor::RunTextEdit {
            target: label.anchor.clone(),
            text: replacement.into(),
        }],
        paragraphs: &[editor::ParagraphInsertion {
            target: paragraph.anchor.clone(),
            paragraph: inserted_paragraph.clone(),
        }],
        ranges: &[editor::TextRangeEdit {
            target: targets[0].anchor.clone(),
            start: 0,
            end: 8,
            text: "Ranged 📝".into(),
        }],
        text_insertions: &[editor::TextInsertion {
            target: targets[0].anchor.clone(),
            position: 8,
            text: " + point".into(),
        }],
        resources: &[
            editor::ResourceEdit::Numbering(editor::NumberingEdit::ReplaceAbstract {
                abstract_id: 0,
                levels: vec![rocx::NumberingLevel {
                    level: 0,
                    start: 3,
                    format: "upperLetter".into(),
                    text: "%1)".into(),
                    ..Default::default()
                }],
            }),
            editor::ResourceEdit::Numbering(editor::NumberingEdit::AbstractLevel {
                abstract_id: 1,
                level: 0,
                value: Some(Box::new(rocx::NumberingLevel {
                    level: 0,
                    start: 4,
                    format: "upperRoman".into(),
                    text: "%1.".into(),
                    ..Default::default()
                })),
            }),
            editor::ResourceEdit::Numbering(editor::NumberingEdit::RelinkInstance {
                num_id: 8,
                abstract_id: 0,
            }),
            editor::ResourceEdit::Numbering(editor::NumberingEdit::ReplaceInstance(Box::new(
                rocx::NumberingDefinition {
                    id: 9,
                    abstract_id: Some(1),
                    start_overrides: vec![rocx::NumberingStartOverride {
                        level: 0,
                        start: 12,
                    }],
                    ..Default::default()
                },
            ))),
            editor::ResourceEdit::Numbering(editor::NumberingEdit::StoryParagraph {
                target: imported.story_paragraph_anchor(&review.part_name, 0, Some(&review.id))?,
                value: Some(rocx::NumberingReference { id: 9049, level: 0 }),
            }),
            editor::ResourceEdit::Numbering(editor::NumberingEdit::AddAbstract {
                abstract_id: 9048,
                levels: vec![rocx::NumberingLevel {
                    level: 0,
                    start: 2,
                    format: "decimal".into(),
                    text: "%1.".into(),
                    ..Default::default()
                }],
            }),
            editor::ResourceEdit::Numbering(editor::NumberingEdit::InstanceLevel {
                num_id: 9049,
                level: 0,
                value: Some(Box::new(rocx::NumberingLevel {
                    level: 0,
                    start: 1,
                    format: "lowerRoman".into(),
                    text: "(%1)".into(),
                    ..Default::default()
                })),
            }),
            editor::ResourceEdit::Numbering(editor::NumberingEdit::AddInstance {
                num_id: 9049,
                abstract_id: 9048,
            }),
            editor::ResourceEdit::Numbering(editor::NumberingEdit::InstanceStart {
                num_id: 9049,
                level: 0,
                start: Some(6),
            }),
            editor::ResourceEdit::Numbering(editor::NumberingEdit::Paragraph {
                target: targets[0].anchor.clone(),
                value: Some(rocx::NumberingReference { id: 9049, level: 0 }),
            }),
            editor::ResourceEdit::CoreTitle(Some("Edited resource sample".into())),
            editor::ResourceEdit::CoreSubject(Some("Imported resource edits".into())),
            editor::ResourceEdit::Metadata(
                editor::MetadataField::Creator,
                Some("Review team".into()),
            ),
            editor::ResourceEdit::Metadata(
                editor::MetadataField::Manager,
                Some("Review lead".into()),
            ),
            editor::ResourceEdit::CustomProperty(
                "Reviewed".into(),
                Some(editor::CustomValue::Boolean(false)),
            ),
            editor::ResourceEdit::DefaultAlignment(Some(Alignment::Center)),
            editor::ResourceEdit::Default(
                editor::DefaultField::Bold,
                Some(editor::DefaultValue::Bool(false)),
            ),
            editor::ResourceEdit::Default(
                editor::DefaultField::FootnoteStart,
                Some(editor::DefaultValue::Integer(5)),
            ),
            editor::ResourceEdit::Default(editor::DefaultField::AfterTwips, None),
            editor::ResourceEdit::AbstractStart(editor::AbstractStart {
                abstract_id: 100,
                level: 0,
                start: Some(7),
            }),
        ],
        controls: &[editor::ControlEdit {
            target: caption.anchor.clone(),
            alias: Some("Imported editor caption".into()),
            tag: None,
            text: None,
        }],
        ..Default::default()
    })?;
    assert_eq!(preview.paragraph_insertions(), &[paragraph_index]);
    assert!(
        preview
            .run_text_changes()
            .iter()
            .any(|c| c.before == INLINE_LABEL)
    );
    let whole = imported.preview_with_inlines(
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[
            editor::InlineEdit {
                target: label.anchor.clone(),
                action: editor::InlineAction::SetToggle(editor::RunToggle::Italic, Some(true)),
            },
            editor::InlineEdit {
                target: label.anchor.clone(),
                action: editor::InlineAction::PatchFormat {
                    set: Box::new(rocx::RunFormat::default().with_size(12.5)),
                    clear: vec![editor::RunFormatField::FontEastAsia],
                },
            },
            editor::InlineEdit {
                target: link.anchor.clone(),
                action: editor::InlineAction::InsertAfter(inserted_paragraph.runs()[0].clone()),
            },
        ],
    )?;
    assert_eq!(whole.inline_changes().last(), Some(&(paragraph_index, 1)));
    assert_eq!(label.italic, None);
    let removal = imported.preview_with_run_removals(
        &[],
        &[],
        &[],
        &[],
        std::slice::from_ref(&label.anchor),
    )?;
    assert_eq!(removal.run_removals()[0].before, INLINE_LABEL);
    let edited_bytes = preview.to_docx()?;
    let resource_result = editor::ImportedDocument::open(&edited_bytes)?;
    assert_eq!(
        resource_result
            .metadata_field(editor::MetadataField::Manager)?
            .as_deref(),
        Some("Review lead")
    );
    assert_eq!(
        resource_result.custom_property("Reviewed")?,
        Some(editor::CustomValue::Boolean(false))
    );
    std::fs::write(output.with_extension("run-text.docx"), edited_bytes)?;
    assert_eq!(
        label.items,
        [editor::ImportedInline::Text(INLINE_LABEL.into())]
    );
    assert_eq!(link.bookmark.as_deref(), Some("CatalogBookmark"));
    assert_eq!(link.relationship_id, None);
    assert!(
        matches!(imported.inline_at(&link.anchor)?, Link(found) if found.anchor == link.anchor)
    );
    let [Run(first), Run(second)] = link.children.as_slice() else {
        panic!("expected two hyperlink runs");
    };
    for (run, text) in [first, second].into_iter().zip(INLINE_LINK_RUNS) {
        assert_eq!(run.items, [editor::ImportedInline::Text(text.into())]);
        assert!(
            matches!(imported.inline_at(&run.anchor)?, Run(found) if found.anchor == run.anchor)
        );
    }
    let mut table = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![run(
                "Imported editor table: bold label",
                RunFormat::default().with_bold(true),
            )])
            .into(),
            TableCell::new(vec![
                Paragraph::from_children(vec![
                    TextRun::new("Fixed-width ").into(),
                    TextRun::new("second column").into(),
                ])
                .into(),
            ])
            .into(),
        ])
        .into(),
    ]);
    table.properties = TableProperties {
        column_widths: Some(vec![2160, 2160]),
        layout: Some(TableLayout::Fixed),
        ..Default::default()
    }
    .into();
    let resolved_table = table.await?.expect("nonempty showcase table");
    let preview = imported.preview_batch(
        &[editor::ParagraphAlignmentEdit {
            target: targets[0].anchor.clone(),
            alignment: Alignment::Center,
        }],
        &[editor::TableInsertion {
            target: targets[0].anchor.clone(),
            table: resolved_table.clone(),
        }],
    )?;
    let edited = preview.to_docx()?;
    // Typed physical-table readback from the edited original package.
    let reopened = editor::ImportedDocument::open(&edited)?;
    let after = preview.insertions()[0].after_body_index + 1;
    assert!(reopened.inline_at(&link.anchor).is_err());
    assert!(matches!(
        &reopened.body()[paragraph_index + 1],
        editor::BodyBlock::Paragraph(p) if p.text == inline_label
    ));
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
    let [Run(first)] = cell.inline_structure(0).unwrap() else {
        panic!("expected one run in first cell");
    };
    let editor::ImportedRowChild::Cell(second) = &row.children[1] else {
        panic!("expected second cell");
    };
    let [Run(prefix), Run(suffix)] = second.inline_structure(0).unwrap() else {
        panic!("expected two runs in second cell");
    };
    assert_eq!(
        prefix.items,
        [editor::ImportedInline::Text("Fixed-width ".into())]
    );
    assert_eq!(
        suffix.items,
        [editor::ImportedInline::Text("second column".into())]
    );
    assert_ne!(first.anchor, prefix.anchor);
    assert!(matches!(reopened.inline_at(&suffix.anchor)?, Run(run) if run.anchor == suffix.anchor));
    assert!(imported.inline_at(&suffix.anchor).is_err());
    // Layout sample: change the direct layout while preserving grid widths.
    assert_eq!(table.layout, Some(TableLayout::Fixed));
    assert_eq!(table.grid_widths, [Some(2160), Some(2160)]);
    let layout = reopened
        .preview_with_table_structure(
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[editor::TableStructureEdit {
                target: table.anchor.clone(),
                operation: editor::TableOperation::Layout(Some(TableLayout::Autofit)),
            }],
        )?
        .to_docx()?;
    assert!(
        matches!(&editor::ImportedDocument::open(&layout)?.body()[after], editor::BodyBlock::Table(t) if t.layout == Some(TableLayout::Autofit) && t.grid_widths == table.grid_widths)
    );
    // Structural sample: append an empty row to this original table.
    let appended = reopened
        .preview_with_table_structure(
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[editor::TableStructureEdit {
                target: table.anchor.clone(),
                operation: editor::TableOperation::InsertRow(1),
            }],
        )?
        .to_docx()?;
    let appended = editor::ImportedDocument::open(&appended)?;
    assert!(matches!(&appended.body()[after],editor::BodyBlock::Table(t) if t.children.len()==2));
    let cell_preview = reopened.preview_with_cells(
        &[],
        &[],
        &[],
        &[],
        &[editor::CellAlignmentEdit {
            target: table.anchor.clone(),
            row_index: 0,
            cell_index: 0,
            alignment: VerticalAlignment::Center,
        }],
    )?;
    assert_eq!(cell_preview.cell_alignment_changes().len(), 1);
    std::fs::write(
        output.with_extension("edited.docx"),
        cell_preview.to_docx()?,
    )?;

    // Both previews start from the same snapshot: two sections share one header part.
    let mut first = Section::new(vec![p(HEADER_BATCH_LABEL)]);
    first.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![
            Paragraph::from_children(vec![
                TextRun::new(HEADER_INLINE_LABEL).into(),
                Hyperlink::new(
                    HyperlinkTarget::external("https://example.invalid/catalog-header")?,
                    HEADER_LINK_RUNS
                        .into_iter()
                        .map(|text| TextRun::new(text).into())
                        .collect::<InlineChildren>(),
                )
                .into(),
            ])
            .into(),
        ])
        .into(),
    );
    for (variant, name) in [
        (HeaderFooterType::Default, "default"),
        (HeaderFooterType::First, "first"),
        (HeaderFooterType::Even, "even"),
    ] {
        if variant != HeaderFooterType::Default {
            first.headers.insert(
                variant,
                Header::new(vec![p(format!("Story inspection: {name} header"))]).into(),
            );
        }
        first.footers.insert(
            variant,
            Footer::new(vec![p(format!("Story inspection: {name} footer"))]).into(),
        );
    }
    let second = Section::new(vec![p(
        "Header image scope: second section shares the same header.",
    )]);
    let mut source = Document::new(vec![first.into(), second.into()]);
    source.timestamp = Some(STAMP.into()).into();
    let source = source.resolve().await?.to_docx()?;
    std::fs::write(output.with_extension("header-source.docx"), &source)?;
    let imported = editor::ImportedDocument::open(&source)?;
    // Independent revision previews: track text or move a run, then review only the pair.
    let run_at = |block: usize| {
        let editor::BodyBlock::Paragraph(p) = &imported.body()[block] else {
            panic!("body paragraph")
        };
        let Run(r) = &p.inline_structure[0] else {
            panic!("body run")
        };
        r.anchor.clone()
    };
    let from = run_at(0);
    let tracked = imported.preview_tracked_text(
        &from,
        editor::TrackedText::Replace("Reviewed label".into()),
        "Reviewer",
        Some(STAMP),
    )?;
    let tracked = editor::ImportedDocument::open(&tracked.to_docx()?)?;
    assert!(matches!(&tracked.body()[0], editor::BodyBlock::Paragraph(p) if p.text.is_empty()));
    let moved =
        imported.preview_tracked_move(&from, &run_at(2), "CatalogMove", "Reviewer", STAMP)?;
    let pair = editor::ImportedDocument::open(&moved.to_docx()?)?;
    let wrapper = |block: usize, index: usize| {
        let editor::BodyBlock::Paragraph(p) = &pair.body()[block] else {
            panic!("body paragraph")
        };
        let editor::ImportedInlineNode::Revision(r) = &p.inline_structure[index] else {
            panic!("move wrapper")
        };
        r.anchor.clone()
    };
    let restored = pair.preview_revision(
        &wrapper(0, 1),
        TextRevisionAction::Reject,
        Some(&wrapper(2, 2)),
    )?;
    let restored = editor::ImportedDocument::open(&restored.to_docx()?)?;
    assert!(
        matches!(&restored.body()[0], editor::BodyBlock::Paragraph(p) if p.text == HEADER_BATCH_LABEL)
    );

    // Independent block previews use the first paragraph of the inherited section.
    let editor::BodyBlock::Paragraph(last) = &imported.body()[2] else {
        unreachable!()
    };
    let target = &last.anchor;
    let payload = Component::resolve(Paragraph::new("Block editor: replacement paragraph."))
        .await?
        .unwrap();
    use editor::BlockEdit;
    for edit in [
        BlockEdit::Replace(target.clone(), Box::new(payload.clone())),
        BlockEdit::Remove(target.clone()),
        BlockEdit::ReplaceSection(target.clone(), vec![ResolvedBlock::Paragraph(payload)]),
        BlockEdit::KeepWithNext(target.clone(), true),
        BlockEdit::ParagraphFlag(
            target.clone(),
            editor::ParagraphFlag::KeepTogether,
            Some(true),
        ),
        BlockEdit::SectionFlag(target.clone(), editor::SectionFlag::RtlGutter, Some(true)),
        BlockEdit::SectionType(target.clone(), Some(SectionType::EvenPage)),
    ] {
        assert_eq!(imported.preview_blocks(&[edit])?.block_changes(), &[2]);
    }
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
    // Fresh headers begin with the built-in Header style paragraph.
    let [
        editor::ImportedHeaderBlock::Paragraph(empty),
        editor::ImportedHeaderBlock::Paragraph(items),
    ] = headers[1].blocks.as_slice()
    else {
        panic!("expected default-header paragraph")
    };
    assert!(empty.is_empty());
    assert_eq!(
        items,
        &[
            HEADER_INLINE_LABEL,
            HEADER_LINK_RUNS[0],
            HEADER_LINK_RUNS[1]
        ]
        .map(|text| editor::ImportedInline::Text(text.into()))
    );
    let [Run(label), Link(link)] = headers[0].inline_structure(1).unwrap() else {
        panic!("expected run and hyperlink")
    };
    let [_, Link(inherited)] = headers[1].inline_structure(1).unwrap() else {
        panic!("expected inherited hyperlink")
    };
    assert!(headers[1].inherited && link.anchor == inherited.anchor);
    assert!(link.relationship_id.is_some());
    assert!(matches!(imported.inline_at(&label.anchor)?, Run(run) if run.anchor == label.anchor));
    assert!(
        matches!(imported.inline_at(&link.anchor)?, Link(found) if found.anchor == link.anchor)
    );
    for (child, text) in link.children.iter().zip(HEADER_LINK_RUNS) {
        let Run(run) = child else {
            panic!("expected hyperlink run")
        };
        assert!(matches!(imported.inline_at(&run.anchor)?, Run(found)
            if found.items == [editor::ImportedInline::Text(text.into())]));
    }
    assert_eq!(link.children.len(), HEADER_LINK_RUNS.len());
    let stories = imported.stories()?;
    let (explicit, inherited) = stories.split_at(6);
    assert_eq!(inherited.len(), explicit.len());
    for (first, second) in explicit.iter().zip(inherited) {
        assert_eq!((first.section_index, second.section_index), (0, 1));
        assert!(!first.inherited && second.inherited);
        assert_eq!((first.kind, first.variant), (second.kind, second.variant));
        assert_eq!(first.part_name, second.part_name);
        let (Some(Run(a)), Some(Run(b))) = (
            first.inline_structure(1).and_then(|nodes| nodes.first()),
            second.inline_structure(1).and_then(|nodes| nodes.first()),
        ) else {
            panic!("expected labeled story runs");
        };
        assert_eq!(a.anchor, b.anchor);
        assert!(matches!(imported.inline_at(&b.anchor)?, Run(run) if run.anchor == a.anchor));
    }
    assert!(
        matches!(stories[0].inline_structure(1).unwrap(), [Run(run), _] if run.anchor == label.anchor)
    );
    let editor::BodyBlock::Paragraph(body_target) = &imported.body()[0] else {
        panic!("expected labeled body paragraph")
    };
    assert_eq!(body_target.text, HEADER_BATCH_LABEL);
    let target = &headers[1];
    let resolved = image("png", None).await?.expect("showcase image");
    let footer = stories.last().unwrap(); // inherited even footer
    let Run(label) = &footer.inline_structure(1).unwrap()[0] else {
        unreachable!()
    };
    let bytes = imported.edit_story_text(
        &footer.part_name,
        &[editor::RunTextEdit {
            target: label.anchor.clone(),
            text: "Edited shared even footer".into(),
        }],
    )?;
    let edited_footer = editor::ImportedDocument::open(&bytes)?;
    let edit = editor::HeaderImageEdit {
        target: edited_footer.stories()?.pop().unwrap().anchor.unwrap(),
        mode: editor::HeaderEditMode::SectionLocal,
        image: resolved.image().unwrap().clone(),
    };
    std::fs::write(
        output.with_extension("footer-even.docx"),
        edited_footer.preview_header_image(&edit)?.to_docx()?,
    )?;
    let first_page = stories
        .iter()
        .find(|story| {
            story.section_index == 1
                && story.kind == editor::SectionStoryKind::Header
                && story.variant == HeaderFooterType::First
        })
        .expect("first-page header");
    for (mode, suffix) in [
        (editor::HeaderEditMode::Shared, "header-first-shared.docx"),
        (
            editor::HeaderEditMode::SectionLocal,
            "header-first-local.docx",
        ),
    ] {
        let preview = imported.preview_header_image(&editor::HeaderImageEdit {
            target: first_page.anchor.clone().expect("first-page edit anchor"),
            mode,
            image: resolved.image().expect("resolved image").clone(),
        })?;
        assert_eq!(
            preview.change().affected_sections,
            if mode == editor::HeaderEditMode::Shared {
                &[0, 1][..]
            } else {
                &[1][..]
            }
        );
        std::fs::write(output.with_extension(suffix), preview.to_docx()?)?;
    }
    for (mode, suffix) in [
        (editor::HeaderEditMode::Shared, "header-shared.docx"),
        (editor::HeaderEditMode::SectionLocal, "header-local.docx"),
    ] {
        let preview = imported.preview_transaction(
            &[editor::ParagraphAlignmentEdit {
                target: body_target.anchor.clone(),
                alignment: Alignment::Center,
            }],
            &[editor::TableInsertion {
                target: body_target.anchor.clone(),
                table: resolved_table.clone(),
            }],
            &[editor::HeaderImageEdit {
                target: target.anchor.clone(),
                mode,
                image: resolved.image().expect("resolved image").clone(),
            }],
        )?;
        let affected: &[_] = match mode {
            editor::HeaderEditMode::Shared => &[0, 1],
            editor::HeaderEditMode::SectionLocal => &[1],
        };
        assert_eq!(preview.header_changes()[0].affected_sections, affected);
        std::fs::write(output.with_extension(suffix), preview.to_docx()?)?;
    }
    Ok(())
}
