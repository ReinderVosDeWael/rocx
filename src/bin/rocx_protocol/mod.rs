//! JSON boundary only: resolve declarations and bind addresses to this open snapshot.
use rocx::editor::*;
use rocx::{
    Alignment, Component, HeaderFooterType, ImageRun, Paragraph, Table, TableCell, TableRow,
    VerticalAlignment,
};
use serde::Deserialize;
use serde_json::{Value, json};

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
    #[serde(untagged)]
    LegacyAlignment(AlignmentPayload),
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
                   "has_opaque_content":p.has_opaque_content,"runs":runs})
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
            json!({"type":"table","index":index,"target":index,"cells":cells})
        }
        BodyBlock::ContentControl(c) => json!({"type":"content-control","index":index,
            "tag":c.tag,"alias":c.alias,"lock":c.lock}),
        BodyBlock::Opaque(name) => json!({"type":"opaque","index":index,"name":name}),
        _ => json!({"type":"unknown","index":index}),
    }
}

pub(super) fn inspect(doc: &ImportedDocument, snapshot: String) -> Value {
    let body: Vec<_> = doc.body().iter().enumerate().map(inspect_block).collect();
    let mut result = json!({"snapshot":snapshot,"body":body});
    match doc.stories() {
        Ok(stories) => {
            result["headers"] = stories
                .iter()
                .filter(|s| s.anchor.is_some())
                .map(|s| {
                    let variant = match s.variant {
                        HeaderFooterType::Default => "default",
                        HeaderFooterType::First => "first",
                        HeaderFooterType::Even => "even",
                    };
                    json!({"target":{"section_index":s.section_index,"variant":variant},
                       "part":s.part_name, "inherited":s.inherited})
                })
                .collect();
        }
        Err(e) => result["headers_error"] = json!(e.to_string()),
    }
    result
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

pub(super) fn preview(doc: &ImportedDocument, edits: Vec<Edit>) -> Result<EditPreview, String> {
    let (mut align, mut tables, mut headers, mut texts, mut paragraphs, mut cells, mut removals) =
        (vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    let mut stories = None;
    for edit in edits {
        match edit {
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
            Edit::InsertTable { target, rows } => {
                let width = rows.first().map_or(0, Vec::len);
                if width == 0 || rows.iter().any(|r| r.len() != width) {
                    return Err("rows must form a nonempty rectangular table".into());
                }
                let mut result = Vec::new();
                for row in rows {
                    let mut cells = Vec::new();
                    for text in row {
                        cells.push(TableCell::new(vec![Paragraph::new(text).into()]).into());
                    }
                    result.push(TableRow::new(cells).into());
                }
                tables.push(TableInsertion {
                    target: paragraph(doc, target)?.anchor.clone(),
                    table: resolve(Table::new(result))?,
                })
            }
            Edit::HeaderImage {
                target,
                mode,
                bytes,
            } => {
                let variant = match target.variant.as_str() {
                    "default" => HeaderFooterType::Default,
                    "first" => HeaderFooterType::First,
                    _ => return Err("unknown header variant".into()),
                };
                let mode = match mode.as_str() {
                    "shared" => HeaderEditMode::Shared,
                    "section-local" => HeaderEditMode::SectionLocal,
                    _ => return Err("unknown header edit mode".into()),
                };
                if stories.is_none() {
                    stories = Some(doc.stories().map_err(|e| e.to_string())?);
                }
                let anchor = stories
                    .iter()
                    .flatten()
                    .filter(|s| s.section_index == target.section_index && s.variant == variant)
                    .find_map(|s| s.anchor.clone())
                    .ok_or("missing header target")?;
                let inline = resolve(ImageRun::new(bytes))?;
                headers.push(HeaderImageEdit {
                    target: anchor,
                    mode,
                    image: inline.image().ok_or("expected image")?.clone(),
                });
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
    doc.preview_mixed(
        &align,
        &tables,
        &headers,
        &texts,
        &paragraphs,
        &cells,
        &removals,
    )
    .map_err(|e| format!("preview: {e}"))
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
