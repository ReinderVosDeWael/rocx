use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Slot {
    section_index: usize,
    kind: String,
    variant: String,
}
impl Slot {
    pub(super) fn legacy(h: HeaderAddress) -> Self {
        Self {
            kind: "header".into(),
            section_index: h.section_index,
            variant: h.variant,
        }
    }
    pub(super) fn image(
        self,
        doc: &ImportedDocument,
        stories: &mut std::cell::OnceCell<Result<Vec<ImportedStory>, String>>,
        mode: &str,
        bytes: Vec<u8>,
    ) -> Result<HeaderImageEdit, String> {
        let mode = match mode {
            "shared" => HeaderEditMode::Shared,
            "section-local" => HeaderEditMode::SectionLocal,
            _ => return Err("unknown image edit mode".into()),
        };
        let target = self.anchor(doc, stories)?;
        Ok(HeaderImageEdit {
            target,
            mode,
            image: resolve(ImageRun::new(bytes))?
                .image()
                .ok_or("expected image")?
                .clone(),
        })
    }
    fn anchor(
        &self,
        doc: &ImportedDocument,
        stories: &std::cell::OnceCell<Result<Vec<ImportedStory>, String>>,
    ) -> Result<HeaderAnchor, String> {
        stories
            .get_or_init(|| doc.stories().map_err(|e| e.to_string()))
            .as_ref()
            .map_err(Clone::clone)?
            .iter()
            .find(|s| {
                s.kind.as_str() == self.kind
                    && s.section_index == self.section_index
                    && variant(s.variant) == self.variant
            })
            .and_then(|s| s.anchor.clone())
            .ok_or_else(|| "unknown story slot".into())
    }
}
pub(super) fn variant(v: HeaderFooterType) -> &'static str {
    match v {
        HeaderFooterType::Default => "default",
        HeaderFooterType::First => "first",
        HeaderFooterType::Even => "even",
    }
}

