//! Direct Rust byte oracle for the independent body block fixture.
use rocx::{
    Alignment, Component, Paragraph, ParagraphFormat, ResolvedBlock, ResolvedParagraph, RunFormat,
    SectionType, TextRun,
    editor::{
        BlockEdit, BodyBlock, EditBatch, ImportedDocument, ImportedInlineNode, ImportedParagraph,
        ParagraphAnchor, ParagraphFlag, RunTextEdit, SectionFlag,
    },
};

fn paragraph(text: &str, bold: bool, alignment: Option<Alignment>) -> ResolvedParagraph {
    let run = TextRun::new(text).with_format(RunFormat {
        bold: bold.then_some(true),
        ..Default::default()
    });
    futures_executor::block_on(
        Paragraph::from_children(vec![run.into()])
            .with_format(ParagraphFormat {
                alignment,
                ..Default::default()
            })
            .resolve(),
    )
    .expect("resolve fixture paragraph")
    .expect("fixture paragraph is enabled")
}

fn flags_and_types(anchor: impl Fn(usize) -> ParagraphAnchor) -> Vec<BlockEdit> {
    let mut blocks = Vec::new();
    for (index, value) in [Some(true), Some(false), None].into_iter().enumerate() {
        blocks.extend(
            ParagraphFlag::ALL
                .iter()
                .map(|&flag| BlockEdit::ParagraphFlag(anchor(2 * index), flag, value)),
        );
        blocks.extend(
            SectionFlag::ALL
                .iter()
                .map(|&flag| BlockEdit::SectionFlag(anchor(2 * index), flag, value)),
        );
    }
    for (index, value) in [
        Some(SectionType::NextPage),
        Some(SectionType::NextColumn),
        Some(SectionType::Continuous),
        Some(SectionType::EvenPage),
        Some(SectionType::OddPage),
        None,
    ]
    .into_iter()
    .enumerate()
    {
        blocks.push(BlockEdit::SectionType(anchor(2 * index), value));
    }
    blocks
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let [source, output, mode] = args.as_slice() else {
        return Err(
            "usage: mcp_block_oracle SOURCE OUTPUT mixed|split|merge|flags-and-types".into(),
        );
    };
    let document = ImportedDocument::open(&std::fs::read(source)?)?;
    let sections = document.sections()?;
    let p = |index| -> &ImportedParagraph {
        let BodyBlock::Paragraph(paragraph) = &document.body()[index] else {
            panic!("expected fixture paragraph at {index}");
        };
        paragraph
    };
    let anchor = |index| p(index).anchor.clone();
    if mode == "flags-and-types" {
        let preview = document.preview_edits(EditBatch {
            blocks: &flags_and_types(anchor),
            ..Default::default()
        })?;
        std::fs::write(output, preview.to_docx()?)?;
        return Ok(());
    }
    let (blocks, run_index, text) = match mode.to_str() {
        Some("mixed") => (
            vec![
                BlockEdit::ReplaceSection(
                    anchor(5),
                    vec![ResolvedBlock::Paragraph(paragraph(
                        "new section",
                        false,
                        None,
                    ))],
                ),
                BlockEdit::Remove(anchor(2)),
                BlockEdit::Replace(
                    anchor(1),
                    Box::new(paragraph("new & bold", true, Some(Alignment::Center))),
                ),
                BlockEdit::KeepWithNext(anchor(0), false),
                BlockEdit::ParagraphFlag(anchor(0), ParagraphFlag::WidowControl, None),
                BlockEdit::SectionFlag(anchor(0), SectionFlag::TitlePage, None),
                BlockEdit::SectionFlag(anchor(8), SectionFlag::TitlePage, Some(false)),
                BlockEdit::SectionType(anchor(0), Some(SectionType::EvenPage)),
                BlockEdit::SectionType(anchor(8), None),
            ],
            3,
            "edited original run",
        ),
        Some("split") => (
            vec![BlockEdit::SplitSection(sections[0].anchor.clone(), 2)],
            9,
            "edited far section",
        ),
        Some("merge") => (
            vec![BlockEdit::MergeSection(sections[0].anchor.clone())],
            9,
            "edited far section",
        ),
        _ => return Err("unknown block oracle mode".into()),
    };
    let [ImportedInlineNode::Run(run)] = p(run_index).inline_structure.as_slice() else {
        return Err("expected one fixture run".into());
    };
    let preview = document.preview_edits(EditBatch {
        blocks: &blocks,
        run_text: &[RunTextEdit {
            target: run.anchor.clone(),
            text: text.into(),
        }],
        ..Default::default()
    })?;
    std::fs::write(output, preview.to_docx()?)?;
    Ok(())
}
