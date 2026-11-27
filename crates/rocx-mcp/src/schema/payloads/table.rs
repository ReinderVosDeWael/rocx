use rmcp::schemars::{self, JsonSchema};
use rocx::{
    BorderLine, BorderSide, CellBorder, CellBorderSide, CellMargins, CellTextDirection,
    ResolvedTable, RowHeight, Table, TableAlignment, TableBorder, TableCell, TableCellProperties,
    TableLayout, TableLook, TableProperties, TableRow, TableRowProperties, Value, VerticalMerge,
};
use serde::Deserialize;

use super::{BlockInput, resolve_component};
use crate::schema::CellAlignment;

macro_rules! choices {
    ($wire:ident => $native:ident { $($variant:ident),+ }) => {
        #[derive(Deserialize, JsonSchema)]
        #[serde(rename_all = "kebab-case")]
        enum $wire { $($variant),+ }
        impl From<$wire> for $native {
            fn from(value: $wire) -> Self {
                match value { $($wire::$variant => Self::$variant),+ }
            }
        }
    };
}

choices!(LayoutInput => TableLayout { Fixed, Autofit });
choices!(AlignmentInput => TableAlignment { Left, Center, Right, Start, End });
choices!(MergeInput => VerticalMerge { Restart, Continue });
choices!(TableSide => BorderSide { Top, Left, Bottom, Right, InsideH, InsideV });
choices!(CellSide => CellBorderSide { Top, Bottom, Start, End });
choices!(BorderInput => BorderLine { Single, Dashed });
choices!(Direction => CellTextDirection {
    LeftToRight, TopToBottomRightToLeft, BottomToTopLeftToRight, LeftToRightVertical,
    TopToBottomRightToLeftVertical, TopToBottomLeftToRightVertical
});

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Border<S> {
    side: S,
    /// Border width in eighth-points.
    size: u32,
    color: [u8; 3],
    line: BorderInput,
}

impl From<Border<TableSide>> for TableBorder {
    fn from(value: Border<TableSide>) -> Self {
        Self {
            side: value.side.into(),
            sz: value.size,
            color: value.color,
            val: value.line.into(),
        }
    }
}

impl From<Border<CellSide>> for CellBorder {
    fn from(value: Border<CellSide>) -> Self {
        Self {
            side: value.side.into(),
            sz: value.size,
            color: value.color,
            val: value.line.into(),
        }
    }
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TableInputProperties {
    /// Explicit grid column widths in twips.
    column_widths: Option<Vec<u32>>,
    layout: Option<LayoutInput>,
    style: Option<String>,
    #[serde(default)]
    borders: Vec<Border<TableSide>>,
    look: Option<Look>,
    alignment: Option<AlignmentInput>,
}

impl From<TableInputProperties> for TableProperties {
    fn from(value: TableInputProperties) -> Self {
        Self {
            column_widths: value.column_widths,
            layout: value.layout.map(Into::into),
            style: value.style,
            borders: value.borders.into_iter().map(Into::into).collect(),
            look: value.look.map(|v| TableLook {
                first_row: v.first_row,
                last_row: v.last_row,
                first_column: v.first_column,
                last_column: v.last_column,
                horizontal_banding: v.horizontal_banding,
                vertical_banding: v.vertical_banding,
            }),
            alignment: value.alignment.map(Into::into),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Look {
    first_row: Option<bool>,
    last_row: Option<bool>,
    first_column: Option<bool>,
    last_column: Option<bool>,
    horizontal_banding: Option<bool>,
    vertical_banding: Option<bool>,
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RowProperties {
    height: Option<Height>,
    repeat_header: Option<bool>,
    cant_split: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum Height {
    Exact { points: f64 },
    AtLeast { points: f64 },
}

#[derive(Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CellProperties {
    grid_span: Option<u32>,
    vmerge: Option<MergeInput>,
    #[serde(default)]
    borders: Vec<Border<CellSide>>,
    background_color: Option<[u8; 3]>,
    vertical_alignment: Option<CellAlignment>,
    margins: Option<Margins>,
    text_direction: Option<Direction>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Margins {
    top: Option<f64>,
    start: Option<f64>,
    bottom: Option<f64>,
    end: Option<f64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RichTableInput {
    #[schemars(length(min = 1))]
    rows: Vec<RowInput>,
    properties: Option<TableInputProperties>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RowInput {
    /// An empty row is padded to the grid established by the other rows.
    cells: Vec<CellInput>,
    properties: Option<RowProperties>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CellInput {
    blocks: Vec<BlockInput>,
    properties: Option<CellProperties>,
}

impl RichTableInput {
    pub(crate) fn resolve(self) -> Result<ResolvedTable, String> {
        if self.rows.iter().all(|row| row.cells.is_empty()) {
            return Err("a rich table requires at least one row with cells".into());
        }
        let rows: Vec<Value<TableRow>> = self
            .rows
            .into_iter()
            .map(|row| {
                let cells: Vec<Value<TableCell>> = row
                    .cells
                    .into_iter()
                    .map(|cell| {
                        let blocks = cell
                            .blocks
                            .into_iter()
                            .map(|block| block.resolve().map(Into::into))
                            .collect::<Result<_, _>>()?;
                        let p = cell.properties.unwrap_or_default();
                        Ok(TableCell::new(blocks)
                            .with_properties(TableCellProperties {
                                grid_span: p.grid_span,
                                vmerge: p.vmerge.map(Into::into),
                                borders: p.borders.into_iter().map(Into::into).collect(),
                                background_color: p.background_color,
                                vertical_alignment: p.vertical_alignment.map(Into::into),
                                margins: p.margins.map(|v| CellMargins {
                                    top: v.top,
                                    start: v.start,
                                    bottom: v.bottom,
                                    end: v.end,
                                }),
                                text_direction: p.text_direction.map(Into::into),
                                content_control: None,
                            })
                            .into())
                    })
                    .collect::<Result<_, String>>()?;
                let p = row.properties.unwrap_or_default();
                Ok(TableRow::new(cells)
                    .with_properties(TableRowProperties {
                        height: p.height.map(|v| match v {
                            Height::Exact { points } => RowHeight::Exact(points),
                            Height::AtLeast { points } => RowHeight::AtLeast(points),
                        }),
                        repeat_header: p.repeat_header,
                        cant_split: p.cant_split,
                        content_control: None,
                    })
                    .into())
            })
            .collect::<Result<_, String>>()?;
        let props = TableProperties::from(self.properties.unwrap_or_default());
        resolve_component(Table::new(rows).with_properties(props))
    }
}
