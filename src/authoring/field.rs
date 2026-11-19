//! Deferred fields; balanced structure is constructed rather than marker-authored.
use crate::{Condition, FieldInstruction, FieldProperties, Inline, InlineChildren, Value};

#[derive(Debug)]
pub struct Field {
    pub instruction: Value<FieldInstruction>,
    pub children: Value<InlineChildren>,
    pub properties: Value<FieldProperties>,
    pub condition: Condition,
}
impl Field {
    pub fn new(
        instruction: impl Into<Value<FieldInstruction>>,
        children: impl Into<Value<InlineChildren>>,
    ) -> Self {
        Self {
            instruction: instruction.into(),
            children: children.into(),
            properties: Value::default(),
            condition: Condition::default(),
        }
    }
    pub fn with_properties(mut self, properties: impl Into<Value<FieldProperties>>) -> Self {
        self.properties = properties.into();
        self
    }
    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }
}
impl From<Field> for Inline {
    fn from(value: Field) -> Self {
        Self::Field(value)
    }
}
impl From<Field> for Value<Inline> {
    fn from(value: Field) -> Self {
        Self::ready(Inline::Field(value))
    }
}
