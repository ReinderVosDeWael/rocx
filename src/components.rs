use crate::model::{ResolvedInlineKind, ResolvedTextRun};
use crate::*;

#[derive(Debug, Default)]
pub struct Tab {
    pub condition: Condition,
}
#[derive(Debug)]
pub struct Break {
    pub r#type: BreakType,
    pub condition: Condition,
}
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
    pub transformation: Option<ImageTransformation>,
    pub alt_text: Option<AltText>,
    pub condition: Condition,
}
impl ImageRun {
    pub fn new(data: impl Into<ImageData>) -> Self {
        Self {
            data: Value::ready(data.into()),
            r#type: None,
            transformation: None,
            alt_text: None,
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
    pub(crate) async fn resolve_at(self, path: String) -> Result<Option<ResolvedTextRun>> {
        if !self.condition.evaluate() {
            return Ok(None);
        }
        let data = self
            .data
            .resolve()
            .await
            .map_err(|e| e.at(format!("{path}.data")))?;
        let (bytes, name) = match data {
            ImageData::Bytes(b) => (b, None),
            ImageData::Path(p) => {
                let b = std::fs::read(&p).map_err(|e| Error::from(e).at(format!("{path}.data")))?;
                (b, p.file_name().map(|s| s.to_string_lossy().into_owned()))
            }
        };
        let image = crate::image::resolve(bytes, name, self.transformation, self.alt_text)
            .map_err(|e| e.at(path))?;
        Ok(Some(ResolvedTextRun {
            kind: ResolvedInlineKind::Image(image),
            ..ResolvedTextRun::default()
        }))
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedImage {
    pub(crate) data: Vec<u8>,
    pub(crate) extension: String,
    pub(crate) content_type: String,
    pub(crate) width: i64,
    pub(crate) height: i64,
    pub(crate) name: String,
    pub(crate) alt_text: Option<AltText>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedComment {
    pub text: String,
    pub author: Option<String>,
}
pub(crate) async fn resolve_comment(
    text: Option<Value<String>>,
    author: Option<Value<String>>,
    path: &str,
) -> Result<Option<ResolvedComment>> {
    let Some(text) = text else { return Ok(None) };
    let text_work = async {
        text.resolve()
            .await
            .map_err(|e| e.at(format!("{path}.comment_text")))
    };
    let author_work = async {
        match author {
            Some(a) => Ok(Some(
                a.resolve()
                    .await
                    .map_err(|e| e.at(format!("{path}.comment_author")))?,
            )),
            None => Ok::<_, Error>(None),
        }
    };
    let (text, author) = futures_util::future::try_join(text_work, author_work).await?;
    crate::render::validate_text(&text).map_err(|e| e.at(format!("{path}.comment_text")))?;
    if let Some(a) = &author {
        crate::render::validate_text(a).map_err(|e| e.at(format!("{path}.comment_author")))?;
    }
    Ok((!text.is_empty()).then_some(ResolvedComment { text, author }))
}
#[derive(Debug, Default)]
pub struct Header {
    pub children: Value<BlockChildren>,
    pub condition: Condition,
}
#[derive(Debug, Default)]
pub struct Footer {
    pub children: Value<BlockChildren>,
    pub condition: Condition,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedHeaderFooter {
    pub(crate) blocks: Vec<ResolvedBlock>,
}
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
            pub(crate) async fn resolve_at(
                self,
                path: String,
            ) -> Result<Option<ResolvedHeaderFooter>> {
                if !self.condition.evaluate() {
                    return Ok(None);
                }
                let path = format!("{path}.children");
                let children = self.children.resolve().await.map_err(|e| e.at(&path))?;
                Ok(Some(ResolvedHeaderFooter {
                    blocks: crate::table::resolve_blocks(children, path).await?,
                }))
            }
        }
    };
}
hf!(Header);
hf!(Footer);

impl ResolvedImage {
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    pub fn dimensions_emu(&self) -> (i64, i64) {
        (self.width, self.height)
    }
    pub fn content_type(&self) -> &str {
        &self.content_type
    }
}
impl ResolvedHeaderFooter {
    pub fn blocks(&self) -> &[ResolvedBlock] {
        &self.blocks
    }
}

impl Tab {
    pub(crate) async fn resolve_at(self, _path: String) -> Result<Option<ResolvedTextRun>> {
        Ok(self.condition.evaluate().then(|| ResolvedTextRun {
            text: "\t".into(),
            kind: ResolvedInlineKind::Tab,
            ..ResolvedTextRun::default()
        }))
    }
}
impl Break {
    pub(crate) async fn resolve_at(self, _path: String) -> Result<Option<ResolvedTextRun>> {
        Ok(self.condition.evaluate().then(|| ResolvedTextRun {
            text: if matches!(self.r#type, BreakType::Line | BreakType::TextWrapping) {
                "\n".into()
            } else {
                String::new()
            },
            kind: ResolvedInlineKind::Break(self.r#type),
            ..ResolvedTextRun::default()
        }))
    }
}
