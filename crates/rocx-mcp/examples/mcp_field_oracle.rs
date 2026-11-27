//! Direct Rust byte oracle for the independently authored MCP field fixture.
use rocx::{
    BookmarkName, Component, FieldForm, FieldInstruction, FieldProperties, Hyperlink,
    HyperlinkTarget, ResolvedField, ResolvedInline, RunFormat, TextRun,
    editor::{
        BodyBlock, EditBatch, FieldAction, FieldEdit, FieldPatch, ImportedDocument,
        ImportedInlineNode, ImportedParagraph, InlineAction, InlineEdit, RunTextEdit,
    },
};

fn text(value: &str) -> Result<ResolvedInline, rocx::Error> {
    let run = futures_executor::block_on(TextRun::new(value).resolve())?
        .expect("text run enabled in fixture");
    Ok(run)
}

fn instruction_variants(document: &ImportedDocument) -> rocx::Result<Vec<u8>> {
    let instructions = [
        FieldInstruction::Page,
        FieldInstruction::NumPages,
        FieldInstruction::Reference(BookmarkName::new("FieldTarget")?),
        FieldInstruction::TableOfContents,
        FieldInstruction::raw("  DOCVARIABLE Customer  ")?,
    ];
    let properties = [
        (FieldForm::Simple, None, None),
        (FieldForm::Complex, Some(false), Some(true)),
        (FieldForm::Simple, Some(true), Some(false)),
        (FieldForm::Complex, None, Some(false)),
        (FieldForm::Simple, Some(false), None),
    ];
    let mut edits = Vec::new();
    for (index, (instruction, (form, dirty, locked))) in
        instructions.into_iter().zip(properties).enumerate()
    {
        let BodyBlock::Paragraph(paragraph) = &document.body()[index + 1] else {
            panic!("expected field fixture paragraph");
        };
        let [ImportedInlineNode::Run(run)] = paragraph.inline_structure.as_slice() else {
            panic!("expected a single run");
        };
        edits.push(InlineEdit {
            target: run.anchor.clone(),
            action: InlineAction::InsertAfter(ResolvedInline::Field(ResolvedField::new(
                instruction,
                vec![text(&format!("cached {index}"))?],
                FieldProperties {
                    form,
                    dirty,
                    locked,
                },
            )?)),
        });
    }
    document
        .preview_edits(EditBatch {
            inline_edits: &edits,
            ..Default::default()
        })?
        .to_docx()
}

fn cache_without_separator(document: &ImportedDocument) -> rocx::Result<Vec<u8>> {
    let BodyBlock::Paragraph(paragraph) = &document.body()[0] else {
        panic!("expected the separatorless field fixture paragraph");
    };
    let edit = FieldEdit {
        target: paragraph.anchor.clone(),
        inline_index: 1,
        action: FieldAction::Cache(vec![text("new display")?]),
    };
    let rejected = match document.preview_edits(EditBatch {
        fields: &[edit.clone(), edit.clone()],
        ..Default::default()
    }) {
        Err(error) => error,
        Ok(_) => panic!("expected native rejection of duplicate cache edits"),
    };
    println!(
        "{}",
        serde_json::json!({
            "message": rejected.to_string(),
            "rule": rejected.rule().map(rocx::Rule::code),
            "path": rejected.path(),
        })
    );
    document
        .preview_edits(EditBatch {
            fields: &[edit],
            ..Default::default()
        })?
        .to_docx()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if let [source, output, mode] = args.as_slice()
        && mode == "cache-without-separator"
    {
        let document = ImportedDocument::open(&std::fs::read(source)?)?;
        std::fs::write(output, cache_without_separator(&document)?)?;
        return Ok(());
    }
    if let [source, output, mode] = args.as_slice()
        && mode == "instruction-variants"
    {
        let document = ImportedDocument::open(&std::fs::read(source)?)?;
        std::fs::write(output, instruction_variants(&document)?)?;
        return Ok(());
    }
    if let [source, output, mode] = args.as_slice()
        && mode == "inspect-fields"
    {
        let document = ImportedDocument::open(&std::fs::read(source)?)?;
        let mut fields = Vec::new();
        for (body, inline) in [(1, 1), (1, 2), (2, 1), (4, 3), (5, 0), (2, 0)] {
            let BodyBlock::Paragraph(paragraph) = &document.body()[body] else {
                panic!("expected fixture paragraph at {body}");
            };
            let value = match document.field_at(&paragraph.anchor, inline) {
                Ok(field) => serde_json::json!({
                    "complex": field.complex,
                    "instruction": field.instruction,
                    "dirty_xml": field.dirty_xml,
                    "locked_xml": field.locked_xml,
                    "inline_count": field.inline_count,
                }),
                Err(error) => serde_json::json!({
                    "message": error.to_string(),
                    "rule": error.rule().map(rocx::Rule::code),
                    "path": error.path(),
                }),
            };
            fields.push(value);
        }
        std::fs::write(output, serde_json::to_vec(&fields)?)?;
        return Ok(());
    }
    let [source, output] = args.as_slice() else {
        return Err(
            "usage: mcp_field_oracle SOURCE OUTPUT [instruction-variants|inspect-fields|cache-without-separator]".into(),
        );
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