pub(super) fn inline_anchor(
    doc: &ImportedDocument,
    target: usize,
    index: usize,
) -> Result<InlineAnchor, String> {
    match paragraph(doc, target)?.inline_structure.get(index) {
        Some(ImportedInlineNode::Run(r)) => Ok(r.anchor.clone()),
        Some(ImportedInlineNode::Hyperlink(h)) => Ok(h.anchor.clone()),
        Some(ImportedInlineNode::Fraction { anchor, .. }) => Ok(anchor.clone()),
        _ => Err("expected a direct run, hyperlink or fraction".into()),
    }
}
impl RunAddress {
    fn run(&self, doc: &ImportedDocument) -> Result<InlineAnchor, String> {
        run(doc, self.target, self.inline_index)
    }
    fn revision(&self, doc: &ImportedDocument) -> Result<InlineAnchor, String> {
        match paragraph(doc, self.target)?
            .inline_structure
            .get(self.inline_index)
        {
            Some(ImportedInlineNode::Revision(r)) => Ok(r.anchor.clone()),
            Some(ImportedInlineNode::Run(r)) if r.property_revision.is_some() => {
                Ok(r.anchor.clone())
            }
            _ => Err("expected direct revision wrapper or run property revision".into()),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoryRun {
    entry_index: Option<usize>,
    block_index: usize,
    inline_index: usize,
    text: String,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum Tracking {
    InsertAfter { text: String },
    Replace { text: String },
    Delete {},
}
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Review {
    Accept,
    Reject,
}
impl From<Review> for rocx::TextRevisionAction {
    fn from(value: Review) -> Self {
        match value {
            Review::Accept => Self::Accept,
            Review::Reject => Self::Reject,
        }
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum ParagraphTracking {
    InsertBefore { text: String },
    Delete {},
}
pub(super) fn preview_scoped(
    doc: &ImportedDocument,
    edit: Edit,
    snapshot: String,
) -> Result<(Vec<u8>, Value), String> {
    let p = match edit {
        Edit::PartTables {
            owner_part,
            operations,
        } => {
            return structure::preview_physical(doc, owner_part, operations, snapshot);
        }
        Edit::StoryReference { target, action } => {
            let kind = match target.kind.as_str() {
                "header" => SectionStoryKind::Header,
                "footer" => SectionStoryKind::Footer,
                _ => return Err("unknown story kind".into()),
            };
            let variant = match target.variant.as_str() {
                "default" => HeaderFooterType::Default,
                "first" => HeaderFooterType::First,
                "even" => HeaderFooterType::Even,
                _ => return Err("unknown story variant".into()),
            };
            let action = match action {
                ReferenceAction::Create { blocks } => {
                    StoryReferenceEdit::Create(resolve_blocks(blocks)?)
                }
                ReferenceAction::Link { part } => StoryReferenceEdit::Link(part),
                ReferenceAction::Remove {} => StoryReferenceEdit::Remove,
            };
            return Ok((
                doc.edit_story_reference(target.section_index, kind, variant, action)
                    .map_err(|e| e.to_string())?,
                json!({"snapshot":snapshot,"section_index":target.section_index}),
            ));
        }
        Edit::StoryBlocks {
            target,
            mode,
            index,
            remove,
            blocks,
        } => {
            let anchor = target.anchor(doc, &std::cell::OnceCell::new())?;
            let mode = match mode.as_str() {
                "shared" => HeaderEditMode::Shared,
                "section-local" => HeaderEditMode::SectionLocal,
                _ => return Err("unknown story block edit mode".into()),
            };
            return Ok((
                doc.edit_story_blocks(&anchor, mode, index, remove, &resolve_blocks(blocks)?)
                    .map_err(|e| e.to_string())?,
                json!({"snapshot":snapshot,"section_index":target.section_index}),
            ));
        }
        Edit::StoryText { part, runs } => {
            let stories = if runs.iter().any(|r| r.entry_index.is_none()) {
                doc.stories().map_err(|e| e.to_string())?
            } else {
                vec![]
            };
            let mut edits = Vec::new();
            for r in runs {
                let nodes = if let Some(index) = r.entry_index {
                    doc.entry_stories()
                        .map_err(|e| e.to_string())?
                        .get(index)
                        .filter(|e| e.part_name == part)
                        .and_then(|e| e.inline_structure(r.block_index))
                } else {
                    stories
                        .iter()
                        .find(|s| s.part_name == part)
                        .and_then(|s| s.inline_structure(r.block_index))
                };
                let Some(ImportedInlineNode::Run(run)) = nodes.and_then(|n| n.get(r.inline_index))
                else {
                    return Err("unknown physical story run".into());
                };
                edits.push(RunTextEdit {
                    target: run.anchor.clone(),
                    text: r.text,
                });
            }
            return Ok((
                doc.edit_story_text(&part, &edits)
                    .map_err(|e| e.to_string())?,
                json!({"snapshot":snapshot,"story_part":part}),
            ));
        }
        Edit::TrackedText {
            target,
            action,
            author,
            date,
        } => doc.preview_tracked_text(
            &target.run(doc)?,
            match action {
                Tracking::InsertAfter { text } => TrackedText::InsertAfter(text),
                Tracking::Replace { text } => TrackedText::Replace(text),
                Tracking::Delete {} => TrackedText::Delete,
            },
            &author,
            date.as_deref(),
        ),
        Edit::TrackedMove {
            target,
            after,
            name,
            author,
            date,
        } => doc.preview_tracked_move(&target.run(doc)?, &after.run(doc)?, &name, &author, &date),
        Edit::TrackedBold {
            target,
            value,
            author,
            date,
        } => doc.preview_tracked_bold(&target.run(doc)?, value, &author, date.as_deref()),
        Edit::TrackedParagraph {
            target,
            operation,
            author,
            date,
        } => doc.preview_tracked_paragraph(
            &paragraph(doc, target)?.anchor,
            match operation {
                ParagraphTracking::InsertBefore { text } => TrackedParagraph::InsertBefore(text),
                ParagraphTracking::Delete {} => TrackedParagraph::Delete,
            },
            &author,
            date.as_deref(),
        ),
        Edit::ReviewParagraphRevision { target, action } => {
            doc.preview_paragraph_revision(&paragraph(doc, target)?.anchor, action.into())
        }
        Edit::ReviewRevision {
            target,
            action,
            paired,
        } => doc.preview_revision(
            &target.revision(doc)?,
            action.into(),
            paired.map(|p| p.revision(doc)).transpose()?.as_ref(),
        ),
        _ => unreachable!(),
    }
    .map_err(|e| e.to_string())?;
    Ok((
        p.to_docx().map_err(|e| e.to_string())?,
        report(&p, snapshot),
    ))
}
