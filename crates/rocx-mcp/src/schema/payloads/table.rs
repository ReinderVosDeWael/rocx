use rmcp::schemars::{self, JsonSchema};
use rocx::{
    BorderLine, BorderSide, CellBorder, CellBorderSide, CellMargins, CellTextDirection,
    ResolvedBlock, ResolvedTable, RowHeight, Table, TableAlignment, TableBorder, TableCell,
    TableCellProperties, TableLayout, TableLook, TableProperties, TableRow, TableRowProperties,
    Value, VerticalMerge,
};
use serde::Deserialize;

use super::{BlockInput, ParagraphInput, resolve_component};
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
#[serde(untagged)]
pub(crate) enum RichTableInput {
    Nested(TableInput<BlockInput>),
    Flat(FlatTableInput),
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TableInput<B> {
    #[schemars(length(min = 1))]
    rows: Vec<RowInput<B>>,
    properties: Option<TableInputProperties>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RowInput<B> {
    /// An empty row is padded to the grid established by the other rows.
    cells: Vec<CellInput<B>>,
    properties: Option<RowProperties>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CellInput<B> {
    blocks: Vec<B>,
    properties: Option<CellProperties>,
}

#[derive(Clone, Copy)]
enum TableUse {
    Fragment,
    PropertyDonor,
}

impl RichTableInput {
    pub(crate) fn resolve(self) -> Result<ResolvedTable, String> {
        self.resolve_for(TableUse::Fragment)
    }

    pub(crate) fn resolve_donor(self) -> Result<ResolvedTable, String> {
        self.resolve_for(TableUse::PropertyDonor)
    }

    fn resolve_for(self, usage: TableUse) -> Result<ResolvedTable, String> {
        match self {
            Self::Nested(table) => table.resolve(usage, BlockInput::resolve),
            Self::Flat(tree) => tree.resolve(usage),
        }
    }
}

impl<B> TableInput<B> {
    fn resolve(
        self,
        usage: TableUse,
        mut resolve_block: impl FnMut(B) -> Result<ResolvedBlock, String>,
    ) -> Result<ResolvedTable, String> {
        match usage {
            TableUse::Fragment if self.rows.iter().all(|row| row.cells.is_empty()) => {
                return Err("a rich table requires at least one row with cells".into());
            }
            // A property copy consumes formatting rather than rendering the
            // source, so it can use a donor row with no cells.
            TableUse::PropertyDonor if self.rows.is_empty() => {
                return Err("a rich table property donor requires at least one row".into());
            }
            _ => {}
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
                            .map(|block| resolve_block(block).map(Into::into))
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

const MAX_TABLE_NODES: usize = 1024;
const MAX_TABLE_ENTRIES: usize = 16384;
// Each grid column adds one XML element. Reserve at most ~6.6% of the
// native 1,000,000-node per-part import budget for a declaration's grids.
// This is not a guarantee that the edited part, including existing content,
// will fit the native import budget.
const MAX_GRID_COLUMNS: usize = 65536;
const MAX_RENDERED_CELLS: usize = 16384;
// A table level consumes three XML elements (tbl/tr/tc). A header can
// reopen 84 levels with an empty terminal cell under the native XML depth
// limit; body scope and rich descendants consume additional XML depth.
const MAX_TABLE_DEPTH: usize = 84;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct FlatTableInput {
    /// Node 0 is the whole table. Every later node is referenced exactly once
    /// from an earlier node's cell, at its position in the blocks array.
    /// Limited to 84 table levels, 16,384 row/cell/block entries, 65,536
    /// emitted grid columns, and 16,384 emitted cells including padding, in
    /// aggregate across the declaration. Native XML depth limits still apply.
    #[schemars(length(min = 1, max = MAX_TABLE_NODES))]
    nodes: Vec<TableInput<FlatBlock>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum FlatBlock {
    Paragraph {
        paragraph: ParagraphInput,
    },
    Table {
        /// Nonempty rectangular array of plain-text cells.
        #[schemars(length(min = 1))]
        rows: Vec<Vec<String>>,
        /// Grid widths in twips, one per column.
        column_widths: Option<Vec<u32>>,
    },
    TableRef {
        /// Zero-based index in nodes, strictly after the referencing node.
        node: usize,
    },
}

impl FlatTableInput {
    fn validate(&self) -> Result<(), String> {
        if !(1..=MAX_TABLE_NODES).contains(&self.nodes.len()) {
            return Err(format!(
                "flat rich table requires 1 to {MAX_TABLE_NODES} nodes"
            ));
        }
        let mut depths = vec![None; self.nodes.len()];
        depths[0] = Some(1);
        let mut entries = 0;
        for (index, table) in self.nodes.iter().enumerate() {
            let depth = depths[index]
                .ok_or_else(|| format!("unreferenced flat rich table node {index}"))?;
            count_entries(&mut entries, table.rows.len())?;
            for row in &table.rows {
                count_entries(&mut entries, row.cells.len())?;
                for cell in &row.cells {
                    count_entries(&mut entries, cell.blocks.len())?;
                    for block in &cell.blocks {
                        match block {
                            FlatBlock::TableRef { node } => {
                                if *node >= self.nodes.len() {
                                    return Err(format!(
                                        "flat rich table node {index} references out-of-bounds node {node}"
                                    ));
                                }
                                if *node <= index {
                                    return Err(format!(
                                        "flat rich table node {index} must reference a later node, got {node}"
                                    ));
                                }
                                if depths[*node].is_some() {
                                    return Err(format!(
                                        "flat rich table node {node} is referenced more than once"
                                    ));
                                }
                                depths[*node] = Some(child_depth(depth)?);
                            }
                            FlatBlock::Table { rows, .. } => {
                                child_depth(depth)?;
                                count_entries(&mut entries, rows.len())?;
                                for row in rows {
                                    count_entries(&mut entries, row.len())?;
                                }
                            }
                            FlatBlock::Paragraph { .. } => {}
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn resolve(self, root_use: TableUse) -> Result<ResolvedTable, String> {
        self.validate()?;
        let mut resolved: Vec<Option<ResolvedTable>> =
            (0..self.nodes.len()).map(|_| None).collect();
        let mut render_budget = RenderBudget::default();
        for (index, node) in self.nodes.into_iter().enumerate().rev() {
            let usage = if index == 0 {
                root_use
            } else {
                TableUse::Fragment
            };
            let table = node.resolve(usage, |block| match block {
                FlatBlock::Paragraph { paragraph } => BlockInput::Paragraph { paragraph }.resolve(),
                FlatBlock::Table {
                    rows,
                    column_widths,
                } => {
                    let block = BlockInput::Table {
                        rows,
                        column_widths,
                    }
                    .resolve()?;
                    if let ResolvedBlock::Table(table) = &block {
                        render_budget.count(table)?;
                    }
                    Ok(block)
                }
                FlatBlock::TableRef { node } => Ok(ResolvedBlock::Table(
                    resolved[node]
                        .take()
                        .expect("validated single forward reference"),
                )),
            })?;
            render_budget.count(&table)?;
            resolved[index] = Some(table);
        }
        Ok(resolved[0].take().expect("validated nonempty table tree"))
    }
}

fn child_depth(depth: usize) -> Result<usize, String> {
    if depth >= MAX_TABLE_DEPTH {
        Err(format!("flat rich table exceeds depth {MAX_TABLE_DEPTH}"))
    } else {
        Ok(depth + 1)
    }
}

fn count_entries(total: &mut usize, count: usize) -> Result<(), String> {
    *total = total.saturating_add(count);
    if *total > MAX_TABLE_ENTRIES {
        return Err(format!(
            "flat rich table exceeds {MAX_TABLE_ENTRIES} row, cell and block entries"
        ));
    }
    Ok(())
}

#[derive(Default)]
struct RenderBudget {
    grid_columns: usize,
    cells: usize,
}

impl RenderBudget {
    fn count(&mut self, table: &ResolvedTable) -> Result<(), String> {
        self.grid_columns += table.columns();
        if self.grid_columns > MAX_GRID_COLUMNS {
            return Err(format!(
                "flat rich table exceeds {MAX_GRID_COLUMNS} emitted grid columns"
            ));
        }
        for row in table.rows() {
            // Mirror the writer: a spanning cell emits one w:tc, and only
            // uncovered columns at the end of a row get padding cells.
            // Native resolution guarantees the spans sum to at most columns.
            let covered: usize = row
                .cells()
                .iter()
                .map(|cell| cell.grid_span() as usize)
                .sum();
            self.cells += row.cells().len() + (table.columns() - covered);
            if self.cells > MAX_RENDERED_CELLS {
                return Err(format!(
                    "flat rich table exceeds {MAX_RENDERED_CELLS} emitted cells"
                ));
            }
        }
        Ok(())
    }
}
