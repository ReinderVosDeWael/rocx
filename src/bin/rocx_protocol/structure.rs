//! Snapshot-bound wire adapters for layout and physical-owner structure.
use super::*;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PhysicalOperation {
    path: Vec<usize>,
    operation: Geometry,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum ReferenceAction {
    Create { blocks: Vec<Block> },
    Link { part: String },
    Remove {},
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum LayoutInput {
    Integer(i32),
    Token(String),
    Boolean(bool),
    Tabs(Vec<TabInput>),
    Border(BorderInput),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TabInput {
    position: i32,
    alignment: String,
    leader: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BorderInput {
    size: Option<i32>,
    space: Option<i32>,
    color: Option<String>,
}

impl From<LayoutInput> for LayoutValue {
    fn from(value: LayoutInput) -> Self {
        match value {
            LayoutInput::Integer(v) => Self::Integer(v),
            LayoutInput::Token(v) => Self::Token(v),
            LayoutInput::Boolean(v) => Self::Boolean(v),
            LayoutInput::Tabs(v) => Self::Tabs(
                v.into_iter()
                    .map(|t| LayoutTab {
                        position: t.position,
                        alignment: t.alignment,
                        leader: t.leader,
                    })
                    .collect(),
            ),
            LayoutInput::Border(v) => Self::Border(LayoutBorder {
                size: v.size,
                space: v.space,
                color: v.color,
            }),
        }
    }
}

pub(super) fn layout_view(view: &BTreeMap<LayoutField, LayoutValue>) -> Value {
    view.iter()
        .map(|(field, value)| {
            (
                field.python_name(),
                match value {
                    LayoutValue::Integer(v) => json!(v),
                    LayoutValue::Token(v) => json!(v),
                    LayoutValue::Boolean(v) => json!(v),
                    LayoutValue::Tabs(v) => json!(
                        v.iter()
                            .map(|v| json!({
                                "position":v.position,"alignment":v.alignment,"leader":v.leader
                            }))
                            .collect::<Vec<_>>()
                    ),
                    LayoutValue::Border(v) => {
                        json!({"size":v.size,"space":v.space,"color":v.color})
                    }
                },
            )
        })
        .collect::<serde_json::Map<String, Value>>()
        .into()
}

pub(super) fn section_anchor(
    doc: &ImportedDocument,
    index: usize,
) -> Result<SectionAnchor, String> {
    doc.sections()
        .map_err(|e| e.to_string())?
        .into_iter()
        .nth(index)
        .map(|s| s.anchor)
        .ok_or_else(|| "section target out of bounds".into())
}

impl Edit {
    pub(super) fn is_direct_layout(&self) -> bool {
        matches!(
            self,
            Self::ParagraphLayout { .. } | Self::SectionLayout { .. }
        )
    }
}

pub(super) fn preview_layout(
    doc: &ImportedDocument,
    edits: Vec<Edit>,
    snapshot: String,
) -> Result<(Vec<u8>, Value), String> {
    let field = |name: &str| {
        LayoutField::ALL
            .iter()
            .copied()
            .find(|f| f.python_name() == name)
            .ok_or_else(|| format!("unknown layout field: {name}"))
    };
    let edits = edits
        .into_iter()
        .map(|e| match e {
            Edit::ParagraphLayout {
                target,
                field: name,
                value,
            } => Ok(LayoutEdit::Paragraph(
                paragraph(doc, target)?.anchor.clone(),
                field(&name)?,
                value.map(Into::into),
            )),
            Edit::SectionLayout {
                target,
                field: name,
                value,
            } => Ok(LayoutEdit::Section(
                section_anchor(doc, target)?,
                field(&name)?,
                value.map(Into::into),
            )),
            _ => Err("direct layout edits cannot be mixed with other operations".into()),
        })
        .collect::<Result<Vec<_>, String>>()?;
    let preview = doc.preview_layout(&edits).map_err(|e| e.to_string())?;
    Ok((
        preview.to_docx().map_err(|e| e.to_string())?,
        report(&preview, snapshot),
    ))
}

pub(super) fn discover_tables(doc: &ImportedDocument, owner: Option<&str>, result: &mut Value) {
    match doc.part_tables(owner) {
        Ok(tables) => {
            result["part_tables"] = json!(tables.into_iter().map(|t| json!({
            "owner_part":t.part_name,"path":t.path,"depth":t.depth,"entry_id":t.entry_id,
            "properties":t.properties,"grid_widths":t.grid_widths,
            "row_properties":t.row_properties,"cell_properties":t.cell_properties
        })).collect::<Vec<_>>())
        }
        Err(e) => result["part_tables_error"] = json!(e.to_string()),
    }
}

pub(super) fn preview_physical(
    doc: &ImportedDocument,
    part: String,
    operations: Vec<PhysicalOperation>,
    snapshot: String,
) -> Result<(Vec<u8>, Value), String> {
    let known: BTreeMap<_, _> = doc
        .part_tables(Some(&part))
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| (t.path, t.anchor))
        .collect();
    let edits = operations
        .into_iter()
        .map(|e| {
            let target = known
                .get(&e.path)
                .ok_or("unknown physical table path in owner")?
                .clone();
            Ok(TableStructureEdit {
                target,
                operation: e.operation.resolve()?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let bytes = doc
        .edit_part_tables(&part, &edits)
        .map_err(|e| e.to_string())?;
    Ok((bytes, json!({"snapshot":snapshot,"table_owner":part})))
}
