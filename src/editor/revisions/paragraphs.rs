//! Mark the paragraph terminator as well as its text (§17.13.5.14, §17.13.5.30).
use super::*;

#[derive(Clone, Debug)]
pub enum TrackedParagraph {
    InsertBefore(String),
    Delete,
}

fn position(body: &Node, anchor: &ParagraphAnchor) -> Result<usize> {
    body.children
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, Content::Element(n) if n.name != "w:sectPr"))
        .nth(anchor.body_index)
        .filter(|(_, c)| node(c).name == "w:p")
        .map(|(i, _)| i)
        .ok_or_else(|| unsupported("missing direct body paragraph"))
}

// Removing a paragraph mark joins with its successor. Never simulate that
// operation across a table, section break, final body boundary or opaque node.
fn successor(body: &Node, start: usize) -> Result<()> {
    let next = body.children[start..]
        .iter()
        .find(|c| !matches!(c, Content::Text(s) if s.trim().is_empty()));
    if !matches!(next, Some(Content::Element(n)) if n.name == "w:p" && blocks::local(n, "body")) {
        return Err(unsupported(
            "paragraph revision needs an ordinary following paragraph",
        ));
    }
    Ok(())
}

impl ImportedDocument {
    /// Track a plain paragraph inserted before an ordinary paragraph, or the
    /// deletion of a single-run paragraph immediately preceding one.
    pub fn preview_tracked_paragraph(
        &self,
        target: &ParagraphAnchor,
        edit: TrackedParagraph,
        author: &str,
        date: Option<&str>,
    ) -> Result<EditPreview> {
        self.validate_snapshot(&target.snapshot)?;
        self.stage_revision(|body| {
            let i = position(body, target)?;
            guard_boundary(body, (i, 0))?;
            let (kind, run, end) = match edit {
                TrackedParagraph::InsertBefore(text) => {
                    successor(body, i)?;
                    crate::xml::validate_text(&text)?;
                    if text.is_empty() || text.contains(['\n', '\r', '\t']) {
                        return Err(unsupported("expected nonempty single-line paragraph text"));
                    }
                    let mut leaf = Node::new("w:t");
                    if text.starts_with(' ') || text.ends_with(' ') {
                        leaf.set("xml:space", "preserve");
                    }
                    (
                        RevisionKind::Insertion,
                        Node::new("w:r").with(leaf.text(text)),
                        i,
                    )
                }
                TrackedParagraph::Delete => {
                    successor(body, i + 1)?;
                    let p = node(&body.children[i]);
                    if !allowed(p, &[]) || p.elements().count() != 1 {
                        return Err(unsupported(
                            "expected paragraph with one text run and no paragraph properties",
                        ));
                    }
                    let mut run = p.elements().next().unwrap().clone();
                    simple_text(&run, "w:t")?;
                    run.child_mut("w:t").unwrap().name = "w:delText".into();
                    (RevisionKind::Deletion, run, i + 1)
                }
            };
            let rev = metadata(kind, author, date)?;
            let mut ids = check_references(&self.package, None, &[])?;
            let mark = ids.paragraph_mark(&rev)?;
            let mut p =
                Node::new("w:p").with(Node::new("w:pPr").with(Node::new("w:rPr").with(mark)));
            for wrapper in ids.wrap(&self.package, run, &rev)? {
                p.push(wrapper);
            }
            body.children.splice(i..end, [Content::Element(p)]);
            Ok(true)
        })
    }

    /// Review the paragraph mark and its matching text wrapper together.
    pub fn preview_paragraph_revision(
        &self,
        target: &ParagraphAnchor,
        action: TextRevisionAction,
    ) -> Result<EditPreview> {
        self.validate_snapshot(&target.snapshot)?;
        self.stage_revision(|body| {
            let i = position(body, target)?;
            guard_boundary(body, (i, 0))?;
            successor(body, i + 1)?;
            let p = node(&body.children[i]);
            let bad = || unsupported("expected matching paragraph mark and single-run revision");
            let (Some(props), Some(wrapper), None) = ({
                let mut children = p.elements();
                (children.next(), children.next(), children.next())
            }) else {
                return Err(bad());
            };
            if !allowed(p, &[])
                || props.name != "w:pPr"
                || !allowed(props, &[])
                || props.elements().count() != 1
            {
                return Err(bad());
            }
            let rp = props.child("w:rPr").ok_or_else(bad)?;
            if !allowed(rp, &[]) || rp.elements().count() != 1 {
                return Err(bad());
            }
            let mark = rp.elements().next().unwrap();
            let tag = match wrapper.name.as_str() {
                "w:ins" => "w:t",
                "w:del" => "w:delText",
                _ => return Err(bad()),
            };
            if mark.name != wrapper.name
                || !allowed(mark, &["w:id", "w:author", "w:date"])
                || mark.elements().next().is_some()
                || mark.get("w:author") != wrapper.get("w:author")
                || mark.get("w:date") != wrapper.get("w:date")
            {
                return Err(bad());
            }
            check_references(&self.package, None, &[(id(mark)?, 1), (id(wrapper)?, 1)])?;
            let mut run = unwrap_run(wrapper, tag)?;
            run.child_mut(tag).unwrap().name = "w:t".into();
            if (action == TextRevisionAction::Accept) == (tag == "w:t") {
                body.children[i] = Content::Element(Node::new("w:p").with(run));
            } else {
                body.children.remove(i);
            }
            Ok(true)
        })
    }
}
