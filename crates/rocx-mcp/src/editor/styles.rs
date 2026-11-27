use std::collections::BTreeMap;

use rocx::editor::{ImportedStyles, StyleEdit, StyleKind};
use serde_json::{Value, json};

use super::Snapshot;
use crate::schema::styles::StyleChange;

const PREFIX: &str = "style:";

impl Snapshot {
    fn styles(&self) -> Result<Option<&ImportedStyles>, String> {
        self.styles
            .get_or_init(|| self.document.styles().map_err(|e| e.to_string()))
            .as_ref()
            .map(Option::as_ref)
            .map_err(Clone::clone)
    }

    pub fn styles_part_name(&self) -> Result<Option<&str>, String> {
        Ok(self.styles()?.map(|catalog| catalog.part_name.as_str()))
    }

    pub fn style_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        let Some(catalog) = self.styles()? else {
            return Ok((vec![], None));
        };
        let mut counts = BTreeMap::new();
        for style in &catalog.styles {
            if let Some(id) = style.id.as_deref() {
                *counts.entry(id).or_insert(0usize) += 1;
            }
        }
        let rows: Vec<_> = catalog.styles.iter().skip(start).take(limit).map(|style| {
            let unsupported_kind = match &style.kind {
                StyleKind::Unsupported(value) => Some(value),
                _ => None,
            };
            let mut row = json!({
                "id": style.id, "kind": style.kind.as_str(), "unsupported_kind": unsupported_kind,
                "name": style.name, "based_on": style.based_on, "next_style": style.next_style,
                "linked_style": style.linked_style, "property_xml": style.property_xml,
            });
            if let Some(id) = style.id.as_deref().filter(|id| counts.get(id) == Some(&1)) {
                row["anchor"] = json!(format!("{PREFIX}{id}"));
            }
            row
        }).collect();
        let end = start + rows.len();
        Ok((rows, (end < catalog.styles.len()).then_some(end)))
    }

    pub(super) fn style_edit(&self, change: StyleChange) -> Result<StyleEdit, String> {
        let anchor = |local: &str| {
            let id = local
                .strip_prefix(PREFIX)
                .ok_or("wrong_anchor_type: expected a style anchor")?;
            self.document.style_anchor(id).map_err(|e| e.to_string())
        };
        Ok(match change {
            StyleChange::Add { definition } => StyleEdit::Add(definition.resolve()?),
            StyleChange::Replace {
                anchor: local,
                definition,
            } => StyleEdit::Replace {
                target: anchor(&local)?,
                definition: definition.resolve()?,
            },
            StyleChange::Patch {
                anchor: local,
                bold,
                italic,
            } => StyleEdit::Patch {
                target: anchor(&local)?,
                bold,
                italic,
            },
            StyleChange::Field {
                anchor: local,
                region,
                field,
                value,
            } => StyleEdit::Field {
                target: anchor(&local)?,
                region: region.map(|r| r.key().into()),
                field,
                value: value.into_option(),
            },
            StyleChange::Region {
                anchor: local,
                region,
                value,
            } => StyleEdit::Region {
                target: anchor(&local)?,
                region: region.key().into(),
                format: value.into_option().map(|format| Box::new((*format).into())),
            },
            StyleChange::Remove { anchor: local } => StyleEdit::Remove(anchor(&local)?),
            StyleChange::Retarget {
                anchor: local,
                replacement,
            } => StyleEdit::Retarget {
                target: anchor(&local)?,
                replacement: anchor(&replacement)?,
            },
        })
    }
}
