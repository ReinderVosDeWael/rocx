//! Stable paired range markers. Conditions can omit markers; validation catches dangling pairs.
use crate::{BookmarkName, Condition, Inline, RangeId, RangeKind, ResolvedComment, Value};

#[derive(Debug)]
pub struct RangeStart {
    pub id: Value<RangeId>,
    pub kind: Value<RangeKind>,
    pub condition: Condition,
}
#[derive(Debug)]
pub struct RangeEnd {
    pub id: Value<RangeId>,
    pub condition: Condition,
}
impl RangeStart {
    pub fn new(id: impl Into<Value<RangeId>>, kind: impl Into<Value<RangeKind>>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            condition: Condition::default(),
        }
    }
    pub fn bookmark(name: BookmarkName) -> Self {
        Self::new(name.range_id(), RangeKind::Bookmark(name))
    }
    pub fn comment(id: RangeId, comment: ResolvedComment) -> Self {
        Self::new(id, RangeKind::Comment(comment))
    }
    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }
}
impl RangeEnd {
    pub fn new(id: impl Into<Value<RangeId>>) -> Self {
        Self {
            id: id.into(),
            condition: Condition::default(),
        }
    }
    pub fn bookmark(name: &BookmarkName) -> Self {
        Self::new(name.range_id())
    }
    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }
}
impl From<RangeStart> for Inline {
    fn from(value: RangeStart) -> Self {
        Self::RangeStart(value)
    }
}
impl From<RangeStart> for Value<Inline> {
    fn from(value: RangeStart) -> Self {
        Self::ready(Inline::RangeStart(value))
    }
}
impl From<RangeEnd> for Inline {
    fn from(value: RangeEnd) -> Self {
        Self::RangeEnd(value)
    }
}
impl From<RangeEnd> for Value<Inline> {
    fn from(value: RangeEnd) -> Self {
        Self::ready(Inline::RangeEnd(value))
    }
}
