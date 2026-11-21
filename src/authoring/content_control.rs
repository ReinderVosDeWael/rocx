//! Inline rich-text control authoring.
use crate::document::setters;
use crate::{
    Block, BlockChildren, Condition, ContentControlProperties, Inline, InlineChildren, Value,
};

#[derive(Debug)]
pub struct InlineContentControl {
    pub children: Value<InlineChildren>,
    pub properties: Value<ContentControlProperties>,
    pub condition: Condition,
}
setters!(InlineContentControl {
    with_children => children: into<Value<InlineChildren>>,
    with_properties => properties: into<Value<ContentControlProperties>>,
});
impl InlineContentControl {
    pub fn new(children: impl Into<Value<InlineChildren>>) -> Self {
        Self {
            children: children.into(),
            properties: Value::default(),
            condition: Condition::default(),
        }
    }

    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }
}
impl From<InlineContentControl> for Inline {
    fn from(value: InlineContentControl) -> Self {
        Self::ContentControl(value)
    }
}
impl From<InlineContentControl> for Value<Inline> {
    fn from(value: InlineContentControl) -> Self {
        Self::ready(Inline::ContentControl(value))
    }
}

#[derive(Debug)]
pub struct BlockContentControl {
    pub children: Value<BlockChildren>,
    pub properties: Value<ContentControlProperties>,
    pub condition: Condition,
}
setters!(BlockContentControl {
    with_children => children: into<Value<BlockChildren>>,
    with_properties => properties: into<Value<ContentControlProperties>>,
});
impl BlockContentControl {
    pub fn new(children: impl Into<Value<BlockChildren>>) -> Self {
        Self {
            children: children.into(),
            properties: Value::default(),
            condition: Condition::default(),
        }
    }

    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }
}
impl From<BlockContentControl> for Block {
    fn from(value: BlockContentControl) -> Self {
        Self::ContentControl(value)
    }
}
impl From<BlockContentControl> for Value<Block> {
    fn from(value: BlockContentControl) -> Self {
        Self::ready(Block::ContentControl(value))
    }
}
