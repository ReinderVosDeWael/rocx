//! Table declarations, resolution, and resolved grid content.
//!
//! References: ECMA-376 Part 1 (2016), §17.4 (tables), §17.4.17
//! (`gridSpan`), and §17.4.84 (`vMerge`).
//! [Official Part 1](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip).
use crate::{Block, BlockChildren, Condition, TableCellProperties, TableProperties, Value};
#[derive(Debug, Default)]
pub struct Table {
    pub rows: Value<Vec<Value<TableRow>>>,
    pub properties: Value<TableProperties>,
    pub condition: Condition,
}
#[derive(Debug, Default)]
pub struct TableRow {
    pub children: Value<Vec<Value<TableCell>>>,
    pub condition: Condition,
}
#[derive(Debug, Default)]
pub struct TableCell {
    pub children: Value<BlockChildren>,
    pub properties: Value<TableCellProperties>,
    pub condition: Condition,
}
impl Table {
    pub fn new(rows: Vec<Value<TableRow>>) -> Self {
        Self {
            rows: rows.into(),
            ..Self::default()
        }
    }
    pub fn when(mut self, c: impl Into<Condition>) -> Self {
        self.condition = c.into();
        self
    }
}
impl TableRow {
    pub fn new(children: Vec<Value<TableCell>>) -> Self {
        Self {
            children: children.into(),
            ..Self::default()
        }
    }
    pub fn when(mut self, c: impl Into<Condition>) -> Self {
        self.condition = c.into();
        self
    }
}
impl TableCell {
    pub fn new(children: BlockChildren) -> Self {
        Self {
            children: children.into(),
            ..Self::default()
        }
    }
    pub fn when(mut self, c: impl Into<Condition>) -> Self {
        self.condition = c.into();
        self
    }
}
impl From<Table> for Block {
    fn from(t: Table) -> Self {
        Self::Table(t)
    }
}
impl From<Table> for Value<Block> {
    fn from(t: Table) -> Self {
        Self::Ready(Block::Table(t))
    }
}
