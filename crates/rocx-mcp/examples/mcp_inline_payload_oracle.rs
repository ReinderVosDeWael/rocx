//! Direct Rust oracle for formatted leaf runs and hyperlink declaration attributes.
use rocx::editor::{
    BodyBlock, EditBatch, ImportedDocument, ImportedInlineNode, InlineAction, InlineEdit,
};
use rocx::{
    AltText, BreakType, Component, Hyperlink, HyperlinkProperties, HyperlinkTarget, ImageData,
    ImageRun, ImageTransformation, ResolvedInline, RunFormat,
};

fn main() -> rocx::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let source = args.next().expect("source path");
    let output = args.next().expect("output path");
    let rich = match args.next().as_deref().and_then(|arg| arg.to_str()) {
        Some("rich") => true,
        Some("legacy") => false,
        _ => panic!("expected rich or legacy mode"),
    };
    let document = ImportedDocument::open(&std::fs::read(source)?)?;
    let BodyBlock::Paragraph(paragraph) = &document.body()[0] else {
        panic!("expected source paragraph")
    };
    let [ImportedInlineNode::Run(lead), ImportedInlineNode::Run(tail)] =
        paragraph.inline_structure.as_slice()
    else {
        panic!("expected two source runs")
    };

    let bytes = include_bytes!("../../../tests/fixtures/images/sample.png").to_vec();
    let mut image = ImageRun::new(if rich {
        ImageData::NamedBytes {
            bytes,
            source_name: "logo & seal.jpg".into(),
        }
    } else {
        ImageData::Bytes(bytes)
    });
    if rich {
        image = image
            .with_transformation(Some(ImageTransformation::default().with_width(24.)))
            .with_alt_text(Some(AltText::default().with_description("A logo")));
    }
    let ResolvedInline::Image { image, .. } =
        futures_executor::block_on(image.resolve())?.expect("unconditional picture")
    else {
        panic!("expected resolved picture")
    };
    let children = vec![
        ResolvedInline::Tab {
            format: RunFormat {
                bold: rich.then_some(false),
                ..Default::default()
            },
            comment: None,
        }
        .into(),
        ResolvedInline::Break {
            kind: if rich {
                BreakType::Column
            } else {
                BreakType::Line
            },
            format: RunFormat {
                italic: rich.then_some(true),
                ..Default::default()
            },
            comment: None,
        }
        .into(),
    ];
    let link = Hyperlink::new(
        HyperlinkTarget::external("https://example.org/payload")?,
        children,
    )
    .with_properties(HyperlinkProperties {
        tooltip: rich.then(|| "Open <example>".into()),
        history: rich.then_some(false),
    });
    let resolved = futures_executor::block_on(link.resolve())?.expect("unconditional link");
    let preview = document.preview_edits(EditBatch {
        inline_edits: &[
            InlineEdit {
                target: lead.anchor.clone(),
                action: InlineAction::InsertAfter(resolved),
            },
            InlineEdit {
                target: tail.anchor.clone(),
                action: InlineAction::InsertAfter(ResolvedInline::Image {
                    image,
                    format: RunFormat {
                        strike: rich.then_some(true),
                        ..Default::default()
                    },
                    comment: None,
                }),
            },
        ],
        ..Default::default()
    })?;
    std::fs::write(output, preview.to_docx()?)?;
    Ok(())
}
