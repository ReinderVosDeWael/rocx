//! JSON boundary only: resolve declarations and bind addresses to this open snapshot.
use rocx::editor::*;
use rocx::{
    Alignment, Component, HeaderFooterType, ImageRun, Paragraph, Table, TableCell, TableRow,
    VerticalAlignment,
};
use serde::Deserialize;
use serde_json::{Value, json};
mod declarations;
mod discovery;
mod resources;
mod scoped;
use declarations::*;
use resources::*;
use scoped::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AlignmentPayload {
    target: usize,
    alignment: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RunAddress {
    target: usize,
    inline_index: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HeaderAddress {
    section_index: usize,
    variant: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum Edit {
    Alignment(AlignmentPayload),
    InsertParagraph {
        target: usize,
        text: String,
    },
    InsertTable {
        target: usize,
        rows: Vec<Vec<String>>,
    },
    HeaderImage {
        target: HeaderAddress,
        mode: String,
        bytes: Vec<u8>,
    },
    RunText {
        target: usize,
        inline_index: usize,
        text: String,
    },
    RunRemove(RunAddress),
    CellAlignment {
        target: usize,
        row_index: usize,
        cell_index: usize,
        alignment: String,
    },
    TextRange {
        target: usize,
        start: usize,
        end: usize,
        text: String,
    },
    TextInsertion {
        target: usize,
        position: usize,
        text: String,
    },
    ReplaceParagraph {
        target: usize,
        text: String,
    },
    RemoveParagraph {
        target: usize,
    },
    ReplaceSection {
        target: usize,
        blocks: Vec<Block>,
    },
    KeepWithNext {
        target: usize,
        value: bool,
    },
    ParagraphFlag {
        target: usize,
        flag: String,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<bool>,
    },
    SectionFlag {
        target: usize,
        flag: String,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<bool>,
    },
    SectionType {
        target: usize,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<String>,
    },
    InsertInline {
        target: usize,
        inline_index: usize,
        inline: Inline,
    },
    ReplaceInline {
        target: usize,
        inline_index: usize,
        inline: Inline,
    },
    RunBold {
        target: usize,
        inline_index: usize,
        value: bool,
    },
    TableStructure {
        target: usize,
        operation: Geometry,
    },
    CoreTitle {
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<String>,
    },
    CoreSubject {
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<String>,
    },
    Metadata {
        field: String,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<String>,
    },
    CustomProperty {
        name: String,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<CustomScalar>,
    },
    DefaultAlignment {
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<String>,
    },
    AbstractStart {
        abstract_id: u32,
        level: u8,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<u32>,
    },
    DocumentDefault {
        field: String,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<DefaultScalar>,
    },
    AddAbstract {
        abstract_id: u32,
        levels: Vec<Level>,
    },
    RemoveAbstract {
        abstract_id: u32,
    },
    AddInstance {
        num_id: u32,
        abstract_id: u32,
    },
    RemoveInstance {
        num_id: u32,
    },
    InstanceStart {
        num_id: u32,
        level: u8,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<u32>,
    },
    InstanceLevel {
        num_id: u32,
        level: u8,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<Level>,
    },
    ParagraphNumbering {
        target: usize,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<ListRef>,
    },
    StyleField {
        id: String,
        region: Option<String>,
        field: String,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<String>,
    },
    StyleRegion {
        id: String,
        region: String,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        value: Option<Region>,
    },
    StyleAdd {
        definition: Style,
    },
    StyleReplace {
        id: String,
        definition: Style,
    },
    StylePatch {
        id: String,
        bold: Option<bool>,
        italic: Option<bool>,
    },
    StyleRemove {
        id: String,
    },
    Control {
        target: usize,
        tag: Option<String>,
        alias: Option<String>,
        text: Option<String>,
    },
    StoryImage {
        target: Slot,
        mode: String,
        bytes: Vec<u8>,
    },
    StoryText {
        part: String,
        runs: Vec<StoryRun>,
    },
    TrackedText {
        target: RunAddress,
        action: Tracking,
        author: String,
        date: Option<String>,
    },
    TrackedMove {
        target: RunAddress,
        after: RunAddress,
        name: String,
        author: String,
        date: String,
    },
    ReviewRevision {
        target: RunAddress,
        action: Review,
        paired: Option<RunAddress>,
    },
    #[serde(untagged)]
    LegacyAlignment(AlignmentPayload),
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum CustomScalar {
    Text(String),
    Boolean(bool),
}

fn section_type(value: &str) -> Result<rocx::SectionType, String> {
    use rocx::SectionType::*;
    match value {
        "nextPage" => Ok(NextPage),
        "nextColumn" => Ok(NextColumn),
        "evenPage" => Ok(EvenPage),
        "oddPage" => Ok(OddPage),
        "continuous" => Ok(Continuous),
        _ => Err("unknown section type".into()),
    }
}

fn alignment(value: &str) -> Result<Alignment, String> {
    match value {
        "left" => Ok(Alignment::Left),
        "center" => Ok(Alignment::Center),
        "right" => Ok(Alignment::Right),
        "justify" => Ok(Alignment::Justify),
        "distribute" => Ok(Alignment::Distribute),
        "justify-medium" => Ok(Alignment::JustifyMedium),
        "justify-high" => Ok(Alignment::JustifyHigh),
        "justify-low" => Ok(Alignment::JustifyLow),
        "thai-justify" => Ok(Alignment::ThaiJustify),
        _ => Err(format!("unknown alignment: {value}")),
    }
}

fn inspect_block((index, block): (usize, &BodyBlock)) -> Value {
    match block {
        BodyBlock::Paragraph(p) => {
            let runs: Vec<_> = p
                .inline_structure
                .iter()
                .enumerate()
                .filter_map(|(i, n)| {
                    let ImportedInlineNode::Run(run) = n else {
                        return None;
                    };
                    let text = match run.items.as_slice() {
                        [ImportedInline::Text(s)] => Some(s),
                        _ => None,
                    };
                    Some(json!({"inline_index":i, "text":text}))
                })
                .collect();
            json!({"type":"paragraph","target":index,"text":p.text,"alignment_xml":p.alignment_xml,
                   "flags": p.flags.iter().map(|(f,v)|(f.python_name(),v)).collect::<std::collections::BTreeMap<_,_>>(),
                   "has_opaque_content":p.has_opaque_content,"runs":runs,"inlines":discovery::inlines(&p.inline_structure)})
        }
        BodyBlock::Table(t) => {
            let mut cells = Vec::new();
            for (r, row) in t.children.iter().enumerate() {
                if let ImportedTableChild::Row(row) = row {
                    for (c, child) in row.children.iter().enumerate() {
                        if matches!(child, ImportedRowChild::Cell(_)) {
                            cells.push(json!({"row_index":r,"cell_index":c}));
                        }
                    }
                }
            }
            json!({"type":"table","index":index,"target":index,"cells":cells,"grid_widths":t.grid_widths,
                "layout":t.layout.map(|l|match l {rocx::TableLayout::Autofit=>"autofit",rocx::TableLayout::Fixed=>"fixed"})})
        }
        BodyBlock::ContentControl(c) => json!({"type":"content-control","index":index,
            "tag":c.tag,"alias":c.alias,"lock":c.lock}),
        BodyBlock::Opaque(name) => json!({"type":"opaque","index":index,"name":name}),
        _ => json!({"type":"unknown","index":index}),
    }
}

pub(super) fn inspect(
    doc: &ImportedDocument,
    snapshot: String,
    fields: Vec<String>,
    names: Vec<String>,
    numbering_targets: Vec<usize>,
) -> Result<Value, String> {
    let body: Vec<_> = doc.body().iter().enumerate().map(inspect_block).collect();
    let mut result = json!({"snapshot":snapshot,"body":body});
    discovery::extend(doc, &mut result);
    for target in numbering_targets {
        let reference = doc
            .paragraph_numbering(&paragraph(doc, target)?.anchor)
            .map_err(|e| e.to_string())?;
        result["paragraph_numbering"][target.to_string()] =
            json!(reference.map(|r| json!({"id":r.id,"level":r.level})));
    }
    for field in fields {
        let key = field.parse().map_err(|e: rocx::Error| e.to_string())?;
        result["metadata"][field] = json!(doc.metadata_field(key).map_err(|e| e.to_string())?);
    }
    for name in names {
        let value = doc.custom_property(&name).map_err(|e| e.to_string())?;
        result["custom_properties"][name] = match value {
            Some(CustomValue::Text(s)) => json!(s),
            Some(CustomValue::Boolean(b)) => json!(b),
            None => Value::Null,
        };
    }
    Ok(result)
}

fn paragraph(doc: &ImportedDocument, index: usize) -> Result<&ImportedParagraph, String> {
    match doc.body().get(index) {
        Some(BodyBlock::Paragraph(p)) => Ok(p),
        _ => Err(format!("target {index} is not a direct body paragraph")),
    }
}

fn run(doc: &ImportedDocument, target: usize, index: usize) -> Result<InlineAnchor, String> {
    match paragraph(doc, target)?.inline_structure.get(index) {
        Some(ImportedInlineNode::Run(r)) => Ok(r.anchor.clone()),
        _ => Err(format!("target {target}:{index} is not a direct body run")),
    }
}

fn resolve<C: Component>(c: C) -> Result<C::Resolved, String> {
    futures_executor::block_on(c.resolve())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "declaration produced no content".into())
}

pub(super) fn preview(
    doc: &ImportedDocument,
    edits: Vec<Edit>,
    snapshot: String,
) -> Result<(Vec<u8>, Value), String> {
    let (mut align, mut tables, mut headers, mut texts, mut paragraphs, mut cells, mut removals) =
        (vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    let mut stories = std::cell::OnceCell::new();
    let mut points = Vec::new();
    let (
        mut ranges,
        mut blocks,
        mut inline_edits,
        mut structural,
        mut styles,
        mut resources,
        mut controls,
    ) = (vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    let count = edits.len();
    for edit in edits {
        match edit {
            e @ (Edit::StoryText { .. }
            | Edit::TrackedText { .. }
            | Edit::TrackedMove { .. }
            | Edit::ReviewRevision { .. }) => {
                if count != 1 {
                    return Err("story text or revision operation must be the only edit".into());
                }
                return scoped::preview_scoped(doc, e, snapshot);
            }
            Edit::TextRange {
                target,
                start,
                end,
                text,
            } => ranges.push(TextRangeEdit {
                target: paragraph(doc, target)?.anchor.clone(),
                start,
                end,
                text,
            }),
            Edit::TextInsertion {
                target,
                position,
                text,
            } => points.push(TextInsertion {
                target: paragraph(doc, target)?.anchor.clone(),
                position,
                text,
            }),
            Edit::ReplaceParagraph { target, text } => blocks.push(BlockEdit::Replace(
                paragraph(doc, target)?.anchor.clone(),
                Box::new(resolve(Paragraph::new(text))?),
            )),
            Edit::RemoveParagraph { target } => {
                blocks.push(BlockEdit::Remove(paragraph(doc, target)?.anchor.clone()))
            }
            Edit::ReplaceSection {
                target,
                blocks: payload,
            } => blocks.push(BlockEdit::ReplaceSection(
                paragraph(doc, target)?.anchor.clone(),
                resolve_blocks(payload)?,
            )),
            Edit::KeepWithNext { target, value } => blocks.push(BlockEdit::KeepWithNext(
                paragraph(doc, target)?.anchor.clone(),
                value,
            )),
            Edit::ParagraphFlag {
                target,
                flag,
                value,
            } => blocks.push(BlockEdit::ParagraphFlag(
                paragraph(doc, target)?.anchor.clone(),
                *ParagraphFlag::ALL
                    .iter()
                    .find(|f| f.python_name() == flag)
                    .ok_or("unknown paragraph flag")?,
                value,
            )),
            Edit::SectionFlag {
                target,
                flag,
                value,
            } => blocks.push(BlockEdit::SectionFlag(
                paragraph(doc, target)?.anchor.clone(),
                *SectionFlag::ALL
                    .iter()
                    .find(|f| f.python_name() == flag)
                    .ok_or("unknown section flag")?,
                value,
            )),
            Edit::SectionType { target, value } => blocks.push(BlockEdit::SectionType(
                paragraph(doc, target)?.anchor.clone(),
                value.as_deref().map(section_type).transpose()?,
            )),
            Edit::InsertInline {
                target,
                inline_index,
                inline,
            } => inline_edits.push(InlineEdit {
                target: inline_anchor(doc, target, inline_index)?,
                action: InlineAction::InsertAfter(inline.resolve()?),
            }),
            Edit::ReplaceInline {
                target,
                inline_index,
                inline,
            } => inline_edits.push(InlineEdit {
                target: inline_anchor(doc, target, inline_index)?,
                action: InlineAction::Replace(inline.resolve()?),
            }),
            Edit::RunBold {
                target,
                inline_index,
                value,
            } => inline_edits.push(InlineEdit {
                target: run(doc, target, inline_index)?,
                action: InlineAction::SetBold(value),
            }),
            Edit::TableStructure { target, operation } => {
                let Some(BodyBlock::Table(t)) = doc.body().get(target) else {
                    return Err("expected direct body table".into());
                };
                structural.push(TableStructureEdit {
                    target: t.anchor.clone(),
                    operation: operation.resolve()?,
                });
            }
            Edit::CoreTitle { value } => resources.push(ResourceEdit::CoreTitle(value)),
            Edit::CoreSubject { value } => resources.push(ResourceEdit::CoreSubject(value)),
            Edit::Metadata { field, value } => resources.push(ResourceEdit::Metadata(
                field.parse().map_err(|e: rocx::Error| e.to_string())?,
                value,
            )),
            Edit::CustomProperty { name, value } => resources.push(ResourceEdit::CustomProperty(
                name,
                value.map(|v| match v {
                    CustomScalar::Text(s) => CustomValue::Text(s),
                    CustomScalar::Boolean(b) => CustomValue::Boolean(b),
                }),
            )),
            Edit::DefaultAlignment { value } => resources.push(ResourceEdit::DefaultAlignment(
                value.as_deref().map(alignment).transpose()?,
            )),
            Edit::AbstractStart {
                abstract_id,
                level,
                value,
            } => resources.push(ResourceEdit::AbstractStart(AbstractStart {
                abstract_id,
                level,
                start: value,
            })),
            Edit::StyleAdd { definition } => styles.push(StyleEdit::Add(definition.into())),
            Edit::StyleField {
                id,
                region,
                field,
                value,
            } => styles.push(StyleEdit::Field {
                target: doc.style_anchor(&id).map_err(|e| e.to_string())?,
                region,
                field,
                value,
            }),
            Edit::StyleRegion { id, region, value } => styles.push(StyleEdit::Region {
                target: doc.style_anchor(&id).map_err(|e| e.to_string())?,
                region,
                format: value.map(Region::resolve).transpose()?,
            }),
            Edit::DocumentDefault { field, value } => resources.push(ResourceEdit::Default(
                DefaultField::from_key(&field).ok_or("unknown document default")?,
                value.map(Into::into),
            )),
            Edit::AddAbstract {
                abstract_id,
                levels,
            } => resources.push(ResourceEdit::Numbering(NumberingEdit::AddAbstract {
                abstract_id,
                levels: levels.into_iter().map(Into::into).collect(),
            })),
            Edit::RemoveAbstract { abstract_id } => {
                resources.push(ResourceEdit::Numbering(NumberingEdit::RemoveAbstract {
                    abstract_id,
                }))
            }
            Edit::AddInstance {
                num_id,
                abstract_id,
            } => resources.push(ResourceEdit::Numbering(NumberingEdit::AddInstance {
                num_id,
                abstract_id,
            })),
            Edit::RemoveInstance { num_id } => {
                resources.push(ResourceEdit::Numbering(NumberingEdit::RemoveInstance {
                    num_id,
                }))
            }
            Edit::InstanceStart {
                num_id,
                level,
                value,
            } => resources.push(ResourceEdit::Numbering(NumberingEdit::InstanceStart {
                num_id,
                level,
                start: value,
            })),
            Edit::InstanceLevel {
                num_id,
                level,
                value,
            } => resources.push(ResourceEdit::Numbering(NumberingEdit::InstanceLevel {
                num_id,
                level,
                value: value.map(|l| Box::new(l.into())),
            })),
            Edit::ParagraphNumbering { target, value } => {
                resources.push(ResourceEdit::Numbering(NumberingEdit::Paragraph {
                    target: paragraph(doc, target)?.anchor.clone(),
                    value: value.map(|r| rocx::NumberingReference {
                        id: r.id,
                        level: r.level,
                    }),
                }))
            }
            Edit::StyleReplace { id, definition } => styles.push(StyleEdit::Replace {
                target: doc.style_anchor(&id).map_err(|e| e.to_string())?,
                definition: definition.into(),
            }),
            Edit::StyleRemove { id } => styles.push(StyleEdit::Remove(
                doc.style_anchor(&id).map_err(|e| e.to_string())?,
            )),
            Edit::StylePatch { id, bold, italic } => styles.push(StyleEdit::Patch {
                target: doc.style_anchor(&id).map_err(|e| e.to_string())?,
                bold,
                italic,
            }),
            Edit::Control {
                target,
                tag,
                alias,
                text,
            } => controls.push(ControlEdit {
                target: doc
                    .controls()
                    .get(target)
                    .ok_or("unknown control index")?
                    .anchor
                    .clone(),
                tag,
                alias,
                text,
            }),
            Edit::StoryImage {
                target,
                mode,
                bytes,
            } => headers.push(target.image(doc, &mut stories, &mode, bytes)?),
            Edit::LegacyAlignment(e) | Edit::Alignment(e) => align.push(ParagraphAlignmentEdit {
                target: paragraph(doc, e.target)?.anchor.clone(),
                alignment: alignment(&e.alignment)?,
            }),
            Edit::RunText {
                target,
                inline_index,
                text,
            } => texts.push(RunTextEdit {
                target: run(doc, target, inline_index)?,
                text,
            }),
            Edit::RunRemove(e) => removals.push(run(doc, e.target, e.inline_index)?),
            Edit::InsertParagraph { target, text } => paragraphs.push(ParagraphInsertion {
                target: paragraph(doc, target)?.anchor.clone(),
                paragraph: resolve(Paragraph::new(text))?,
            }),
            Edit::InsertTable { target, rows } => tables.push(TableInsertion {
                target: paragraph(doc, target)?.anchor.clone(),
                table: table(rows)?,
            }),
            Edit::HeaderImage {
                target,
                mode,
                bytes,
            } => {
                if !matches!(target.variant.as_str(), "default" | "first") {
                    return Err("unknown header variant".into());
                }
                headers.push(Slot::legacy(target).image(doc, &mut stories, &mode, bytes)?);
            }
            Edit::CellAlignment {
                target,
                row_index,
                cell_index,
                alignment,
            } => {
                let Some(BodyBlock::Table(table)) = doc.body().get(target) else {
                    return Err(format!("target {target} is not a direct body table"));
                };
                let alignment = match alignment.as_str() {
                    "top" => VerticalAlignment::Top,
                    "center" => VerticalAlignment::Center,
                    "bottom" => VerticalAlignment::Bottom,
                    "both" => VerticalAlignment::Both,
                    _ => return Err("unknown vertical alignment".into()),
                };
                cells.push(CellAlignmentEdit {
                    target: table.anchor.clone(),
                    row_index,
                    cell_index,
                    alignment,
                });
            }
        }
    }
    let preview = doc
        .preview_edits(EditBatch {
            alignments: &align,
            tables: &tables,
            header_images: &headers,
            run_text: &texts,
            paragraphs: &paragraphs,
            cells: &cells,
            run_removals: &removals,
            ranges: &ranges,
            text_insertions: &points,
            blocks: &blocks,
            inline_edits: &inline_edits,
            structural: &structural,
            styles: &styles,
            resources: &resources,
            controls: &controls,
            ..Default::default()
        })
        .map_err(|e| format!("preview: {e}"))?;
    Ok((
        preview.to_docx().map_err(|e| e.to_string())?,
        report(&preview, snapshot),
    ))
}

pub(super) fn report(p: &EditPreview, snapshot: String) -> Value {
    fn records<T>(items: &[T], record: impl Fn(&T) -> Value) -> Vec<Value> {
        items.iter().map(record).collect()
    }
    let headers = records(p.header_changes(), |c| {
        let mode = match c.mode {
            HeaderEditMode::Shared => "shared",
            HeaderEditMode::SectionLocal => "section-local",
        };
        json!({"source_part": c.source_part, "output_part": c.output_part, "mode": mode,
               "affected_sections": c.affected_sections, "restored_section": c.restored_section})
    });
    json!({
        "snapshot": snapshot,
        "changes": records(p.changes(), |c| json!({
            "target": c.body_index, "before": c.before, "after": format!("{:?}", c.after)
        })),
        "table_insertions": records(p.insertions(), |c| json!({
            "target": c.after_body_index, "rows": c.rows, "columns": c.columns
        })),
        "paragraph_insertions": p.paragraph_insertions(),
        "block_changes": p.block_changes(),
        "inline_changes": p.inline_changes(),
        "header_images": headers,
        "run_text": records(p.run_text_changes(), |c| json!({
            "target": c.body_index, "inline_index": c.inline_index, "before": c.before, "after": c.after
        })),
        "run_removals": records(p.run_removals(), |c| json!({
            "target": c.body_index, "inline_index": c.inline_index, "before": c.before
        })),
        "cell_alignments": records(p.cell_alignment_changes(), |c| json!({
            "target": c.body_index, "row_index": c.row_index, "cell_index": c.cell_index,
            "before": c.before, "after": format!("{:?}", c.after).to_ascii_lowercase()
        })),
    })
}
