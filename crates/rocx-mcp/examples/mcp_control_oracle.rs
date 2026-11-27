use rocx::editor::{
    BodyBlock, ControlContext, ControlEdit, EditBatch, ImportedDocument, ImportedInlineNode,
    RunTextEdit, WrapperEdit,
};
use rocx::{
    Component, ContentControlListItem, ContentControlProperties, ContentControlText, Paragraph,
    ResolvedBlock, RunFormat, TextRun,
};

fn main() -> rocx::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let doc = ImportedDocument::open(&std::fs::read(args.next().unwrap()).unwrap())?;
    let action = args.next().unwrap().into_string().unwrap();
    let output = args.next().unwrap();
    let target = |index: usize| doc.controls()[index].anchor.clone();
    let site = |context, text| {
        doc.control_sites()
            .iter()
            .find(|s| s.context == context && s.text == text)
            .unwrap()
            .clone()
    };
    let choices = || {
        ["Alpha", "Gamma"]
            .into_iter()
            .map(|s| ContentControlListItem {
                display_text: s.into(),
                value: s.into(),
            })
            .collect()
    };
    let BodyBlock::Paragraph(outside) = &doc.body()[10] else {
        panic!("expected ordinary paragraph")
    };
    let ImportedInlineNode::Run(outside) = &outside.inline_structure[0] else {
        panic!("expected ordinary run")
    };
    let rich_run = || -> rocx::Result<_> {
        Ok(futures_executor::block_on(
            TextRun::new("new rich text")
                .with_format(RunFormat::default().with_bold(true))
                .resolve(),
        )?
        .unwrap())
    };
    let preview = match action.as_str() {
        "mixed" => doc.preview_edits(EditBatch {
            controls: &[ControlEdit {
                target: target(0),
                tag: Some("patched".into()),
                alias: Some("New alias".into()),
                text: Some("updated plain".into()),
            }],
            wrappers: &[
                WrapperEdit::ReplaceInlines(target(1), vec![rich_run()?]),
                WrapperEdit::Choice {
                    target: target(2),
                    selected: "Gamma".into(),
                    items: Some(choices()),
                },
                WrapperEdit::Unwrap(target(3)),
                WrapperEdit::ReplaceBlocks(
                    target(4),
                    vec![ResolvedBlock::Paragraph(
                        futures_executor::block_on(Paragraph::new("new block").resolve())?.unwrap(),
                    )],
                ),
                WrapperEdit::Define(
                    target(5),
                    ContentControlProperties {
                        id: Some(42),
                        text: ContentControlText::Plain {
                            multi_line: Some(false),
                        },
                        ..Default::default()
                    },
                ),
            ],
            run_text: &[RunTextEdit {
                target: outside.anchor.clone(),
                text: "edited outside".into(),
            }],
            ..Default::default()
        })?,
        "block" => doc.preview_edits(EditBatch {
            wrappers: &[WrapperEdit::Wrap {
                first: site(ControlContext::Block, "block first"),
                last: Some(site(ControlContext::Block, "block last")),
                tag: "wrapped".into(),
            }],
            ..Default::default()
        })?,
        "inline" => {
            doc.preview_wrap_control(&site(ControlContext::Inline, "block first"), "wrapped")?
        }
        "row" => doc.preview_wrap_control(&site(ControlContext::Row, "table text"), "wrapped")?,
        "cell" => doc.preview_wrap_control(&site(ControlContext::Cell, "table text"), "wrapped")?,
        _ => panic!("unknown control oracle action"),
    };
    std::fs::write(output, preview.to_docx()?).unwrap();
    Ok(())
}
