use rv_docx_rs::*;
fn cell(text: &str) -> Value<TableCell> {
    TableCell::new(vec![Paragraph::new(text).into()]).into()
}
pub fn document() -> Document {
    let paragraph_style = ParagraphStyleDefinition {
        name: "ReportBody".into(),
        base_style: Some("Normal".into()),
        next_paragraph_style: Some("Normal".into()),
        font: Some("Arial".into()),
        font_size: Some(11.),
        bold: Some(false),
        italic: Some(true),
        underline: Some(false),
        color: Some([12, 34, 56]),
        alignment: Some(Alignment::Left),
        spacing_before: Some(6.),
        spacing_after: Some(4.),
        line_spacing: Some(1.2),
        left_indent: Some(0.25),
        right_indent: Some(0.5),
        first_line_indent: Some(-0.125),
        keep_together: Some(true),
        keep_with_next: Some(false),
        page_break_before: Some(false),
        widow_control: Some(true),
    };
    let table_style = TableStyleDefinition {
        name: "ReportTable".into(),
        base_style: Some("Normal Table".into()),
        whole_table: Some(TableSectionFormat {
            font: Some("Calibri".into()),
            font_size: Some(10.),
            ..TableSectionFormat::default()
        }),
        first_row: Some(TableSectionFormat {
            bold: Some(true),
            color: Some([255, 255, 255]),
            background: Some([0, 32, 96]),
            ..TableSectionFormat::default()
        }),
        ..TableStyleDefinition::default()
    };
    let mut heading = Paragraph::new("Feature parity report");
    heading.format = ParagraphFormat {
        heading: Some(1),
        ..ParagraphFormat::default()
    }
    .into();
    let mut paragraph = Paragraph::from_children(vec![
        TextRun {
            format: RunFormat {
                bold: Some(true),
                italic: Some(false),
                underline: Some(true.into()),
                font: Some("Arial".into()),
                size: Some(14.),
                color: Some([1, 2, 3].into()),
                strike: Some(true),
                all_caps: Some(true),
                small_caps: Some(false),
                ..RunFormat::default()
            }
            .into(),
            text: "Formatted ".into(),
            comment_text: Some("Run note".into()),
            ..TextRun::default()
        }
        .into(),
        TextRun {
            format: RunFormat {
                superscript: Some(true),
                ..RunFormat::default()
            }
            .into(),
            text: "sup".into(),
            ..TextRun::default()
        }
        .into(),
        TextRun {
            format: RunFormat {
                subscript: Some(true),
                ..RunFormat::default()
            }
            .into(),
            text: "sub".into(),
            ..TextRun::default()
        }
        .into(),
        Tab::default().into(),
        TextRun::new("after tab").into(),
        Break::default().into(),
        TextRun::new("after line").into(),
        Break::new(BreakType::Page).into(),
        Break::new(BreakType::Column).into(),
    ]);
    paragraph.format = ParagraphFormat {
        style: Some("ReportBody".into()),
        alignment: Some(Alignment::Justify),
        spacing_before: Some(12.),
        spacing_after: Some(8.),
        line_spacing: Some(1.5),
        left_indent: Some(18.),
        right_indent: Some(9.),
        first_line_indent: Some(-6.),
        keep_together: Some(true),
        keep_with_next: Some(false),
        page_break_before: Some(false),
        widow_control: Some(false),
        ..ParagraphFormat::default()
    }
    .into();
    paragraph.comment_text = Some("Paragraph note".into());
    paragraph.comment_author = Some("Paragraph Author".into());
    let mut image =
        ImageRun::new(include_bytes!("../../tests/fixtures/images/sample.png").to_vec());
    image.transformation = Some(ImageTransformation {
        width: Some(72.),
        height: None,
    })
    .into();
    let nested = Table::new(vec![TableRow::new(vec![cell("nested")]).into()]);
    let mut first = TableCell::new(vec![Paragraph::new("merged").into()]);
    first.grid_span = Some(2);
    first.vmerge = Some(VerticalMerge::Restart);
    first.background_color = Some([255, 238, 221]);
    first.vertical_alignment = Some(VerticalAlignment::Center);
    first.borders = vec![CellBorder {
        side: CellBorderSide::Top,
        sz: 4,
        color: [16, 32, 48],
        val: BorderLine::Dashed,
    }];
    let mut continuation = TableCell::new(vec![Paragraph::new("continuation").into()]);
    continuation.grid_span = Some(2);
    continuation.vmerge = Some(VerticalMerge::Continue);
    let mut table = Table::new(vec![
        TableRow::new(vec![
            first.into(),
            TableCell::new(vec![nested.into(), Paragraph::new("after nested").into()]).into(),
        ])
        .into(),
        TableRow::new(vec![continuation.into(), cell("tail")]).into(),
        TableRow::new(vec![cell("short"), cell("row")]).into(),
        TableRow::new(vec![cell("hidden")]).when(false).into(),
    ]);
    table.column_widths = Some(vec![1000, 2000, 3000]);
    table.style = Some("ReportTable".into());
    table.borders = vec![TableBorder {
        side: BorderSide::InsideH,
        sz: 8,
        color: [128, 0, 0],
        ..TableBorder::default()
    }];
    let mut section = Section::new(vec![
        heading.into(),
        paragraph.into(),
        Paragraph::from_children(vec![image.into()]).into(),
        table.into(),
    ]);
    section.properties = Some(SectionProperties {
        page_orientation: Some(Orientation::Portrait),
        page_margins: Some(PageMargins {
            top: Some(914400),
            bottom: Some(914400),
            left: Some(1143000),
            right: Some(1143000),
        }),
        ..SectionProperties::default()
    });
    section.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![Paragraph::new("Default header").into()]).into(),
    );
    section.headers.insert(
        HeaderFooterType::First,
        Header::new(vec![Paragraph::new("First header").into()]).into(),
    );
    section.headers.insert(
        HeaderFooterType::Even,
        Header::new(vec![Paragraph::new("Even header").into()]).into(),
    );
    section.footers.insert(
        HeaderFooterType::Default,
        Footer::new(vec![Paragraph::new("Footer").into()]).into(),
    );
    let second = Section {
        children: vec![Paragraph::new("Landscape section").into()].into(),
        properties: Some(SectionProperties {
            page_orientation: Some(Orientation::Landscape),
            ..SectionProperties::default()
        }),
        ..Section::default()
    };
    let mut document = Document::new(vec![section.into(), second.into()]);
    document.creator = Some("Rust Author".into());
    document.title = Some("Parity report".into());
    document.subject = Some("Testing".into());
    document.description = Some("Description".into());
    document.comments = Some("Core comments".into());
    document.keywords = Some("rust, docx".into());
    document.category = Some("Reports".into());
    document.version = Some("1.0".into());
    document.comment_author = Some("Default Author".into());
    document.styles = vec![paragraph_style.into(), table_style.into()];
    document
}
