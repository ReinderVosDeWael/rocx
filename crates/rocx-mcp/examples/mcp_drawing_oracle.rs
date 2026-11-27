//! Direct Rust oracle for original-occurrence drawing and math edits.
use rocx::editor::{
    BodyBlock, DrawingProperty as P, EditBatch, ImportedDocument, ImportedInlineNode,
    InlineAction as A, InlineEdit,
};
use rocx::{
    AltText, ColumnChartProperties, ColumnChartRun, Component, ImageAnchor,
    ImageAnchorHorizontalAlignment as H, ImageAnchorReference as R,
    ImageAnchorVerticalAlignment as V, ImageCrop, ImageRun, ImageTransformation, ImageWrap as W,
    ImageWrapSide as S, InlineFraction, ResolvedInline, ShapeGeometry, ShapeOutline,
    ShapeProperties, ShapeRun, ShapeTextAlignment, ShapeTextFormat, ShapeTextVerticalAlignment,
};

const JPEG: &[u8] = include_bytes!("../../../tests/fixtures/images/sample.jpg");

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

fn anchored_pictures() -> Vec<ResolvedInline> {
    let polygon = vec![(-10, 5), (21600, 0), (0, 21600)];
    let anchors = [
        ImageAnchor::default()
            .with_horizontal_reference(R::LeftMargin)
            .with_vertical_reference(R::TopMargin)
            .with_horizontal_alignment(H::Inside)
            .with_vertical_alignment(V::Top)
            .with_wrap(W::Tight)
            .with_wrap_side(S::Largest)
            .with_wrap_polygon(polygon.clone())
            .with_distance_left(2.5)
            .with_distance_right(4.)
            .with_relative_height(u32::MAX)
            .with_behind_text(true)
            .with_locked(true)
            .with_layout_in_cell(false)
            .with_allow_overlap(false),
        ImageAnchor::default()
            .with_horizontal_reference(R::RightMargin)
            .with_vertical_reference(R::BottomMargin)
            .with_horizontal_alignment(H::Outside)
            .with_vertical_alignment(V::Bottom)
            .with_wrap(W::Through)
            .with_wrap_side(S::Right)
            .with_wrap_polygon(polygon),
        ImageAnchor::default()
            .with_horizontal_reference(R::InsideMargin)
            .with_vertical_reference(R::OutsideMargin)
            .with_horizontal_alignment(H::Left)
            .with_vertical_alignment(V::Outside)
            .with_wrap(W::TopAndBottom)
            .with_distance_top(0.)
            .with_distance_bottom(3.),
        ImageAnchor::default()
            .with_horizontal_reference(R::OutsideMargin)
            .with_vertical_reference(R::InsideMargin)
            .with_horizontal_alignment(H::Right)
            .with_vertical_alignment(V::Inside)
            .with_wrap(W::Square)
            .with_wrap_side(S::BothSides),
        ImageAnchor::default()
            .with_horizontal_reference(R::Column)
            .with_vertical_reference(R::Paragraph)
            .with_horizontal_alignment(H::Center)
            .with_vertical_alignment(V::Center)
            .with_wrap(W::Square)
            .with_wrap_side(S::Left),
        ImageAnchor::default()
            .with_horizontal_reference(R::Character)
            .with_vertical_reference(R::Line)
            .with_x(-12.5)
            .with_y(20.)
            .with_wrap(W::None),
        ImageAnchor::default()
            .with_horizontal_reference(R::Margin)
            .with_vertical_reference(R::Margin),
        ImageAnchor::default()
            .with_horizontal_reference(R::Page)
            .with_vertical_reference(R::Page),
    ];
    anchors
        .into_iter()
        .enumerate()
        .map(|(index, anchor)| {
            let mut transform = ImageTransformation::default().with_anchor(anchor);
            let mut image = ImageRun::new(JPEG.to_vec());
            if index == 0 {
                transform = transform
                    .with_width(61.5)
                    .with_height(30.25)
                    .with_rotation(-12.5)
                    .with_flip_horizontal(false)
                    .with_flip_vertical(true)
                    .with_crop(
                        ImageCrop::default()
                            .with_left(10.25)
                            .with_top(5.)
                            .with_right(20.)
                            .with_bottom(15.),
                    );
                image = image.with_alt_text(Some(
                    AltText::default()
                        .with_name("Seal & crest")
                        .with_title("Picture title")
                        .with_description("Decorative <seal>"),
                ));
            }
            resolved(image.with_transformation(Some(transform)))
        })
        .collect()
}

