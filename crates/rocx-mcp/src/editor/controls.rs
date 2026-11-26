use rocx::editor::{ControlEdit, ImportedControl, WrapperEdit};
use serde_json::{Value, json};

use super::{Batch, Snapshot};
use crate::schema::controls::ControlChange;

const CONTROL: &str = "control:";
const SITE: &str = "control-site:";

fn describe(control: &ImportedControl) -> Value {
    let choices: Vec<_> = control
        .choices
        .iter()
        .map(|choice| json!({"display_text": choice.display_text, "value": choice.value}))
        .collect();
    json!({
        "context": control.context.as_str(),
        "id": control.id, "tag": control.tag, "alias": control.alias,
        "lock": control.lock, "bound": control.bound, "kind": control.kind,
        "choices": choices, "selected": control.selected, "multi_line": control.multi_line,
        "owner_part": control.owner_part, "entry_id": control.entry_id,
        "entry_index": control.entry_index,
    })
}

fn page<T>(
    items: &[T],
    start: usize,
    limit: usize,
    row: impl Fn(usize, &T) -> Value,
) -> (Vec<Value>, Option<usize>) {
    let rows: Vec<_> = items
        .iter()
        .enumerate()
        .skip(start)
        .take(limit)
        .map(|(index, item)| row(index, item))
        .collect();
    let end = start + rows.len();
    (rows, (end < items.len()).then_some(end))
}

impl Snapshot {
    pub fn control_page(&self, start: usize, limit: usize) -> (Vec<Value>, Option<usize>) {
        page(self.document.controls(), start, limit, |i, control| {
            let mut row = describe(control);
            row["anchor"] = json!(format!("{CONTROL}{i}"));
            row
        })
    }

    pub fn control_site_page(&self, start: usize, limit: usize) -> (Vec<Value>, Option<usize>) {
        page(self.document.control_sites(), start, limit, |i, site| {
            json!({
                "anchor": format!("{SITE}{i}"), "context": site.context.as_str(), "text": site.text,
            })
        })
    }

    pub fn story_control_page(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Vec<Value>, Option<usize>), String> {
        let controls = self
            .story_controls
            .get_or_init(|| self.document.story_controls().map_err(|e| e.to_string()))
            .as_ref()
            .map_err(Clone::clone)?;
        Ok(page(controls, start, limit, |_, control| describe(control)))
    }

    pub(super) fn control_edit(
        &self,
        change: ControlChange,
        batch: &mut Batch,
    ) -> Result<(), String> {
        fn lookup<'a, T>(anchor: &str, prefix: &str, items: &'a [T]) -> Result<&'a T, String> {
            anchor
                .strip_prefix(prefix)
                .and_then(|i| i.parse::<usize>().ok())
                .and_then(|i| items.get(i))
                .ok_or_else(|| {
                    "wrong_anchor_type: expected a control or control-site anchor".into()
                })
        }
        let target = |anchor: &str| {
            lookup(anchor, CONTROL, self.document.controls()).map(|control| control.anchor.clone())
        };
        let site = |anchor: &str| lookup(anchor, SITE, self.document.control_sites()).cloned();
        let wrapper = match change {
            ControlChange::Patch {
                anchor,
                tag,
                alias,
                text,
            } => {
                batch.controls.push(ControlEdit {
                    target: target(&anchor)?,
                    tag,
                    alias,
                    text,
                });
                return Ok(());
            }
            ControlChange::Wrap { first, last, tag } => WrapperEdit::Wrap {
                first: site(&first)?,
                last: last.as_deref().map(site).transpose()?,
                tag,
            },
            ControlChange::Unwrap { anchor } => WrapperEdit::Unwrap(target(&anchor)?),
            ControlChange::Define {
                anchor,
                text,
                id,
                tag,
                alias,
                lock,
            } => WrapperEdit::Define(target(&anchor)?, text.properties(id, tag, alias, lock)),
            ControlChange::Choice {
                anchor,
                selected,
                items,
            } => WrapperEdit::Choice {
                target: target(&anchor)?,
                selected,
                items: items.map(|items| items.into_iter().map(Into::into).collect()),
            },
            ControlChange::ReplaceInlines { anchor, inlines } => WrapperEdit::ReplaceInlines(
                target(&anchor)?,
                inlines
                    .into_iter()
                    .map(|inline| inline.resolve())
                    .collect::<Result<_, _>>()?,
            ),
            ControlChange::ReplaceBlocks { anchor, blocks } => WrapperEdit::ReplaceBlocks(
                target(&anchor)?,
                blocks
                    .into_iter()
                    .map(|block| block.resolve())
                    .collect::<Result<_, _>>()?,
            ),
        };
        batch.wrappers.push(wrapper);
        Ok(())
    }
}
