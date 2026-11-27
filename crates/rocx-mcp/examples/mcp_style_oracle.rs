//! Direct Rust oracle for imported style editing.
use rocx::editor::{BodyBlock, EditBatch, ImportedDocument, ParagraphInsertion, StyleEdit};
use rocx::{
    Alignment, CharacterStyleDefinition, Component, LineHeight, NumberingStyleDefinition,
    Paragraph, ParagraphFormat, ParagraphStyleDefinition, RunFormat, TableSectionFormat,
    TableStyleDefinition,
};

fn main() -> rocx::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let doc = ImportedDocument::open(&std::fs::read(args.next().unwrap()).unwrap())?;
    let action = args.next().unwrap().into_string().unwrap();
    let output = args.next().unwrap();
    let anchor = |id| doc.style_anchor(id).unwrap();
    let region = TableSectionFormat::default()
        .with_bold(false)
        .with_italic(true)
        .with_alignment(Alignment::Center)
        .with_background([1, 2, 3]);
    let mut paragraphs = Vec::new();
    let styles = match action.as_str() {
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
