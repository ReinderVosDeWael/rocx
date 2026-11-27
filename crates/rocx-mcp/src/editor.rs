//! Translate the MCP editor subset into public rocx authoring and editor calls.
use std::collections::BTreeMap;
use std::sync::OnceLock;

use rocx::editor::{
    BlockEdit, BodyBlock, CellAlignmentEdit, EditBatch, EditPreview, FieldEdit, HeaderAnchor,
    HeaderEditMode, HeaderImageEdit, ImportedDocument, ImportedInline, ImportedInlineNode,
    ImportedRowChild, ImportedTableChild, InlineAnchor, InlineEdit, PairedRangeEdit,
    ParagraphAlignmentEdit, ParagraphAnchor, ParagraphInsertion, ResourceEdit, RunTextEdit,
    SectionStoryKind, TableAnchor, TableInsertion, TextInsertion, TextRangeEdit,
};
use rocx::{
    Alignment, BreakType, Component, Document, HeaderFooterType, ImageRun, Paragraph,
    ResolvedTable, Section, Table, TableCell, TableRow, VerticalAlignment,
};
use serde_json::{Value, json};

use crate::schema::{CellAlignment, Edit, HeaderMode, ParagraphAlignment};

mod blocks;
mod body_inlines;
mod body_tables;
mod controls;
mod entries;
mod fields;
mod layout;
mod notes;
mod numbering;
mod paired_ranges;
mod resources;
mod revisions;
mod run_format;
mod stories;
mod styles;
mod tables;

pub struct Snapshot {
    document: ImportedDocument,
    inspection: OnceLock<Result<Inspection, String>>,
    story_controls: OnceLock<Result<Vec<rocx::editor::ImportedControl>, String>>,
    styles: OnceLock<Result<Option<rocx::editor::ImportedStyles>, String>>,
    numbering: OnceLock<Result<rocx::editor::ImportedNumberingCatalog, String>>,
}

enum Target {
    Paragraph(ParagraphAnchor),
    Run(InlineAnchor),
    Cell {
        table: TableAnchor,
        row_index: usize,
        cell_index: usize,
    },
    Header(HeaderAnchor),
}

impl Target {
    fn kind(&self) -> &'static str {
        match self {
            Self::Paragraph(_) => "paragraph",
            Self::Run(_) => "run",
            Self::Cell { .. } => "cell",
            Self::Header(_) => "header",
        }
    }
}

struct Inspection {
    body: Vec<Value>,
    headers: Vec<Value>,
    anchors: BTreeMap<String, Target>,
}

impl Snapshot {
    pub fn open(bytes: &[u8]) -> Result<Self, String> {
        Self::open_with_limits(bytes, rocx::opc::ReadLimits::default())
    }

    pub fn open_with_limits(bytes: &[u8], limits: rocx::opc::ReadLimits) -> Result<Self, String> {
        Ok(Self {
            document: ImportedDocument::open_with_limits(bytes, limits)
                .map_err(|e| e.to_string())?,
            inspection: OnceLock::new(),
            story_controls: OnceLock::new(),
            styles: OnceLock::new(),
            numbering: OnceLock::new(),
        })
    }

    pub fn inspect(&self) -> Result<Value, String> {
        let inspection = self
            .inspection
            .get_or_init(|| inspect(&self.document))
            .as_ref()
            .map_err(Clone::clone)?;
        Ok(json!({"body": inspection.body, "headers": inspection.headers}))
    }

    fn issued_inspection(&self) -> Option<&Inspection> {
        self.inspection.get()?.as_ref().ok()
    }

    pub fn has_issued_anchor(&self, anchor: &str) -> bool {
        self.issued_inspection()
            .is_some_and(|inspection| inspection.anchors.contains_key(anchor))
    }

    pub fn create(mut paragraphs: Vec<String>) -> Result<Vec<u8>, String> {
        if paragraphs.is_empty() {
            paragraphs.push(String::new());
        }
        let blocks = paragraphs
            .into_iter()
            .map(|text| Paragraph::new(text).into())
            .collect();
        futures_executor::block_on(Document::new(vec![Section::new(blocks).into()]).resolve())
            .and_then(|document| document.to_docx())
            .map_err(|e| e.to_string())
    }

