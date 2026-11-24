use rocx::*;
use std::path::Path;

pub fn write(out: &Path, png: &Path) -> Result<()> {
    let ellipse = ShapeProperties::default()
        .with_width(60.)
        .with_height(31.)
        .with_geometry(ShapeGeometry::Ellipse)
        .with_fill([1, 2, 3])
        .with_text("Shape & text")
        .with_outline(ShapeOutline {
            width: 2.,
            color: [10, 20, 30],
        })
        .with_text_format(
            ShapeTextFormat::default()
                .with_font_size(9.25)
                .with_color([30, 20, 10])
                .with_bold(false)
                .with_italic(true)
                .with_alignment(ShapeTextAlignment::Right)
                .with_vertical_alignment(ShapeTextVerticalAlignment::Bottom)
                .with_font("Aptos"),
        )
        .with_anchor(
            ImageAnchor::default()
                .with_horizontal_reference(ImageAnchorReference::Column)
                .with_vertical_reference(ImageAnchorReference::Paragraph)
                .with_x(-2.)
                .with_y(3.),
        );
    let rectangle = ShapeProperties::default()
        .with_text("Rectangle")
        .with_text_format(
            ShapeTextFormat::default()
                .with_bold(true)
                .with_italic(false)
                .with_alignment(ShapeTextAlignment::Center)
                .with_vertical_alignment(ShapeTextVerticalAlignment::Center),
        );
    let text_box = ShapeProperties::default()
        .with_geometry(ShapeGeometry::TextBox)
        .with_text("Text box")
        .with_text_format(
            ShapeTextFormat::default()
                .with_alignment(ShapeTextAlignment::Left)
                .with_vertical_alignment(ShapeTextVerticalAlignment::Top),
        );
    let chart = ColumnChartProperties {
        width: 120.,
        height: 48.,
        categories: vec!["One".into(), "Two & 三".into()],
        values: vec![-1.5, 0.],
    };
    let paragraph = Paragraph::from_children(vec![
        ShapeRun::new(ellipse).into(),
        ShapeRun::new(rectangle).into(),
        ShapeRun::new(text_box).into(),
        ColumnChartRun::new(chart).into(),
        InlineFraction::new("α", "β").into(),
        Tab::default().into(),
        Break::default().into(),
        Break::new(BreakType::Line).into(),
        Break::new(BreakType::TextWrapping).into(),
        Break::new(BreakType::Page).into(),
        Break::new(BreakType::Column).into(),
    ]);
    let document = Document::new(vec![Section::new(vec![paragraph.into()]).into()])
        .with_timestamp(Some("2024-02-03T04:05:06Z".into()));
    let snapshot = futures::executor::block_on(document.resolve())?;
    std::fs::write(out.join("drawings.docx"), snapshot.to_docx()?)?;
    native_pictures(out, png)
}

fn native_pictures(out: &Path, png: &Path) -> Result<()> {
    use ImageAnchorHorizontalAlignment as H;
    use ImageAnchorReference as R;
    use ImageAnchorVerticalAlignment as V;
    use ImageWrap as W;
    use ImageWrapSide as S;
    let data = std::fs::read(png)?;
    let picture = || ImageRun::new(data.clone());
    let contour = vec![(-10, 0), (21600, 0), (0, 21600)];
    let placement = |h, v, wrap| {
        ImageAnchor::default()
            .with_horizontal_reference(h)
            .with_vertical_reference(v)
            .with_wrap(wrap)
    };
    let square = placement(R::Margin, R::Line, W::Square)
        .with_horizontal_alignment(H::Left)
        .with_vertical_alignment(V::Top)
        .with_wrap_side(S::Largest)
        .with_distance_top(1.)
        .with_distance_bottom(2.)
        .with_distance_left(3.)
        .with_distance_right(4.)
        .with_relative_height(17)
        .with_behind_text(true)
        .with_locked(false)
        .with_layout_in_cell(false)
        .with_allow_overlap(false);
    let decorated = ImageTransformation::default()
        .with_width(30.)
        .with_height(20.)
        .with_crop(ImageCrop {
            left: 1.,
            top: 2.,
            right: 3.,
            bottom: 4.,
        })
        .with_rotation(-10.)
        .with_flip_horizontal(false)
        .with_flip_vertical(true)
        .with_anchor(square);
    let alt = AltText::default()
        .with_name("Logo")
        .with_title("Brand")
        .with_description("Logo & art");
    let mut children = vec![
        picture()
            .with_transformation(Value::ready(Some(decorated)))
            .with_alt_text(Value::ready(Some(alt)))
            .into(),
        picture()
            .with_transformation(Value::ready(Some(
                ImageTransformation::default()
                    .with_height(10.)
                    .with_flip_horizontal(true)
                    .with_flip_vertical(false),
            )))
            .into(),
        picture().into(),
        picture()
            .with_transformation(Value::ready(Some(
                ImageTransformation::default().with_width(15.),
            )))
            .into(),
    ];
    for anchor in [
        placement(R::Column, R::Paragraph, W::None)
            .with_x(-2.)
            .with_y(3.),
        placement(R::Character, R::TopMargin, W::Tight)
            .with_horizontal_alignment(H::Center)
            .with_vertical_alignment(V::Center)
            .with_wrap_side(S::Left)
            .with_wrap_polygon(contour.clone()),
        placement(R::LeftMargin, R::BottomMargin, W::Through)
            .with_horizontal_alignment(H::Right)
            .with_vertical_alignment(V::Bottom)
            .with_wrap_side(S::Right)
            .with_wrap_polygon(contour),
        placement(R::RightMargin, R::InsideMargin, W::TopAndBottom)
            .with_horizontal_alignment(H::Inside)
            .with_vertical_alignment(V::Inside)
            .with_distance_bottom(5.)
            .with_locked(true)
            .with_layout_in_cell(true)
            .with_allow_overlap(true),
        placement(R::OutsideMargin, R::OutsideMargin, W::Square)
            .with_horizontal_alignment(H::Outside)
            .with_vertical_alignment(V::Outside)
            .with_wrap_side(S::BothSides)
            .with_behind_text(false),
        placement(R::InsideMargin, R::Margin, W::None),
        placement(R::Page, R::Page, W::None),
    ] {
        children.push(
            picture()
                .with_transformation(Value::ready(Some(
                    ImageTransformation::default().with_anchor(anchor),
                )))
                .into(),
        );
    }
    let document = Document::new(vec![
        Section::new(vec![Paragraph::from_children(children).into()]).into(),
    ])
    .with_timestamp(Some("2024-02-03T04:05:06Z".into()));
    let snapshot = futures::executor::block_on(document.resolve())?;
    std::fs::write(out.join("pictures.docx"), snapshot.to_docx()?)?;
    Ok(())
}
