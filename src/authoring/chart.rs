use crate::document::setters;
use crate::{ColumnChartProperties, Condition, Inline, Value};

/// An inline workbook-free column chart in a body paragraph.
#[derive(Debug)]
pub struct ColumnChartRun {
    pub properties: Value<ColumnChartProperties>,
    pub condition: Condition,
}
setters!(ColumnChartRun {
    with_properties => properties: into<Value<ColumnChartProperties>>,
});
impl ColumnChartRun {
    pub fn new(properties: impl Into<Value<ColumnChartProperties>>) -> Self {
        Self {
            properties: properties.into(),
            condition: Condition::default(),
        }
    }

    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }
}
impl From<ColumnChartRun> for Inline {
    fn from(value: ColumnChartRun) -> Self {
        Self::ColumnChartRun(value)
    }
}
impl From<ColumnChartRun> for Value<Inline> {
    fn from(value: ColumnChartRun) -> Self {
        Self::ready(Inline::ColumnChartRun(value))
    }
}