    pub fn preview(&self, edits: Vec<Edit>) -> Result<(Vec<u8>, Value), String> {
        if edits.is_empty() {
            return Err("edits must not be empty".into());
        }
        let mut batch = Batch::default();
        for (index, mut edit) in edits.into_iter().enumerate() {
            let number = index + 1;
            let locate = |e: String| {
                if e.starts_with("wrong_anchor_type: ") {
                    e
                } else {
                    format!("edit {number}: {e}")
                }
            };
            if let Edit::Resource { edit } = edit {
                batch.resources.push(edit.into());
                continue;
            }
            if let Edit::Numbering { edit } = edit {
                batch.resources.push(
                    edit.resolve(
                        |local| self.numbering_paragraph_anchor(local),
                        |local| self.physical_numbering_anchor(local),
                    )
                    .map_err(locate)?,
                );
                continue;
            }
            if let Edit::Control { edit } = edit {
                self.control_edit(edit, &mut batch).map_err(locate)?;
                continue;
            }
            if let Edit::Style { edit } = edit {
                batch.styles.push(self.style_edit(edit).map_err(locate)?);
                continue;
            }
            if let Edit::Block { anchor, edit } = edit {
                batch
                    .blocks
                    .push(self.block_edit(&anchor, edit).map_err(locate)?);
                continue;
            }
            if let Edit::Field { anchor, edit } = edit {
                batch
                    .fields
                    .push(self.field_edit(&anchor, edit).map_err(locate)?);
                continue;
            }
            if let Edit::Inline { anchor, edit } = edit {
                batch
                    .inline_edits
                    .push(self.body_inline_edit(&anchor, edit).map_err(locate)?);
                continue;
            }
            if let Edit::PairedRange {
                id,
                bookmark,
                endpoints,
            } = edit
            {
                batch.paired_ranges.push(
                    self.paired_range_edit(id, bookmark, endpoints)
                        .map_err(locate)?,
                );
                continue;
            }
            if let Edit::TableStructure { anchor, edit } = edit {
                batch
                    .structural
                    .push(self.body_table_edit(&anchor, edit).map_err(locate)?);
                continue;
            }
            let header_image = matches!(edit, Edit::HeaderImage { .. });
            let local = edit.anchor_slot().expect("anchored edit");
            let story_target = header_image
                .then(|| self.story_image_anchor(local).map(Target::Header))
                .flatten();
            let target = story_target
                .as_ref()
                .or_else(|| self.issued_inspection()?.anchors.get(local))
                .ok_or_else(|| {
                    if header_image && local.starts_with("story-") {
                        format!("wrong_anchor_type: edit {number}: expected an existing story slot")
                    } else {
                        format!("edit {number}: unknown local anchor")
                    }
                })?;
            if target.kind() != edit.target_kind() {
                return Err(format!(
                    "wrong_anchor_type: edit {number}: expected {}",
                    edit.target_kind()
                ));
            }
            batch
                .push(edit, target)
                .map_err(|e| format!("edit {number}: {e}"))?;
        }

        let preview = self
            .document
            .preview_edits(EditBatch {
                alignments: &batch.alignments,
                tables: &batch.tables,
                header_images: &batch.header_images,
                run_text: &batch.run_text,
                paragraphs: &batch.paragraphs,
                cells: &batch.cells,
                run_removals: &batch.run_removals,
                ranges: &batch.ranges,
                text_insertions: &batch.text_insertions,
                resources: &batch.resources,
                blocks: &batch.blocks,
                fields: &batch.fields,
                inline_edits: &batch.inline_edits,
                paired_ranges: &batch.paired_ranges,
                controls: &batch.controls,
                styles: &batch.styles,
                wrappers: &batch.wrappers,
                structural: &batch.structural,
            })
            .map_err(|e| format!("preview: {e}"))?;
        encode_preview(&preview)
    }

    pub fn preview_noop(&self) -> Result<(Vec<u8>, Value), String> {
        let preview = self
            .document
            .preview_edits(EditBatch::default())
            .map_err(|e| e.to_string())?;
        encode_preview(&preview)
    }
}

fn encode_preview(preview: &EditPreview) -> Result<(Vec<u8>, Value), String> {
    Ok((
        preview.to_docx().map_err(|e| e.to_string())?,
        report(preview),
    ))
}

