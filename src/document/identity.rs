//! Domain identities; package relationship IDs belong to OPC instead.
use super::{Diagnostic, Rule};
macro_rules! identity {
    ($($name:ident),+)=>{$(
        #[derive(Clone,Copy,Debug,Default,PartialEq,Eq,PartialOrd,Ord,Hash)]
        pub struct $name(u32);
        impl $name {
            pub const fn new(value:u32)->Self{Self(value)}
            pub const fn get(self)->u32{self.0}
            pub fn next(self)->Result<Self,Diagnostic>{self.0.checked_add(1).map(Self).ok_or_else(||Diagnostic::new(Rule::InvalidContent,concat!(stringify!($name)," allocation overflow")))}
        }
        impl std::fmt::Display for $name {fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{self.0.fmt(f)}}
    )+};
}
identity!(StoryId, AssetId, CommentId, DrawingId, NoteId);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StoryKind {
    Body,
    Header,
    Footer,
    Comment,
    Footnote,
    Endnote,
    TextBox,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StyleId(String);
impl StyleId {
    pub fn new(value: impl Into<String>) -> Result<Self, Diagnostic> {
        let value = value.into();
        if value.is_empty() {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "style ID cannot be empty",
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for StyleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
