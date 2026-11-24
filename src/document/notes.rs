//! Note and structured-comment story values; P1 (2016) §§17.11, 17.13.4.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FootnotePosition {
    PageBottom,
    BeneathText,
}
impl FootnotePosition {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::PageBottom => "pageBottom",
            Self::BeneathText => "beneathText",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndnotePosition {
    SectionEnd,
    DocumentEnd,
}
impl EndnotePosition {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::SectionEnd => "sectEnd",
            Self::DocumentEnd => "docEnd",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteNumberRestart {
    Continuous,
    EachSection,
    EachPage,
}
impl NoteNumberRestart {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::Continuous => "continuous",
            Self::EachSection => "eachSect",
            Self::EachPage => "eachPage",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteNumberFormat {
    Decimal,
    LowerRoman,
    UpperRoman,
    LowerLetter,
    UpperLetter,
}
impl NoteNumberFormat {
    pub const fn xml(self) -> &'static str {
        match self {
            Self::Decimal => "decimal",
            Self::LowerRoman => "lowerRoman",
            Self::UpperRoman => "upperRoman",
            Self::LowerLetter => "lowerLetter",
            Self::UpperLetter => "upperLetter",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NoteNumbering {
    pub format: Option<NoteNumberFormat>,
    pub start: Option<u32>,
    pub restart: Option<NoteNumberRestart>,
}
setters!(NoteNumbering {
    with_format => format: optional<NoteNumberFormat>,
    with_start => start: optional<u32>,
    with_restart => restart: optional<NoteNumberRestart>,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoteKind {
    Footnote,
    Endnote,
}
impl NoteKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Footnote => "footnote",
            Self::Endnote => "endnote",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NoteTarget {
    pub kind: NoteKind,
    pub id: NoteId,
}
setters!(NoteTarget {
    with_kind => kind: value<NoteKind>,
    with_id => id: value<NoteId>,
});
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedNote {
    pub(crate) target: NoteTarget,
    pub(crate) blocks: Vec<ResolvedBlock>,
}
impl ResolvedNote {
    pub fn target(&self) -> NoteTarget {
        self.target
    }
    pub fn blocks(&self) -> &[ResolvedBlock] {
        &self.blocks
    }
}

/// Deliberate base-comment metadata; absent date uses the captured snapshot time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommentMetadata {
    pub author: Option<String>,
    pub date: Option<String>,
    pub initials: Option<String>,
}
setters!(CommentMetadata {
    with_author => author: optional_into<String>,
    with_date => date: optional_into<String>,
    with_initials => initials: optional_into<String>,
});
impl CommentMetadata {
    /// Supplied dates use the supported UTC-seconds subset, not arbitrary XSD dates.
    pub fn validate(&self) -> Result<(), Diagnostic> {
        let Some(date) = &self.date else {
            return Ok(());
        };
        validate_utc_date(date, "comment")
    }
}

pub(crate) fn validate_utc_date(date: &str, owner: &str) -> Result<(), Diagnostic> {
    let invalid = || {
        Diagnostic::new(
            Rule::InvalidContent,
            format!("{owner} date must be a valid YYYY-MM-DDTHH:MM:SSZ UTC timestamp"),
        )
        .at("date")
    };
    let b = date.as_bytes();
    if b.len() != 20
        || !b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            10 => *c == b'T',
            13 | 16 => *c == b':',
            19 => *c == b'Z',
            _ => c.is_ascii_digit(),
        })
    {
        return Err(invalid());
    }
    let number = |bytes: &[u8]| {
        bytes
            .iter()
            .fold(0u32, |value, c| value * 10 + u32::from(c - b'0'))
    };
    let year = number(&b[0..4]);
    let month = number(&b[5..7]);
    let day = number(&b[8..10]);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0
        || day == 0
        || day > days
        || number(&b[11..13]) > 23
        || number(&b[14..16]) > 59
        || number(&b[17..19]) > 59
    {
        return Err(invalid());
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCommentBody {
    pub(crate) blocks: Vec<ResolvedBlock>,
    pub(crate) metadata: CommentMetadata,
}
impl ResolvedCommentBody {
    pub fn blocks(&self) -> &[ResolvedBlock] {
        &self.blocks
    }
    pub fn metadata(&self) -> &CommentMetadata {
        &self.metadata
    }
}