#[derive(Default)]
struct Batch {
    alignments: Vec<ParagraphAlignmentEdit>,
    tables: Vec<TableInsertion>,
    header_images: Vec<HeaderImageEdit>,
    run_text: Vec<RunTextEdit>,
    paragraphs: Vec<ParagraphInsertion>,
    cells: Vec<CellAlignmentEdit>,
    run_removals: Vec<InlineAnchor>,
    ranges: Vec<TextRangeEdit>,
    text_insertions: Vec<TextInsertion>,
    resources: Vec<ResourceEdit>,
    blocks: Vec<BlockEdit>,
    fields: Vec<FieldEdit>,
    inline_edits: Vec<InlineEdit>,
    paired_ranges: Vec<PairedRangeEdit>,
    controls: Vec<rocx::editor::ControlEdit>,
    styles: Vec<rocx::editor::StyleEdit>,
    wrappers: Vec<rocx::editor::WrapperEdit>,
    structural: Vec<rocx::editor::TableStructureEdit>,
}

impl Batch {
    fn push(&mut self, edit: Edit, target: &Target) -> Result<(), String> {
        match (edit, target) {
            (Edit::Alignment { alignment, .. }, Target::Paragraph(target)) => {
                self.alignments.push(ParagraphAlignmentEdit {
                    target: target.clone(),
                    alignment: alignment.into(),
                });
            }
            (Edit::InsertParagraph { text, .. }, Target::Paragraph(target)) => {
                self.paragraphs.push(ParagraphInsertion {
                    target: target.clone(),
                    paragraph: resolve(Paragraph::new(text))?,
                });
            }
            (Edit::InsertRichParagraph { paragraph, .. }, Target::Paragraph(target)) => {
                self.paragraphs.push(ParagraphInsertion {
                    target: target.clone(),
                    paragraph: paragraph.resolve()?,
                });
            }
            (Edit::InsertTable { rows, .. }, Target::Paragraph(target)) => {
                self.tables.push(TableInsertion {
                    target: target.clone(),
                    table: resolve_table(rows)?,
                });
            }
            (Edit::InsertRichTable { table, .. }, Target::Paragraph(target)) => {
                self.tables.push(TableInsertion {
                    target: target.clone(),
                    table: table.resolve()?,
                });
            }
            (Edit::RunText { text, .. }, Target::Run(target)) => {
                self.run_text.push(RunTextEdit {
                    target: target.clone(),
                    text,
                });
            }
            (Edit::RunRemove { .. }, Target::Run(target)) => self.run_removals.push(target.clone()),
            (
                Edit::TextRange {
                    start, end, text, ..
                },
                Target::Paragraph(target),
            ) => self.ranges.push(TextRangeEdit {
                target: target.clone(),
                start,
                end,
                text,
            }),
            (Edit::InsertText { offset, text, .. }, Target::Paragraph(target)) => {
                self.text_insertions.push(TextInsertion {
                    target: target.clone(),
                    position: offset,
                    text,
                });
            }
            (
                Edit::CellAlignment { alignment, .. },
                Target::Cell {
                    table,
                    row_index,
                    cell_index,
                },
            ) => {
                self.cells.push(CellAlignmentEdit {
                    target: table.clone(),
                    row_index: *row_index,
                    cell_index: *cell_index,
                    alignment: alignment.into(),
                });
            }
            (Edit::HeaderImage { mode, bytes, .. }, Target::Header(target)) => {
                self.header_images.push(HeaderImageEdit {
                    target: target.clone(),
                    mode: mode.into(),
                    image: resolve(ImageRun::new(bytes))?
                        .image()
                        .ok_or("expected image")?
                        .clone(),
                });
            }
            _ => unreachable!("target kind validated before adding edit"),
        }
        Ok(())
    }
}

impl From<ParagraphAlignment> for Alignment {
    fn from(alignment: ParagraphAlignment) -> Self {
        match alignment {
            ParagraphAlignment::Left => Self::Left,
            ParagraphAlignment::Center => Self::Center,
            ParagraphAlignment::Right => Self::Right,
            ParagraphAlignment::Justify => Self::Justify,
            ParagraphAlignment::Distribute => Self::Distribute,
            ParagraphAlignment::JustifyMedium => Self::JustifyMedium,
            ParagraphAlignment::JustifyHigh => Self::JustifyHigh,
            ParagraphAlignment::JustifyLow => Self::JustifyLow,
            ParagraphAlignment::ThaiJustify => Self::ThaiJustify,
        }
    }
}

