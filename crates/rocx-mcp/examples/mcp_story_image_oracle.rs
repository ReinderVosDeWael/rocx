use rocx::{
    AltText, Component, HeaderFooterType, ImageCrop, ImageData, ImageRun, ImageTransformation,
    editor::{HeaderEditMode, HeaderImageEdit, ImportedDocument},
};
use std::{env, fs};

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let [output, source, png, slot, mode] = args.as_slice() else {
        panic!("usage: mcp_story_image_oracle OUTPUT SOURCE PNG KIND:VARIANT MODE");
    };
    let (kind, variant) = slot.split_once(':').unwrap();
    let variant = match variant {
        "default" => HeaderFooterType::Default,
        "first" => HeaderFooterType::First,
        "even" => HeaderFooterType::Even,
        _ => panic!("invalid story variant"),
    };
    let mode = match mode.as_str() {
        "shared" => HeaderEditMode::Shared,
        "section-local" => HeaderEditMode::SectionLocal,
        _ => panic!("invalid story image mode"),
    };
    let document = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    let target = document
        .stories()
        .unwrap()
        .into_iter()
        .find(|row| row.section_index == 1 && row.kind.as_str() == kind && row.variant == variant)
        .unwrap()
        .anchor
        .unwrap();
    let run = ImageRun::new(ImageData::NamedBytes {
        bytes: fs::read(png).unwrap(),
        source_name: "source-logo.png".into(),
    })
    .with_transformation(Some(ImageTransformation {
        width: Some(84.0),
        height: Some(42.0),
        crop: Some(ImageCrop {
            left: 10.0,
            top: 5.0,
            right: 15.0,
            bottom: 20.0,
        }),
        rotation: Some(30.5),
        flip_horizontal: Some(true),
        flip_vertical: Some(false),
        ..ImageTransformation::default()
    }))
    .with_alt_text(Some(AltText {
        name: Some("Story logo".into()),
        title: Some("Story logo title".into()),
        description: Some("Story logo description".into()),
    }));
    let resolved = futures_executor::block_on(run.resolve()).unwrap().unwrap();
    let preview = document
        .preview_header_image(&HeaderImageEdit {
            target,
            mode,
            image: resolved.image().unwrap().clone(),
        })
        .unwrap();
    fs::write(output, preview.to_docx().unwrap()).unwrap();
}
