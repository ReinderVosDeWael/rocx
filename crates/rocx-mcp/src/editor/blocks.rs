use std::collections::BTreeMap;

use rocx::editor::{BlockEdit, BodyBlock, ParagraphAnchor, SectionAnchor};
use serde_json::{Value, json};

use super::Snapshot;
use crate::schema::blocks::{BlockChange, section_type_name};

const SECTION: &str = "section:";
const PARAGRAPH: &str = "paragraph:";

impl Snapshot {
    pub fn section_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        if limit == 0 {
            return Err("section page limit must be positive".into());
        }
        let sections = self.document.sections().map_err(|e| e.to_string())?;
        let start = start.min(sections.len());
        let end = start.saturating_add(limit).min(sections.len());
        let rows = (start..end)
            .map(|index| {
                let section = &sections[index];
                // Legacy section block edits require a paragraph at the start of
                // the range; a later paragraph is not a substitute.
                let first_paragraph = if section.body_start < section.body_end
                    && let Some(BodyBlock::Paragraph(paragraph)) =
                        self.document.body().get(section.body_start)
                {
                    Some(json!({
                        "body_index": section.body_start,
                        "text": paragraph.text,
                        "has_opaque_content": paragraph.has_opaque_content,
                    }))
                } else {
                    None
                };
                let flags: BTreeMap<_, _> = section
                    .flags
                    .iter()
                    .map(|(flag, value)| (flag.name(), value))
                    .collect();
                json!({
                    "index": index,
                    "anchor": format!("{SECTION}{index}"),
                    "body_start": section.body_start,
                    "body_end": section.body_end,
                    "section_type_xml": section.section_type_xml,
                    "section_type": section.section_type.map(section_type_name),
                    "flags": flags,
                    "layout": super::layout::layout_values(&section.layout),
                    "first_paragraph": first_paragraph,
                    "story_slots": super::stories::section_story_slots(index),
                })
            })
            .collect();
        Ok((rows, (end < sections.len()).then_some(end)))
    }

    // Session must resolve only anchors issued for the caller's snapshot.
    pub(super) fn block_edit(&self, local: &str, edit: BlockChange) -> Result<BlockEdit, String> {
        let paragraph = || self.block_paragraph(local);
        let section = || self.block_section(local);
        Ok(match edit {
            BlockChange::SplitSection { offset } => BlockEdit::SplitSection(section()?, offset),
            BlockChange::MergeSection {} => BlockEdit::MergeSection(section()?),
            BlockChange::ReplaceParagraph { paragraph: payload } => {
                BlockEdit::Replace(paragraph()?, Box::new(payload.resolve()?))
            }
            BlockChange::RemoveParagraph {} => BlockEdit::Remove(paragraph()?),
            BlockChange::ReplaceSection { blocks } => BlockEdit::ReplaceSection(
                paragraph()?,
                blocks
                    .into_iter()
                    .map(|block| block.resolve())
                    .collect::<Result<_, _>>()?,
            ),
            BlockChange::KeepWithNext { value } => BlockEdit::KeepWithNext(paragraph()?, value),
            BlockChange::ParagraphFlag { field, value } => {
                BlockEdit::ParagraphFlag(paragraph()?, field.into(), value.into_option())
            }
            BlockChange::SectionFlag { field, value } => {
                BlockEdit::SectionFlag(paragraph()?, field.into(), value.into_option())
            }
            BlockChange::SectionType { value } => {
                BlockEdit::SectionType(paragraph()?, value.into_option().map(Into::into))
            }
        })
    }

    fn block_paragraph(&self, local: &str) -> Result<ParagraphAnchor, String> {
        match local_index(local, PARAGRAPH).and_then(|index| self.document.body().get(index)) {
            Some(BodyBlock::Paragraph(paragraph)) => Ok(paragraph.anchor.clone()),
            _ => Err("wrong_anchor_type: expected a compact body paragraph anchor".into()),
        }
    }

    fn block_section(&self, local: &str) -> Result<SectionAnchor, String> {
        let wrong_type = || "wrong_anchor_type: expected a sections view anchor".to_owned();
        let index = local_index(local, SECTION).ok_or_else(wrong_type)?;
        self.document
            .sections()
            .map_err(|e| e.to_string())?
            .into_iter()
            .nth(index)
            .map(|section| section.anchor)
            .ok_or_else(wrong_type)
    }
}