impl From<CellAlignment> for VerticalAlignment {
    fn from(alignment: CellAlignment) -> Self {
        match alignment {
            CellAlignment::Top => Self::Top,
            CellAlignment::Center => Self::Center,
            CellAlignment::Bottom => Self::Bottom,
            CellAlignment::Both => Self::Both,
        }
    }
}

impl From<HeaderMode> for HeaderEditMode {
    fn from(mode: HeaderMode) -> Self {
        match mode {
            HeaderMode::Shared => Self::Shared,
            HeaderMode::SectionLocal => Self::SectionLocal,
        }
    }
}

fn resolve<C: Component>(value: C) -> Result<C::Resolved, String> {
    futures_executor::block_on(value.resolve())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "declaration produced no content".into())
}

fn resolve_table(rows: Vec<Vec<String>>) -> Result<ResolvedTable, String> {
    let width = rows.first().map_or(0, Vec::len);
    if width == 0 || rows.iter().any(|row| row.len() != width) {
        return Err("rows must form a nonempty rectangular table".into());
    }
    let rows = rows
        .into_iter()
        .map(|row| {
            let cells = row
                .into_iter()
                .map(|text| TableCell::new(vec![Paragraph::new(text).into()]).into())
                .collect();
            TableRow::new(cells).into()
        })
        .collect();
    resolve(Table::new(rows))
}

fn report(preview: &EditPreview) -> Value {
    fn records<T>(items: &[T], record: impl Fn(&T) -> Value) -> Vec<Value> {
        items.iter().map(record).collect()
    }

    json!({
        "changes": records(preview.changes(), |change| json!({
            "target": change.body_index, "before": change.before,
            "after": format!("{:?}", change.after),
        })),
        "table_insertions": records(preview.insertions(), |change| json!({
            "target": change.after_body_index, "rows": change.rows, "columns": change.columns,
        })),
        "paragraph_insertions": preview.paragraph_insertions(),
        "block_changes": preview.block_changes(),
        "inline_changes": preview.inline_changes(),
        "header_images": records(preview.header_changes(), |change| {
            let mode = match change.mode {
                HeaderEditMode::Shared => "shared",
                HeaderEditMode::SectionLocal => "section-local",
            };
            json!({
                "source_part": change.source_part, "output_part": change.output_part,
                "mode": mode, "affected_sections": change.affected_sections,
                "restored_section": change.restored_section,
            })
        }),
        "run_text": records(preview.run_text_changes(), |change| json!({
            "target": change.body_index, "inline_index": change.inline_index,
            "before": change.before, "after": change.after,
        })),
        "run_removals": records(preview.run_removals(), |change| json!({
            "target": change.body_index, "inline_index": change.inline_index, "before": change.before,
        })),
        "cell_alignments": records(preview.cell_alignment_changes(), |change| json!({
            "target": change.body_index, "row_index": change.row_index, "cell_index": change.cell_index,
            "before": change.before, "after": format!("{:?}", change.after).to_ascii_lowercase(),
        })),
    })
}

fn projected_run_text(items: &[ImportedInline]) -> (String, bool) {
    let mut text = String::new();
    let mut opaque = false;
    for item in items {
        match item {
            ImportedInline::Text(value) => text.push_str(value),
            ImportedInline::Tab => text.push('\t'),
            ImportedInline::Break(BreakType::Line | BreakType::TextWrapping) => text.push('\n'),
            _ => opaque = true,
        }
    }
    (text, opaque)
}

fn scalar_length(inlines: &[ImportedInlineNode]) -> Option<usize> {
    // This is coordinate metadata, not an edit guard. In particular, the
    // projection may not show a field or paired range begun in another block.
    inlines.iter().try_fold(0, |length, inline| {
        let ImportedInlineNode::Run(run) = inline else {
            return None;
        };
        let [ImportedInline::Text(text)] = run.items.as_slice() else {
            return None;
        };
        if text.contains(['\t', '\r', '\n']) {
            return None;
        }
        Some(length + text.chars().count())
    })
}

