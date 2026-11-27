//! Direct Rust oracle for scoped inline editing in physical entry parts.
use rocx::editor::{
    DrawingProperty, ImportedDocument, ImportedInlineNode, InlineAction, InlineEdit, RunToggle,
};
use rocx::{
    AltText, Component, Hyperlink, HyperlinkTarget, ImageRun, ImageTransformation, TextRun,
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
    let id = if kind == "comment" { "0" } else { "007" };
    let entry = doc
        .entry_stories()?
        .iter()
        .find(|e| e.kind.as_str() == kind && e.id == id)
        .unwrap();
    let text = 1; // The first run is the note/comment reference mark.
    let edit = |index, action| {
        let ImportedInlineNode::Run(run) = &entry.inline_structure(0).unwrap()[index] else {
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
        "description" => vec![edit(text + 1, InlineAction::SetDrawingDescription(None))],
        "remove" => vec![edit(text + 1, InlineAction::RemoveObject)],
        "link" => vec![edit(
            text,
            InlineAction::InsertAfter(resolve(Hyperlink::new(
                HyperlinkTarget::external("https://example.test/entry")?,
                vec![TextRun::new("entry link").into()],
            ))),
        )],
        _ => panic!("unknown oracle command"),
    };
    std::fs::write(output, doc.edit_entry_inlines(&entry.part_name, &edits)?).unwrap();
    Ok(())
}
