//! Paired anchors, not nested containers. P1 (2016) §§17.13.4, 17.13.6.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// Stable authoring identity; never a package's numeric bookmark/comment ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RangeId(String);
impl RangeId {
    pub fn new(value: impl Into<String>) -> Result<Self, Diagnostic> {
        let value = value.into();
        if value.is_empty() {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "range identity cannot be empty",
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Word-compatible authoring policy: 1–40 ASCII letters/digits/underscores,
/// beginning with a letter or underscore. This is narrower than ST_String.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BookmarkName(String);
impl BookmarkName {
    pub fn new(value: impl Into<String>) -> Result<Self, Diagnostic> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 40
            || !value.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            || !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "invalid bookmark name (Word-compatible 1–40 character policy)",
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn range_id(&self) -> RangeId {
        RangeId(format!("bookmark:{}", self.0))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RangeKind {
    Bookmark(BookmarkName),
    Comment(ResolvedComment),
    StructuredComment(ResolvedCommentBody),
}

/// Flatten supported domain content in story order, retaining structural paths.
/// The body spans sections; each explicit header/footer is its own logical story.
pub(crate) fn story_inlines(
    doc: &ResolvedDocument,
) -> Vec<(StoryKind, Vec<(&ResolvedInline, String)>)> {
    fn inlines<'a>(
        runs: &'a [ResolvedInline],
        path: &str,
        out: &mut Vec<(&'a ResolvedInline, String)>,
    ) {
        for (i, run) in runs.iter().enumerate() {
            let path = format!("{path}[{i}]");
            out.push((run, path.clone()));
            if let ResolvedInline::Hyperlink(link) = run {
                inlines(link.children(), &format!("{path}.children"), out);
            }
            if let ResolvedInline::Field(field) = run {
                inlines(field.children(), &format!("{path}.children"), out);
            }
        }
    }
    fn blocks<'a>(
        values: &'a [ResolvedBlock],
        path: &str,
        out: &mut Vec<(&'a ResolvedInline, String)>,
    ) {
        for (i, block) in values.iter().enumerate() {
            let path = format!("{path}[{i}]");
            match block {
                ResolvedBlock::Paragraph(p) => inlines(p.runs(), &format!("{path}.children"), out),
                ResolvedBlock::Table(t) => {
                    for (r, row) in t.rows().iter().enumerate() {
                        for (c, cell) in row.cells().iter().enumerate() {
                            blocks(
                                cell.blocks(),
                                &format!("{path}.rows[{r}].children[{c}].children"),
                                out,
                            );
                        }
                    }
                }
            }
        }
    }
    let mut stories = vec![(StoryKind::Body, Vec::new())];
    for (i, section) in doc.sections().iter().enumerate() {
        blocks(
            section.blocks(),
            &format!("sections[{i}].children"),
            &mut stories[0].1,
        );
        for (name, kind, values) in [
            ("headers", StoryKind::Header, section.headers()),
            ("footers", StoryKind::Footer, section.footers()),
        ] {
            for (typ, story) in values {
                let mut runs = Vec::new();
                blocks(
                    story.blocks(),
                    &format!("sections[{i}].{name}.{typ:?}.children"),
                    &mut runs,
                );
                stories.push((kind, runs));
            }
        }
    }
    for (i, note) in doc.notes().iter().enumerate() {
        let mut runs = Vec::new();
        blocks(note.blocks(), &format!("notes[{i}].children"), &mut runs);
        stories.push((
            match note.target().kind {
                NoteKind::Footnote => StoryKind::Footnote,
                NoteKind::Endnote => StoryKind::Endnote,
            },
            runs,
        ));
    }
    let mut index = 0;
    while index < stories.len() {
        let comments: Vec<_> = stories[index]
            .1
            .iter()
            .filter_map(|(run, path)| match *run {
                ResolvedInline::RangeStart {
                    kind: RangeKind::StructuredComment(body),
                    ..
                } => Some((body, path.clone())),
                _ => None,
            })
            .collect();
        for (body, path) in comments {
            let mut runs = Vec::new();
            blocks(
                body.blocks(),
                &format!("{path}.content.children"),
                &mut runs,
            );
            stories.push((StoryKind::Comment, runs));
        }
        index += 1;
    }
    stories
}

pub(crate) fn validate_ranges(doc: &ResolvedDocument) -> Result<(), Diagnostic> {
    let mut identities = BTreeSet::new();
    let mut names = BTreeSet::new();
    for (story_kind, runs) in story_inlines(doc) {
        let mut active = BTreeMap::new();
        let mut position = 0usize;
        for (run, path) in runs {
            match run {
                ResolvedInline::RangeStart { id, kind } => {
                    if !identities.insert(id.clone()) {
                        return Err(Diagnostic::new(
                            Rule::InvalidContent,
                            "duplicate range identity",
                        )
                        .at(path));
                    }
                    match kind {
                        RangeKind::Bookmark(name) if !names.insert(name.clone()) => {
                            return Err(Diagnostic::new(
                                Rule::InvalidContent,
                                "duplicate bookmark name",
                            )
                            .at(path));
                        }
                        RangeKind::Comment(_) | RangeKind::StructuredComment(_)
                            if story_kind != StoryKind::Body =>
                        {
                            return Err(Diagnostic::new(
                                Rule::InvalidContent,
                                "comment ranges are supported only in the body story",
                            )
                            .at(path));
                        }
                        _ => (),
                    }
                    active.insert(id.clone(), (kind, position, path));
                }
                ResolvedInline::RangeEnd(id) => {
                    let Some((kind, start, _)) = active.remove(id) else {
                        return Err(Diagnostic::new(
                            Rule::InvalidContent,
                            "range end has no preceding start in this story",
                        )
                        .at(path));
                    };
                    if matches!(
                        kind,
                        RangeKind::Comment(_) | RangeKind::StructuredComment(_)
                    ) && start == position
                    {
                        return Err(Diagnostic::new(
                            Rule::InvalidContent,
                            "comment range must contain a run",
                        )
                        .at(path));
                    }
                }
                ResolvedInline::Hyperlink(_) | ResolvedInline::Field(_) => (),
                _ => position += 1,
            }
        }
        if let Some((_, (_, _, path))) = active.into_iter().next() {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "range start has no end in this story",
            )
            .at(path));
        }
    }
    Ok(())
}