fn inspect(document: &ImportedDocument) -> Result<Inspection, String> {
    let mut anchors = BTreeMap::new();
    let body = document
        .body()
        .iter()
        .enumerate()
        .map(|(index, block)| match block {
            BodyBlock::Paragraph(paragraph) => {
                let anchor = format!("paragraph:{index}");
                anchors.insert(anchor.clone(), Target::Paragraph(paragraph.anchor.clone()));
                let runs: Vec<_> = paragraph
                    .inline_structure
                    .iter()
                    .enumerate()
                    .filter_map(|(inline_index, inline)| {
                        let ImportedInlineNode::Run(run) = inline else {
                            return None;
                        };
                        let (text, opaque) = projected_run_text(&run.items);
                        let mut value = json!({
                            "inline_index": inline_index,
                            "text": text,
                            "has_opaque_content": opaque,
                            "has_opaque_formatting": run.has_opaque_formatting,
                        });
                        if matches!(run.items.as_slice(), [ImportedInline::Text(_)]) {
                            let anchor = format!("run:{index}:{inline_index}");
                            anchors.insert(anchor.clone(), Target::Run(run.anchor.clone()));
                            value["anchor"] = json!(anchor);
                        }
                        Some(value)
                    })
                    .collect();
                json!({
                    "type": "paragraph", "index": index, "anchor": anchor,
                    "text": paragraph.text, "alignment_xml": paragraph.alignment_xml,
                    "has_opaque_content": paragraph.has_opaque_content, "runs": runs,
                    "scalar_length": scalar_length(&paragraph.inline_structure),
                })
            }
            BodyBlock::Table(table) => {
                let mut cells = Vec::new();
                let mut opaque = false;
                for (row_index, row) in table.children.iter().enumerate() {
                    let ImportedTableChild::Row(row) = row else {
                        opaque = true;
                        continue;
                    };
                    for (cell_index, child) in row.children.iter().enumerate() {
                        let ImportedRowChild::Cell(cell) = child else {
                            opaque = true;
                            continue;
                        };
                        let anchor = format!("cell:{index}:{row_index}:{cell_index}");
                        anchors.insert(
                            anchor.clone(),
                            Target::Cell {
                                table: table.anchor.clone(),
                                row_index,
                                cell_index,
                            },
                        );
                        cells.push(json!({
                            "row_index": row_index, "cell_index": cell_index, "anchor": anchor,
                            "grid_span": cell.grid_span, "vertical_merge": cell.vertical_merge,
                        }));
                    }
                }
                json!({"type": "table", "index": index, "cells": cells, "has_opaque_content": opaque})
            }
            BodyBlock::ContentControl(control) => json!({
                "type": "content-control", "index": index, "tag": control.tag,
                "alias": control.alias, "lock": control.lock, "has_opaque_content": true,
            }),
            BodyBlock::Opaque(name) => json!({"type": "opaque", "index": index, "name": name}),
            _ => json!({"type": "opaque", "index": index}),
        })
        .collect();

    let stories = document.stories().map_err(|e| e.to_string())?;
    let mut uses = BTreeMap::new();
    // Count even-header aliases too: shared edits can affect those slots even
    // though the current MCP subset does not issue anchors for them.
    for story in &stories {
        if story.kind == SectionStoryKind::Header {
            *uses.entry(story.part_name.as_str()).or_insert(0usize) += 1;
        }
    }
    let headers = stories
        .iter()
        .filter_map(|story| {
            let variant = match (story.kind, story.variant) {
                (SectionStoryKind::Header, HeaderFooterType::Default) => "default",
                (SectionStoryKind::Header, HeaderFooterType::First) => "first",
                _ => return None,
            };
            let mut value = json!({
                "section_index": story.section_index, "variant": variant,
                "part_name": story.part_name, "inherited": story.inherited,
                "shared": uses[story.part_name.as_str()] > 1,
            });
            if let Some(target) = &story.anchor {
                let anchor = format!("header:{}:{variant}", story.section_index);
                anchors.insert(anchor.clone(), Target::Header(target.clone()));
                value["anchor"] = json!(anchor);
            }
            Some(value)
        })
        .collect();
    Ok(Inspection {
        body,
        headers,
        anchors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocx::{Header, TextRun};

    const IMAGE: &[u8] = include_bytes!("../../../tests/fixtures/images/sample.png");

    fn authored(sections: Vec<rocx::Value<Section>>) -> Vec<u8> {
        futures_executor::block_on(Document::new(sections).resolve())
            .unwrap()
            .to_docx()
            .unwrap()
    }

    fn open(bytes: &[u8]) -> Snapshot {
        Snapshot::open(bytes).unwrap()
    }

    fn anchor(value: &Value) -> String {
        value.as_str().unwrap().to_owned()
    }

    #[test]
    fn creates_an_editable_blank_paragraph_or_requested_text_in_order() {
        assert!(Snapshot::open(b"not a DOCX").is_err());
        let blank = open(&Snapshot::create(vec![]).unwrap());
        let inspection = blank.inspect().unwrap();
        assert_eq!(inspection, blank.inspect().unwrap());
        let body = inspection["body"].as_array().unwrap();
        assert_eq!(body.len(), 1);
        assert_eq!(body[0]["text"], "");
        assert_eq!(body[0]["scalar_length"], 0);
        let (preview, _) = blank
            .preview(vec![Edit::InsertParagraph {
                anchor: anchor(&body[0]["anchor"]),
                text: "after".into(),
            }])
            .unwrap();
        assert_eq!(
            open(&preview).inspect().unwrap()["body"][1]["text"],
            "after"
        );

        let supplied = Snapshot::create(vec!["one".into(), "two".into()]).unwrap();
        let inspection = open(&supplied).inspect().unwrap();
        let body = inspection["body"].as_array().unwrap();
        assert_eq!(body.len(), 2);
        assert_eq!(body[0]["text"], "one");
        assert_eq!(body[1]["text"], "two");
    }

    #[test]
    fn mixed_body_edits_match_native_bytes_and_allow_retry_after_rejection() {
        let table = |text| {
            Table::new(vec![
                TableRow::new(vec![
                    TableCell::new(vec![Paragraph::new(text).into()]).into(),
                ])
                .into(),
            ])
        };
        let input = authored(vec![
            Section::new(vec![
                Paragraph::new("first").into(),
                Paragraph::from_children(vec![
                    TextRun::new("second").into(),
                    TextRun::new("drop").into(),
                ])
                .into(),
                table("existing").into(),
                Paragraph::from_children(vec![ImageRun::new(IMAGE.to_vec()).into()]).into(),
            ])
            .into(),
        ]);
        let snapshot = open(&input);
        let inspection = snapshot.inspect().unwrap();
        let body = &inspection["body"];
        let first = &body[0]["anchor"];
        let second = &body[1]["anchor"];
        let keep = &body[1]["runs"][0]["anchor"];
        let remove = &body[1]["runs"][1]["anchor"];
        let cell = &body[2]["cells"][0]["anchor"];
        let drawing = &body[3]["runs"][0];
        assert!(body[3]["scalar_length"].is_null());
        assert_eq!(drawing["has_opaque_content"], true);
        assert!(drawing.get("anchor").is_none());
        let alignment = || Edit::Alignment {
            anchor: anchor(first),
            alignment: ParagraphAlignment::Center,
        };
        assert!(snapshot.preview(vec![]).is_err());
        assert!(
            snapshot
                .preview(vec![alignment(), alignment()])
                .unwrap_err()
                .contains("multiple alignment edits")
        );
        for invalid in [
            Edit::RunRemove {
                anchor: anchor(cell),
            },
            Edit::InsertTable {
                anchor: anchor(first),
                rows: vec![vec!["ok".into()], vec![]],
            },
            Edit::RunRemove {
                anchor: "not issued".into(),
            },
        ] {
            assert!(snapshot.preview(vec![invalid]).is_err());
        }
        assert!(
            snapshot
                .preview(vec![Edit::RunRemove {
                    anchor: anchor(first),
                }])
                .unwrap_err()
                .starts_with("wrong_anchor_type: ")
        );

        let (bytes, changes) = snapshot
            .preview(vec![
                Edit::InsertParagraph {
                    anchor: anchor(first),
                    text: "inserted".into(),
                },
                Edit::RunText {
                    anchor: anchor(keep),
                    text: "kept".into(),
                },
                Edit::InsertTable {
                    anchor: anchor(second),
                    rows: vec![vec!["new".into()]],
                },
                Edit::RunRemove {
                    anchor: anchor(remove),
                },
                Edit::CellAlignment {
                    anchor: anchor(cell),
                    alignment: CellAlignment::Bottom,
                },
                alignment(),
            ])
            .unwrap();
        let doc = ImportedDocument::open(&input).unwrap();
        let BodyBlock::Paragraph(a) = &doc.body()[0] else {
            panic!("first paragraph")
        };
        let BodyBlock::Paragraph(b) = &doc.body()[1] else {
            panic!("second paragraph")
        };
        let BodyBlock::Table(t) = &doc.body()[2] else {
            panic!("existing table")
        };
        let [ImportedInlineNode::Run(x), ImportedInlineNode::Run(y)] =
            b.inline_structure.as_slice()
        else {
            panic!("two direct runs")
        };
        let direct = doc
            .preview_edits(EditBatch {
                alignments: &[ParagraphAlignmentEdit {
                    target: a.anchor.clone(),
                    alignment: Alignment::Center,
                }],
                paragraphs: &[ParagraphInsertion {
                    target: a.anchor.clone(),
                    paragraph: resolve(Paragraph::new("inserted")).unwrap(),
                }],
                tables: &[TableInsertion {
                    target: b.anchor.clone(),
                    table: resolve(table("new")).unwrap(),
                }],
                run_text: &[RunTextEdit {
                    target: x.anchor.clone(),
                    text: "kept".into(),
                }],
                run_removals: std::slice::from_ref(&y.anchor),
                cells: &[CellAlignmentEdit {
                    target: t.anchor.clone(),
                    row_index: 0,
                    cell_index: 0,
                    alignment: VerticalAlignment::Bottom,
                }],
                ..EditBatch::default()
            })
            .unwrap()
            .to_docx()
            .unwrap();
        assert_eq!(bytes, direct);
        assert_eq!(changes["run_text"][0]["after"], "kept");
        assert_eq!(changes["run_removals"][0]["before"], "drop");
        assert_eq!(changes["cell_alignments"][0]["target"], 2);
        assert_eq!(snapshot.inspect().unwrap(), inspection);
        let reopened = open(&bytes).inspect().unwrap();
        assert_eq!(reopened["body"][1]["text"], "inserted");
        assert_eq!(reopened["body"][2]["text"], "kept");
        assert_eq!(reopened["body"][3]["type"], "table");
    }

    #[test]
    fn scalar_edits_mix_at_original_coordinates_and_match_native_bytes() {
        let input = authored(vec![
            Section::new(vec![
                Paragraph::from_children(vec![
                    TextRun::new("A😀").into(),
                    TextRun::new("BéC").into(),
                ])
                .into(),
                Paragraph::from_children(vec![]).into(),
                Paragraph::new("mixed\trun").into(),
            ])
            .into(),
        ]);
        let snapshot = open(&input);
        let inspection = snapshot.inspect().unwrap();
        let body = &inspection["body"];
        assert_eq!(body[0]["scalar_length"], 5);
        assert_eq!(body[1]["scalar_length"], 0);
        assert!(body[2]["scalar_length"].is_null());
        let first = || anchor(&body[0]["anchor"]);
        let blank = || anchor(&body[1]["anchor"]);
        let (bytes, changes) = snapshot
            .preview(vec![
                Edit::TextRange {
                    anchor: first(),
                    start: 3,
                    end: 4,
                    text: String::new(),
                },
                Edit::InsertText {
                    anchor: first(),
                    offset: 2,
                    text: "+".into(),
                },
                Edit::InsertText {
                    anchor: blank(),
                    offset: 0,
                    text: "🦀".into(),
                },
                Edit::TextRange {
                    anchor: first(),
                    start: 1,
                    end: 2,
                    text: "long".into(),
                },
            ])
            .unwrap();
        let native = ImportedDocument::open(&input).unwrap();
        let [BodyBlock::Paragraph(a), BodyBlock::Paragraph(b), _] = native.body() else {
            panic!("three paragraphs");
        };
        let expected = native
            .preview_edits(EditBatch {
                ranges: &[
                    TextRangeEdit {
                        target: a.anchor.clone(),
                        start: 3,
                        end: 4,
                        text: String::new(),
                    },
                    TextRangeEdit {
                        target: a.anchor.clone(),
                        start: 1,
                        end: 2,
                        text: "long".into(),
                    },
                ],
                text_insertions: &[
                    TextInsertion {
                        target: a.anchor.clone(),
                        position: 2,
                        text: "+".into(),
                    },
                    TextInsertion {
                        target: b.anchor.clone(),
                        position: 0,
                        text: "🦀".into(),
                    },
                ],
                ..EditBatch::default()
            })
            .unwrap()
            .to_docx()
            .unwrap();
        assert_eq!(bytes, expected);
        assert_eq!(
            changes["run_text"],
            json!([
                {"target":0,"inline_index":0,"before":"A😀","after":"Along"},
                {"target":0,"inline_index":1,"before":"BéC","after":"+BC"},
                {"target":1,"inline_index":0,"before":"","after":"🦀"},
            ])
        );
        let actual = open(&bytes).inspect().unwrap();
        assert_eq!(actual["body"][0]["text"], "Along+BC");
        assert_eq!(actual["body"][1]["text"], "🦀");
        assert_eq!(snapshot.inspect().unwrap(), inspection);
    }

    #[test]
    fn scalar_coordinates_do_not_bypass_cross_paragraph_protection() {
        let bytes = Snapshot::create(
            ["before", "start", "protected", "end", "after"]
                .map(str::to_owned)
                .to_vec(),
        )
        .unwrap();
        let doc = ImportedDocument::open(&bytes).unwrap();
        let run = |index| {
            let BodyBlock::Paragraph(paragraph) = &doc.body()[index] else {
                panic!("direct paragraph");
            };
            let [ImportedInlineNode::Run(run)] = paragraph.inline_structure.as_slice() else {
                panic!("one direct run");
            };
            &run.anchor
        };
        let bytes = doc
            .set_paired_range(1, Some("guard"), Some((run(1), run(3))))
            .unwrap();
        let snapshot = open(&bytes);
        let inspection = snapshot.inspect().unwrap();
        let body = inspection["body"].as_array().unwrap();
        let lengths: Vec<_> = body.iter().map(|p| p["scalar_length"].as_u64()).collect();
        assert_eq!(lengths, [Some(6), None, Some(9), None, Some(5)]);
        assert_eq!(body[2]["text"], "protected");
        assert_eq!(body[2]["has_opaque_content"], false);
        let safe_edit = || Edit::InsertText {
            anchor: anchor(&body[0]["anchor"]),
            offset: 6,
            text: "!".into(),
        };
        let err = snapshot
            .preview(vec![
                safe_edit(),
                Edit::InsertText {
                    anchor: anchor(&body[2]["anchor"]),
                    offset: 0,
                    text: "+".into(),
                },
            ])
            .unwrap_err();
        assert!(err.contains("outside fields and ranges"), "{err}");
        assert_eq!(snapshot.inspect().unwrap(), inspection);
        let (retry, _) = snapshot.preview(vec![safe_edit()]).unwrap();
        let output = open(&retry).inspect().unwrap();
        assert_eq!(output["body"][0]["text"], "before!");
        assert_eq!(output["body"][2]["text"], "protected");
    }

    #[test]
    fn previews_shared_and_local_edits_to_an_inherited_first_header() {
        let first_section = Section::new(vec![Paragraph::new("one").into()]).with_headers(
            [
                (
                    HeaderFooterType::Default,
                    Header::new(vec![Paragraph::new("default").into()]).into(),
                ),
                (
                    HeaderFooterType::First,
                    Header::new(vec![Paragraph::new("first").into()]).into(),
                ),
            ]
            .into_iter()
            .collect(),
        );
        let input = authored(vec![
            first_section.into(),
            Section::new(vec![Paragraph::new("two").into()]).into(),
        ]);
        let snapshot = open(&input);
        let inspection = snapshot.inspect().unwrap();
        let headers = inspection["headers"].as_array().unwrap();
        assert_eq!(headers.len(), 4);
        let original = &headers[1];
        let inherited = &headers[3];
        assert_eq!(original["variant"], "first");
        assert_eq!(inherited["section_index"], 1);
        assert_eq!(inherited["variant"], "first");
        assert_eq!(inherited["inherited"], true);
        assert_eq!(inherited["shared"], true);
        assert_eq!(inherited["part_name"], original["part_name"]);
        assert_ne!(inherited["anchor"], original["anchor"]);
        for (mode, name, sections) in [
            (HeaderMode::Shared, "shared", json!([0, 1])),
            (HeaderMode::SectionLocal, "section-local", json!([1])),
        ] {
            let (bytes, changes) = snapshot
                .preview(vec![Edit::HeaderImage {
                    anchor: anchor(&inherited["anchor"]),
                    mode,
                    bytes: IMAGE.to_vec(),
                }])
                .unwrap();
            let change = &changes["header_images"][0];
            assert_eq!(change["mode"], name);
            assert_eq!(change["affected_sections"], sections);
            let reopened = open(&bytes).inspect().unwrap();
            assert_eq!(reopened["headers"][3]["part_name"], change["output_part"]);
            assert_eq!(reopened["headers"][3]["shared"], name == "shared");
        }
    }
}
