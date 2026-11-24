//! Set, move, or remove both endpoints using original physical addresses.
use super::{
    BoundaryState, EntryKind, ImportedDocument, InlineAnchor, InlineLocation, text, unsupported,
};
use crate::{
    BookmarkName, Result,
    ooxml::{WordParts, decimal_id},
    opc::Package,
    xml::Node,
};

struct Scan<'a> {
    tags: [&'static str; 3],
    id: String,
    name: Option<&'a str>,
    counts: [usize; 3],
    referenced: bool,
}
impl Scan<'_> {
    fn marker(&self, n: &Node) -> Option<usize> {
        self.tags.iter().position(|&tag| {
            n.name == tag && n.get("w:id").and_then(decimal_id).as_ref() == Some(&self.id)
        })
    }
    fn scan(&mut self, n: &Node) -> Result<()> {
        if let Some(i) = self.marker(n) {
            self.counts[i] += 1;
        }
        if let Some(name) = self.name {
            if n.name == "w:bookmarkStart"
                && n.get("w:name")
                    .is_some_and(|s| s.eq_ignore_ascii_case(name))
                && self.marker(n) != Some(0)
            {
                return Err(unsupported("bookmark name already exists"));
            }
            // Instructions may be split across runs or indirect. Do not guess
            // whether a field might depend on a bookmark being removed.
            self.referenced |= n.name == "w:fldSimple"
                || n.name == "w:instrText"
                || n.get("w:anchor")
                    .is_some_and(|s| s.eq_ignore_ascii_case(name));
        }
        for c in n.elements() {
            self.scan(c)?;
        }
        Ok(())
    }
}
type Site = (usize, usize);
#[derive(Clone, Debug)]
pub struct PairedRangeEdit {
    pub id: u32,
    pub bookmark: Option<String>,
    pub endpoints: Option<(InlineAnchor, InlineAnchor)>,
}
/// Physical direct paragraph marker; ID spelling is retained even on an
/// ambiguous input. Inspection is not an assertion that the pair is editable.
#[derive(Debug)]
pub struct BodyRangeMarker {
    pub body_index: usize,
    pub inline_index: usize,
    pub kind: &'static str,
    pub id: Option<String>,
    pub name: Option<String>,
}
impl ImportedDocument {
    pub fn body_range_markers(&self) -> Result<Vec<BodyRangeMarker>> {
        let root = self.package.word(&self.main)?;
        let mut markers = Vec::new();
        for (body_index, p) in root
            .child("w:body")
            .into_iter()
            .flat_map(Node::elements)
            .filter(|n| n.name != "w:sectPr")
            .enumerate()
        {
            if p.name != "w:p" {
                continue;
            }
            for (inline_index, n) in p.elements().filter(|n| n.name != "w:pPr").enumerate() {
                let mut push = |kind: &'static str, n: &Node| {
                    markers.push(BodyRangeMarker {
                        body_index,
                        inline_index,
                        kind,
                        id: n.get("w:id").map(str::to_owned),
                        name: n.get("w:name").map(str::to_owned),
                    })
                };
                match n.name.as_str() {
                    "w:bookmarkStart" => push("bookmarkStart", n),
                    "w:bookmarkEnd" => push("bookmarkEnd", n),
                    "w:commentRangeStart" => push("commentRangeStart", n),
                    "w:commentRangeEnd" => push("commentRangeEnd", n),
                    "w:r" => {
                        for r in n.elements().filter(|r| r.name == "w:commentReference") {
                            push("commentReference", r);
                        }
                    }
                    _ => (),
                }
            }
        }
        Ok(markers)
    }
    fn paired_address(&self, a: &InlineAnchor) -> Result<Site> {
        self.validate_snapshot(&a.snapshot)?;
        match (&a.location, a.path.as_slice()) {
            (InlineLocation::Body(b), [i]) => Ok((*b, *i)),
            _ => Err(unsupported("range endpoint needs a direct body inline")),
        }
    }
    /// Wrap inclusive direct body run/link anchors, or move the identified pair
    /// there. No endpoints removes the pair. A name denotes a bookmark; otherwise
    /// the ID must identify exactly one existing comment definition.
    pub fn set_paired_range(
        &self,
        id: u32,
        bookmark: Option<&str>,
        endpoints: Option<(&InlineAnchor, &InlineAnchor)>,
    ) -> Result<Vec<u8>> {
        self.preview_edits(super::EditBatch {
            paired_ranges: &[PairedRangeEdit {
                id,
                bookmark: bookmark.map(str::to_owned),
                endpoints: endpoints.map(|(a, b)| (a.clone(), b.clone())),
            }],
            ..Default::default()
        })?
        .to_docx()
    }
    pub(super) fn plan_pair(
        &self,
        root: &Node,
        edit: &PairedRangeEdit,
        slots: &mut super::batch::Slots,
    ) -> Result<()> {
        let (id, bookmark, endpoints) =
            (edit.id, edit.bookmark.as_deref(), edit.endpoints.as_ref());
        let err = || unsupported("ambiguous, missing or unsupported range markers");
        let sites = endpoints
            .map(|(a, b)| Ok::<_, crate::Error>((self.paired_address(a)?, self.paired_address(b)?)))
            .transpose()?;
        if sites.is_some_and(|(a, b)| a > b) {
            return Err(unsupported("range endpoints are reversed"));
        }
        let id = id.to_string();
        let tags = if let Some(name) = bookmark {
            BookmarkName::new(name)?;
            ["w:bookmarkStart", "w:bookmarkEnd", ""]
        } else {
            if self
                .entry_stories()?
                .iter()
                .filter(|e| e.kind == EntryKind::Comment && decimal_id(&e.id).as_ref() == Some(&id))
                .count()
                != 1
            {
                return Err(unsupported("expected one existing comment definition"));
            }
            [
                "w:commentRangeStart",
                "w:commentRangeEnd",
                "w:commentReference",
            ]
        };
        let mut scan = Scan {
            tags,
            id: id.clone(),
            name: bookmark,
            counts: [0; 3],
            referenced: false,
        };
        scan.scan(root)?;
        for part in crate::ooxml::related_story_parts(&self.package, &self.main)? {
            scan.scan(&self.package.word(&part)?)?;
        }
        let expected = [1, 1, usize::from(bookmark.is_none())];
        let existing = scan.counts == expected;
        if !(existing || sites.is_some() && scan.counts == [0; 3]) {
            return Err(err());
        }
        if !existing {
            // A newly created pair must not invalidate an existing revision's
            // package-wide ID or move-name uniqueness check on later review.
            let mut collision = false;
            self.package.visit_source_xml(|n| {
                let move_start = matches!(
                    n.name.as_str(),
                    "w:moveFromRangeStart" | "w:moveToRangeStart"
                );
                let revision = move_start
                    || matches!(
                        n.name.as_str(),
                        "w:ins"
                            | "w:del"
                            | "w:moveFrom"
                            | "w:moveTo"
                            | "w:moveFromRangeEnd"
                            | "w:moveToRangeEnd"
                            | "w:rPrChange"
                    );
                collision |= revision && n.get("w:id").and_then(decimal_id).as_ref() == Some(&id)
                    || move_start
                        && bookmark.is_some_and(|name| {
                            n.get("w:name")
                                .is_some_and(|s| s.eq_ignore_ascii_case(name))
                        });
            })?;
            if collision {
                return Err(unsupported(
                    "range ID or bookmark name collides with a tracked revision",
                ));
            }
        }
        if sites.is_none() && scan.referenced {
            return Err(unsupported(
                "bookmark may be referenced by a hyperlink or field",
            ));
        }
        let mut state = BoundaryState::default();
        let mut found = [false; 2];
        let mut old = Vec::new();
        let mut marker_nodes = vec![
            Node::new(tags[0]).attr("w:id", &id),
            Node::new(tags[1]).attr("w:id", &id),
        ];
        if let Some(name) = bookmark {
            marker_nodes[0].set("w:name", name);
        } else {
            marker_nodes.push(Node::new("w:r").with(Node::new(tags[2]).attr("w:id", &id)));
        }
        let body = root.child("w:body").ok_or_else(err)?;
        for (bi, p) in body.elements().filter(|n| n.name != "w:sectPr").enumerate() {
            if p.name != "w:p" {
                state.advance(p);
                continue;
            }
            for (i, n) in p.elements().filter(|n| n.name != "w:pPr").enumerate() {
                let pos = (bi, i);
                if let Some((a, b)) = sites
                    && (a == pos || b == pos)
                {
                    if !state.paired_site() {
                        return Err(unsupported("range endpoint in protected markup"));
                    }
                    text::check_whole(n, false)?;
                    found[0] |= a == pos;
                    found[1] |= b == pos;
                }
                let kind = scan.marker(n).or_else(|| {
                    (n.name == "w:r" && n.child(tags[2]).and_then(|r| scan.marker(r)) == Some(2))
                        .then_some(2)
                });
                if let Some(k) = kind {
                    if k != old.len() || !state.paired_site() {
                        return Err(err());
                    }
                    if k < 2
                        && (!n.children.is_empty()
                            || text::has_other_attrs(
                                n,
                                if k == 0 && bookmark.is_some() {
                                    &["w:id", "w:name"]
                                } else {
                                    &["w:id"]
                                },
                            )
                            || k == 0 && bookmark.is_some() && n.get("w:name") != bookmark)
                    {
                        return Err(err());
                    }
                    if k == 2 {
                        let r = n.child(tags[2]).ok_or_else(err)?;
                        if !r.children.is_empty()
                            || text::has_other_attrs(r, &["w:id"])
                            || text::has_other_attrs(n, &[])
                            || text::opaque_content(n)
                            || !n.elements().all(|c| {
                                c.name == tags[2]
                                    || c.name == "w:rPr"
                                        && !text::has_other_attrs(c, &[])
                                        && !text::opaque_content(c)
                                        && c.elements().all(|s| {
                                            s.name == "w:rStyle"
                                                && s.children.is_empty()
                                                && !text::has_other_attrs(s, &["w:val"])
                                        })
                            })
                        {
                            return Err(err());
                        }
                    }
                    marker_nodes[k] = n.clone();
                    old.push(pos);
                }
                state.advance(n);
            }
        }
        if sites.is_some() && found != [true, true] || existing && old.len() != marker_nodes.len() {
            return Err(err());
        }
        for pos in old {
            if slots.contains_key(&pos) {
                return Err(unsupported("conflicting range markers"));
            }
            slots.entry(pos).or_default().replacement = Some(Vec::new());
        }
        if let Some((a, b)) = sites {
            slots
                .entry(a)
                .or_default()
                .before
                .push(marker_nodes.remove(0));
            slots.entry(b).or_default().after.extend(marker_nodes);
        }
        Ok(())
    }
}

pub(super) fn recheck_removals(
    package: &Package,
    main: &str,
    edits: &[PairedRangeEdit],
) -> Result<()> {
    for e in edits
        .iter()
        .filter(|e| e.bookmark.is_some() && e.endpoints.is_none())
    {
        let mut scan = Scan {
            tags: ["w:bookmarkStart", "w:bookmarkEnd", ""],
            id: e.id.to_string(),
            name: e.bookmark.as_deref(),
            counts: [0; 3],
            referenced: false,
        };
        scan.scan(&package.word(main)?)?;
        for part in crate::ooxml::related_story_parts(package, main)? {
            scan.scan(&package.word(&part)?)?;
        }
        if scan.referenced || scan.counts != [0; 3] {
            return Err(unsupported(
                "removed bookmark may be referenced in staged content",
            ));
        }
    }
    Ok(())
}
