//! Direct Rust byte oracle for the independently authored MCP field fixture.
use rocx::{
    Component, FieldForm, FieldInstruction, FieldProperties, Hyperlink, HyperlinkTarget,
    ResolvedField, ResolvedInline, RunFormat, TextRun,
    editor::{
        BodyBlock, EditBatch, FieldAction, FieldEdit, FieldPatch, ImportedDocument,
        ImportedInlineNode, ImportedParagraph, RunTextEdit,
    },
};

fn text(value: &str) -> Result<ResolvedInline, rocx::Error> {
    let run = futures_executor::block_on(TextRun::new(value).resolve())?
        .expect("text run enabled in fixture");
    Ok(run)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: mcp_field_oracle SOURCE OUTPUT".into());
    };
    let document = ImportedDocument::open(&std::fs::read(source)?)?;
    let paragraph = |index| -> &ImportedParagraph {
        let BodyBlock::Paragraph(paragraph) = &document.body()[index] else {
            panic!("expected fixture paragraph at {index}");
        };
        paragraph
    };
    let field = |body_index, inline_index, action| FieldEdit {
        target: paragraph(body_index).anchor.clone(),
        inline_index,
        action,
    };
    let link = futures_executor::block_on(
        Hyperlink::new(
            HyperlinkTarget::external("https://example.test/cache")?,
            vec![
                TextRun::new("linked display")
                    .with_format(RunFormat {
                        bold: Some(true),
                        ..Default::default()
                    })
                    .into(),
            ],
        )
        .resolve(),
    )?
    .expect("hyperlink enabled in fixture");
    let replacement = ResolvedInline::Field(ResolvedField::new(
        FieldInstruction::Page,
        vec![text("replacement display")?],
        FieldProperties {
            form: FieldForm::Complex,
            dirty: Some(true),
            locked: None,
        },
    )?);
    let [ImportedInlineNode::Run(run)] = paragraph(6).inline_structure.as_slice() else {
        return Err("expected one disjoint fixture run".into());
    };
    let preview = document.preview_edits(EditBatch {
        fields: &[
            field(1, 1, FieldAction::Patch(FieldPatch::Dirty(Some(false)))),
            field(1, 1, FieldAction::Patch(FieldPatch::Locked(None))),
            field(1, 1, FieldAction::Cache(vec![link])),
            field(
                2,
                1,
                FieldAction::Patch(FieldPatch::Instruction(" NUMPAGES ".into())),
            ),
            field(2, 1, FieldAction::Patch(FieldPatch::Dirty(None))),
            field(2, 1, FieldAction::Patch(FieldPatch::Locked(Some(true)))),
            field(2, 1, FieldAction::Cache(vec![text("complex display")?])),
            field(1, 2, FieldAction::Replace(Box::new(replacement))),
            field(4, 3, FieldAction::Remove),
        ],
        run_text: &[RunTextEdit {
            target: run.anchor.clone(),
            text: "ordinary edited".into(),
        }],
        ..Default::default()
    })?;
    std::fs::write(output, preview.to_docx()?)?;
    Ok(())
}
