use rocx::{
    Block, BorderLine, BorderSide, CellBorder, CellBorderSide, CellMargins, CellTextDirection,
    ContentControlLock, ContentControlProperties, Document, Paragraph, RowHeight, Section,
    StyleDefinition, Table, TableAlignment, TableBorder, TableCell, TableCellProperties,
    TableLayout, TableLook, TableProperties, TableRow, TableRowProperties, TableStyleDefinition,
    Value, VerticalAlignment, VerticalMerge,
};
use std::path::Path;

const TIMESTAMP: &str = "2022-02-03T04:05:06Z";

fn paragraph(text: &'static str) -> Value<Block> {
    Paragraph::default().with_text(text).into()
}

fn cell(children: Vec<Value<Block>>) -> Value<TableCell> {
    TableCell::new(children).into()
}

fn nested() -> Table {
    Table::new(vec![
        TableRow::new(vec![
            cell(vec![paragraph("nested A")]),
            cell(vec![paragraph("nested B")]),
        ])
        .into(),
    ])
}

fn document(children: Vec<Value<Block>>) -> Document {
    Document::new(vec![Section::new(children).into()]).with_timestamp(Some(TIMESTAMP.to_owned()))
}

pub async fn write(out: &Path) -> rocx::Result<()> {
    let table = Table::new(vec![
        TableRow::new(vec![
            TableCell::new(vec![nested().into()])
                .with_properties(
                    TableCellProperties::default()
                        .with_grid_span(2)
                        .with_vmerge(VerticalMerge::Restart),
                )
                .into(),
            cell(vec![paragraph("right")]),
        ])
        .into(),
        TableRow::new(vec![
            TableCell::new(vec![])
                .with_properties(
                    TableCellProperties::default()
                        .with_grid_span(2)
                        .with_vmerge(VerticalMerge::Continue),
                )
                .into(),
        ])
        .into(),
    ])
    .with_properties(
        TableProperties::default()
            .with_layout(TableLayout::Fixed)
            .with_column_widths(vec![1200, 2400, 3600])
            .with_alignment(TableAlignment::End)
            .with_style("Data grid")
            .with_look(
                TableLook::default()
                    .with_first_row(true)
                    .with_last_row(false)
                    .with_first_column(false)
                    .with_last_column(true)
                    .with_horizontal_banding(false)
                    .with_vertical_banding(true),
            )
            .with_borders(
                [
                    BorderSide::Top,
                    BorderSide::Left,
                    BorderSide::Bottom,
                    BorderSide::Right,
                    BorderSide::InsideH,
                    BorderSide::InsideV,
                ]
                .into_iter()
                .enumerate()
                .map(|(index, side)| TableBorder {
                    side,
                    sz: 4 + index as u32,
                    color: [12, 34, 56],
                    val: if index % 2 == 0 {
                        BorderLine::Single
                    } else {
                        BorderLine::Dashed
                    },
                })
                .collect(),
            ),
    );
    let resolved = document(vec![table.into()])
        .with_styles(vec![StyleDefinition::Table(Box::new(
            TableStyleDefinition::default().with_name("Data grid"),
        ))])
        .resolve()
        .await?;
    std::fs::write(out.join("tables.docx"), resolved.to_docx()?)?;

    let autofit = Table::new(vec![
        TableRow::new(vec![
            cell(vec![]),
            TableCell::new(vec![nested().into()])
                .with_properties(TableCellProperties::default().with_grid_span(2))
                .into(),
        ])
        .into(),
    ])
    .with_properties(
        TableProperties::default()
            .with_layout(TableLayout::Autofit)
            .with_column_widths(vec![0, 0]),
    );
    let resolved = document(vec![autofit.into()]).resolve().await?;
    std::fs::write(out.join("tables-auto.docx"), resolved.to_docx()?)?;

    let resolved = document(layout_tables()).resolve().await?;
    std::fs::write(out.join("tables-layout.docx"), resolved.to_docx()?)?;
    Ok(())
}

fn wrapper(alias: &str, tag: &str, id: u32) -> ContentControlProperties {
    ContentControlProperties::default()
        .with_alias(alias)
        .with_tag(tag)
        .with_lock(ContentControlLock::ControlLocked)
        .with_id(id)
}

fn layout_tables() -> Vec<Value<Block>> {
    use CellTextDirection::*;
    use VerticalAlignment::*;
    let directions = [
        LeftToRight,
        TopToBottomRightToLeft,
        BottomToTopLeftToRight,
        LeftToRightVertical,
        TopToBottomRightToLeftVertical,
        TopToBottomLeftToRightVertical,
    ];
    let cells = directions
        .into_iter()
        .zip([Top, Center, Bottom, Both].into_iter().cycle())
        .enumerate()
        .map(|(index, (direction, alignment))| {
            let mut props = TableCellProperties::default()
                .with_text_direction(direction)
                .with_vertical_alignment(alignment);
            if index == 0 {
                let borders = [
                    CellBorderSide::Top,
                    CellBorderSide::Bottom,
                    CellBorderSide::Start,
                    CellBorderSide::End,
                ]
                .into_iter()
                .enumerate()
                .map(|(index, side)| CellBorder {
                    side,
                    sz: 4 + index as u32,
                    color: [12, 34, 56],
                    val: if index % 2 == 0 {
                        BorderLine::Single
                    } else {
                        BorderLine::Dashed
                    },
                })
                .collect();
                props = props
                    .with_margins(
                        CellMargins::default()
                            .with_top(0.)
                            .with_start(1.25)
                            .with_bottom(2.)
                            .with_end(3.),
                    )
                    .with_background_color([255, 0, 12])
                    .with_borders(borders)
                    .with_content_control(wrapper("Cell wrapper", "cell-control", 102));
            }
            TableCell::new(vec![Paragraph::new(format!("direction {index}")).into()])
                .with_properties(props)
                .into()
        })
        .collect();
    let layout = Table::new(vec![
        TableRow::new(cells)
            .with_properties(
                TableRowProperties::default()
                    .with_height(RowHeight::Exact(18.))
                    .with_repeat_header(true)
                    .with_cant_split(false)
                    .with_content_control(wrapper("Row wrapper", "row-control", 101)),
            )
            .into(),
        TableRow::new(vec![cell(vec![])])
            .with_properties(
                TableRowProperties::default()
                    .with_height(RowHeight::AtLeast(11.25))
                    .with_repeat_header(false)
                    .with_cant_split(true),
            )
            .into(),
        TableRow::new(vec![cell(vec![])]).into(),
    ]);
    let mut tables = vec![layout.into()];
    tables.extend(
        [
            TableAlignment::Left,
            TableAlignment::Center,
            TableAlignment::Right,
            TableAlignment::Start,
            TableAlignment::End,
        ]
        .into_iter()
        .map(|alignment| -> Value<Block> {
            Table::new(vec![TableRow::new(vec![cell(vec![])]).into()])
                .with_properties(TableProperties::default().with_alignment(alignment))
                .into()
        }),
    );
    tables
}
