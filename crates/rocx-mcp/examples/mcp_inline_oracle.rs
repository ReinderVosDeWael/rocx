//! Direct Rust oracle for `tests/mcp/test_body_inlines.py`.
use rocx::{
    FieldInstruction, FieldProperties, ResolvedField, ResolvedInline, RunFormat,
    editor::{
        BodyBlock, EditBatch, ImportedDocument, ImportedInlineNode, InlineAction, InlineEdit,
    },
};
use std::{env, fs};

fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let [mode, output, source] = args.as_slice() else {
        panic!("expected MODE OUTPUT SOURCE");
    };
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let BodyBlock::Paragraph(p) = &doc.body()[0] else {
        panic!("expected paragraph");
    };
    let [
        ImportedInlineNode::Run(lead),
        ImportedInlineNode::Hyperlink(link),
        ImportedInlineNode::Run(tail),
    ] = p.inline_structure.as_slice()
    else {
        panic!("unexpected fixture inlines");
    };
    let edits = if mode == "remove" {
        vec![InlineEdit {
            target: link.anchor.clone(),
            action: InlineAction::RemoveHyperlink,
        }]
    } else {
        assert_eq!(mode, "mixed");
        let BodyBlock::Paragraph(second) = &doc.body()[1] else {
            panic!("expected sentinel paragraph");
        };
        let ImportedInlineNode::Run(sentinel) = &second.inline_structure[0] else {
            panic!("expected sentinel run");
        };
        let field = ResolvedInline::Field(
            ResolvedField::new(
                FieldInstruction::Page,
                vec![ResolvedInline::Text {
                    text: "1".into(),
                    format: RunFormat::default(),
                    comment: None,
                    revision: None,
                }],
                FieldProperties::default(),
            )
            .unwrap(),
        );
        vec![
            InlineEdit {
                target: lead.anchor.clone(),
                action: InlineAction::InsertAfter(field),
            },
            InlineEdit {
                target: link.anchor.clone(),
                action: InlineAction::HyperlinkTooltip(None),
            },
            InlineEdit {
                target: link.anchor.clone(),
                action: InlineAction::HyperlinkHistory(Some(false)),
            },
            InlineEdit {
                target: tail.anchor.clone(),
                action: InlineAction::Replace(ResolvedInline::Fraction {
                    numerator: "1".into(),
                    denominator: "2".into(),
                }),
            },
            InlineEdit {
                target: sentinel.anchor.clone(),
                action: InlineAction::SetBold(true),
            },
        ]
    };
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
