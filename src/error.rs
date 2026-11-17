use std::{error, fmt, io};

/// Errors from deferred content, XML validation, packaging, or file output.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    Producer(Box<dyn error::Error + Send + Sync>),
    At { path: String, source: Box<Error> },
    InvalidXmlCharacter { character: char, byte_offset: usize },
    InvalidParagraphContent,
    Validation(String),
    Io(io::Error),
    Zip(zip::result::ZipError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Wrap an application error from a deferred producer.
    pub fn producer(error: impl error::Error + Send + Sync + 'static) -> Self {
        Self::Producer(Box::new(error))
    }

    pub(crate) fn at(self, path: impl Into<String>) -> Self {
        Self::At {
            path: path.into(),
            source: Box::new(self),
        }
    }

    /// The declaration field that failed, when the error arose in the tree.
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::At { path, .. } => Some(path),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => f.write_str(message),
            Self::Producer(source) => write!(f, "content producer failed: {source}"),
            Self::At { path, source } => write!(f, "{path}: {source}"),
            Self::InvalidXmlCharacter {
                character,
                byte_offset,
            } => write!(
                f,
                "invalid XML 1.0 character U+{:04X} at byte offset {byte_offset}",
                *character as u32
            ),
            Self::Io(source) => write!(f, "document I/O failed: {source}"),
            Self::Zip(source) => write!(f, "DOCX packaging failed: {source}"),
            Self::InvalidParagraphContent => {
                write!(f, "paragraph requires exactly one of text or children")
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(match self {
            Self::Producer(source) => source.as_ref(),
            Self::At { source, .. } => source.as_ref(),
            Self::Io(source) => source,
            Self::Zip(source) => source,
            Self::InvalidXmlCharacter { .. }
            | Self::InvalidParagraphContent
            | Self::Validation(_) => return None,
        })
    }
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Self::Io(source)
    }
}
impl From<zip::result::ZipError> for Error {
    fn from(source: zip::result::ZipError) -> Self {
        Self::Zip(source)
    }
}
