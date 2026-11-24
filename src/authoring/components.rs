//! Inline, image, comment, header, and footer declarations and resolved values.
//!
//! References: ECMA-376 Part 1 (2016), §§17.10 (headers/footers), 17.13.4
//! (comments), 20.2 (pictures), and 20.4 (WordprocessingML drawings).
//! [Official Part 1](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip).

use crate::document::setters;
use crate::{AltText, BlockChildren, BreakType, Condition, ImageTransformation, Inline, Value};

#[derive(Debug, Default)]
pub struct Tab {
    pub condition: Condition,
}
#[derive(Debug)]
pub struct Break {
    pub r#type: BreakType,
    pub condition: Condition,
}
setters!(Break { with_type => r#type: value<BreakType> });
impl Default for Break {
    fn default() -> Self {
        Self {
            r#type: BreakType::Line,
            condition: Condition::default(),
        }
    }
}
impl Tab {
    pub fn when(mut self, c: impl Into<Condition>) -> Self {
        self.condition = c.into();
        self
    }
}
impl Break {
    pub fn new(kind: BreakType) -> Self {
        Self {
            r#type: kind,
            ..Self::default()
        }
    }
    pub fn when(mut self, c: impl Into<Condition>) -> Self {
        self.condition = c.into();
        self
    }
}
macro_rules! inline_from {
    ($t:ty,$v:ident) => {
        impl From<$t> for Inline {
            fn from(v: $t) -> Self {
                Self::$v(v)
            }
        }
        impl From<$t> for Value<Inline> {
            fn from(v: $t) -> Self {
                Self::Ready(Inline::$v(v))
            }
        }
    };
}
inline_from!(Tab, Tab);
inline_from!(Break, Break);
inline_from!(ImageRun, ImageRun);
#[derive(Debug, Clone, PartialEq)]
pub enum ImageData {
    Bytes(Vec<u8>),
    Path(std::path::PathBuf),
    /// Encoded bytes loaded by a host, with the source name (typically a basename).
    /// The resolver does not use this name to read a file or identify the format.
    NamedBytes {
        bytes: Vec<u8>,
        source_name: String,
    },
}
impl From<Vec<u8>> for ImageData {
    fn from(v: Vec<u8>) -> Self {
        Self::Bytes(v)
    }
}
impl From<std::path::PathBuf> for ImageData {
    fn from(v: std::path::PathBuf) -> Self {
        Self::Path(v)
    }
}
impl From<&str> for ImageData {
    fn from(v: &str) -> Self {
        Self::Path(v.into())
    }
}
#[derive(Debug)]
pub struct ImageRun {
    pub data: Value<ImageData>,
    pub r#type: Option<String>,
    /// Optional dimensions in points, resolved concurrently with data and alt text.
    pub transformation: Value<Option<ImageTransformation>>,
    /// Optional accessibility metadata, using the same property-reference mechanism.
    pub alt_text: Value<Option<AltText>>,
    pub condition: Condition,
}
setters!(ImageRun {
    with_data => data: into<Value<ImageData>>,
    with_type => r#type: optional_into<String>,
    with_transformation => transformation: into<Value<Option<ImageTransformation>>>,
    with_alt_text => alt_text: into<Value<Option<AltText>>>,
});
impl ImageRun {
    pub fn new(data: impl Into<ImageData>) -> Self {
        Self {
            data: Value::ready(data.into()),
            r#type: None,
            transformation: Value::default(),
            alt_text: Value::default(),
            condition: Condition::default(),
        }
    }
    pub fn from_path(path: impl Into<std::path::PathBuf>) -> Self {
        Self::new(ImageData::Path(path.into()))
    }
    pub fn deferred(data: Value<ImageData>) -> Self {
        Self {
            data,
            ..Self::new(Vec::new())
        }
    }
    pub fn when(mut self, c: impl Into<Condition>) -> Self {
        self.condition = c.into();
        self
    }
}

#[derive(Debug, Default)]
pub struct Header {
    pub children: Value<BlockChildren>,
    pub condition: Condition,
}
setters!(Header { with_children => children: into<Value<BlockChildren>> });
#[derive(Debug, Default)]
pub struct Footer {
    pub children: Value<BlockChildren>,
    pub condition: Condition,
}
setters!(Footer { with_children => children: into<Value<BlockChildren>> });
macro_rules! hf {
    ($t:ty) => {
        impl $t {
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
    };
}
hf!(Header);
hf!(Footer);
