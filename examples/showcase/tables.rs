use super::*;
pub const REGIONS: [&str; 13] = [
    "whole_table",
    "first_row",
    "last_row",
    "first_column",
    "last_column",
    "banding_1_row",
    "banding_2_row",
    "banding_1_column",
    "banding_2_column",
    "top_left_cell",
    "top_right_cell",
    "bottom_left_cell",
    "bottom_right_cell",
];
pub fn styles() -> Vec<StyleDefinition> {
    REGIONS
        .iter()
        .map(|name| {
            let fmt = Some(TableSectionFormat {
                bold: Some(true),
                italic: Some(false),
                underline: Some(false),
                font: Some("Calibri".into()),
                font_size: Some(10.),
                color: Some([32, 48, 96]),
                background: Some([224, 238, 255]),
                alignment: Some(Alignment::Center),
                spacing_before: Some(2.),
                spacing_after: Some(2.),
                line_height: Some(LineHeight::AtLeast(12.)),
                tab_stops: vec![TabStop {
                    position: 36.,
                    ..Default::default()
                }],
                bidi: Some(false),
                language: Some(RunLanguage {
                    latin: Some("en-US".into()),
                    east_asia: Some("ja-JP".into()),
                    bidi: Some("ar-SA".into()),
                }),
                bold_complex_script: Some(false),
                italic_complex_script: Some(false),
                size_complex_script: Some(10.),
                rtl: Some(false),
                complex_script: Some(false),
                font_east_asia: Some("Yu Mincho".into()),
                font_complex_script: Some("Arial".into()),
            });
            let mut style = TableStyleDefinition {
                name: format!("Region {name}"),
                base_style: Some("Table Grid".into()),
                ..Default::default()
            };
            match *name {
                "whole_table" => style.whole_table = fmt,
                "first_row" => style.first_row = fmt,
                "last_row" => style.last_row = fmt,
                "first_column" => style.first_column = fmt,
                "last_column" => style.last_column = fmt,
                "banding_1_row" => style.banding_1_row = fmt,
                "banding_2_row" => style.banding_2_row = fmt,
                "banding_1_column" => style.banding_1_column = fmt,
                "banding_2_column" => style.banding_2_column = fmt,
                "top_left_cell" => style.top_left_cell = fmt,
                "top_right_cell" => style.top_right_cell = fmt,
                "bottom_left_cell" => style.bottom_left_cell = fmt,
                "bottom_right_cell" => style.bottom_right_cell = fmt,
                _ => unreachable!(),
            }
            style.into()
        })
        .collect()
}
pub fn blocks(out: &mut BlockChildren) {
    out.push(heading(
        "04 Tables: layout, rows, cells, merges and nested content",
    ));
    out.push(p("Fixed table: 3 columns of 1440 twips; all six table-border sides; top/bottom/start/end cell borders; padding top=3, start=0, bottom=3, end=6 pt. Header row repeats; cannot split; minimum height=28 pt."));
    let mut top = TableCell::new(vec![p(
        "Horizontal span=2; vertical merge restart; shaded; vertically centered",
    )]);
    top.properties = TableCellProperties {
        grid_span: Some(2),
        vmerge: Some(VerticalMerge::Restart),
        background_color: Some([255, 238, 204]),
        vertical_alignment: Some(VerticalAlignment::Center),
        margins: Some(CellMargins {
            top: Some(3.),
            start: Some(0.),
            bottom: Some(3.),
            end: Some(6.),
        }),
        borders: [
            CellBorderSide::Top,
            CellBorderSide::Bottom,
            CellBorderSide::Start,
            CellBorderSide::End,
        ]
        .iter()
        .map(|side| CellBorder {
            side: *side,
            sz: 8,
            color: [128, 64, 32],
            val: BorderLine::Dashed,
        })
        .collect(),
        ..Default::default()
    }
    .into();
    let mut continuation = TableCell::new(vec![p("")]);
    continuation.properties = TableCellProperties {
        grid_span: Some(2),
        vmerge: Some(VerticalMerge::Continue),
        ..Default::default()
    }
    .into();
    let mut row = TableRow::new(vec![top.into(), cell("Top aligned cell")]);
    row.properties = TableRowProperties {
        content_control: None,
        height: Some(RowHeight::AtLeast(28.)),
        repeat_header: Some(true),
        cant_split: Some(true),
    }
    .into();
    let mut bottom = TableCell::new(vec![p("Bottom aligned cell")]);
    bottom.properties = TableCellProperties {
        vertical_alignment: Some(VerticalAlignment::Bottom),
        ..Default::default()
    }
    .into();
    let mut second = TableRow::new(vec![continuation.into(), bottom.into()]);
    second.properties = TableRowProperties {
        content_control: None,
        height: Some(RowHeight::Exact(48.)),
        repeat_header: Some(false),
        cant_split: Some(false),
    }
    .into();
    let mut table = Table::new(vec![
        row.into(),
        second.into(),
        TableRow::new(vec![cell("Short row: padded grid"), cell("Second cell")]).into(),
    ]);
    table.properties = TableProperties {
        column_widths: Some(vec![1440, 1440, 1440]),
        layout: Some(TableLayout::Fixed),
        borders: [
            BorderSide::Top,
            BorderSide::Left,
            BorderSide::Bottom,
            BorderSide::Right,
            BorderSide::InsideH,
            BorderSide::InsideV,
        ]
        .iter()
        .map(|side| TableBorder {
            side: *side,
            sz: 8,
            color: [32, 64, 96],
            val: BorderLine::Single,
        })
        .collect(),
        ..Default::default()
    }
    .into();
    out.push(table.into());
    out.push(p("Autofit table with nested table and trailing paragraph (nested tables need trailing paragraphs)."));
    let mut nested = Table::new(vec![
        TableRow::new(vec![cell("Nested fixed-width table")]).into(),
    ]);
    nested.properties = TableProperties {
        column_widths: Some(vec![1440]),
        layout: Some(TableLayout::Fixed),
        ..Default::default()
    }
    .into();
    let mut table = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![nested.into(), p("Paragraph after nested table")]).into(),
            cell("Autofit neighbor"),
        ])
        .into(),
    ]);
    table.properties = TableProperties {
        layout: Some(TableLayout::Autofit),
        column_widths: Some(vec![2000, 2000]),
        ..Default::default()
    }
    .into();
    out.push(table.into());
    for (direction, name) in [
        (CellTextDirection::LeftToRight, "lrTb"),
        (CellTextDirection::TopToBottomRightToLeft, "tbRl"),
        (CellTextDirection::BottomToTopLeftToRight, "btLr"),
        (CellTextDirection::LeftToRightVertical, "lrTbV"),
        (CellTextDirection::TopToBottomRightToLeftVertical, "tbRlV"),
        (CellTextDirection::TopToBottomLeftToRightVertical, "tbLrV"),
    ] {
        out.push(p(format!(
            "Cell text_direction={name}; independent from paragraph bidi."
        )));
        let mut cell = TableCell::new(vec![p(format!("Text flow {name}"))]);
        cell.properties = TableCellProperties {
            text_direction: Some(direction),
            vertical_alignment: Some(VerticalAlignment::Top),
            ..Default::default()
        }
        .into();
        let mut row = TableRow::new(vec![cell.into()]);
        row.properties = TableRowProperties {
            height: Some(RowHeight::AtLeast(72.)),
            ..Default::default()
        }
        .into();
        out.push(Table::new(vec![row.into()]).into());
    }
    for (alignment, name) in [
        (TableAlignment::Left, "left"),
        (TableAlignment::Center, "center"),
        (TableAlignment::Right, "right"),
        (TableAlignment::Start, "start"),
        (TableAlignment::End, "end"),
    ] {
        out.push(p(format!("Inline table alignment={name}; fixed width=2160 twips. No floating positioning or paragraph alignment is requested.")));
        let mut table = Table::new(vec![
            TableRow::new(vec![cell(format!("Table alignment: {name}"))]).into(),
        ]);
        table.properties = TableProperties {
            alignment: Some(alignment),
            column_widths: Some(vec![2160]),
            ..Default::default()
        }
        .into();
        out.push(table.into());
    }
    out.push(p("Repeated-header pagination sample: 36 rows follow. The consumer decides actual page breaks."));
    let mut rows = Vec::new();
    for i in 0..36 {
        let mut row = TableRow::new(vec![
            cell(if i == 0 {
                "Repeated header: index".to_owned()
            } else {
                format!("Row {i}")
            }),
            cell("Minimum 20 pt; cannot split"),
        ]);
        row.properties = TableRowProperties {
            content_control: None,
            height: Some(RowHeight::AtLeast(20.)),
            repeat_header: Some(i == 0),
            cant_split: Some(true),
        }
        .into();
        rows.push(row.into());
    }
    out.push(Table::new(rows).into());
    out.push(p("Table look: all six activation flags=false (mask 0600). Whole-table formatting remains unconditional; no region formatting is copied to direct cells."));
    let mut disabled = Table::new(vec![
        TableRow::new(vec![cell(
            "All conditional regions disabled; base whole_table still applies",
        )])
        .into(),
    ]);
    disabled.properties = TableProperties {
        style: Some("Region whole_table".into()),
        look: Some(TableLook {
            first_row: Some(false),
            last_row: Some(false),
            first_column: Some(false),
            last_column: Some(false),
            horizontal_banding: Some(false),
            vertical_banding: Some(false),
        }),
        ..Default::default()
    }
    .into();
    out.push(disabled.into());
    out.push(heading("05 Conditional table styles: all 13 regions"));
    out.push(p("Each table references a separate region style. All six table-look activation flags are explicitly enabled (mask 01E0). Region fill/font/paragraph/script requests stay in styles.xml, not direct cell formatting; the viewer applies conditional precedence. Base whole_table is unconditional."));
    for name in REGIONS {
        out.push(p(format!("Conditional region={name}: blue shading, bold 10 pt, centered, minimum line=12 pt, tab=36 pt, bidi/RTL=false; language/script/font slots authored.")));
        let rows = (1..=3)
            .map(|r| {
                TableRow::new((1..=3).map(|c| cell(format!("{name} r{r}c{c}"))).collect()).into()
            })
            .collect();
        let mut table = Table::new(rows);
        table.properties = TableProperties {
            style: Some(format!("Region {name}")),
            look: Some(TableLook {
                first_row: Some(true),
                last_row: Some(true),
                first_column: Some(true),
                last_column: Some(true),
                horizontal_banding: Some(true),
                vertical_banding: Some(true),
            }),
            ..Default::default()
        }
        .into();
        out.push(table.into());
    }
}
