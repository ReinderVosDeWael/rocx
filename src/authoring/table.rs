//! Table declarations, resolution, and resolved grid content.
//!
//! References: ECMA-376 Part 1 (2016), §17.4 (tables), §17.4.17
//! (`gridSpan`), and §17.4.84 (`vMerge`).
//! [Official Part 1](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip).
use crate::document::setters;
use crate::{Block, BlockChildren, Condition, TableCellProperties, TableProperties, Value};
#[derive(Debug, Default)]
pub struct Table {
    pub rows: Value<Vec<Value<TableRow>>>,
    pub properties: Value<TableProperties>,
    pub condition: Condition,
}
setters!(Table {
    with_rows => rows: into<Value<Vec<Value<TableRow>>>>,
    with_properties => properties: into<Value<TableProperties>>,
});
#[derive(Debug, Default)]
pub struct TableRow {
    pub children: Value<Vec<Value<TableCell>>>,
    pub properties: Value<crate::TableRowProperties>,
    pub condition: Condition,
}
setters!(TableRow {
    with_children => children: into<Value<Vec<Value<TableCell>>>>,
});
#[derive(Debug, Default)]
pub struct TableCell {
    pub children: Value<BlockChildren>,
    pub properties: Value<TableCellProperties>,
    pub condition: Condition,
}
setters!(TableCell {
    with_children => children: into<Value<BlockChildren>>,
    with_properties => properties: into<Value<TableCellProperties>>,
});
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
    pub fn with_properties(
        mut self,
        properties: impl Into<Value<crate::TableRowProperties>>,
    ) -> Self {
        self.properties = properties.into();
        self
    }
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
