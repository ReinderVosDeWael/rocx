//! Native byte oracle for `tests/mcp/test_field_tree.py`; no MCP declaration parsing.
use rocx::{
    BreakType, FieldInstruction, FieldProperties, HyperlinkProperties, HyperlinkTarget,
    ResolvedField, ResolvedHyperlink, ResolvedInline, RunFormat,
    editor::{
        BodyBlock, EditBatch, FieldAction, FieldEdit, ImportedDocument, ImportedInlineNode,
        InlineAction, InlineEdit,
    },
};
use std::{env, fs};

fn cached_link(label: &str) -> Result<ResolvedInline, rocx::Diagnostic> {
    Ok(ResolvedInline::Hyperlink(ResolvedHyperlink::new(
        HyperlinkTarget::external(format!("https://example.org/{label}"))?,
        vec![
            ResolvedInline::Text {
                text: label.into(),
                format: RunFormat {
                    italic: Some(true),
                    ..Default::default()
                },
                comment: None,
                revision: None,
            },
            ResolvedInline::Tab {
                format: RunFormat::default(),
                comment: None,
            },
            ResolvedInline::Break {
                kind: BreakType::Line,
                format: RunFormat::default(),
                comment: None,
            },
        ],
        HyperlinkProperties {
            tooltip: Some(format!("cached & {label}")),
            history: Some(false),
        },
    )?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let (source, output, depth, rich, wide) = match args.as_slice() {
        [source, output, depth] => (source, output, depth, false, false),
        [source, output, depth, mode] if mode == "rich" => (source, output, depth, true, false),
        [source, output, depth, mode] if mode == "wide" => (source, output, depth, false, true),
        _ => return Err("usage: mcp_field_tree_oracle SOURCE OUTPUT DEPTH [rich|wide]".into()),
    };
    let depth = depth
        .to_str()
        .ok_or("expected integer depth")?
        .parse::<usize>()?;
    let doc = ImportedDocument::open(&fs::read(source)?)?;
    let BodyBlock::Paragraph(paragraph) = &doc.body()[0] else {
        return Err("expected fixture paragraph".into());
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
        let children = if wide && index + 1 == depth {
            let mut children = vec![child];
            children.extend((0..1_100).map(|_| ResolvedInline::Tab {
                format: RunFormat::default(),
                comment: None,
            }));
            for sibling in 0..300 {
                children.push(ResolvedInline::Field(ResolvedField::new(
                    FieldInstruction::raw(format!(" SIBLING {sibling} "))?,
                    Vec::new(),
                    FieldProperties::default(),
                )?));
            }
            children
        } else if rich && index + 1 == depth {
            vec![child, cached_link("deepest")?]
        } else if rich && index + 2 == depth {
            vec![cached_link("before")?, child, cached_link("after")?]
        } else {
            vec![child]
        };
        child = ResolvedInline::Field(ResolvedField::new(
            FieldInstruction::raw(format!(" LEVEL {index} "))?,
            children,
            FieldProperties {
                dirty: (index == 0).then_some(false),
                locked: (index + 1 == depth).then_some(true),
                ..Default::default()
            },
        )?);
    }
    let preview = if wide {
        assert_eq!(doc.field_at(&paragraph.anchor, 1)?.instruction, " PAGE ");
        doc.preview_edits(EditBatch {
            fields: &[FieldEdit {
                target: paragraph.anchor.clone(),
                inline_index: 1,
                action: FieldAction::Cache(vec![child]),
            }],
            ..Default::default()
        })?
    } else {
        let [ImportedInlineNode::Run(run)] = paragraph.inline_structure.as_slice() else {
            return Err("expected one fixture run".into());
        };
        doc.preview_edits(EditBatch {
            inline_edits: &[InlineEdit {
                target: run.anchor.clone(),
                action: InlineAction::InsertAfter(child),
            }],
            ..Default::default()
        })?
    };
    fs::write(output, preview.to_docx()?)?;
    Ok(())
}
