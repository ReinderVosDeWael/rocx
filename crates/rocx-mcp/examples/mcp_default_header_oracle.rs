use std::{env, fs};

use rocx::{
    BreakType, Component, HeaderFooterType, ImageRun, Paragraph, ResolvedBlock,
    editor::{
        HeaderEditMode, HeaderImageEdit, ImportedDocument, ImportedHeaderBlock, ImportedInline,
        ImportedInlineNode, InlineAction, InlineEdit, RunTextEdit, SectionStoryKind,
        StoryReferenceEdit,
    },
};
use serde_json::{Value, json};

fn items(items: &[ImportedInline]) -> Vec<Value> {
    items
        .iter()
        .map(|item| match item {
            ImportedInline::Text(text) => json!({"kind":"text", "text":text}),
            ImportedInline::Tab => json!({"kind":"tab"}),
            ImportedInline::Break(BreakType::Page) => json!({"kind":"break", "break_type":"page"}),
            ImportedInline::Opaque(name) => json!({"kind":"opaque", "name":name}),
            _ => panic!("unexpected inline item: {item:?}"),
        })
        .collect()
}

fn typed(node: &ImportedInlineNode) -> Value {
    match node {
        ImportedInlineNode::Run(run) => json!({
            "kind":"run", "bold":run.bold, "items":items(&run.items),
        }),
        ImportedInlineNode::Hyperlink(link) => json!({
            "kind":"hyperlink", "bookmark":link.bookmark, "tooltip":link.tooltip,
            "history_xml":link.history_xml, "children":link.children.iter().map(typed).collect::<Vec<_>>(),
        }),
        ImportedInlineNode::Revision(revision) => json!({
            "kind":"revision", "revision_kind":revision.kind, "id":revision.id, "author":revision.author,
            "children":revision.children.iter().map(typed).collect::<Vec<_>>(),
        }),
        _ => panic!("unexpected typed inline: {node:?}"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let source = args.next().expect("source path");
    let document = ImportedDocument::open(&fs::read(source)?)?;
    if let Some(output) = args.next() {
        if let Some(edit) = args.next() {
            assert!(document.stories().is_err());
            let headers = document.default_headers()?;
            let header = &headers[0];
            let bytes = match edit.to_str() {
                Some("text") => {
                    let [ImportedInlineNode::Run(run)] = header.inline_structure(3).unwrap() else {
                        panic!("fixture direct run");
                    };
                    document.edit_story_text(
                        &header.part_name,
                        &[RunTextEdit {
                            target: run.anchor.clone(),
                            text: "Edited header 😀".into(),
                        }],
                    )?
                }
                Some("format") => {
                    let [
                        ImportedInlineNode::Run(run),
                        ImportedInlineNode::Hyperlink(link),
                        ..,
                    ] = header.inline_structure(0).unwrap()
                    else {
                        panic!("fixture rich header");
                    };
                    let [ImportedInlineNode::Run(child)] = link.children.as_slice() else {
                        panic!("fixture hyperlink run");
                    };
                    document.edit_run_format(
                        Some(&header.part_name),
                        &[
                            InlineEdit {
                                target: run.anchor.clone(),
                                action: InlineAction::SetBold(false),
                            },
                            InlineEdit {
                                target: child.anchor.clone(),
                                action: InlineAction::SetBold(true),
                            },
                        ],
                    )?
                }
                Some("image" | "blocks") => {
                    let target = headers[1].anchor.clone();
                    let mode = match args.next().as_deref().and_then(|arg| arg.to_str()) {
                        Some("shared") => HeaderEditMode::Shared,
                        Some("section-local") => HeaderEditMode::SectionLocal,
                        _ => panic!("expected shared or section-local"),
                    };
                    if edit == "image" {
                        let png = fs::read(args.next().expect("PNG path"))?;
                        let image = futures_executor::block_on(ImageRun::new(png).resolve())?
                            .expect("fixture image");
                        document
                            .preview_header_image(&HeaderImageEdit {
                                target,
                                mode,
                                image: image.image().expect("resolved image").clone(),
                            })?
                            .to_docx()?
                    } else {
                        let paragraph =
                            futures_executor::block_on(Paragraph::new("Added block 😀").resolve())?
                                .expect("fixture paragraph");
                        document.edit_story_blocks(
                            &target,
                            mode,
                            headers[1].blocks.len(),
                            0,
                            &[ResolvedBlock::Paragraph(paragraph)],
                        )?
                    }
                }
                _ => panic!("expected text, format, image or blocks"),
            };
            assert!(args.next().is_none());
            fs::write(output, bytes)?;
            return Ok(());
        }
        assert!(args.next().is_none());
        assert!(document.stories().is_err());
        let paragraph =
            futures_executor::block_on(Paragraph::new("From section slot 😀").resolve())?
                .expect("fixture paragraph");
        let bytes = document.edit_story_reference(
            0,
            SectionStoryKind::Header,
            HeaderFooterType::Default,
            StoryReferenceEdit::Create(vec![ResolvedBlock::Paragraph(paragraph)]),
        )?;
        fs::write(output, bytes)?;
        return Ok(());
    }
    let rows: Vec<_> = document
        .default_headers()?
        .iter()
        .map(|header| {
            let blocks: Vec<_> = header
                .blocks
                .iter()
                .enumerate()
                .map(|(index, block)| match block {
                    ImportedHeaderBlock::Paragraph(flat) => {
                        let native = header.inline_structure(index).expect("paragraph");
                        let nodes: Vec<_> = native.iter().map(typed).collect();
                        if let Some(ImportedInlineNode::Run(run)) = native.first() {
                            assert_eq!(nodes[0], typed(document.inline_at(&run.anchor).unwrap()));
                        }
                        json!({
                            "index":index, "type":"paragraph", "inline_items":items(flat), "inlines":nodes,
                        })
                    }
                    ImportedHeaderBlock::Opaque(name) => {
                        assert!(header.inline_structure(index).is_none());
                        json!({"index":index, "type":"opaque", "name":name})
                    }
                    _ => panic!("unexpected header block: {block:?}"),
                })
                .collect();
            json!({
                "section_index":header.section_index, "part_name":header.part_name,
                "inherited":header.inherited, "blocks":blocks,
            })
        })
        .collect();
    println!(
        "{}",
        json!({"default_headers":rows, "stories_ok":document.stories().is_ok()})
    );
    Ok(())
}
