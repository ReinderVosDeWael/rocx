//! Direct Rust oracle for original-occurrence drawing and math edits.
use rocx::editor::{
    BodyBlock, DrawingProperty as P, EditBatch, ImportedDocument, ImportedInlineNode,
    InlineAction as A, InlineEdit,
};
use rocx::{
    AltText, ColumnChartProperties, ColumnChartRun, Component, ImageAnchor, ImageCrop, ImageRun,
    ImageTransformation, ImageWrap, ImageWrapSide, InlineFraction, ShapeGeometry, ShapeOutline,
    ShapeProperties, ShapeRun, ShapeTextAlignment, ShapeTextFormat,
};

fn resolved<C: Component>(component: C) -> C::Resolved {
    futures_executor::block_on(component.resolve())
        .unwrap()
        .unwrap()
}

fn chart(width: f64, height: f64, categories: &[&str], values: Vec<f64>) -> rocx::ResolvedInline {
    resolved(ColumnChartRun::new(ColumnChartProperties {
        width,
        height,
        categories: categories.iter().map(|s| (*s).into()).collect(),
        values,
    }))
}

fn main() -> rocx::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let doc = ImportedDocument::open(&std::fs::read(args.next().unwrap()).unwrap())?;
    let action = args.next().unwrap().into_string().unwrap();
    let destination = args.next().unwrap();
    if action == "extent" {
        let BodyBlock::Paragraph(paragraph) = &doc.body()[0] else {
            panic!("expected paragraph")
        };
        let ImportedInlineNode::Run(run) = &paragraph.inline_structure[0] else {
            panic!("expected drawing run");
        };
        let drawing = run.drawing.as_ref().unwrap();
        println!(
            "{}",
            serde_json::json!({
                "width": drawing.width_emu.map(|width| width.to_string()),
                "height": drawing.height_emu.map(|height| height.to_string()),
            })
        );
        return Ok(());
    }
    let edit = |body: usize, inline: usize, action| {
        let BodyBlock::Paragraph(paragraph) = &doc.body()[body] else {
            panic!("expected paragraph")
        };
        let target = match &paragraph.inline_structure[inline] {
            ImportedInlineNode::Run(run) => run.anchor.clone(),
            ImportedInlineNode::Fraction { anchor, .. } => anchor.clone(),
            _ => panic!("expected direct run or fraction"),
        };
        InlineEdit { target, action }
    };
    let jpeg = include_bytes!("../../../tests/fixtures/images/sample.jpg");
    let edits = match action.as_str() {
        "seed" => vec![
            edit(
                1,
                0,
                A::InsertAfter(resolved(ShapeRun::new(
                    ShapeProperties::default().with_text("original shape"),
                ))),
            ),
            edit(
                2,
                0,
                A::InsertAfter(chart(100., 75., &["A", "B"], vec![1., 2.])),
            ),
            edit(
                3,
                0,
                A::InsertAfter(resolved(InlineFraction::new("1", "2"))),
            ),
        ],
        "mixed" => vec![
            edit(
                0,
                0,
                A::SetImageAsset(
                    resolved(ImageRun::new(jpeg.to_vec()))
                        .image()
                        .unwrap()
                        .clone(),
                ),
            ),
            edit(0, 1, A::SetDrawingDescription(None)),
            edit(
                1,
                1,
                A::PatchDrawing(
                    P::ShapeGeometry,
                    resolved(ShapeRun::new(
                        ShapeProperties::default().with_geometry(ShapeGeometry::Ellipse),
                    )),
                ),
            ),
            edit(
                2,
                1,
                A::PatchDrawing(P::ChartSeries, chart(12., 10., &["New"], vec![0.])),
            ),
            edit(3, 1, A::RemoveObject),
        ],
        "size" | "placement" | "crop" | "transform" | "accessibility" => {
            let image = resolved(
                ImageRun::new(jpeg.to_vec())
                    .with_transformation(Some(ImageTransformation {
                        width: Some(30.),
                        height: Some(20.),
                        rotation: Some(90.),
                        flip_horizontal: Some(true),
                        flip_vertical: Some(false),
                        crop: Some(ImageCrop::default().with_left(15.)),
                        anchor: Some(
                            ImageAnchor::default()
                                .with_x(10.)
                                .with_wrap(ImageWrap::Square)
                                .with_wrap_side(ImageWrapSide::Left)
                                .with_allow_overlap(false),
                        ),
                    }))
                    .with_alt_text(Some(
                        AltText::default()
                            .with_title("A title")
                            .with_description("Accessible"),
                    )),
            );
            vec![edit(
                0,
                0,
                A::PatchDrawing(P::parse(&action).unwrap(), image),
            )]
        }
        "shapeFill" | "shapeOutline" | "shapeText" => {
            let shape = resolved(ShapeRun::new(
                ShapeProperties::default()
                    .with_fill([1, 2, 3])
                    .with_outline(ShapeOutline {
                        width: 3.,
                        color: [4, 5, 6],
                    })
                    .with_text("Revised\nshape")
                    .with_text_format(
                        ShapeTextFormat::default()
                            .with_bold(false)
                            .with_font_size(12.)
                            .with_alignment(ShapeTextAlignment::Center),
                    ),
            ));
            vec![edit(
                1,
                1,
                A::PatchDrawing(P::parse(&action).unwrap(), shape),
            )]
        }
        _ => panic!("unknown drawing oracle action"),
    };
    let preview = doc.preview_edits(EditBatch {
        inline_edits: &edits,
        ..Default::default()
    })?;
    std::fs::write(destination, preview.to_docx()?).unwrap();
    println!("{}", serde_json::json!(preview.inline_changes()));
    Ok(())
}
