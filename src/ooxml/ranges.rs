//! Word range allocation/anchors. P1 (2016) §§17.13.4.3–5, 17.13.6.1–2.
use super::{Compiler, PartContext};
use crate::document::story_inlines;
use crate::opc::Package;
use crate::xml::{Node, val};
use crate::{
    BookmarkName, CommentId, Diagnostic, RangeId, RangeKind, ResolvedDocument, ResolvedInline,
    Result, Rule, StoryId,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct RangeCatalog {
    next_bookmark: u64,
    next_comment: u64,
    active: BTreeMap<RangeId, (StoryId, WireRange)>,
}
#[derive(Clone)]
pub(super) enum WireRange {
    Bookmark { id: u32, name: BookmarkName },
    Comment(CommentId),
}
impl WireRange {
    pub fn start(&self) -> Node {
        match self {
            Self::Bookmark { id, name } => Node::new("w:bookmarkStart")
                .attr("w:id", id)
                .attr("w:name", name.as_str()),
            Self::Comment(id) => Node::new("w:commentRangeStart").attr("w:id", id),
        }
    }
    pub fn end(&self) -> Vec<Node> {
        match self {
            Self::Bookmark { id, .. } => vec![Node::new("w:bookmarkEnd").attr("w:id", id)],
            Self::Comment(id) => vec![
                Node::new("w:commentRangeEnd").attr("w:id", id),
                Node::new("w:r")
                    .with(Node::new("w:rPr").with(val("rStyle", "CommentReference")))
                    .with(Node::new("w:commentReference").attr("w:id", id)),
            ],
        }
    }
}
impl RangeCatalog {
    pub fn new(package: &Package, main: &str, document: &ResolvedDocument) -> Result<Self> {
        fn scan(
            node: &Node,
            names: &mut BTreeMap<String, usize>,
            bookmarks: &mut Option<u64>,
            comments: &mut Option<u64>,
        ) {
            if node.name == "w:bookmarkStart"
                && let Some(name) = node.get("w:name")
            {
                *names.entry(name.into()).or_default() += 1;
            }
            let counter = match node.name.as_str() {
                "w:bookmarkStart" | "w:bookmarkEnd" => Some(&mut *bookmarks),
                "w:comment"
                | "w:commentRangeStart"
                | "w:commentRangeEnd"
                | "w:commentReference" => Some(&mut *comments),
                _ => None,
            };
            if let Some(counter) = counter
                && let Some(id) = node
                    .get("w:id")
                    .and_then(|id| id.trim().parse::<i128>().ok())
                    .and_then(|id| u64::try_from(id).ok())
            {
                *counter = Some(counter.map_or(id, |old| old.max(id)));
            }
            for child in node.elements() {
                scan(child, names, bookmarks, comments);
            }
        }
        let mut parts = BTreeSet::from([main.to_owned()]);
        for kind in ["header", "footer", "comments", "footnotes", "endnotes"] {
            for (id, _) in package.relationships(main, kind)? {
                if let Some((_, part)) = package.related(main, &format!("#{id}"))? {
                    parts.insert(part);
                }
            }
        }
        let mut names = BTreeMap::new();
        let (mut bookmarks, mut comments) = (None, None);
        for part in parts {
            scan(
                &package.xml(&part)?,
                &mut names,
                &mut bookmarks,
                &mut comments,
            );
        }
        let stories = story_inlines(document);
        for (_, runs) in &stories {
            for (run, path) in runs {
                if let ResolvedInline::RangeStart {
                    kind: RangeKind::Bookmark(name),
                    ..
                } = run
                {
                    if names.contains_key(name.as_str()) {
                        return Err(Diagnostic::new(
                            Rule::InvalidContent,
                            "bookmark name collides with an existing bookmark",
                        )
                        .at(path)
                        .into());
                    }
                    names.insert(name.as_str().into(), 1);
                }
            }
        }
        for (_, runs) in &stories {
            for (run, path) in runs {
                let target = match run {
                    ResolvedInline::Hyperlink(link) => link.target().bookmark_name(),
                    ResolvedInline::Field(field) => field.instruction().bookmark_name(),
                    _ => None,
                };
                if let Some(name) = target
                    && names.get(name.as_str()) != Some(&1)
                {
                    return Err(Diagnostic::new(
                        Rule::InvalidRelationship,
                        "internal reference bookmark is missing or ambiguous",
                    )
                    .at(format!("{path}.target"))
                    .into());
                }
            }
        }
        // Preserve high imported IDs when no new IDs are needed. The generated
        // 32-bit allocation limit is checked only at an actual allocation.
        fn next(value: Option<u64>) -> u64 {
            value.map_or(0, |v| v.saturating_add(1))
        }
        Ok(Self {
            next_bookmark: next(bookmarks),
            next_comment: next(comments),
            active: BTreeMap::new(),
        })
    }
    pub fn comment_id(&mut self) -> Result<CommentId> {
        let id = CommentId::new(u32::try_from(self.next_comment).map_err(|_| {
            Diagnostic::new(Rule::InvalidContent, "comment ID allocation overflow")
        })?);
        self.next_comment += 1;
        Ok(id)
    }
}
impl Compiler<'_> {
    pub(super) fn allocate_range(
        &mut self,
        kind: &RangeKind,
        context: &PartContext,
    ) -> Result<WireRange> {
        match kind {
            RangeKind::Bookmark(name) => {
                let id = u32::try_from(self.ranges.next_bookmark).map_err(|_| {
                    Diagnostic::new(Rule::InvalidContent, "bookmark ID allocation overflow")
                })?;
                self.ranges.next_bookmark += 1;
                Ok(WireRange::Bookmark {
                    id,
                    name: name.clone(),
                })
            }
            RangeKind::Comment(comment) => {
                Ok(WireRange::Comment(self.add_comment(comment, context)?))
            }
            RangeKind::StructuredComment(body) => Ok(WireRange::Comment(
                self.add_structured_comment(body, context)?,
            )),
        }
    }
    pub(super) fn range_start(
        &mut self,
        id: &RangeId,
        kind: &RangeKind,
        context: &PartContext,
    ) -> Result<Node> {
        let anchor = self.allocate_range(kind, context)?;
        let node = anchor.start();
        self.ranges
            .active
            .insert(id.clone(), (context.story, anchor));
        Ok(node)
    }
    pub(super) fn range_end(&mut self, id: &RangeId, context: &PartContext) -> Result<Vec<Node>> {
        let Some((story, anchor)) = self.ranges.active.remove(id) else {
            return Err(
                Diagnostic::new(Rule::InvalidContent, "range end has no emitted start")
                    .at(context.source.as_str())
                    .into(),
            );
        };
        if story != context.story {
            return Err(Diagnostic::new(
                Rule::InvalidContent,
                "range endpoints belong to different stories",
            )
            .at(context.source.as_str())
            .into());
        }
        Ok(anchor.end())
    }
}