fn local_index(local: &str, prefix: &str) -> Option<usize> {
    let index = local.strip_prefix(prefix)?;
    if index.is_empty()
        || (index.len() > 1 && index.starts_with('0'))
        || !index.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    index.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocx::{Document, Paragraph, Section, SectionProperties, SectionType};

    fn change(input: Value) -> BlockChange {
        serde_json::from_value(input).unwrap()
    }

    #[test]
    fn sections_page_preserves_native_bounds_and_direct_values() {
        let properties = SectionProperties {
            r#type: Some(SectionType::Continuous),
            title_page: Some(true),
            ..SectionProperties::default()
        };
        let bytes = futures_executor::block_on(
            Document::new(vec![
                Section::new(vec![Paragraph::new("first").into()])
                    .with_properties(Some(properties))
                    .into(),
                Section::new(vec![Paragraph::new("last").into()]).into(),
            ])
            .resolve(),
        )
        .unwrap()
        .to_docx()
        .unwrap();
        let snapshot = Snapshot::open(&bytes).unwrap();
        let sections = snapshot.document.sections().unwrap();
        let (first, next) = snapshot.section_page(0, 1).unwrap();
        assert_eq!(next, Some(1));
        assert_eq!(
            first,
            vec![json!({
                "index": 0, "anchor": "section:0",
                "body_start": sections[0].body_start, "body_end": sections[0].body_end,
                "section_type_xml": "continuous", "section_type": "continuous",
                "flags": {"titlePage": true},
                "layout": super::super::layout::layout_values(&sections[0].layout),
                "first_paragraph": {"body_index": 0, "text": "first", "has_opaque_content": false},
                "story_slots": [
                    {"anchor": "story-slot:0", "kind": "header", "variant": "default"},
                    {"anchor": "story-slot:1", "kind": "header", "variant": "first"},
                    {"anchor": "story-slot:2", "kind": "header", "variant": "even"},
                    {"anchor": "story-slot:3", "kind": "footer", "variant": "default"},
                    {"anchor": "story-slot:4", "kind": "footer", "variant": "first"},
                    {"anchor": "story-slot:5", "kind": "footer", "variant": "even"},
                ]
            })]
        );
        assert_eq!(first[0]["layout"]["titlePage"], true);
        assert_eq!(first[0]["layout"]["sectionType"], "continuous");
        let (last, next) = snapshot.section_page(1, usize::MAX).unwrap();
        assert_eq!(next, None);
        assert_eq!(last[0]["anchor"], "section:1");
        assert_eq!(last[0]["body_start"], sections[1].body_start);
        assert_eq!(last[0]["body_end"], sections[1].body_end);
        assert_eq!(snapshot.section_page(2, 1).unwrap(), (vec![], None));
        assert!(snapshot.section_page(0, 0).is_err());
    }

    #[test]
    fn block_edits_use_native_targets_and_keep_scopes_separate() {
        let bytes = Snapshot::create(vec!["first".into(), "last".into()]).unwrap();
        let snapshot = Snapshot::open(&bytes).unwrap();
        for (local, input) in [
            ("paragraph:0", json!({"kind":"split-section","offset":1})),
            ("section:0", json!({"kind":"remove-paragraph"})),
            ("layout-section:0", json!({"kind":"merge-section"})),
            ("run:0:0", json!({"kind":"keep-with-next","value":false})),
        ] {
            let error = snapshot.block_edit(local, change(input)).unwrap_err();
            assert!(error.starts_with("wrong_anchor_type:"), "{error}");
        }
        let split = snapshot
            .block_edit(
                "section:0",
                change(json!({"kind":"split-section","offset":1})),
            )
            .unwrap();
        let flag = snapshot
            .block_edit(
                "paragraph:0",
                change(json!({"kind":"paragraph-flag","field":"keepWithNext","value":false})),
            )
            .unwrap();
        assert!(
            snapshot
                .document
                .preview_blocks(&[split.clone(), flag.clone()])
                .is_err()
        );
        let preview = snapshot.document.preview_blocks(&[split]).unwrap();
        let split_snapshot = Snapshot::open(&preview.to_docx().unwrap()).unwrap();
        assert_eq!(split_snapshot.section_page(0, 10).unwrap().0.len(), 2);
        let merge = split_snapshot
            .block_edit("section:0", change(json!({"kind":"merge-section"})))
            .unwrap();
        let preview = split_snapshot.document.preview_blocks(&[merge]).unwrap();
        let merged = Snapshot::open(&preview.to_docx().unwrap()).unwrap();
        assert_eq!(merged.section_page(0, 10).unwrap().0.len(), 1);
        let preview = snapshot.document.preview_blocks(&[flag]).unwrap();
        let flagged = Snapshot::open(&preview.to_docx().unwrap()).unwrap();
        let BodyBlock::Paragraph(paragraph) = &flagged.document.body()[0] else {
            panic!("expected first body paragraph");
        };
        assert_eq!(
            paragraph
                .flags
                .get(&rocx::editor::ParagraphFlag::KeepWithNext),
            Some(&false)
        );
    }

    #[test]
    fn block_edits_preserve_payload_order_and_set_or_clear_direct_values() {
        let bytes = Snapshot::create(vec!["first".into(), "middle".into(), "last".into()]).unwrap();
        let snapshot = Snapshot::open(&bytes).unwrap();
        let edit = |index, value| {
            snapshot
                .block_edit(&format!("paragraph:{index}"), change(value))
                .unwrap()
        };
        let paragraph = json!({"inlines":[{"kind":"text","text":"replacement"}]});
        let batch = [
            edit(
                0,
                json!({"kind":"section-flag","field":"titlePage","value":false}),
            ),
            edit(0, json!({"kind":"section-type","value":"oddPage"})),
            edit(0, json!({"kind":"keep-with-next","value":true})),
            edit(1, json!({"kind":"replace-paragraph","paragraph":paragraph})),
            edit(2, json!({"kind":"remove-paragraph"})),
        ];
        let result = snapshot.document.preview_blocks(&batch).unwrap();
        let edited = Snapshot::open(&result.to_docx().unwrap()).unwrap();
        let page = edited.section_page(0, 1).unwrap().0;
        assert_eq!(page[0]["flags"], json!({"titlePage": false}));
        assert_eq!(page[0]["section_type"], "oddPage");
        let [BodyBlock::Paragraph(first), BodyBlock::Paragraph(second)] = edited.document.body()
        else {
            panic!("expected two direct paragraphs");
        };
        assert_eq!(
            first.flags.get(&rocx::editor::ParagraphFlag::KeepWithNext),
            Some(&true)
        );
        assert_eq!(second.text, "replacement");

        let clear = [
            json!({"kind":"section-flag","field":"titlePage","value":null}),
            json!({"kind":"section-type","value":null}),
            json!({"kind":"paragraph-flag","field":"keepWithNext","value":null}),
        ]
        .map(|value| edited.block_edit("paragraph:0", change(value)).unwrap());
        let result = edited.document.preview_blocks(&clear).unwrap();
        let cleared = Snapshot::open(&result.to_docx().unwrap()).unwrap();
        let page = cleared.section_page(0, 1).unwrap().0;
        assert_eq!(page[0]["flags"], json!({}));
        assert!(page[0]["section_type_xml"].is_null());
        let BodyBlock::Paragraph(first) = &cleared.document.body()[0] else {
            panic!("expected direct paragraph");
        };
        assert!(first.flags.is_empty());

        let replacement = edit(
            0,
            json!({
                "kind":"replace-section",
                "blocks":[{"kind":"table","rows":[["cell"]]}, {"kind":"paragraph","paragraph":paragraph}]
            }),
        );
        let result = snapshot.document.preview_blocks(&[replacement]).unwrap();
        let replaced = Snapshot::open(&result.to_docx().unwrap()).unwrap();
        let [BodyBlock::Table(_), BodyBlock::Paragraph(last)] = replaced.document.body() else {
            panic!("expected table followed by a paragraph");
        };
        assert_eq!(last.text, "replacement");
        assert!(replaced.section_page(0, 1).unwrap().0[0]["first_paragraph"].is_null());
    }
}
