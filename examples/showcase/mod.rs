//! Independent Rust authoring of the same catalog as all_features.py.
use rocx::*;
use std::path::Path;
mod formatting;
mod stories;
mod tables;

pub const STAMP: &str = "2026-11-19T00:00:00Z";
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
    }
}
pub async fn generate(output: &Path) -> Result<()> {
    // A real two-stage template edit exercises insertion and cross-run replacement.
    let seed = Paragraph::from_children(vec![
        TextRun::new("Template {{re").into(),
        TextRun::new("place}}: cross-run replacement.").into(),
    ]);
    let mut template_document = Document::new(vec![
        Section::new(vec![
            seed.into(),
            p("Template retained paragraph: insertion appears before this."),
        ])
        .into(),
    ]);
    template_document.timestamp = Some(STAMP.into()).into();
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
        let mut section = Section::new(vec![
            p(format!(
                "09 Section: break={label}; landscape; 2 columns with separator; page numbering lowerRoman starts at {}; vertical alignment={}; section bidi={}; rtl_gutter={}.",
                i + 1,
                ["top", "center", "bottom", "both", "top"][i],
                i % 2 == 1,
                i % 2 == 1
            )),
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
        paragraph: Some(ParagraphFormat {
            spacing_after: Some(4.),
            line_height: Some(LineHeight::Multiple(1.1)),
            ..Default::default()
        }),
    })
    .into();
    let mut styles = formatting::styles();
    styles.extend(tables::styles());
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
    Ok(())
}
