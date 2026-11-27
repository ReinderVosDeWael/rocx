//! Direct Rust oracle for ordinary body-inline property edits.
use rocx::{
    RunFormat,
    editor::{
        BodyBlock, EditBatch, ImportedDocument, ImportedInlineNode, InlineAction as A, InlineEdit,
        RunFormatField, RunToggle,
    },
};
use std::{env, fs};

fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let [mode, output, source] = args.as_slice() else {
        panic!("expected MODE OUTPUT SOURCE");
    };
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let edit = |body: usize, index: usize, action| {
        let BodyBlock::Paragraph(p) = &doc.body()[body] else {
            panic!("expected fixture paragraph");
        };
        let anchor = match &p.inline_structure[index] {
            ImportedInlineNode::Run(run) => &run.anchor,
            ImportedInlineNode::Hyperlink(link) => &link.anchor,
            _ => panic!("expected fixture run or hyperlink"),
        };
        InlineEdit {
            target: anchor.clone(),
            action,
        }
    };
    let mut edits = Vec::new();
    let mut toggles = |index, value| {
        for toggle in [
            RunToggle::Bold,
            RunToggle::Italic,
            RunToggle::Strike,
            RunToggle::Hidden,
        ] {
            edits.push(edit(2, index, A::SetToggle(toggle, value)));
        }
    };
    let patch = |size, clear| A::PatchFormat {
        set: Box::new(RunFormat {
            size: Some(size),
            ..RunFormat::default()
        }),
        clear: vec![clear],
    };
    match mode.to_str() {
        Some("mixed") => {
            toggles(0, Some(true));
            toggles(1, Some(false));
            toggles(2, None);
            edits.extend([
                edit(2, 3, patch(13.5, RunFormatField::Underline)),
                edit(0, 1, A::HyperlinkTooltip(Some("Updated & tip".into()))),
                edit(0, 1, A::HyperlinkHistory(Some(false))),
                edit(2, 4, A::HyperlinkTooltip(None)),
                edit(2, 4, A::HyperlinkHistory(None)),
                edit(2, 5, A::RemoveObject),
                edit(
                    2,
                    6,
                    A::SetDrawingDescription(Some("Alt <updated> & kept".into())),
                ),
            ]);
        }
        Some("noop") => {
            toggles(0, None);
            edits.extend([
                edit(2, 3, patch(15.0, RunFormatField::Color)),
                edit(0, 1, A::HyperlinkTooltip(Some("Original tip".into()))),
                edit(0, 1, A::HyperlinkHistory(Some(true))),
                edit(2, 6, A::SetDrawingDescription(Some("sample.jpg".into()))),
            ]);
        }
        _ => panic!("unexpected oracle mode"),
    }
    let preview = doc
        .preview_edits(EditBatch {
            inline_edits: &edits,
            ..EditBatch::default()
        })
        .unwrap();
    fs::write(output, preview.to_docx().unwrap()).unwrap();
    println!(
        "{}",
        serde_json::to_string(preview.inline_changes()).unwrap()
    );
}
