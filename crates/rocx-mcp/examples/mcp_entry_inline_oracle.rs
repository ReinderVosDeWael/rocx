//! Direct Rust oracle for scoped inline editing in physical entry parts.
use rocx::editor::{
    BodyBlock, DrawingProperty, EditBatch, ImportedDocument, ImportedInlineNode, InlineAction,
    InlineEdit, RunToggle,
};
use rocx::{
    AltText, ColumnChartProperties, ColumnChartRun, Component, Hyperlink, HyperlinkTarget,
    ImageAnchor, ImageCrop, ImageRun, ImageTransformation, ImageWrap, ImageWrapSide,
    ResolvedInline, RunFormat, ShapeProperties, ShapeRun, TextRun,
};

fn resolve<C: Component>(component: C) -> C::Resolved {
    futures_executor::block_on(component.resolve())
        .unwrap()
        .unwrap()
}

fn main() -> rocx::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let doc = ImportedDocument::open(&std::fs::read(args.next().unwrap()).unwrap())?;
    let kind = args.next().unwrap().into_string().unwrap();
    let command = args.next().unwrap().into_string().unwrap();
    let output = args.next().unwrap();
    if command == "seed-drawings" {
        let Some(BodyBlock::Paragraph(paragraph)) = doc.body().last() else {
            panic!("expected a donor paragraph")
        };
        let insert = |index, inline| {
            let ImportedInlineNode::Run(run) = &paragraph.inline_structure[index] else {
                panic!("expected donor run")
            };
            InlineEdit {
                target: run.anchor.clone(),
                action: InlineAction::InsertAfter(inline),
            }
        };
        let preview = doc.preview_edits(EditBatch {
            inline_edits: &[
                insert(
                    0,
                    resolve(ShapeRun::new(ShapeProperties::default().with_text("shape"))),
                ),
                insert(
                    1,
                    resolve(ColumnChartRun::new(ColumnChartProperties {
                        width: 100.,
                        height: 75.,
                        categories: vec!["A".into()],
                        values: vec![1.],
                    })),
                ),
            ],
            ..Default::default()
        })?;
        std::fs::write(output, preview.to_docx()?).unwrap();
        return Ok(());
    }
    let id = if kind == "comment" { "0" } else { "007" };
    let entry = doc
        .entry_stories()?
        .iter()
        .find(|e| e.kind.as_str() == kind && e.id == id)
        .unwrap();
    let text = 1; // The first run is the note/comment reference mark.
    let nodes = entry.inline_structure(0).unwrap();
    let edit = |index, action| {
        let ImportedInlineNode::Run(run) = &nodes[index] else {
            panic!("expected fixture run")
        };
        InlineEdit {
            target: run.anchor.clone(),
            action,
        }
    };
    let png = include_bytes!("../../../tests/fixtures/images/sample.png").to_vec();
    let jpeg = include_bytes!("../../../tests/fixtures/images/sample.jpg").to_vec();
    let edits = match command.as_str() {
        "insert" => vec![edit(
            text,
            InlineAction::InsertAfter(resolve(ImageRun::new(png))),
        )],
        "patch" => vec![
            edit(text, InlineAction::SetToggle(RunToggle::Bold, Some(true))),
            edit(
                text + 1,
                InlineAction::PatchDrawing(
                    DrawingProperty::Transform,
                    resolve(
                        ImageRun::new(jpeg).with_transformation(Some(
                            ImageTransformation::default()
                                .with_rotation(90.)
                                .with_flip_vertical(false),
                        )),
                    ),
                ),
            ),
        ],
        "size" | "crop" | "accessibility" | "placement" => vec![edit(
            text + 1,
            InlineAction::PatchDrawing(
                DrawingProperty::parse(&command).unwrap(),
                resolve(
                    ImageRun::new(jpeg)
                        .with_transformation(Some(
                            ImageTransformation::default()
                                .with_width(30.)
                                .with_height(20.)
                                .with_crop(ImageCrop::default().with_left(15.))
                                .with_anchor(
                                    ImageAnchor::default()
                                        .with_x(10.)
                                        .with_wrap(ImageWrap::Square)
                                        .with_wrap_side(ImageWrapSide::Left)
                                        .with_allow_overlap(false),
                                ),
                        ))
                        .with_alt_text(Some(
                            AltText::default()
                                .with_title("A title")
                                .with_description("Accessible"),
                        )),
                ),
            ),
        )],
        "redirect" => vec![edit(
            text + 1,
            InlineAction::SetImageAsset(resolve(ImageRun::new(jpeg)).image().unwrap().clone()),
        )],
        "replace" => vec![edit(
            text + 1,
            InlineAction::Replace(resolve(
                ImageRun::new(png)
                    .with_alt_text(Some(AltText::default().with_description("Replacement"))),
            )),
        )],
        "replace-tab" => vec![edit(
            text,
            InlineAction::Replace(ResolvedInline::Tab {
                format: RunFormat::default().with_bold(true),
                comment: None,
            }),
        )],
        "description" => vec![edit(text + 1, InlineAction::SetDrawingDescription(None))],
        "remove" => vec![edit(text + 1, InlineAction::RemoveObject)],
        "link" => vec![edit(
            text,
            InlineAction::InsertAfter(resolve(Hyperlink::new(
                HyperlinkTarget::external("https://example.test/entry")?,
                vec![
                    TextRun::new("first child").into(),
                    TextRun::new("entry link").into(),
                ],
            ))),
        )],
        "format-link" => {
            let ImportedInlineNode::Hyperlink(link) = &nodes[text + 1] else {
                panic!("expected fixture hyperlink")
            };
            let ImportedInlineNode::Run(child) = &link.children[1] else {
                panic!("expected fixture hyperlink child run")
            };
            vec![
                edit(text, InlineAction::SetToggle(RunToggle::Italic, Some(true))),
                InlineEdit {
                    target: child.anchor.clone(),
                    action: InlineAction::SetBold(true),
                },
            ]
        }
        "format-drawings" => vec![
            edit(text + 1, InlineAction::SetBold(true)),
            edit(text + 2, InlineAction::SetBold(true)),
        ],
        _ => panic!("unknown oracle command"),
    };
    std::fs::write(output, doc.edit_entry_inlines(&entry.part_name, &edits)?).unwrap();
    Ok(())
}
