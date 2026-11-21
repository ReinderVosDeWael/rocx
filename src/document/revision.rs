//! Text-run tracked changes, P1 (2016) §17.13.5.
use super::{BookmarkName, Diagnostic, ResolvedInline, Rule, StoryKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevisionKind {
    Insertion,
    Deletion,
    MoveSource(BookmarkName),
    MoveDestination(BookmarkName),
}
impl RevisionKind {
    pub fn move_name(&self) -> Option<&BookmarkName> {
        match self {
            Self::MoveSource(name) | Self::MoveDestination(name) => Some(name),
            Self::Insertion | Self::Deletion => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRevision {
    pub kind: RevisionKind,
    pub author: String,
    pub date: Option<String>,
}
setters!(TextRevision {
    with_kind => kind: value<RevisionKind>,
    with_author => author: into<String>,
    with_date => date: optional_into<String>,
});

impl TextRevision {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.author.trim().is_empty() {
            return Err(
                Diagnostic::new(Rule::InvalidContent, "revision author cannot be empty")
                    .at("author"),
            );
        }
        if self.kind.move_name().is_some() && self.date.is_none() {
            return Err(
                Diagnostic::new(Rule::InvalidContent, "text move requires a date").at("date"),
            );
        }
        if let Some(date) = &self.date {
            super::notes::validate_utc_date(date, "revision")?;
        }
        Ok(())
    }
}

pub(super) fn validate_moves(
    stories: &[(StoryKind, Vec<(&ResolvedInline, String)>)],
) -> Result<(), Diagnostic> {
    type MovedRun<'a> = (&'a str, &'a TextRevision, String);
    let mut moves: std::collections::BTreeMap<&BookmarkName, (Option<MovedRun>, Option<MovedRun>)> =
        std::collections::BTreeMap::new();
    for (story, runs) in stories {
        for (run, path) in runs {
            let ResolvedInline::Text {
                revision: Some(revision),
                text,
                comment,
                ..
            } = run
            else {
                continue;
            };
            let Some(name) = revision.kind.move_name() else {
                continue;
            };
            if *story != StoryKind::Body || comment.is_some() || text.is_empty() {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "text moves require nonempty body text without run-level comments",
                )
                .at(path));
            }
            let (source, destination) = moves.entry(name).or_default();
            let slot = match revision.kind {
                RevisionKind::MoveSource(_) => source,
                RevisionKind::MoveDestination(_) => destination,
                _ => unreachable!("move name is present"),
            };
            if slot.is_some() {
                return Err(Diagnostic::new(
                    Rule::InvalidContent,
                    "text move must have one source and one destination",
                )
                .at(path));
            }
            *slot = Some((text, revision, path.clone()));
        }
    }
    for (name, (source, destination)) in moves {
        let (Some((source, from, _)), Some((destination, to, path))) = (source, destination) else {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                format!(
                    "text move {:?} must have one source and one destination",
                    name.as_str()
                ),
            ));
        };
        if source != destination || from.author != to.author || from.date != to.date {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "text move source and destination must have identical text, author and date",
            )
            .at(path));
        }
    }
    Ok(())
}
