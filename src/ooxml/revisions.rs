//! Run changes, P1 (2016) §17.13.5 and Annex A.1 CT_RunTrackChange, CT_MoveBookmark.
use super::Compiler;
use crate::opc::Package;
use crate::xml::{Node, validate_text};
use crate::{Diagnostic, Result, RevisionKind, Rule, TextRevision};
use std::collections::BTreeSet;

pub(crate) struct RevisionIds {
    used: BTreeSet<u32>,
    imported_moves: Option<BTreeSet<String>>,
    last: u32,
}

impl RevisionIds {
    pub(crate) fn property_change(&mut self, revision: &TextRevision) -> Result<Node> {
        Ok(annotate("w:rPrChange", self.next()?, revision))
    }

    pub(crate) fn paragraph_mark(&mut self, revision: &TextRevision) -> Result<Node> {
        Ok(annotate(
            match revision.kind {
                RevisionKind::Insertion => "w:ins",
                RevisionKind::Deletion => "w:del",
                _ => unreachable!("paragraph insert/delete"),
            },
            self.next()?,
            revision,
        ))
    }

    pub(crate) fn reserved(used: BTreeSet<u32>, move_names: BTreeSet<String>) -> Self {
        Self {
            used,
            imported_moves: Some(move_names),
            last: 0,
        }
    }

    fn from_package(package: &Package) -> Self {
        let used = super::defaults::imported_word_values(package, |node| {
            node.get("w:id")?.trim().parse().ok()
        });
        Self {
            used,
            imported_moves: None,
            last: 0,
        }
    }

    fn next(&mut self) -> Result<u32> {
        loop {
            self.last = self.last.checked_add(1).ok_or_else(|| {
                Diagnostic::new(Rule::InvalidContent, "text revision IDs exhausted")
            })?;
            if !self.used.contains(&self.last) {
                return Ok(self.last);
            }
        }
    }
}

fn annotate(name: &str, id: u32, revision: &TextRevision) -> Node {
    let mut node = Node::new(name)
        .attr("w:id", id)
        .attr("w:author", &revision.author);
    if let Some(date) = &revision.date {
        node.set("w:date", date);
    }
    node
}

impl Compiler<'_> {
    pub(super) fn wrap_revision(
        &mut self,
        run: Node,
        revision: &TextRevision,
    ) -> Result<Vec<Node>> {
        validate_text(&revision.author).map_err(|error| error.at("revision.author"))?;
        let ids = self
            .revisions
            .get_or_insert_with(|| RevisionIds::from_package(&self.package));
        ids.wrap(&self.package, run, revision)
    }
}

impl RevisionIds {
    pub(crate) fn wrap(
        &mut self,
        package: &Package,
        run: Node,
        revision: &TextRevision,
    ) -> Result<Vec<Node>> {
        let (tag, start_tag, end_tag) = match &revision.kind {
            RevisionKind::Insertion => {
                return Ok(vec![annotate("w:ins", self.next()?, revision).with(run)]);
            }
            RevisionKind::Deletion => {
                return Ok(vec![annotate("w:del", self.next()?, revision).with(run)]);
            }
            RevisionKind::MoveSource(_) => {
                ("w:moveFrom", "w:moveFromRangeStart", "w:moveFromRangeEnd")
            }
            RevisionKind::MoveDestination(_) => {
                ("w:moveTo", "w:moveToRangeStart", "w:moveToRangeEnd")
            }
        };
        let name = revision.kind.move_name().expect("move revision").as_str();
        let imported = self.imported_moves.get_or_insert_with(|| {
            super::defaults::imported_word_values(package, |node| {
                if matches!(
                    node.name.as_str(),
                    "w:moveFromRangeStart" | "w:moveToRangeStart"
                ) {
                    node.get("w:name").map(str::to_owned)
                } else {
                    None
                }
            })
        });
        if imported.contains(name) {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "text move name already exists in template",
            )
            .at("revision.move_name")
            .into());
        }
        let range = self.next()?;
        Ok(vec![
            annotate(start_tag, range, revision).attr("w:name", name),
            annotate(tag, self.next()?, revision).with(run),
            Node::new(end_tag).attr("w:id", range),
        ])
    }
}
