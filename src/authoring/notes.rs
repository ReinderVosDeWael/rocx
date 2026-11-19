//! Generic deferred note and structured comment story declarations.
use crate::{
    BlockChildren, CommentMetadata, Condition, Inline, NoteId, NoteKind, NoteTarget, Value,
};

#[derive(Debug)]
pub struct Note {
    pub target: Value<NoteTarget>,
    pub children: Value<BlockChildren>,
    pub condition: Condition,
}
impl Note {
    pub fn new(
        target: impl Into<Value<NoteTarget>>,
        children: impl Into<Value<BlockChildren>>,
    ) -> Self {
        Self {
            target: target.into(),
            children: children.into(),
            condition: Condition::default(),
        }
    }
    pub fn footnote(id: NoteId, children: impl Into<Value<BlockChildren>>) -> Self {
        Self::new(
            NoteTarget {
                kind: NoteKind::Footnote,
                id,
            },
            children,
        )
    }
    pub fn endnote(id: NoteId, children: impl Into<Value<BlockChildren>>) -> Self {
        Self::new(
            NoteTarget {
                kind: NoteKind::Endnote,
                id,
            },
            children,
        )
    }
    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }
}
#[derive(Debug)]
pub struct NoteReference {
    pub target: Value<NoteTarget>,
    pub condition: Condition,
}
impl NoteReference {
    pub fn new(target: impl Into<Value<NoteTarget>>) -> Self {
        Self {
            target: target.into(),
            condition: Condition::default(),
        }
    }
    pub fn footnote(id: NoteId) -> Self {
        Self::new(NoteTarget {
            kind: NoteKind::Footnote,
            id,
        })
    }
    pub fn endnote(id: NoteId) -> Self {
        Self::new(NoteTarget {
            kind: NoteKind::Endnote,
            id,
        })
    }
    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }
}
impl From<NoteReference> for Inline {
    fn from(value: NoteReference) -> Self {
        Self::NoteReference(value)
    }
}
impl From<NoteReference> for Value<Inline> {
    fn from(value: NoteReference) -> Self {
        Self::ready(Inline::NoteReference(value))
    }
}

#[derive(Debug, Default)]
pub struct Comment {
    pub children: Value<BlockChildren>,
    pub metadata: Value<CommentMetadata>,
    pub condition: Condition,
}
impl Comment {
    pub fn new(children: impl Into<Value<BlockChildren>>) -> Self {
        Self {
            children: children.into(),
            ..Self::default()
        }
    }
    pub fn with_metadata(mut self, metadata: impl Into<Value<CommentMetadata>>) -> Self {
        self.metadata = metadata.into();
        self
    }
    pub fn when(mut self, condition: impl Into<Condition>) -> Self {
        self.condition = condition.into();
        self
    }
}
