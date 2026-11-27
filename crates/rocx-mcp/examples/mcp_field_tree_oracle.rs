//! Native byte oracle for `tests/mcp/test_field_tree.py`; no MCP declaration parsing.
use rocx::{
    FieldInstruction, FieldProperties, ResolvedField, ResolvedInline, RunFormat,
    editor::{
        BodyBlock, EditBatch, ImportedDocument, ImportedInlineNode, InlineAction, InlineEdit,
    },
};
use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let [source, output, depth] = args.as_slice() else {
        return Err("usage: mcp_field_tree_oracle SOURCE OUTPUT DEPTH".into());
    };
    let depth = depth
        .to_str()
        .ok_or("expected integer depth")?
        .parse::<usize>()?;
    let doc = ImportedDocument::open(&fs::read(source)?)?;
    let BodyBlock::Paragraph(paragraph) = &doc.body()[0] else {
        return Err("expected fixture paragraph".into());
    };
    let [ImportedInlineNode::Run(run)] = paragraph.inline_structure.as_slice() else {
        return Err("expected one fixture run".into());
    };
    let mut child = ResolvedInline::Text {
        text: "cached & <value>".into(),
        format: RunFormat {
            bold: Some(true),
            ..Default::default()
        },
        comment: None,
        revision: None,
    };
    for index in (0..depth).rev() {
        child = ResolvedInline::Field(ResolvedField::new(
            FieldInstruction::raw(format!(" LEVEL {index} "))?,
            vec![child],
            FieldProperties {
                dirty: (index == 0).then_some(false),
                locked: (index + 1 == depth).then_some(true),
                ..Default::default()
            },
        )?);
    }
    let preview = doc.preview_edits(EditBatch {
        inline_edits: &[InlineEdit {
            target: run.anchor.clone(),
            action: InlineAction::InsertAfter(child),
        }],
        ..Default::default()
    })?;
    fs::write(output, preview.to_docx()?)?;
    Ok(())
}