fn styled_shapes() -> Vec<ResolvedInline> {
    use ShapeGeometry::{Ellipse, Rectangle, TextBox};
    use ShapeTextAlignment::{Center, Left, Right};
    use ShapeTextVerticalAlignment::{Bottom, Center as Middle, Top};
    let mut shapes = Vec::new();
    for (geometry, alignment, vertical) in [
        (Rectangle, Right, Bottom),
        (Ellipse, Center, Middle),
        (TextBox, Left, Top),
    ] {
        let mut shape = ShapeProperties::default()
            .with_geometry(geometry)
            .with_width(42.5)
            .with_height(23.25)
            .with_text("Heading & <leaf>\r\n\r\n尾")
            .with_text_format(
                ShapeTextFormat::default()
                    .with_font_size(12.25)
                    .with_bold(false)
                    .with_italic(true)
                    .with_color([7, 16, 255])
                    .with_font("Example & Serif")
                    .with_alignment(alignment)
                    .with_vertical_alignment(vertical),
            );
        if geometry == Rectangle {
            shape = shape.with_fill([17, 34, 51]).with_outline(
                ShapeOutline {
                    width: 0.,
                    color: [0; 3],
                }
                .with_width(1.25)
                .with_color([68, 85, 102]),
            );
        }
        if geometry == TextBox {
            shape = shape.with_anchor(ImageAnchor::default().with_y(5.));
        }
        shapes.push(resolved(ShapeRun::new(shape)));
    }
    shapes
}

fn literal_chart() -> ResolvedInline {
    resolved(ColumnChartRun::new(
        ColumnChartProperties {
            width: 72.,
            height: 72.,
            categories: Vec::new(),
            values: Vec::new(),
        }
        .with_width(102.5)
        .with_height(64.25)
        .with_categories(vec![
            "<&😀>".into(),
            String::new(),
            "same".into(),
            "same".into(),
            "negative".into(),
            "end".into(),
        ])
        .with_values(vec![
            -3.25,
            0.,
            0.12345678901234561,
            0.12345678901234562,
            -0.12345678901234561,
            9.5,
        ]),
    ))
}

fn main() -> rocx::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let doc = ImportedDocument::open(&std::fs::read(args.next().unwrap()).unwrap())?;
    let action = args.next().unwrap().into_string().unwrap();
    let destination = args.next().unwrap();
    if action == "drawing-readback" {
        let rows: Vec<_> = doc
            .body()
            .iter()
            .map(|block| {
                let BodyBlock::Paragraph(paragraph) = block else {
                    panic!("expected paragraph");
                };
                let ImportedInlineNode::Run(run) = &paragraph.inline_structure[1] else {
                    panic!("expected drawing run");
                };
                let d = run.drawing.as_ref().unwrap();
                serde_json::json!({
                    "kind": d.kind, "placement": d.placement, "name": d.name,
                    "title": d.title, "description": d.description, "geometry": d.geometry,
                    "shape_text": d.shape_text, "relationship_id": d.relationship_id,
                    "crop": d.crop, "rotation": d.rotation, "flip_horizontal": d.flip_horizontal,
                    "flip_vertical": d.flip_vertical, "width_emu": d.width_emu, "height_emu": d.height_emu,
                })
            })
            .collect();
        println!("{}", serde_json::json!(rows));
        return Ok(());
    }
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
    let edits = match action.as_str() {
        "picture-declarations" | "shape-declarations" => {
            let inlines = if action == "picture-declarations" {
                anchored_pictures()
            } else {
                styled_shapes()
            };
            inlines
                .into_iter()
                .enumerate()
                .map(|(body, inline)| edit(body, 0, A::InsertAfter(inline)))
                .collect()
        }
        "chart-declaration" => vec![edit(0, 0, A::InsertAfter(literal_chart()))],
        "chart-readback" => {
            let BodyBlock::Paragraph(paragraph) = &doc.body()[0] else {
                panic!("expected chart paragraph")
            };
            let ImportedInlineNode::Run(run) = &paragraph.inline_structure[1] else {
                panic!("expected chart run")
            };
            let (categories, values) = doc.chart_series(&run.anchor)?;
            println!(
                "{}",
                serde_json::json!({"categories": categories, "values": values})
            );
            return Ok(());
        }
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
                    resolved(ImageRun::new(JPEG.to_vec()))
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
                ImageRun::new(JPEG.to_vec())
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
                                .with_wrap(W::Square)
                                .with_wrap_side(S::Left)
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
