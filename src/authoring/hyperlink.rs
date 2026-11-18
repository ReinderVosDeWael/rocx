//! External hyperlink authoring; display children are independent of the target.
use crate::{Condition, HyperlinkProperties, HyperlinkTarget, Inline, InlineChildren, Value};

#[derive(Debug)]
pub struct Hyperlink {
    pub target: Value<HyperlinkTarget>,
    pub children: Value<InlineChildren>,
    pub properties: Value<HyperlinkProperties>,
    pub condition: Condition,
}
impl Hyperlink {
    pub fn new(
        target: impl Into<Value<HyperlinkTarget>>,
        children: impl Into<Value<InlineChildren>>,
    ) -> Self {
        Self {
            target: target.into(),
            children: children.into(),
            properties: Value::default(),
            condition: Condition::default(),
        }
    }
    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }
    pub fn with_properties(mut self, properties: impl Into<Value<HyperlinkProperties>>) -> Self {
        self.properties = properties.into();
        self
    }
}
impl From<Hyperlink> for Inline {
    fn from(value: Hyperlink) -> Self {
        Self::Hyperlink(value)
    }
}
impl From<Hyperlink> for Value<Inline> {
    fn from(value: Hyperlink) -> Self {
        Self::ready(Inline::Hyperlink(value))
    }
}
