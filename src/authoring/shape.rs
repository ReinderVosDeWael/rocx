use crate::document::setters;
use crate::{Condition, Inline, ShapeProperties, Value};

#[derive(Debug, Default)]
pub struct ShapeRun {
    pub properties: Value<ShapeProperties>,
    pub condition: Condition,
}
setters!(ShapeRun {
    with_properties => properties: into<Value<ShapeProperties>>,
});
impl ShapeRun {
    pub fn new(properties: impl Into<Value<ShapeProperties>>) -> Self {
        Self::default().with_properties(properties)
    }
    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }
}
impl From<ShapeRun> for Inline {
    fn from(value: ShapeRun) -> Self {
        Self::ShapeRun(value)
    }
}
impl From<ShapeRun> for Value<Inline> {
    fn from(value: ShapeRun) -> Self {
        Self::ready(Inline::ShapeRun(value))
    }
}
